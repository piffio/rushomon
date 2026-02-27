#!/bin/bash
#
# oauth.sh - OAuth configuration helpers for Rushomon
#
# This library provides:
#   - GitHub OAuth setup instructions
#   - Google OAuth setup instructions
#   - OAuth credential validation
#   - Interactive OAuth configuration
#

# Source common utilities if not already loaded
if [ -z "$BLUE" ]; then
  SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  source "$SCRIPT_DIR/common.sh"
  source "$SCRIPT_DIR/validation.sh"
fi

# Show GitHub OAuth setup instructions
show_github_oauth_instructions() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}GitHub OAuth Setup${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo "Follow these steps to create a GitHub OAuth App:"
  echo ""
  echo -e "1. Go to: ${CYAN}https://github.com/settings/developers${NC}"
  echo ""
  echo "2. Click \"OAuth Apps\" → \"New OAuth App\""
  echo ""
  echo "3. Fill in the application details:"
  echo -e "   ${YELLOW}Application name:${NC} Rushomon (${ENVIRONMENT_NAME})"
  echo -e "   ${YELLOW}Homepage URL:${NC} https://${MAIN_DOMAIN}"
  echo -e "   ${YELLOW}Authorization callback URL:${NC} https://${API_DOMAIN}/api/auth/callback"
  echo ""
  echo "4. Click \"Register application\""
  echo ""
  echo -e "5. Copy the ${GREEN}Client ID${NC}"
  echo ""
  echo -e "6. Click \"Generate a new client secret\" and copy the ${GREEN}Client Secret${NC}"
  echo ""
  echo -e "${YELLOW}Important:${NC} Keep your client secret secure. Never commit it to version control."
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  read -p "Press Enter once you've created the OAuth app..."
}

# Show Google OAuth setup instructions
show_google_oauth_instructions() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}Google OAuth Setup${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo "Follow these steps to create a Google OAuth Client:"
  echo ""
  echo -e "1. Go to: ${CYAN}https://console.cloud.google.com/apis/credentials${NC}"
  echo ""
  echo "2. Create a new project (or select an existing one)"
  echo ""
  echo "3. Click \"Create Credentials\" → \"OAuth 2.0 Client ID\""
  echo ""
  echo "4. Configure the OAuth consent screen if prompted:"
  echo "   - User Type: External"
  echo "   - App name: Rushomon"
  echo "   - User support email: your@email.com"
  echo "   - Developer contact: your@email.com"
  echo ""
  echo "5. Create OAuth 2.0 Client ID:"
  echo -e "   ${YELLOW}Application type:${NC} Web application"
  echo -e "   ${YELLOW}Name:${NC} Rushomon (${ENVIRONMENT_NAME})"
  echo ""
  echo "6. Add authorized redirect URI:"
  echo -e "   ${YELLOW}Authorized redirect URI:${NC} https://${API_DOMAIN}/api/auth/callback"
  echo ""
  echo "7. Click \"Create\""
  echo ""
  echo -e "8. Copy the ${GREEN}Client ID${NC} and ${GREEN}Client Secret${NC}"
  echo ""
  echo -e "${YELLOW}Note:${NC} Your app will be in \"Testing\" mode by default."
  echo "       Add test users or publish the app for public access."
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  read -p "Press Enter once you've created the OAuth client..."
}

# Configure GitHub OAuth interactively
configure_github_oauth() {
  info "Configuring GitHub OAuth..."

  if ! prompt_yes_no "Enable GitHub OAuth authentication?" "y"; then
    export GITHUB_ENABLED=false
    export GITHUB_CLIENT_ID=""
    export GITHUB_CLIENT_SECRET=""
    return 0
  fi

  export GITHUB_ENABLED=true

  show_github_oauth_instructions

  # Prompt for Client ID
  while true; do
    export GITHUB_CLIENT_ID=$(prompt_input "GitHub Client ID" "")

    if validate_github_client_id "$GITHUB_CLIENT_ID"; then
      break
    fi
    warning "Invalid Client ID format, please try again"
  done

  # Prompt for Client Secret
  while true; do
    export GITHUB_CLIENT_SECRET=$(prompt_secret "GitHub Client Secret")

    if validate_oauth_client_secret "GitHub" "$GITHUB_CLIENT_SECRET"; then
      break
    fi
    warning "Invalid Client Secret, please try again"
  done

  success "GitHub OAuth configured"
  return 0
}

# Configure Google OAuth interactively
configure_google_oauth() {
  info "Configuring Google OAuth..."

  if ! prompt_yes_no "Enable Google OAuth authentication?" "n"; then
    export GOOGLE_ENABLED=false
    export GOOGLE_CLIENT_ID=""
    export GOOGLE_CLIENT_SECRET=""
    return 0
  fi

  export GOOGLE_ENABLED=true

  show_google_oauth_instructions

  # Prompt for Client ID
  while true; do
    export GOOGLE_CLIENT_ID=$(prompt_input "Google Client ID" "")

    if validate_google_client_id "$GOOGLE_CLIENT_ID"; then
      break
    fi
    warning "Invalid Client ID format, please try again"
  done

  # Prompt for Client Secret
  while true; do
    export GOOGLE_CLIENT_SECRET=$(prompt_secret "Google Client Secret")

    if validate_oauth_client_secret "Google" "$GOOGLE_CLIENT_SECRET"; then
      break
    fi
    warning "Invalid Client Secret, please try again"
  done

  success "Google OAuth configured"
  return 0
}

# Configure all OAuth providers
configure_oauth() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}OAuth Provider Configuration${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo "Rushomon requires at least one OAuth provider for authentication."
  echo "You can enable GitHub, Google, or both."
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  # Configure GitHub OAuth (recommended)
  configure_github_oauth

  echo ""

  # Configure Google OAuth (optional)
  configure_google_oauth

  # Validate at least one provider is enabled
  if [ "$GITHUB_ENABLED" = false ] && [ "$GOOGLE_ENABLED" = false ]; then
    error "At least one OAuth provider must be enabled"
    info "Rushomon requires authentication to function"
    return 1
  fi

  echo ""
  success "OAuth configuration complete"

  if [ "$GITHUB_ENABLED" = true ]; then
    info "  - GitHub OAuth: Enabled"
  fi

  if [ "$GOOGLE_ENABLED" = true ]; then
    info "  - Google OAuth: Enabled"
  fi

  return 0
}

# Test OAuth callback URL
test_oauth_callback() {
  local base_url="$1"
  local provider="$2"

  info "Testing OAuth callback endpoint..."

  local callback_url="${base_url}/api/auth/callback"

  if ! command -v curl &>/dev/null; then
    warning "curl not found, skipping callback test"
    return 0
  fi

  local status=$(curl -s -o /dev/null -w "%{http_code}" "$callback_url" 2>/dev/null || echo "000")

  if [ "$status" = "000" ]; then
    warning "OAuth callback endpoint not reachable: $callback_url"
    info "This is expected before deployment"
    return 1
  elif [ "$status" = "400" ] || [ "$status" = "302" ]; then
    # 400 or 302 is expected (missing code/state parameters)
    success "OAuth callback endpoint is accessible"
    return 0
  else
    warning "OAuth callback returned unexpected status: HTTP $status"
    return 1
  fi
}

# Test OAuth redirect
test_oauth_redirect() {
  local base_url="$1"
  local provider="$2"

  info "Testing OAuth redirect endpoint..."

  local redirect_url="${base_url}/api/auth/${provider}"

  if ! command -v curl &>/dev/null; then
    warning "curl not found, skipping redirect test"
    return 0
  fi

  local status=$(curl -s -o /dev/null -w "%{http_code}" "$redirect_url" 2>/dev/null || echo "000")

  if [ "$status" = "302" ]; then
    success "OAuth redirect working correctly (HTTP 302)"
    return 0
  elif [ "$status" = "000" ]; then
    warning "OAuth redirect endpoint not reachable: $redirect_url"
    return 1
  else
    warning "OAuth redirect returned unexpected status: HTTP $status"
    return 1
  fi
}

# Validate OAuth configuration
validate_oauth_config() {
  info "Validating OAuth configuration..."

  local valid=true

  if [ "$GITHUB_ENABLED" = true ]; then
    if ! validate_github_client_id "$GITHUB_CLIENT_ID"; then
      valid=false
    fi

    if [ -z "$GITHUB_CLIENT_SECRET" ]; then
      error "GitHub client secret is empty"
      valid=false
    fi
  fi

  if [ "$GOOGLE_ENABLED" = true ]; then
    if ! validate_google_client_id "$GOOGLE_CLIENT_ID"; then
      valid=false
    fi

    if [ -z "$GOOGLE_CLIENT_SECRET" ]; then
      error "Google client secret is empty"
      valid=false
    fi
  fi

  if [ "$valid" = false ]; then
    error "OAuth configuration validation failed"
    return 1
  fi

  success "OAuth configuration is valid"
  return 0
}

# Show OAuth configuration summary
show_oauth_summary() {
  echo ""
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}OAuth Configuration Summary${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""

  if [ "$GITHUB_ENABLED" = true ]; then
    echo -e "${GREEN}GitHub OAuth:${NC} Enabled"
    echo "  Client ID: $GITHUB_CLIENT_ID"
    echo "  Callback URL: https://${API_DOMAIN}/api/auth/callback"
    echo ""
  fi

  if [ "$GOOGLE_ENABLED" = true ]; then
    echo -e "${GREEN}Google OAuth:${NC} Enabled"
    echo "  Client ID: $GOOGLE_CLIENT_ID"
    echo "  Callback URL: https://${API_DOMAIN}/api/auth/callback"
    echo ""
  fi

  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}
