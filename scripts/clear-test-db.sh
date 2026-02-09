#!/bin/bash

# Clear the local D1 database for fresh integration tests
# This removes all data to simulate a fresh instance

set -e

echo "🧹 Clearing local D1 database..."

# Drop and recreate the local database
echo "Dropping local database..."
wrangler d1 execute rushomon --local --command "DROP TABLE IF EXISTS users; DROP TABLE IF EXISTS organizations; DROP TABLE IF EXISTS links; DROP TABLE IF EXISTS analytics_events;" 2>/dev/null || true

# Clear migration state and re-run migrations
echo "Clearing migration state..."
rm -rf .wrangler/state/v3/d1/miniflare-D1DatabaseObject/ 2>/dev/null || true

echo "Reapplying migrations..."
wrangler d1 migrations apply rushomon --local

echo "✅ Local D1 database cleared"
echo ""
echo "Now run: ./scripts/run-integration-tests.sh"
echo "The first user created will be assigned admin role."
