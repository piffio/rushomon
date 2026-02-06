#!/bin/bash
set -e

# Create D1 Database
# Usage: ./create-d1.sh <database_name> <account_id> <api_token>

DB_NAME="${1:?Database name required}"
ACCOUNT_ID="${2:?Account ID required}"
API_TOKEN="${3:?API token required}"

echo "Creating D1 database: $DB_NAME"

RESPONSE=$(curl -s -X POST \
  "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/d1/database" \
  -H "Authorization: Bearer $API_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"name\": \"$DB_NAME\"}")

DB_ID=$(echo "$RESPONSE" | jq -r '.result.uuid // .result.id // empty')

if [ -z "$DB_ID" ]; then
  echo "❌ Failed to create D1 database"
  echo "Response: $RESPONSE"
  exit 1
fi

echo "✅ Created D1 database"
echo "Database ID: $DB_ID"
echo "$DB_ID"
