#!/bin/bash

# Integration Test Runner
# This script runs integration tests using a mock OAuth server for authentication.
# It tests the full OAuth flow without requiring real GitHub credentials.
#
# Usage:
#   ./scripts/run-integration-tests.sh [OPTIONS] [-- CARGO_TEST_ARGS]
#
# Options:
#   --ci      CI mode: fresh start, stop all servers after tests
#   --fresh   Force restart servers even if already running
#   --no-test Skip running tests (just setup environment)
#
# Examples:
#   ./scripts/run-integration-tests.sh                    # Run all tests
#   ./scripts/run-integration-tests.sh --ci               # CI mode (clean start/stop)
#   ./scripts/run-integration-tests.sh --fresh            # Force restart servers
#   ./scripts/run-integration-tests.sh -- --test links    # Run only links tests

set -e  # Exit on any error

# Ensure wrangler.toml exists (copy from example if missing)
if [ ! -f "wrangler.toml" ] && [ -f "wrangler.example.toml" ]; then
    echo "📄 Creating wrangler.toml from wrangler.example.toml..."
    cp wrangler.example.toml wrangler.toml
fi

# Configuration
DB_NAME="rushomon"
KV_NAMESPACE="URL_MAPPINGS"
JWT_SECRET="test-jwt-secret-32-chars-minimum!!"
WORKER_PORT=8787
MOCK_OAUTH_PORT=9999
BASE_URL="http://localhost:${WORKER_PORT}"
MOCK_OAUTH_URL="http://localhost:${MOCK_OAUTH_PORT}"

# Parse arguments
CI_MODE=false
FRESH_START=false
RUN_TESTS=true
CARGO_TEST_ARGS=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --ci)
            CI_MODE=true
            FRESH_START=true
            shift
            ;;
        --fresh)
            FRESH_START=true
            shift
            ;;
        --no-test)
            RUN_TESTS=false
            shift
            ;;
        --)
            shift
            CARGO_TEST_ARGS="$*"
            break
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--ci] [--fresh] [--no-test] [-- CARGO_TEST_ARGS]"
            exit 1
            ;;
    esac
done

# Cleanup function - always clean up servers after tests
cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    pkill -f "wrangler dev" 2>/dev/null || true
    pkill -f "mock_oauth_server" 2>/dev/null || true
    
    # Restore .dev.vars if backup exists
    if [ -f .dev.vars.backup ]; then
        echo "🔄 Restoring original .dev.vars..."
        mv .dev.vars.backup .dev.vars
        echo "✅ .dev.vars restored"
    fi
}

# Always clean up on exit
trap cleanup EXIT

echo "🔧 Integration Test Runner"
echo "  Mode: $([ "$CI_MODE" = true ] && echo "CI" || echo "Local")"
echo "  Fresh start: $FRESH_START"
echo ""

# Step 1: Stop any existing servers if fresh start requested
if [ "$FRESH_START" = true ]; then
    echo "🔄 Stopping existing servers..."
    pkill -f "wrangler dev" 2>/dev/null || true
    pkill -f "mock_oauth_server" 2>/dev/null || true
    sleep 2
fi

# Step 2: Build the mock OAuth server
echo "🔨 Building mock OAuth server..."
cargo build --features test-utils --bin mock_oauth_server --quiet

# Step 3: Backup existing .dev.vars and create mock OAuth config
echo "🔐 Setting up mock OAuth configuration..."
if [ -f .dev.vars ]; then
    echo "� Backing up existing .dev.vars..."
    cp .dev.vars .dev.vars.backup
    echo "✅ Backup saved to .dev.vars.backup"
fi

cat > .dev.vars << EOF
JWT_SECRET=${JWT_SECRET}
GITHUB_CLIENT_ID=test-client-id
GITHUB_CLIENT_SECRET=test-client-secret
DOMAIN=localhost:${WORKER_PORT}
GITHUB_AUTHORIZE_URL=${MOCK_OAUTH_URL}/login/oauth/authorize
GITHUB_TOKEN_URL=${MOCK_OAUTH_URL}/login/oauth/access_token
GITHUB_USER_URL=${MOCK_OAUTH_URL}/api/user
EOF

# Step 4: Apply D1 migrations
echo "📦 Applying D1 migrations..."
wrangler d1 migrations apply "${DB_NAME}" --local 2>/dev/null || true

# Step 5: Start mock OAuth server
echo "� Starting mock OAuth server..."
MOCK_OAUTH_PORT=${MOCK_OAUTH_PORT} ./target/debug/mock_oauth_server > /tmp/mock-oauth.log 2>&1 &
MOCK_OAUTH_PID=$!

# Wait for mock OAuth server to be ready
echo "⏳ Waiting for mock OAuth server to start..."
for i in {1..10}; do
    if curl -s "${MOCK_OAUTH_URL}/health" > /dev/null 2>&1; then
        echo "✅ Mock OAuth server is ready (PID: $MOCK_OAUTH_PID)"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "❌ Timeout waiting for mock OAuth server to start"
        echo "Last log output:"
        tail -20 /tmp/mock-oauth.log
        exit 1
    fi
    sleep 0.5
done

# Step 6: Start wrangler dev
echo "🚀 Starting wrangler dev..."
wrangler dev --local --port ${WORKER_PORT} > /tmp/wrangler-dev.log 2>&1 &
WRANGLER_PID=$!

# Wait for wrangler to be ready
echo "⏳ Waiting for wrangler dev to start..."
for i in {1..30}; do
    if curl -s "${BASE_URL}/" > /dev/null 2>&1; then
        echo "✅ Wrangler dev is ready (PID: $WRANGLER_PID)"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Timeout waiting for wrangler dev to start"
        echo "Last log output:"
        tail -20 /tmp/wrangler-dev.log
        exit 1
    fi
    sleep 1
done

# Step 7: Perform OAuth flow to get a real JWT
echo ""
echo "🔑 Performing OAuth flow to get JWT..."

# Call /api/auth/github to initiate OAuth
# This will redirect to our mock OAuth server
AUTH_RESPONSE=$(curl -s -i "${BASE_URL}/api/auth/github" 2>&1)
REDIRECT_URL=$(echo "$AUTH_RESPONSE" | grep -i "^location:" | sed 's/location: //i' | tr -d '\r')

if [ -z "$REDIRECT_URL" ]; then
    echo "❌ Failed to get redirect URL from /api/auth/github"
    echo "Response: $AUTH_RESPONSE"
    exit 1
fi

echo "  → Redirect to mock OAuth: ${REDIRECT_URL:0:80}..."

# Extract state from the redirect URL
STATE=$(echo "$REDIRECT_URL" | grep -oE 'state=[^&]+' | cut -d= -f2)

if [ -z "$STATE" ]; then
    echo "❌ Failed to extract state from redirect URL"
    exit 1
fi

echo "  → OAuth state: ${STATE:0:20}..."

# Call the mock OAuth authorize endpoint (it will redirect back with a code)
# We just need to construct the callback URL ourselves
MOCK_CODE="mock-auth-code-${STATE}"
CALLBACK_URL="${BASE_URL}/api/auth/callback?code=${MOCK_CODE}&state=${STATE}"

echo "  → Calling callback with mock code..."

# Call the callback endpoint - this now sets httpOnly cookies instead of URL params
CALLBACK_RESPONSE=$(curl -s -i "${CALLBACK_URL}" 2>&1)

# Extract the access token JWT from the Set-Cookie header
# The backend now sets: Set-Cookie: rushomon_access={jwt}; HttpOnly; ...
JWT=$(echo "$CALLBACK_RESPONSE" | grep -i "^set-cookie: rushomon_access=" | sed 's/set-cookie: rushomon_access=//i' | cut -d';' -f1 | tr -d '\r')

if [ -z "$JWT" ]; then
    echo "❌ Failed to extract JWT from Set-Cookie header"
    echo "Response headers:"
    echo "$CALLBACK_RESPONSE" | head -20
    echo ""
    echo "Wrangler logs:"
    tail -30 /tmp/wrangler-dev.log
    exit 1
fi

echo "✅ OAuth flow complete!"
echo "  JWT: ${JWT:0:50}..."

# Clear KV store to reset rate limits for local testing
echo "🔄 Clearing KV store rate limits..."
wrangler kv key list --namespace-id "48136fe5129f406184e6956849b5280f" --local 2>/dev/null | jq -r '.[] | select(.name | startswith("ratelimit:")) | .name' | while read key_name; do
    if [ -n "$key_name" ] && [ "$key_name" != "null" ]; then
        wrangler kv key delete "$key_name" --namespace-id "48136fe5129f406184e6956849b5280f" --local 2>/dev/null || true
    fi
done
echo "✅ KV rate limits cleared..."

# Set ALL organizations to unlimited tier for reliable test execution
echo "🔄 Setting up unlimited tier test environment..."
wrangler d1 execute "${DB_NAME}" --local --command "UPDATE organizations SET tier = 'unlimited'" 2>/dev/null || true
echo "✅ All organizations set to unlimited tier"

# Export environment variables for tests
export TEST_JWT="${JWT}"
export TEST_JWT_SECRET="${JWT_SECRET}"

echo ""
echo "🚀 Environment ready!"
echo "  TEST_JWT=${TEST_JWT:0:50}..."
echo ""
echo "💡 Free tier limits tested in dedicated test!"
echo "   - Most tests run with unlimited tier for reliability"
echo "   - test_free_tier_and_unlimited_tier_limits verifies:"
echo "     1. Temporarily sets tier to free"
echo "     2. Tests 15-link limit enforcement"
echo "     3. Resets tier to unlimited for subsequent tests"
echo ""

if [ "$RUN_TESTS" = true ]; then
    echo "🧪 Running integration tests..."
    if [ -n "$CARGO_TEST_ARGS" ]; then
        cargo test --test '*' -- --test-threads=1 --nocapture $CARGO_TEST_ARGS
    else
        cargo test --test '*' -- --test-threads=1 --nocapture
    fi
    
    TEST_EXIT_CODE=$?
    
    if [ $TEST_EXIT_CODE -eq 0 ]; then
        echo ""
        echo "✅ All tests passed!"
    else
        echo ""
        echo "❌ Some tests failed (exit code: $TEST_EXIT_CODE)"
    fi
    
    # Don't exit here - let the cleanup trap handle restoration
    # Then exit with the test code
    trap - EXIT  # Remove the trap temporarily
    cleanup      # Run cleanup manually
    exit $TEST_EXIT_CODE
else
    echo "ℹ️  Skipping tests (--no-test flag)"
    echo ""
    echo "To run tests manually:"
    echo "  export TEST_JWT=\"${JWT}\""
    echo "  cargo test --test '*' -- --test-threads=1 --nocapture"
    echo ""
    echo "Servers are running. Press Ctrl+C to stop."
    wait
fi
