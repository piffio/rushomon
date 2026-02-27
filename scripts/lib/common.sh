#!/bin/bash
#
# common.sh - Shared utilities for Rushomon scripts
#
# This library provides:
#   - Color output functions
#   - Progress indicators
#   - User prompt functions
#   - Config file loading/saving
#   - Logging utilities
#   - Error handling helpers
#

# Colors
export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export CYAN='\033[0;36m'
export NC='\033[0m' # No Color

# Logging functions
info() {
  echo -e "${BLUE}[INFO]${NC} $*" >&2
}

success() {
  echo -e "${GREEN}[SUCCESS]${NC} $*" >&2
}

error() {
  echo -e "${RED}[ERROR]${NC} $*" >&2
}

warning() {
  echo -e "${YELLOW}[WARNING]${NC} $*" >&2
}

debug() {
  if [ "${DEBUG:-false}" = true ]; then
    echo -e "${CYAN}[DEBUG]${NC} $*" >&2
  fi
}

# Progress indicator
show_spinner() {
  local pid=$1
  local delay=0.1
  local spinstr='|/-\'

  while ps -p $pid > /dev/null 2>&1; do
    local temp=${spinstr#?}
    printf " [%c]  " "$spinstr"
    local spinstr=$temp${spinstr%"$temp"}
    sleep $delay
    printf "\b\b\b\b\b\b"
  done
  printf "    \b\b\b\b"
}

# User prompts
prompt_yes_no() {
  local prompt="$1"
  local default="${2:-n}"

  while true; do
    if [ "$default" = "y" ]; then
      read -p "$prompt [Y/n]: " response
      response=${response:-y}
    else
      read -p "$prompt [y/N]: " response
      response=${response:-n}
    fi

    case "$response" in
      [Yy]|[Yy][Ee][Ss])
        return 0
        ;;
      [Nn]|[Nn][Oo])
        return 1
        ;;
      *)
        warning "Please answer yes or no"
        ;;
    esac
  done
}

prompt_input() {
  local prompt="$1"
  local default="$2"
  local validation_func="${3:-}"

  while true; do
    if [ -n "$default" ]; then
      read -p "$prompt [$default]: " response
      response=${response:-$default}
    else
      read -p "$prompt: " response
    fi

    # If validation function provided, call it
    if [ -n "$validation_func" ]; then
      if $validation_func "$response"; then
        echo "$response"
        return 0
      else
        warning "Invalid input, please try again"
        continue
      fi
    else
      echo "$response"
      return 0
    fi
  done
}

prompt_secret() {
  local prompt="$1"
  local validation_func="${2:-}"

  while true; do
    read -s -p "$prompt: " response
    echo ""

    if [ -z "$response" ]; then
      warning "Secret cannot be empty"
      continue
    fi

    # If validation function provided, call it
    if [ -n "$validation_func" ]; then
      if $validation_func "$response"; then
        echo "$response"
        return 0
      else
        warning "Invalid secret, please try again"
        continue
      fi
    else
      echo "$response"
      return 0
    fi
  done
}

# Config file utilities
load_config() {
  local config_file="$1"

  if [ ! -f "$config_file" ]; then
    error "Config file not found: $config_file"
    return 1
  fi

  info "Loading configuration from $config_file"

  # Check if yq is available for proper YAML parsing
  if command -v yq &>/dev/null; then
    load_config_with_yq "$config_file"
  else
    warning "yq not found, using basic parsing (install yq for better support)"
    load_config_basic "$config_file"
  fi

  success "Configuration loaded"
}

load_config_with_yq() {
  local config_file="$1"

  # Domain configuration
  export DOMAIN_STRATEGY=$(yq eval '.domains.strategy // "single"' "$config_file")
  export MAIN_DOMAIN=$(yq eval '.domains.main // ""' "$config_file")
  export API_DOMAIN=$(yq eval '.domains.api // ""' "$config_file")
  export REDIRECT_DOMAIN=$(yq eval '.domains.redirect // ""' "$config_file")

  # OAuth configuration
  export GITHUB_ENABLED=$(yq eval '.oauth.github.enabled // true' "$config_file")
  export GITHUB_CLIENT_ID=$(expand_env_var "$(yq eval '.oauth.github.client_id // ""' "$config_file")")
  export GITHUB_CLIENT_SECRET=$(expand_env_var "$(yq eval '.oauth.github.client_secret // ""' "$config_file")")

  export GOOGLE_ENABLED=$(yq eval '.oauth.google.enabled // false' "$config_file")
  export GOOGLE_CLIENT_ID=$(expand_env_var "$(yq eval '.oauth.google.client_id // ""' "$config_file")")
  export GOOGLE_CLIENT_SECRET=$(expand_env_var "$(yq eval '.oauth.google.client_secret // ""' "$config_file")")

  # Deployment configuration
  export ENVIRONMENT_NAME=$(yq eval '.deployment.environment_name // "production"' "$config_file")
  export WORKER_NAME=$(yq eval '.deployment.worker_name // "rushomon-production"' "$config_file")
  export SAVE_CONFIG=$(yq eval '.deployment.save_config // true' "$config_file")

  # Secrets
  export JWT_SECRET=$(expand_env_var "$(yq eval '.secrets.jwt_secret // ""' "$config_file")")

  # Advanced options
  export ENABLE_KV_RATE_LIMITING=$(yq eval '.advanced.enable_kv_rate_limiting // false' "$config_file")
  export R2_BACKUP_BUCKET=$(yq eval '.advanced.r2_backup_bucket // ""' "$config_file")

  # Cloudflare
  export CLOUDFLARE_ACCOUNT_ID=$(yq eval '.cloudflare.account_id // ""' "$config_file")
}

load_config_basic() {
  local config_file="$1"

  # Basic grep-based parsing (limited functionality)
  export MAIN_DOMAIN=$(grep 'main:' "$config_file" | awk '{print $2}' | tr -d '"' || echo "")
  export API_DOMAIN=$(grep 'api:' "$config_file" | awk '{print $2}' | tr -d '"' || echo "")
  export REDIRECT_DOMAIN=$(grep 'redirect:' "$config_file" | awk '{print $2}' | tr -d '"' || echo "")

  export GITHUB_CLIENT_ID=$(grep 'client_id:' "$config_file" | head -1 | awk '{print $2}' | tr -d '"' || echo "")
  export ENVIRONMENT_NAME=$(grep 'environment_name:' "$config_file" | awk '{print $2}' | tr -d '"' || echo "production")
  export WORKER_NAME=$(grep 'worker_name:' "$config_file" | awk '{print $2}' | tr -d '"' || echo "rushomon-production")
  
  # Cloudflare configuration
  export CLOUDFLARE_ACCOUNT_ID=$(grep 'account_id:' "$config_file" | awk '{print $2}' | tr -d '"' || echo "")

  warning "Basic config parsing is limited - install yq for full support: brew install yq"
}

expand_env_var() {
  local value="$1"

  # Expand ${VAR} or $VAR syntax
  if [[ "$value" =~ ^\$\{([A-Z_]+)\}$ ]]; then
    local var_name="${BASH_REMATCH[1]}"
    echo "${!var_name:-}"
  elif [[ "$value" =~ ^\$([A-Z_]+)$ ]]; then
    local var_name="${BASH_REMATCH[1]}"
    echo "${!var_name:-}"
  else
    echo "$value"
  fi
}

save_config() {
  local config_file="$1"

  info "Saving configuration to $config_file"

  cat > "$config_file" <<EOF
# Rushomon Setup Configuration
# Generated on $(date)

cloudflare:
  account_id: "${CLOUDFLARE_ACCOUNT_ID:-}"

domains:
  strategy: "${DOMAIN_STRATEGY:-single}"
  main: "${MAIN_DOMAIN}"
  api: "${API_DOMAIN}"
  redirect: "${REDIRECT_DOMAIN}"

oauth:
  github:
    enabled: ${GITHUB_ENABLED:-true}
    client_id: "${GITHUB_CLIENT_ID}"
    client_secret: "\${GITHUB_CLIENT_SECRET}"

  google:
    enabled: ${GOOGLE_ENABLED:-false}
    client_id: "${GOOGLE_CLIENT_ID:-}"
    client_secret: "\${GOOGLE_CLIENT_SECRET}"

deployment:
  environment_name: "${ENVIRONMENT_NAME:-production}"
  worker_name: "${WORKER_NAME}"
  save_config: ${SAVE_CONFIG:-true}

secrets:
  jwt_secret: "\${JWT_SECRET}"

advanced:
  enable_kv_rate_limiting: ${ENABLE_KV_RATE_LIMITING:-false}
  r2_backup_bucket: "${R2_BACKUP_BUCKET:-}"
EOF

  success "Configuration saved to $config_file"
}

# Error handling
die() {
  error "$@"
  exit 1
}

# Check if command exists
require_command() {
  local cmd="$1"
  local install_msg="${2:-Install $cmd to continue}"

  if ! command -v "$cmd" &>/dev/null; then
    error "$cmd not found"
    info "$install_msg"
    exit 1
  fi
}

# Cleanup trap
cleanup() {
  local exit_code=$?

  if [ $exit_code -ne 0 ]; then
    error "Script failed with exit code $exit_code"
  fi

  # Add any cleanup logic here
  return $exit_code
}

trap cleanup EXIT
