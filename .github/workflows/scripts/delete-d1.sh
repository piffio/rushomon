#!/bin/bash
set -e

# Delete D1 Database
# Usage: ./delete-d1.sh <database_id> <account_id> <api_token>

DB_ID="${1:?Database ID required}"
ACCOUNT_ID="${2:?Account ID required}"
API_TOKEN="${3:?API token required}"

echo "Deleting D1 database: $DB_ID"

RESPONSE=$(curl -s -X DELETE \
  "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/d1/database/$DB_ID" \
  -H "Authorization: Bearer $API_TOKEN")

SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')

if [ "$SUCCESS" = "true" ]; then
  echo "✅ Successfully deleted D1 database"
else
  echo "❌ Failed to delete D1 database"
  echo "Response: $RESPONSE"
  exit 1
fi
