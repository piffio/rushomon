#!/bin/bash
set -e

# Delete KV Namespace
# Usage: ./delete-kv.sh <namespace_id> <account_id> <api_token>

KV_ID="${1:?Namespace ID required}"
ACCOUNT_ID="${2:?Account ID required}"
API_TOKEN="${3:?API token required}"

echo "Deleting KV namespace: $KV_ID"

RESPONSE=$(curl -s -X DELETE \
  "https://api.cloudflare.com/client/v4/accounts/$ACCOUNT_ID/storage/kv/namespaces/$KV_ID" \
  -H "Authorization: Bearer $API_TOKEN")

SUCCESS=$(echo "$RESPONSE" | jq -r '.success // false')

if [ "$SUCCESS" = "true" ]; then
  echo "✅ Successfully deleted KV namespace"
else
  echo "❌ Failed to delete KV namespace"
  echo "Response: $RESPONSE"
  exit 1
fi
