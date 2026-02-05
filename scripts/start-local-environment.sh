#!/bin/bash

set -e

echo "🚀 Starting Rushomon with GitHub OAuth..."
echo "📍 Backend URL: http://localhost:8787"
echo "📍 Frontend URL: http://localhost:5173"
echo ""

# Check if frontend dependencies are installed
if [ ! -d "frontend/node_modules" ]; then
    echo "📦 Installing frontend dependencies..."
    cd frontend && npm install && cd ..
fi

# Build the worker first to avoid timeouts during startup
echo "🔨 Building worker..."
worker-build --release --quiet

# Start wrangler dev with local environment
echo "⚡ Starting backend..."
# Use unbuffer to preserve colors while maintaining background process
unbuffer wrangler dev --local --port 8787 --config wrangler.toml 2>&1 | tee /tmp/wrangler.log &
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
echo "Press Ctrl+C to stop both services..."

# Cleanup function
cleanup() {
    echo ""
    echo "🛑 Stopping services..."
    kill $WRANGLER_PID 2>/dev/null || true
    kill $FRONTEND_PID 2>/dev/null || true

    # Wait for processes to actually stop
    sleep 2

    # Force kill if still running
    kill -9 $WRANGLER_PID 2>/dev/null || true
    kill -9 $FRONTEND_PID 2>/dev/null || true

    echo "✅ All services stopped"
    exit 0
}

# Wait for interrupt (catch multiple signals)
trap cleanup INT TERM

# Wait for both background processes
wait $WRANGLER_PID $FRONTEND_PID
