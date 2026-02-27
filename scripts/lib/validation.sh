#!/bin/bash
#
# validation.sh - Input validation functions for Rushomon
#
# This library provides:
#   - Domain format validation
#   - Domain zone checking
#   - OAuth credential validation
#   - Secret strength validation
#   - URL reachability testing
#

# Source common utilities if not already loaded
if [ -z "$BLUE" ]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  source "$SCRIPT_DIR/common.sh"
fi

# Validate domain format
validate_domain_format() {
  local domain="$1"

  if [ -z "$domain" ]; then
    error "Domain cannot be empty"
    return 1
  fi

  # Check for valid domain format
  if ! echo "$domain" | grep -qE '^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*\.[a-zA-Z]{2,}$'; then
    error "Invalid domain format: $domain"
    info "Domain should be like: example.com or api.example.com"
    return 1
  fi

  # Check for invalid characters
  if echo "$domain" | grep -qE '[^a-zA-Z0-9.-]'; then
    error "Domain contains invalid characters: $domain"
    return 1
  fi

  # Check for consecutive dots or hyphens
  if echo "$domain" | grep -qE '\.\.|--'; then
    error "Domain contains consecutive dots or hyphens: $domain"
    return 1
  fi

  return 0
}

# Validate GitHub OAuth client ID
validate_github_client_id() {
  local client_id="$1"

  if [ -z "$client_id" ]; then
    error "GitHub client ID cannot be empty"
    return 1
  fi

  # GitHub client IDs start with "Iv1.", "Ov23li", etc. followed by alphanumeric characters
  # or are in the older format with just hex characters
  if ! echo "$client_id" | grep -qE '^(Iv[0-9]\.[a-f0-9]+|Ov[a-zA-Z0-9]+|[a-f0-9]{20})$'; then
    error "Invalid GitHub client ID format: $client_id"
    info "Expected formats: Iv1.xxxxxxxxxxxxxxxx, Ov23lixxxxxxxxxxx, or 20-character hex string"
    return 1
  fi

  return 0
}

# Validate Google OAuth client ID
validate_google_client_id() {
  local client_id="$1"

  if [ -z "$client_id" ]; then
    error "Google client ID cannot be empty"
    return 1
  fi

  # Google client IDs end with .apps.googleusercontent.com
  if ! echo "$client_id" | grep -qE '^[0-9]+-[a-zA-Z0-9]+\.apps\.googleusercontent\.com$'; then
    error "Invalid Google client ID format: $client_id"
    info "Expected format: 123456789-abc...xyz.apps.googleusercontent.com"
    return 1
  fi

  return 0
}

# Validate OAuth client ID (auto-detect provider)
validate_oauth_client_id() {
  local provider="$1"
  local client_id="$2"

  case "$provider" in
    github|GitHub)
      validate_github_client_id "$client_id"
      ;;
    google|Google)
      validate_google_client_id "$client_id"
      ;;
    *)
      error "Unknown OAuth provider: $provider"
      return 1
      ;;
  esac
}

# Validate OAuth client secret (basic check)
validate_oauth_client_secret() {
  local provider="$1"
  local client_secret="$2"

  if [ -z "$client_secret" ]; then
    error "${provider^} client secret cannot be empty"
    return 1
  fi

  # Basic length check
  if [ ${#client_secret} -lt 20 ]; then
    error "${provider^} client secret seems too short (${#client_secret} chars)"
    info "Expected at least 20 characters"
    return 1
  fi

  return 0
}

# Validate JWT secret strength
validate_jwt_secret() {
  local secret="$1"

  if [ -z "$secret" ]; then
    error "JWT secret cannot be empty"
    return 1
  fi

  # Check minimum length
  if [ ${#secret} -lt 32 ]; then
    error "JWT secret must be at least 32 characters (got ${#secret})"
    info "Generate with: openssl rand -base64 32"
    return 1
  fi

  # Check for entropy (should have mix of characters)
  if ! echo "$secret" | grep -qE '[a-zA-Z]'; then
    warning "JWT secret should contain letters for better entropy"
  fi

  if ! echo "$secret" | grep -qE '[0-9]'; then
    warning "JWT secret should contain numbers for better entropy"
  fi

  return 0
}

# Validate email format
validate_email() {
  local email="$1"

  if [ -z "$email" ]; then
    error "Email cannot be empty"
    return 1
  fi

  if ! echo "$email" | grep -qE '^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'; then
    error "Invalid email format: $email"
    return 1
  fi

  return 0
}

# Check if URL is reachable
check_url_reachable() {
  local url="$1"
  local timeout="${2:-5}"

  debug "Checking URL reachability: $url"

  if ! command -v curl &>/dev/null; then
    warning "curl not found, skipping URL check"
    return 0
  fi

  local status_code=$(curl -s -o /dev/null -w "%{http_code}" --max-time "$timeout" "$url" 2>/dev/null || echo "000")

  if [ "$status_code" = "000" ]; then
    warning "URL not reachable: $url"
    return 1
  elif [ "$status_code" -ge 200 ] && [ "$status_code" -lt 400 ]; then
    success "URL reachable: $url (HTTP $status_code)"
    return 0
  else
    warning "URL returned HTTP $status_code: $url"
    return 1
  fi
}

# Validate SSL certificate
check_ssl_certificate() {
  local domain="$1"

  debug "Checking SSL certificate for: $domain"

  if ! command -v openssl &>/dev/null; then
    warning "openssl not found, skipping SSL check"
    return 0
  fi

  local cert_info=$(echo | timeout 5 openssl s_client -connect "${domain}:443" -servername "$domain" 2>/dev/null | openssl x509 -noout -dates 2>/dev/null)

  if [ -z "$cert_info" ]; then
    warning "Could not retrieve SSL certificate for: $domain"
    return 1
  fi

  local expiry=$(echo "$cert_info" | grep "notAfter" | cut -d= -f2)

  if [ -n "$expiry" ]; then
    info "SSL certificate valid until: $expiry"
  fi

  return 0
}

# Check DNS propagation
check_dns_propagation() {
  local domain="$1"

  debug "Checking DNS for: $domain"

  if ! command -v dig &>/dev/null && ! command -v host &>/dev/null; then
    warning "dig/host not found, skipping DNS check"
    return 0
  fi

  if command -v dig &>/dev/null; then
    local ip=$(dig +short "$domain" 2>/dev/null | head -1)
  else
    local ip=$(host "$domain" 2>/dev/null | grep "has address" | awk '{print $NF}' | head -1)
  fi

  if [ -z "$ip" ]; then
    warning "DNS not configured for: $domain"
    return 1
  fi

  success "DNS resolved: $domain -> $ip"
  return 0
}

# Validate worker name
validate_worker_name() {
  local name="$1"

  if [ -z "$name" ]; then
    error "Worker name cannot be empty"
    return 1
  fi

  # Worker names must be lowercase alphanumeric with hyphens
  if ! echo "$name" | grep -qE '^[a-z0-9-]+$'; then
    error "Invalid worker name: $name"
    info "Worker names must be lowercase alphanumeric with hyphens"
    return 1
  fi

  # Check length (Cloudflare limit)
  if [ ${#name} -gt 63 ]; then
    error "Worker name too long: ${#name} chars (max 63)"
    return 1
  fi

  return 0
}

# Validate environment name
validate_environment_name() {
  local env="$1"

  if [ -z "$env" ]; then
    error "Environment name cannot be empty"
    return 1
  fi

  # Environment names should be alphanumeric
  if ! echo "$env" | grep -qE '^[a-zA-Z0-9_-]+$'; then
    error "Invalid environment name: $env"
    info "Environment names must be alphanumeric with underscores/hyphens"
    return 1
  fi

  return 0
}

# Validate Cloudflare account ID
validate_account_id() {
  local account_id="$1"

  if [ -z "$account_id" ]; then
    error "Account ID cannot be empty"
    return 1
  fi

  # Cloudflare account IDs are 32-character hex strings
  if ! echo "$account_id" | grep -qE '^[a-f0-9]{32}$'; then
    error "Invalid Cloudflare account ID format: $account_id"
    info "Expected 32-character hexadecimal string"
    return 1
  fi

  return 0
}

# Validate UUID
validate_uuid() {
  local uuid="$1"
  local name="${2:-UUID}"

  if [ -z "$uuid" ]; then
    error "$name cannot be empty"
    return 1
  fi

  # UUID format: 8-4-4-4-12 hex characters
  if ! echo "$uuid" | grep -qE '^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$'; then
    error "Invalid $name format: $uuid"
    info "Expected UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
    return 1
  fi

  return 0
}

# Validate all domains in config
validate_all_domains() {
  local main="$1"
  local api="$2"
  local redirect="$3"

  local valid=true

  info "Validating domain configuration..."

  if ! validate_domain_format "$main"; then
    valid=false
  fi

  if ! validate_domain_format "$api"; then
    valid=false
  fi

  if ! validate_domain_format "$redirect"; then
    valid=false
  fi

  if [ "$valid" = false ]; then
    error "Domain validation failed"
    return 1
  fi

  success "All domains validated"
  return 0
}
