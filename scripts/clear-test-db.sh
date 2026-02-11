#!/bin/bash

# Clear the local D1 database and KV namespace for fresh integration tests
# This removes all data to simulate a fresh instance

set -e

echo "🧹 Clearing local D1 database and KV namespace..."

# Clear local miniflare state (both D1 and KV)
# This is the fastest approach — wrangler dev uses .wrangler/state/v3/ for local storage
echo "Clearing local KV state (rate limits, sessions, link mappings)..."
rm -rf .wrangler/state/v3/kv/miniflare-KVNamespaceObject/ 2>/dev/null || true

echo "Clearing local D1 state..."
rm -rf .wrangler/state/v3/d1/miniflare-D1DatabaseObject/ 2>/dev/null || true

echo "Reapplying migrations..."
yes | wrangler d1 migrations apply rushomon --local

echo "✅ Local D1 database and KV namespace cleared"
echo ""
echo "Now run: ./scripts/run-integration-tests.sh"
echo "The first user created will be assigned admin role."
