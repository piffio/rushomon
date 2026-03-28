#!/bin/bash

set -e

# Parse command line arguments
ENABLE_POLAR_CLI=false
WRANGLER_ENV=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --enable-polar-cli)
            ENABLE_POLAR_CLI=true
            shift
            ;;
        --env|-e)
            WRANGLER_ENV="$2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

# Determine which vars file to update
VARS_FILE=".dev.vars"
if [ -n "$WRANGLER_ENV" ]; then
    VARS_FILE=".dev.vars.$WRANGLER_ENV"
    echo "🌟 Using environment: $WRANGLER_ENV (loading $VARS_FILE)"
fi

# Ensure wrangler.toml exists (copy from example if missing)
if [ ! -f "wrangler.toml" ] && [ -f "wrangler.example.toml" ]; then
    echo "📄 Creating wrangler.toml from wrangler.example.toml..."
    cp wrangler.example.toml wrangler.toml
fi

echo "🚀 Starting Rushomon with GitHub OAuth..."
echo "📍 Backend URL: http://localhost:8787"
echo "📍 Frontend URL: http://localhost:5173"
if [ "$ENABLE_POLAR_CLI" = true ]; then
    echo "💳 Polar CLI: ENABLED"
else
    echo "💳 Polar CLI: DISABLED (use --enable-polar-cli to enable)"
fi
echo ""

# Setup Polar CLI and webhook secret if enabled
if [ "$ENABLE_POLAR_CLI" = true ]; then
if command -v polar &> /dev/null; then
    echo "⚡ Setting up Polar CLI for webhook forwarding..."
    echo ""
    echo "📝 Polar CLI will start interactively. Please:"
    echo "   1. Select 'Sandbox' environment"
    echo "   2. Select your organization"
    echo "   3. Note the webhook secret when shown"
    echo ""
    echo "⌨️  Press Enter to continue..."
    read -r

    # Start Polar CLI in a separate terminal window
    echo "🚀 Opening Polar CLI in separate terminal window..."
    echo ""
    echo "📋 In the new terminal window:"
    echo "   1. Select 'Sandbox' environment (press Enter)"
    echo "   2. Select your organization (press Enter)"
    echo "   3. Copy the 'Secret xxxxxxxx' value"
    echo "   4. Keep that terminal open (it will handle webhooks)"
    echo ""
    echo "⌨️  Press Enter to open the new terminal..."
    read -r

    # Check if running on macOS with Terminal.app
    if command -v osascript &> /dev/null; then
        # Use AppleScript to open new terminal window with Polar CLI
        # Note: Webhooks are forwarded to backend port 8787, not frontend 5173
        osascript -e "tell application \"Terminal\" to do script \"cd '$PWD' && polar listen http://localhost:8787/api/billing/webhook\""
        echo "✅ Opened Polar CLI in new Terminal window"
    elif command -v gnome-terminal &> /dev/null; then
        # Linux with GNOME Terminal
        gnome-terminal -- bash -c "cd '$PWD' && polar listen http://localhost:8787/api/billing/webhook; exec bash"
        echo "✅ Opened Polar CLI in new terminal window"
    elif command -v xterm &> /dev/null; then
        # Fallback to xterm
        xterm -e "cd '$PWD' && polar listen http://localhost:8787/api/billing/webhook" &
        echo "✅ Opened Polar CLI in new terminal window"
    else
        # Fallback: run in background with instructions
        echo "⚠️  Cannot open new terminal window automatically"
        echo "   Please run this manually in a separate terminal:"
        echo "   cd '$PWD' && polar listen http://localhost:8787/api/billing/webhook"
        echo ""
        echo "⌨️  Press Enter after you've started Polar CLI manually..."
        read -r
    fi

    echo ""
    echo "⏳ Waiting for you to complete Polar CLI setup in the other window..."
    echo "   (Look for the 'Secret xxxxxxxx' line in the other terminal)"
    echo ""
    echo "🔑 Please enter the webhook secret shown in the other terminal:"
    echo "   (It should be a long string like '6t3c8ce2247c493a3ade20uea4484d64')"
    echo -n "Secret: "
    read -r POLAR_SECRET

    echo "✅ Using webhook secret: ${POLAR_SECRET:0:8}..."

    # Update the correct vars file with the webhook secret
    if [ -f "$VARS_FILE" ]; then
        # Remove existing POLAR_WEBHOOK_SECRET line if present
        sed -i '' "/^POLAR_WEBHOOK_SECRET=/d" "$VARS_FILE"
    else
        # Create .dev.vars if it doesn't exist
        touch "$VARS_FILE"
    fi

    # Add the new webhook secret to .dev.vars
    echo "POLAR_WEBHOOK_SECRET=$POLAR_SECRET" >> "$VARS_FILE"
    echo "📝 Updated $VARS_FILE with webhook secret"

    # Also update frontend .env file
    if [ -f "frontend/.env" ]; then
        # Remove existing INTERNAL_WEBHOOK_SECRET line if present
        sed -i '' '/^INTERNAL_WEBHOOK_SECRET=/d' frontend/.env
        # Add the new webhook secret
        echo "INTERNAL_WEBHOOK_SECRET=$POLAR_SECRET" >> frontend/.env
        echo "📝 Updated frontend/.env with webhook secret"
    else
        echo "⚠️  frontend/.env not found - please update it manually"
    fi

    echo "✅ Polar CLI is running in separate terminal for webhook forwarding"
else
    echo "⚠️  Polar CLI not found - webhook testing unavailable"
    echo "   Install with: curl -fsSL https://polar.sh/install.sh | bash"
fi
else
    echo "⏭️  Skipping Polar CLI setup (not enabled)"
fi

echo ""

# Check if frontend dependencies are installed
if [ ! -d "frontend/node_modules" ]; then
    echo "📦 Installing frontend dependencies..."
    cd frontend && npm install && cd ..
fi

# Build the worker first to avoid timeouts during startup
echo "🔨 Building worker..."
worker-build --release --quiet

# Create R2 bucket for logos (if it doesn't exist)
echo "🪣 Setting up R2 assets bucket..."
if ! wrangler r2 bucket list | grep -q "rushomon-assets"; then
    echo "Creating R2 bucket: rushomon-assets"
    wrangler r2 bucket create rushomon-assets
else
    echo "R2 bucket 'rushomon-assets' already exists"
fi

# Apply migrations (passing environment if specified)
echo "🔨 Applying migrations..."
wrangler d1 migrations apply rushomon --local -c wrangler.toml ${WRANGLER_ENV:+--env $WRANGLER_ENV}

# Start wrangler dev with local environment
echo "⚡ Starting backend..."
# Use unbuffer to preserve colors while maintaining background process
unbuffer wrangler dev --local --port 8787 --config wrangler.toml ${WRANGLER_ENV:+--env $WRANGLER_ENV} 2>&1 | tee /tmp/wrangler.log &
WRANGLER_PID=$!

# Wait for worker to start
echo "⏳ Waiting for worker to start..."
for i in {1..30}; do
    if curl -s http://localhost:8787/ > /dev/null 2>&1; then
        echo "✅ Backend is running!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Backend failed to start"
        echo "Last log output:"
        tail -20 /tmp/wrangler.log
        kill $WRANGLER_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

# Start frontend dev server
echo "⚡ Starting frontend..."
cd frontend && unbuffer npm run dev 2>&1 | tee /tmp/frontend.log &
FRONTEND_PID=$!
cd ..

# Wait for frontend to start
echo "⏳ Waiting for frontend to start..."
for i in {1..30}; do
    if curl -s http://localhost:5173/ > /dev/null 2>&1; then
        echo "✅ Frontend is running!"
        break
    fi
    if [ $i -eq 30 ]; then
        echo "❌ Frontend failed to start"
        echo "Last log output:"
        tail -20 /tmp/frontend.log
        kill $WRANGLER_PID 2>/dev/null || true
        kill $FRONTEND_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

# Show Polar CLI status if enabled
if [ "$ENABLE_POLAR_CLI" = true ]; then
    if [ ! -z "$POLAR_SETUP_PID" ] && ps -p $POLAR_SETUP_PID > /dev/null; then
        echo "✅ Polar CLI is running for webhook testing!"
        echo ""
        echo "💳 Polar Webhook Info:"
        echo "  Webhook endpoint: http://localhost:8787/api/billing/webhook"
        echo "  Secret configured in .dev.vars"
        echo ""
    fi
fi

echo ""
echo "🔗 OAuth Test URLs:"
echo "  Initiate OAuth: http://localhost:8787/api/auth/github"
echo "  Callback URL:   http://localhost:8787/api/auth/callback"
echo ""
echo "🌐 Frontend:"
echo "  Dashboard:      http://localhost:5173"
echo "  OAuth Callback: http://localhost:5173/auth/callback"
echo ""
echo "🧪 Manual Testing Steps:"
echo "1. Visit: http://localhost:8787/api/auth/github"
echo "2. You'll be redirected to GitHub for authorization"
echo "3. After authorizing, you'll be redirected back"
echo "4. Check that you receive a session cookie"
echo ""
echo "🔍 Debug Commands:"
echo "  Check session: curl -v -b cookies.txt http://localhost:8787/api/auth/me"
echo "  Test protected: curl -v http://localhost:8787/api/links"
echo ""
echo "Press Ctrl+C to stop all services..."

# Cleanup function with port-based fallback and improved signal handling
cleanup() {
    echo ""
    echo "🛑 Stopping services..."

    # Initial graceful termination
    kill -TERM $WRANGLER_PID 2>/dev/null || true
    kill -TERM $FRONTEND_PID 2>/dev/null || true
    # Note: Polar CLI is running in separate terminal, user needs to close it manually

    # Wait for graceful shutdown
    sleep 2

    # Force kill if still running by PID
    kill -KILL $WRANGLER_PID 2>/dev/null || true
    kill -KILL $FRONTEND_PID 2>/dev/null || true
    # Note: Polar CLI is running in separate terminal, user needs to close it manually

    # Fallback: kill by port to ensure complete cleanup
    echo "🔍 Checking for remaining processes on ports..."
    lsof -ti:8787 2>/dev/null | xargs -r kill -9 2>/dev/null || true
    lsof -ti:5173 2>/dev/null | xargs -r kill -9 2>/dev/null || true

    # Final fallback: kill by process name patterns
    echo "🔍 Checking for remaining development processes..."
    pkill -f "wrangler dev" 2>/dev/null || true
    pkill -f "npm run dev" 2>/dev/null || true
    pkill -f "vite" 2>/dev/null || true
    # Note: Polar CLI is running in separate terminal, user needs to close it manually

    echo "✅ All services stopped"
    exit 0
}

# Wait for interrupt (catch more signal types for better handling)
trap cleanup INT TERM EXIT

# Wait for background processes (Polar CLI runs independently)
wait $WRANGLER_PID $FRONTEND_PID
