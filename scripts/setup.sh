#!/bin/bash
#
# setup.sh - Interactive Rushomon deployment setup
#
# This script guides you through deploying Rushomon to Cloudflare Workers.
#
# Usage:
#   ./scripts/setup.sh [options]
#
# Options:
#   --config FILE    Load configuration from file
#   --update         Update existing deployment
#   --dry-run        Preview configuration without deploying
#   --help           Show this help message
#
# Examples:
#   ./scripts/setup.sh                                    # Interactive setup
#   ./scripts/setup.sh --dry-run                         # Preview config
#   ./scripts/setup.sh --config config/staging.yaml      # Use existing config
#   ./scripts/setup.sh --config config/production.yaml --update
#   ./scripts/setup.sh --config config/staging.yaml --dry-run  # Preview existing config
#
# Domain Types:
#   - Custom Domain: Your own domain (requires DNS setup with Cloudflare)
#   - Workers.dev: Cloudflare's default subdomain (no DNS required, instant setup)
#     Format: <worker-name>.<your-subdomain>.workers.dev
#     Example: rushomon-prod.mycompany.workers.dev
#
# Notes:
#   - --dry-run generates config/{environment}.yaml when no --config is provided
#   - --dry-run shows existing config when --config is provided (no overwriting)
#   - Use environment variables for sensitive data (secrets, API keys)
#   - Workers.dev deployments automatically skip route configuration
#

set -euo pipefail

# Script directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB_DIR="$SCRIPT_DIR/lib"

# Source library functions
source "$LIB_DIR/common.sh"
source "$LIB_DIR/cloudflare.sh"
source "$LIB_DIR/validation.sh"
source "$LIB_DIR/oauth.sh"
source "$LIB_DIR/deployment.sh"

# Global configuration
CONFIG_FILE=""
UPDATE_MODE=false
DRY_RUN=false

# Default values
export ENVIRONMENT_NAME="production"
export WORKER_NAME=""
export DOMAIN_TYPE="custom"  # Default to custom domain for backward compatibility
export DOMAIN_STRATEGY="single"
export MAIN_DOMAIN=""
export API_DOMAIN=""
export REDIRECT_DOMAIN=""
export SKIP_ROUTES="false"  # Whether to skip routes generation (for workers.dev)
export GITHUB_ENABLED=true
export GITHUB_CLIENT_ID=""
export GITHUB_CLIENT_SECRET=""
export GOOGLE_ENABLED=false
export GOOGLE_CLIENT_ID=""
export GOOGLE_CLIENT_SECRET=""
export JWT_SECRET=""
export MAILGUN_DOMAIN=""
export MAILGUN_BASE_URL=""
export MAILGUN_FROM=""
export MAILGUN_API_KEY=""
export POLAR_ENABLED=false
export POLAR_ACCESS_TOKEN=""
export POLAR_WEBHOOK_SECRET=""
export POLAR_PRO_MONTHLY_PRODUCT_ID=""
export POLAR_PRO_ANNUAL_PRODUCT_ID=""
export POLAR_BUSINESS_MONTHLY_PRODUCT_ID=""
export POLAR_BUSINESS_ANNUAL_PRODUCT_ID=""
export ENABLE_KV_RATE_LIMITING=false
export R2_BACKUP_BUCKET=""
export R2_ASSETS_BUCKET_NAME=""
export SAVE_CONFIG=true
export CLOUDFLARE_ACCOUNT_ID=""
export D1_DATABASE_ID=""
export KV_NAMESPACE_ID=""

# Show help
show_help() {
  sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
}

# Parse command-line arguments
parse_arguments() {
  while [[ $# -gt 0 ]]; do
    case $1 in
      --config)
        CONFIG_FILE="$2"
        shift 2
        ;;
      --update)
        UPDATE_MODE=true
        shift
        ;;
      --dry-run)
        DRY_RUN=true
        shift
        ;;
      --help)
        show_help
        exit 0
        ;;
      *)
        error "Unknown option: $1"
        show_help
        exit 1
        ;;
    esac
  done
}

# Show welcome message
show_welcome() {
  cat <<'EOF'

╔════════════════════════════════════════════════════════════════╗
║                                                                ║
║                  Rushomon Setup Assistant                      ║
║                                                                ║
║        Deploy your Rushomon instance to Cloudflare             ║
║                                                                ║
╚════════════════════════════════════════════════════════════════╝

This interactive wizard will guide you through:
  - Domain configuration
  - OAuth provider setup (GitHub, Google)
  - Cloudflare resource creation
  - Database migration
  - Worker deployment

Prerequisites (complete BEFORE running this script):
  ✓ Cloudflare account with Workers enabled
  ✓ Domain(s) added to Cloudflare and DNS configured
  ✓ OAuth apps created (GitHub and/or Google)
  ✓ wrangler CLI installed and authenticated
  ✓ Node.js 20+, Rust toolchain, worker-build

📚 Documentation:
  → Self-Hosting Guide: docs/SELF_HOSTING.md
  → Scripts Documentation: scripts/README.md

EOF

  read -p "Press Enter to continue (or Ctrl+C to exit and read documentation)..."
  echo ""
}

# Check prerequisites
check_prerequisites() {
  info "Checking prerequisites..."

  # Check wrangler
  require_command "wrangler" "Install with: npm install -g wrangler"

  # Check jq (required for JSON parsing)
  require_command "jq" "Install with: brew install jq (macOS) or apt-get install jq (Ubuntu)"

  # Check authentication
  if ! check_wrangler_auth; then
    error "Please authenticate with Cloudflare first"
    info "Run: wrangler login"
    exit 1
  fi

  # Get and store account ID
  export CLOUDFLARE_ACCOUNT_ID=$(get_account_id)
  if [ -z "$CLOUDFLARE_ACCOUNT_ID" ]; then
    error "Failed to retrieve Cloudflare account ID"
    error "This usually means wrangler authentication failed"
    echo ""
    info "To fix this issue:"
    info "  1. Log out of wrangler: wrangler logout"
    info "  2. Log back in: wrangler login"
    info "  3. Verify authentication: wrangler whoami"
    info "  4. Run this script again"
    echo ""
    exit 1
  fi
  success "Authenticated with Cloudflare account: $CLOUDFLARE_ACCOUNT_ID"

  # Check Node.js
  require_command "node" "Install from: https://nodejs.org/"

  # Check Rust
  require_command "cargo" "Install from: https://rustup.rs/"

  # Check worker-build
  if ! command -v worker-build &>/dev/null; then
    warning "worker-build not found, will install during deployment"
  fi

  # Check optional tools
  if ! command -v jq &>/dev/null; then
    warning "jq not found (optional, but recommended)"
    info "Install with: brew install jq (macOS) or apt install jq (Linux)"
  fi

  if ! command -v yq &>/dev/null; then
    warning "yq not found (optional, but recommended for config files)"
    info "Install with: brew install yq (macOS) or snap install yq (Linux)"
  fi

  success "Prerequisites check passed"
  echo ""
}

# Configure basic deployment info (needed for domain configuration)
configure_basic_deployment() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}Basic Deployment Information${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  # Environment name
  export ENVIRONMENT_NAME=$(prompt_input "Environment name" "production" validate_environment_name)

  # Worker name
  local default_worker_name="rushomon-${ENVIRONMENT_NAME}"
  export WORKER_NAME=$(prompt_input "Worker name" "$default_worker_name" validate_worker_name)

  success "Basic deployment info configured"
  echo ""
}

# Configure domain type (Custom vs Workers.dev)
configure_domain_type() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}Domain Type Selection${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo "Choose your deployment type:"
  echo ""
  echo -e "  ${CYAN}1. Custom Domain${NC}"
  echo "     Your own domain (e.g., example.com, app.example.com)"
  echo "     Requires DNS setup with Cloudflare"
  echo "     Supports multi-worker deployment"
  echo "     Professional appearance"
  echo ""
  echo -e "  ${CYAN}2. Workers.dev Subdomain${NC}"
  echo "     Cloudflare's default subdomain (e.g., rushomon-example.workers.dev)"
  echo "     No DNS setup required - instant deployment"
  echo "     Single worker deployment only"
  echo "     Quick and easy to get started"
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  while true; do
    local selection=$(prompt_input "Choose deployment type [1/2]" "2")
    case $selection in
      1)
        export DOMAIN_TYPE="custom"
        break
        ;;
      2)
        export DOMAIN_TYPE="workers_dev"
        break
        ;;
      *)
        warning "Please enter 1 or 2"
        ;;
    esac
  done

  echo ""
  success "Selected: $([ "$DOMAIN_TYPE" = "custom" ] && echo "Custom Domain" || echo "Workers.dev Subdomain")"
  echo ""
}

# Configure domains interactively
configure_domains() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}Domain Configuration${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  # Handle Workers.dev subdomain configuration
  if [ "$DOMAIN_TYPE" = "workers_dev" ]; then
    echo "Configuring Workers.dev subdomain deployment..."
    echo ""

    # Get workers.dev subdomain (the account's workers.dev subdomain)
    # This is the part between worker name and .workers.dev
    local default_subdomain="your-subdomain"  # Default to a reasonable subdomain
    while true; do
      local workers_subdomain=$(prompt_input "Workers.dev account subdomain" "$default_subdomain")
      if [ -n "$workers_subdomain" ] && [[ "$workers_subdomain" =~ ^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?$ ]]; then
        break
      fi
      warning "Subdomain must contain only letters, numbers, and hyphens, and start/end with alphanumeric"
    done

    # Set all domains to the full workers.dev URL
    export MAIN_DOMAIN="${WORKER_NAME}.${workers_subdomain}.workers.dev"
    export API_DOMAIN="${WORKER_NAME}.${workers_subdomain}.workers.dev"
    export REDIRECT_DOMAIN="${WORKER_NAME}.${workers_subdomain}.workers.dev"
    export DOMAIN_STRATEGY="single"  # Workers.dev only supports single domain
    export SKIP_ROUTES="true"  # Don't generate routes for workers.dev

    echo ""
    success "Workers.dev domains configured:"
    echo "  Main:     $MAIN_DOMAIN"
    echo "  API:      $API_DOMAIN"
    echo "  Redirect: $REDIRECT_DOMAIN"
    echo ""
    return
  fi

  # Custom domain configuration (existing logic)
  echo "Choose your domain strategy:"
  echo ""
  echo -e "  ${CYAN}1. Single domain${NC} (simplest)"
  echo "     All services on one domain"
  echo "     Example: myapp.com for everything"
  echo ""
  echo -e "  ${CYAN}2. Multi-domain${NC} (recommended)"
  echo "     Separate domains for different services"
  echo "     Example: myapp.com, api.myapp.com, go.myapp.com"
  echo ""
  echo -e "  ${CYAN}3. Custom subdomains${NC}"
  echo "     All subdomains of one parent domain"
  echo "     Example: app.myapp.com, api.myapp.com, links.myapp.com"
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  read -p "Selection [1-3]: " domain_strategy

  case $domain_strategy in
    1)
      export DOMAIN_STRATEGY="single"
      export SKIP_ROUTES="false"
      while true; do
        export MAIN_DOMAIN=$(prompt_input "Domain" "")
        if validate_domain_format "$MAIN_DOMAIN"; then
          # Auto-detect workers.dev and switch mode
          if echo "$MAIN_DOMAIN" | grep -qE '\.workers\.dev$'; then
            info "Detected Workers.dev subdomain, switching to Workers.dev deployment mode..."
            export DOMAIN_TYPE="workers_dev"
            export DOMAIN_STRATEGY="single"
            export SKIP_ROUTES="true"
            export API_DOMAIN="$MAIN_DOMAIN"
            export REDIRECT_DOMAIN="$MAIN_DOMAIN"
            success "Auto-configured Workers.dev deployment:"
            echo "  Main:     $MAIN_DOMAIN"
            echo "  API:      $API_DOMAIN"
            echo "  Redirect: $REDIRECT_DOMAIN"
            echo ""
            return
          fi
          export API_DOMAIN="$MAIN_DOMAIN"
          export REDIRECT_DOMAIN="$MAIN_DOMAIN"
          break
        fi
      done
      ;;
    2)
      export DOMAIN_STRATEGY="multi"
      export SKIP_ROUTES="false"
      while true; do
        export MAIN_DOMAIN=$(prompt_input "Main domain (web interface)" "")
        if validate_domain_format "$MAIN_DOMAIN"; then
          break
        fi
      done
      while true; do
        export API_DOMAIN=$(prompt_input "API domain" "$MAIN_DOMAIN")
        if validate_domain_format "$API_DOMAIN"; then
          break
        fi
      done
      while true; do
        export REDIRECT_DOMAIN=$(prompt_input "Redirect domain (short links)" "$MAIN_DOMAIN")
        if validate_domain_format "$REDIRECT_DOMAIN"; then
          break
        fi
      done
      ;;
    3)
      export DOMAIN_STRATEGY="custom"
      export SKIP_ROUTES="false"
      local parent_domain=$(prompt_input "Parent domain" "")
      while ! validate_domain_format "$parent_domain"; do
        parent_domain=$(prompt_input "Parent domain" "")
      done

      local web_sub=$(prompt_input "Web subdomain" "app")
      local api_sub=$(prompt_input "API subdomain" "api")
      local redirect_sub=$(prompt_input "Redirect subdomain" "go")

      export MAIN_DOMAIN="${web_sub}.${parent_domain}"
      export API_DOMAIN="${api_sub}.${parent_domain}"
      export REDIRECT_DOMAIN="${redirect_sub}.${parent_domain}"
      ;;
    *)
      error "Invalid selection"
      configure_domains
      return
      ;;
  esac

  # Validate all domains
  info "Validating domain configuration..."
  validate_all_domains "$MAIN_DOMAIN" "$API_DOMAIN" "$REDIRECT_DOMAIN" || exit 1

  echo ""
  success "Domain configuration complete"
  info "  Main:     https://${MAIN_DOMAIN}"
  info "  API:      https://${API_DOMAIN}"
  info "  Redirect: https://${REDIRECT_DOMAIN}"
  echo ""
}

# Configure secrets
configure_secrets() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}Secrets Configuration${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo "Rushomon requires a JWT secret for secure session management."
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  # Check if JWT_SECRET is already set in environment
  if [ -n "${JWT_SECRET:-}" ]; then
    info "JWT_SECRET found in environment"
    if ! prompt_yes_no "Use existing JWT_SECRET?" "y"; then
      export JWT_SECRET=""
    fi
  fi

  # Prompt for JWT secret if not set
  if [ -z "$JWT_SECRET" ]; then
    info "Generating JWT secret..."
    if command -v openssl &>/dev/null; then
      export JWT_SECRET=$(openssl rand -base64 32)
      success "JWT secret generated"
    else
      warning "openssl not found, please enter a JWT secret manually"
      while true; do
        export JWT_SECRET=$(prompt_secret "JWT secret (min 32 chars)")
        if validate_jwt_secret "$JWT_SECRET"; then
          break
        fi
      done
    fi
  fi

  success "Secrets configured"
  echo ""
}

# Configure Polar (optional)
configure_polar() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}Polar Billing Configuration (Optional)${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo "Polar enables paid plans (Pro / Business). Self-hosted instances"
  echo "can skip this — all users will remain on the Free tier."
  echo ""

  if ! prompt_yes_no "Enable Polar billing?" "n"; then
    info "Polar billing disabled — skipping"
    echo ""
    return
  fi

  echo ""
  echo "Get your access token from: https://polar.sh/dashboard/settings/api"
  echo "Use Sandbox environment for local development: https://sandbox.polar.sh"
  echo ""

  while true; do
    export POLAR_ACCESS_TOKEN=$(prompt_secret "Polar access token (polar_oat_...)")
    if [[ "$POLAR_ACCESS_TOKEN" == polar_oat_* ]]; then
      break
    fi
    warning "Token must start with polar_oat_"
  done

  echo ""
  echo "Webhook secret: Polar Dashboard → Settings → Webhooks → your endpoint → Secret"
  while true; do
    export POLAR_WEBHOOK_SECRET=$(prompt_secret "Polar webhook secret (any string)")
    if [ ${#POLAR_WEBHOOK_SECRET} -ge 8 ]; then
      break
    fi
    warning "Secret must be at least 8 characters"
  done

  echo ""
  echo "Product IDs: Polar Dashboard → Products → <plan> → copy product ID"
  echo "Press Enter to leave a product ID empty (plan will be unavailable)."
  echo ""
  export POLAR_PRO_MONTHLY_PRODUCT_ID=$(prompt_input "Pro Monthly product ID" "")
  export POLAR_PRO_ANNUAL_PRODUCT_ID=$(prompt_input "Pro Annual product ID" "")
  export POLAR_BUSINESS_MONTHLY_PRODUCT_ID=$(prompt_input "Business Monthly product ID" "")
  export POLAR_BUSINESS_ANNUAL_PRODUCT_ID=$(prompt_input "Business Annual product ID" "")

  export POLAR_ENABLED=true
  success "Polar billing configured"
  echo ""
}

# Configure advanced deployment options (excluding basic info already set)
configure_deployment_options_advanced() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}Advanced Deployment Options${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  # R2 assets bucket (required for org logo storage)
  echo "R2 bucket for organization logo storage (required for QR code logo feature)."
  local default_assets_bucket="${WORKER_NAME}-assets"
  while true; do
    export R2_ASSETS_BUCKET_NAME=$(prompt_input "R2 assets bucket name" "$default_assets_bucket")
    if [ -n "$R2_ASSETS_BUCKET_NAME" ]; then
      break
    fi
    warning "R2 assets bucket name cannot be empty"
  done

  # Advanced options
  if prompt_yes_no "Configure additional advanced options?" "n"; then
    if prompt_yes_no "Enable KV-based rate limiting? (costs money, Cloudflare rate limiting recommended)" "n"; then
      export ENABLE_KV_RATE_LIMITING=true
    fi

    local r2_bucket=$(prompt_input "R2 bucket for backups (optional)" "")
    if [ -n "$r2_bucket" ]; then
      export R2_BACKUP_BUCKET="$r2_bucket"
    fi
  fi

  success "Advanced deployment options configured"
  echo ""
}

# Configure deployment options (legacy - kept for backward compatibility)
configure_deployment_options() {
  # This function is kept for backward compatibility but should not be called
  # in the normal flow since configure_basic_deployment and configure_deployment_options_advanced
  # handle the setup separately now.
  warning "This function should not be called directly - use configure_basic_deployment and configure_deployment_options_advanced"
}

# Interactive configuration flow
interactive_configuration() {
  info "Starting interactive configuration..."
  echo ""

  # Configure basic deployment info first (needed for domain configuration)
  configure_basic_deployment

  # Configure domain type
  configure_domain_type

  # Configure domains
  configure_domains

  # Configure OAuth
  configure_oauth

  # Configure secrets
  configure_secrets

  # Configure Polar billing (optional)
  configure_polar

  # Configure deployment options (excluding the basic info we already set)
  configure_deployment_options_advanced

  success "Configuration complete"
}

# Show configuration summary
show_configuration_summary() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}Configuration Summary${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo -e "${YELLOW}Environment:${NC} ${ENVIRONMENT_NAME}"
  echo -e "${YELLOW}Worker Name:${NC} ${WORKER_NAME}"
  echo ""
  echo -e "${YELLOW}Domains:${NC}"
  echo "  - Main:     ${MAIN_DOMAIN}"
  echo "  - API:      ${API_DOMAIN}"
  echo "  - Redirect: ${REDIRECT_DOMAIN}"
  echo ""
  echo -e "${YELLOW}OAuth & Email:${NC}"
  echo "  - GitHub:   $([ "$GITHUB_ENABLED" = true ] && echo "Enabled" || echo "Disabled")"
  echo "  - Google:   $([ "$GOOGLE_ENABLED" = true ] && echo "Enabled" || echo "Disabled")"
  echo "  - Mailgun:  $([ -n "$MAILGUN_DOMAIN" ] && echo "Enabled" || echo "Disabled")"
  echo ""
  echo -e "${YELLOW}Options:${NC}"
  echo "  - KV Rate Limiting: $([ "$ENABLE_KV_RATE_LIMITING" = true ] && echo "Enabled" || echo "Disabled")"
  echo "  - R2 Assets Bucket: ${R2_ASSETS_BUCKET_NAME:-Not configured}"
  echo "  - R2 Backups:       ${R2_BACKUP_BUCKET:-None}"
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
}

# Main function
main() {
  parse_arguments "$@"

  # Show welcome message (skip if config file provided)
  if [ -z "$CONFIG_FILE" ]; then
    show_welcome
  fi

  # Check prerequisites
  check_prerequisites

  # Load configuration or run interactive setup
  if [ -n "$CONFIG_FILE" ]; then
    info "Loading configuration from: $CONFIG_FILE"
    load_config "$CONFIG_FILE" || exit 1
    echo ""
  else
    interactive_configuration
  fi

  # Show configuration summary
  show_configuration_summary

  # Dry run mode
  if [ "$DRY_RUN" = true ]; then
    info "DRY RUN MODE - No changes will be made"

    if [ -n "$CONFIG_FILE" ]; then
      info "Using existing configuration file: $CONFIG_FILE"
      info "Configuration preview:"
      echo ""
      cat "$CONFIG_FILE"
      echo ""
      info "To run the actual deployment: ./scripts/setup.sh --config $CONFIG_FILE"
    else
      # Generate configuration file for dry-run
      local config_dir="$PROJECT_ROOT/config"
      mkdir -p "$config_dir"

      local config_file="$config_dir/${ENVIRONMENT_NAME}.yaml"
      info "Generating configuration file: $config_file"

      if save_config "$config_file"; then
        success "Configuration file generated: $config_file"
        info "You can run the actual deployment with: ./scripts/setup.sh --config $config_file"
      else
        error "Failed to generate configuration file"
        exit 1
      fi
    fi

    exit 0
  fi

  # Confirm deployment
  if ! prompt_yes_no "Proceed with deployment?" "y"; then
    info "Deployment cancelled"
    exit 0
  fi

  # Generate wrangler config filename
  local wrangler_config="wrangler.${ENVIRONMENT_NAME}.toml"

  # Deploy environment
  if ! deploy_environment "$wrangler_config" "$PROJECT_ROOT"; then
    error "Deployment failed"
    exit 1
  fi

  # Show deployment summary
  show_deployment_summary

  # Offer to save configuration
  if [ -z "$CONFIG_FILE" ] && [ "$SAVE_CONFIG" = true ]; then
    if prompt_yes_no "Save configuration for future use?" "y"; then
      local config_dir="$PROJECT_ROOT/config"
      mkdir -p "$config_dir"

      local config_file="$config_dir/${ENVIRONMENT_NAME}.yaml"
      save_config "$config_file"

      success "Configuration saved to: $config_file"
      info "You can rerun this setup with: ./scripts/setup.sh --config $config_file"
    fi
  fi

  success "Setup complete!"
}

# Run main function
main "$@"
