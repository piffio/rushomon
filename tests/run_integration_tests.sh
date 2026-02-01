#!/bin/bash
set -e

echo "🚀 Starting Rushomon Integration Tests"
echo ""

# Check if wrangler dev is already running
if ! curl -s http://localhost:8787/ > /dev/null 2>&1; then
    echo "⚠️  Wrangler dev server not running on port 8787"
    echo "Please start it first:"
    echo "  wrangler dev --port 8787"
    exit 1
fi

echo "✓ Dev server is running"
echo ""

# Run integration tests
echo "Running integration tests..."
cargo test --test '*' -- --test-threads=1 --nocapture

echo ""
echo "✨ All tests passed!"
