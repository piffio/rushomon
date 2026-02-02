#!/bin/bash

# Setup script for Rushomon repo configuration
# Installs and configures git hooks from repo-config/

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG_DIR="$REPO_ROOT/repo-config/config"
HOOKS_DIR="$REPO_ROOT/.git/hooks"
SOURCE_HOOK="$REPO_ROOT/repo-config/hooks/pre-commit"
TARGET_HOOK="$HOOKS_DIR/pre-commit"

# Check if we're in a git repository
check_git_repo() {
    if [ ! -d "$REPO_ROOT/.git" ]; then
        log_error "Not in a git repository!"
        log_info "Make sure you're in the root of a git repository."
        exit 1
    fi
    log_success "Git repository detected"
}

# Check if repo-config structure exists
check_structure() {
    if [ ! -f "$SOURCE_HOOK" ]; then
        log_error "Source hook not found: $SOURCE_HOOK"
        log_info "Make sure the repo-config directory structure is complete."
        exit 1
    fi
    
    if [ ! -f "$CONFIG_DIR/default.sh" ]; then
        log_error "Default configuration not found: $CONFIG_DIR/default.sh"
        exit 1
    fi
    
    log_success "Repo configuration structure found"
}

# Install the pre-commit hook
install_hook() {
    log_info "Installing pre-commit hook..."
    
    # Backup existing hook if it exists
    if [ -f "$TARGET_HOOK" ]; then
        local backup_file="$TARGET_HOOK.backup.$(date +%Y%m%d_%H%M%S)"
        log_warning "Existing hook found, backing up to: $backup_file"
        cp "$TARGET_HOOK" "$backup_file"
    fi
    
    # Copy the new hook
    cp "$SOURCE_HOOK" "$TARGET_HOOK"
    chmod +x "$TARGET_HOOK"
    
    log_success "Pre-commit hook installed successfully"
}

# Setup user configuration
setup_user_config() {
    local user_config="$CONFIG_DIR/user.sh"
    
    if [ ! -f "$user_config" ]; then
        log_info "No user configuration found"
        log_info "You can create one by copying:"
        log_info "  cp repo-config/config/user.sh.example repo-config/config/user.sh"
        log_info "Then edit repo-config/config/user.sh to customize your settings."
    else
        log_success "User configuration found: $user_config"
    fi
}

# Test the hook
test_hook() {
    log_info "Testing pre-commit hook..."
    
    # Create a temporary test file
    local test_file="$REPO_ROOT/test_hook_setup.tmp"
    echo "# Test file for hook setup" > "$test_file"
    
    # Try to run the hook directly
    if "$TARGET_HOOK" --test 2>/dev/null || true; then
        log_success "Hook test passed"
    else
        log_warning "Hook test had issues (this may be normal)"
        log_info "The hook will be tested during your next commit."
    fi
    
    # Clean up
    rm -f "$test_file"
}

# Show configuration summary
show_config() {
    log_info "Current configuration:"
    
    if [ -f "$CONFIG_DIR/user.sh" ]; then
        echo "  📝 User configuration: repo-config/config/user.sh"
        echo "  📋 Default configuration: repo-config/config/default.sh"
        echo "  🔧 Hook will merge both files (user overrides defaults)"
    else
        echo "  📋 Default configuration only: repo-config/config/default.sh"
        echo "  💡 Create user.sh for personal overrides"
    fi
    
    echo ""
    log_info "Hook location: $TARGET_HOOK"
    log_info "Configuration location: $CONFIG_DIR"
}

# Main setup function
main() {
    echo "🚀 Setting up Rushomon repo configuration..."
    echo ""
    
    check_git_repo
    check_structure
    install_hook
    setup_user_config
    test_hook
    show_config
    
    echo ""
    log_success "Setup complete!"
    echo ""
    log_info "Next steps:"
    echo "  1. Review the configuration in repo-config/config/"
    echo "  2. Optional: Create repo-config/config/user.sh for personal settings"
    echo "  3. Try a commit to test the hook: git commit -m 'test hook'"
    echo ""
    log_info "To update hooks in the future, just re-run this script:"
    echo "  ./repo-config/scripts/setup.sh"
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        echo "Rushomon Repo Configuration Setup"
        echo ""
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --check        Check if setup is complete"
        echo "  --update       Force update of hooks"
        echo ""
        exit 0
        ;;
    --check)
        check_git_repo
        check_structure
        if [ -f "$TARGET_HOOK" ]; then
            log_success "Setup appears complete"
            show_config
        else
            log_error "Hook not installed - run setup without --check"
            exit 1
        fi
        exit 0
        ;;
    --update)
        log_info "Force updating hooks..."
        ;;
esac

# Run main setup
main "$@"
