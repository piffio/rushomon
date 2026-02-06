#!/bin/bash
set -e

# Create KV Namespace
# Usage: ./create-kv.sh <namespace_name> <account_id> <api_token>

KV_NAME="${1:?Namespace name required}"
ACCOUNT_ID="${2:?Account ID required}"
API_TOKEN="${3:?API token required}"

echo "Creating KV namespace: $KV_NAME"

RESPONSE=$(curl -s -X POST \
  "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/storage/kv/namespaces" \
  -H "Authorization: Bearer $API_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"title\": \"$KV_NAME\"}")

KV_ID=$(echo "$RESPONSE" | jq -r '.result.id // empty')

if [ -z "$KV_ID" ]; then
  echo "❌ Failed to create KV namespace"
  echo "Response: $RESPONSE"
  exit 1
fi

echo "✅ Created KV namespace"
echo "Namespace ID: $KV_ID"
echo "$KV_ID"
