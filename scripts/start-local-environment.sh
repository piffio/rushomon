#!/bin/bash

set -e

echo "🚀 Starting Rushomon with GitHub OAuth..."
echo "📍 Local URL: http://localhost:8787"
echo ""

# Start wrangler dev with local environment
wrangler dev --local --port 8787 --config wrangler.toml &
WRANGLER_PID=$!

# Wait for worker to start
echo "⏳ Waiting for worker to start..."
sleep 5

# Test health endpoint
echo "🏥 Testing health endpoint..."
curl -s http://localhost:8787/ || {
    echo "❌ Worker not responding"
    kill $WRANGLER_PID
    exit 1
}

echo "✅ Worker is running!"
echo ""
echo "🔗 OAuth Test URLs:"
echo "  Initiate OAuth: http://localhost:8787/api/auth/github"
echo "  Callback URL:   http://localhost:8787/api/auth/callback"
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
echo "Press Ctrl+C to stop the worker..."

# Wait for interrupt
trap "echo '🛑 Stopping worker...'; kill $WRANGLER_PID; exit" INT
wait $WRANGLER_PID
