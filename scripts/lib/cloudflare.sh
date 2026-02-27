#!/bin/bash
#
# cloudflare.sh - Cloudflare API wrapper functions for Rushomon
#
# This library provides:
#   - Wrangler authentication checking
#   - Account ID retrieval
#   - D1 database operations
#   - KV namespace operations
#   - Worker deployment helpers
#   - Resource management
#

# Source common utilities if not already loaded
if [ -z "$BLUE" ]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  source "$SCRIPT_DIR/common.sh"
fi

# Check if authenticated with Cloudflare
check_wrangler_auth() {
  if ! wrangler whoami &>/dev/null; then
    error "Not authenticated with Cloudflare"
    info "Run: wrangler login"
    return 1
  fi
  return 0
}

# Get Cloudflare account ID
get_account_id() {
  local account_id=$(wrangler whoami --json 2>/dev/null | jq -r '.accounts[0].id // empty')

  if [ -z "$account_id" ]; then
    error "Could not retrieve account ID"
    return 1
  fi

  echo "$account_id"
}

# List D1 databases
list_d1_databases() {
  wrangler d1 list --json 2>/dev/null | tr -d '\033' | jq -r '.[] | "\(.name) (\(.uuid))"'
}

# Check if D1 database exists
d1_database_exists() {
  local db_name="$1"

  local existing_id=$(wrangler d1 list --json 2>/dev/null | tr -d '\033' | jq -r ".[] | select(.name == \"$db_name\") | .uuid")

  if [ -n "$existing_id" ] && [ "$existing_id" != "null" ]; then
    return 0
  else
    return 1
  fi
}

# Get D1 database ID by name
get_d1_database_id() {
  local db_name="$1"

  local output=$(wrangler d1 list --json 2>/dev/null) || {
    error "Failed to list D1 databases"
    echo "$output"
    return 1
  }

  local db_id=$(echo "$output" | tr -d '\033' | jq -r ".[] | select(.name == \"$db_name\") | .uuid" 2>/dev/null) || {
    error "Failed to parse D1 database list"
    return 1
  }

  if [ -z "$db_id" ] || [ "$db_id" = "null" ]; then
    error "Database '$db_name' not found"
    return 1
  fi

  echo "$db_id"
}

# Create D1 database (or get existing one) using wrangler CLI
create_or_get_d1_database() {
  local db_name="$1"
  local binding="${2:-rushomon}"

  # Check if database already exists
  if d1_database_exists "$db_name"; then
    local existing_id=$(get_d1_database_id "$db_name") || {
      error "Failed to get existing database ID"
      return 1
    }
    info "Database '$db_name' already exists (ID: $existing_id)"
    echo "$existing_id"
    return 0
  fi

  # Create new database using wrangler CLI
  info "Creating D1 database: $db_name"

  # Create a temporary config with just the account_id
  local temp_config=$(mktemp)
  cat > "$temp_config" << EOF
name = "temp"
account_id = "${CLOUDFLARE_ACCOUNT_ID}"
EOF

  local output=$(wrangler d1 create "$db_name" --config "$temp_config" --binding "$binding" 2>/dev/null) || {
    rm -f "$temp_config"
    error "Failed to create database: $db_name"
    return 1
  }

  rm -f "$temp_config"

  debug "Raw D1 create output: $output"

  # Extract database_id from the JSON output
  local db_id=$(echo "$output" | tr -d '\033' | grep -o '"database_id": "[^"]*"' | sed 's/"database_id": "//;s/"//' | tail -1)

  if [ -z "$db_id" ] || [ "$db_id" = "null" ]; then
    error "Failed to create database: $db_name - no ID returned"
    debug "Output was: $output"
    return 1
  fi

  success "Database created: $db_name (ID: $db_id)"
  echo "$db_id"
  return 0
}

# Delete D1 database
delete_d1_database() {
  local db_name="$1"
  local force="${2:-false}"

  if [ "$force" != "true" ]; then
    warning "This will permanently delete database: $db_name"
    if ! prompt_yes_no "Are you sure?"; then
      info "Database deletion cancelled"
      return 1
    fi
  fi

  info "Deleting D1 database: $db_name"

  wrangler d1 delete "$db_name" --skip-confirmation || {
    error "Failed to delete database: $db_name"
    return 1
  }

  success "Database deleted: $db_name"
  return 0
}

# List KV namespaces
list_kv_namespaces() {
  wrangler kv namespace list 2>/dev/null | tr -d '\033' | jq -r '.[] | "\(.title) (\(.id))"'
}

# Check if KV namespace exists
kv_namespace_exists() {
  local kv_title="$1"

  local existing_id=$(wrangler kv namespace list 2>/dev/null | tr -d '\033' | jq -r ".[] | select(.title == \"$kv_title\") | .id")

  if [ -n "$existing_id" ] && [ "$existing_id" != "null" ]; then
    return 0
  else
    return 1
  fi
}

# Get KV namespace ID by title
get_kv_namespace_id() {
  local kv_title="$1"

  local output=$(wrangler kv namespace list 2>/dev/null) || {
    error "Failed to list KV namespaces"
    return 1
  }

  local kv_id=$(echo "$output" | tr -d '\033' | jq -r ".[] | select(.title == \"$kv_title\") | .id" 2>/dev/null) || {
    error "Failed to parse KV namespace list"
    return 1
  }

  if [ -z "$kv_id" ] || [ "$kv_id" = "null" ]; then
    error "KV namespace '$kv_title' not found"
    return 1
  fi

  echo "$kv_id"
}

# Create KV namespace (or get existing one) using wrangler CLI
create_or_get_kv_namespace() {
  local kv_title="$1"
  local binding="${2:-URL_MAPPINGS}"

  # Check if namespace already exists
  if kv_namespace_exists "$kv_title"; then
    local existing_id=$(get_kv_namespace_id "$kv_title") || {
      error "Failed to get existing KV namespace ID"
      return 1
    }
    info "KV namespace '$kv_title' already exists (ID: $existing_id)"
    echo "$existing_id"
    return 0
  fi

  # Create new namespace using wrangler CLI
  info "Creating KV namespace: $kv_title"

  # Create a temporary config with just the account_id
  local temp_config=$(mktemp)
  cat > "$temp_config" << EOF
name = "temp"
account_id = "${CLOUDFLARE_ACCOUNT_ID}"
EOF

  local output=$(wrangler kv namespace create "$kv_title" --config "$temp_config" 2>/dev/null) || {
    rm -f "$temp_config"
    error "Failed to create KV namespace: $kv_title"
    return 1
  }

  rm -f "$temp_config"

  debug "Raw KV create output: $output"

  # Extract id from the JSON output
  local kv_id=$(echo "$output" | tr -d '\033' | grep -o '"id": "[^"]*"' | sed 's/"id": "//;s/"//' | tail -1)

  if [ -z "$kv_id" ] || [ "$kv_id" = "null" ]; then
    error "Failed to create KV namespace: $kv_title - no ID returned"
    debug "Output was: $output"
    return 1
  fi

  success "KV namespace created: $kv_title (ID: $kv_id)"
  echo "$kv_id"
  return 0
}

# Delete KV namespace
delete_kv_namespace() {
  local kv_title="$1"
  local force="${2:-false}"

  if [ "$force" != "true" ]; then
    warning "This will permanently delete KV namespace: $kv_title"
    if ! prompt_yes_no "Are you sure?"; then
      info "KV namespace deletion cancelled"
      return 1
    fi
  fi

  info "Deleting KV namespace: $kv_title"

  local kv_id=$(get_kv_namespace_id "$kv_title")

  if [ -z "$kv_id" ]; then
    error "KV namespace not found: $kv_title"
    return 1
  fi

  wrangler kv namespace delete --namespace-id "$kv_id" || {
    error "Failed to delete KV namespace: $kv_title"
    return 1
  }

  success "KV namespace deleted: $kv_title"
  return 0
}

# Apply D1 migrations
apply_d1_migrations() {
  local db_name="$1"
  local config_file="${2:-wrangler.toml}"

  info "Applying D1 migrations to: $db_name"

  wrangler d1 migrations apply "$db_name" --remote --config "$config_file" || {
    error "Failed to apply migrations"
    return 1
  }

  success "Migrations applied successfully"
  return 0
}

# Set Worker secret
set_worker_secret() {
  local secret_name="$1"
  local secret_value="$2"
  local config_file="${3:-wrangler.toml}"

  info "Setting Worker secret: $secret_name"

  echo "$secret_value" | wrangler secret put "$secret_name" --config "$config_file" || {
    error "Failed to set secret: $secret_name"
    return 1
  }

  success "Secret set: $secret_name"
  return 0
}

# Deploy Worker
deploy_worker() {
  local config_file="${1:-wrangler.toml}"
  local env="${2:-}"

  info "Deploying Worker..."

  if [ -n "$env" ]; then
    wrangler deploy --config "$config_file" --env "$env" || {
      error "Worker deployment failed"
      return 1
    }
  else
    wrangler deploy --config "$config_file" || {
      error "Worker deployment failed"
      return 1
    }
  fi

  success "Worker deployed successfully"
  return 0
}

# List Workers
list_workers() {
  wrangler deployments list --json 2>/dev/null | jq -r '.[] | "\(.id) - \(.created_on)"'
}

# Get Worker URL
get_worker_url() {
  local worker_name="$1"

  # Try to get URL from wrangler deployments
  local url=$(wrangler deployments list --json 2>/dev/null | jq -r ".[0].url // empty")

  if [ -z "$url" ]; then
    # Fallback: construct URL from worker name
    local account_id=$(get_account_id)
    echo "https://${worker_name}.${account_id}.workers.dev"
  else
    echo "$url"
  fi
}

# Check domain zone exists
check_domain_zone() {
  local domain="$1"

  # Extract root domain (e.g., api.myapp.com -> myapp.com)
  local root_domain=$(echo "$domain" | awk -F. '{if (NF >= 2) print $(NF-1)"."$NF; else print $0}')

  local zone_id=$(wrangler zones list --json 2>/dev/null | jq -r ".[] | select(.name == \"$root_domain\") | .id")

  if [ -z "$zone_id" ]; then
    return 1
  fi

  return 0
}

# List zones
list_zones() {
  wrangler zones list --json 2>/dev/null | jq -r '.[] | "\(.name) (\(.id))"'
}

# Create R2 bucket (or get existing one)
create_or_get_r2_bucket() {
  local bucket_name="$1"

  # Check if bucket exists
  if wrangler r2 bucket list --json 2>/dev/null | jq -e ".[] | select(.name == \"$bucket_name\")" >/dev/null; then
    info "R2 bucket '$bucket_name' already exists"
    return 0
  fi

  # Create new bucket
  info "Creating R2 bucket: $bucket_name"

  wrangler r2 bucket create "$bucket_name" || {
    error "Failed to create R2 bucket: $bucket_name"
    return 1
  }

  success "R2 bucket created: $bucket_name"
  return 0
}

# Upload file to R2
upload_to_r2() {
  local bucket_name="$1"
  local local_file="$2"
  local remote_path="${3:-$(basename "$local_file")}"

  info "Uploading to R2: $bucket_name/$remote_path"

  wrangler r2 object put "${bucket_name}/${remote_path}" --file "$local_file" || {
    error "Failed to upload to R2"
    return 1
  }

  success "Uploaded to R2: $bucket_name/$remote_path"
  return 0
}

# Verify wrangler setup
verify_wrangler_setup() {
  info "Verifying wrangler setup..."

  # Check authentication
  if ! check_wrangler_auth; then
    return 1
  fi

  # Get account info
  local account_id=$(get_account_id)
  if [ -z "$account_id" ]; then
    error "Could not retrieve account ID"
    return 1
  fi

  success "Authenticated with account: $account_id"

  # Check wrangler version
  local version=$(wrangler --version 2>/dev/null | head -1)
  info "Wrangler version: $version"

  return 0
}
