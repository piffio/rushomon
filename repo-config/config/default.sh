#!/bin/bash

# Default configuration for Rushomon repo hooks
# This file contains team-wide default settings
# Copy to user.sh to override for personal preferences

# Unit tests - always required for code quality
UNIT_TESTS_ENABLED=true
UNIT_TESTS_COMMAND="cargo test --lib"
UNIT_TESTS_REQUIRED=true
UNIT_TESTS_ERROR_MSG="Unit tests failed! Please fix failing tests before committing."

# Code formatting - required for code quality consistency
CODE_FORMATTING_ENABLED=true
CODE_FORMATTING_COMMAND="cargo fmt -- --check"
CODE_FORMATTING_REQUIRED=true
CODE_FORMATTING_ERROR_MSG="Code formatting issues! Make sure to stage the formatted files before committing."
AUTO_FORMAT_AND_STAGE=false

# Clippy linting - optional but recommended
CLIPPY_ENABLED=true
CLIPPY_COMMAND="cargo clippy -- -D warnings"
CLIPPY_REQUIRED=true
CLIPPY_ERROR_MSG="Clippy found issues! Address warnings before committing."

# Svelte/TypeScript checking - required for frontend quality
SVELTE_CHECK_ENABLED=true
SVELTE_CHECK_COMMAND="if git diff --cached --name-only | grep -E '^frontend/' >/dev/null; then cd frontend && npm run check; else echo 'No frontend changes detected'; fi"
SVELTE_CHECK_REQUIRED=true
SVELTE_CHECK_ERROR_MSG="Svelte/TypeScript issues found! Please fix errors and warnings before committing."

# ESLint checking - required for frontend code quality
ESLINT_CHECK_ENABLED=true
ESLINT_CHECK_COMMAND="run_eslint_check"
ESLINT_CHECK_REQUIRED=true
ESLINT_CHECK_ERROR_MSG="ESLint issues found! Please fix linting errors before committing."
AUTO_FORMAT_ESLINT_AND_STAGE=false

# Frontend formatting - required for frontend code consistency
FRONTEND_FORMATTING_ENABLED=true
FRONTEND_FORMATTING_COMMAND="check_frontend_formatting"
FRONTEND_FORMATTING_REQUIRED=true
FRONTEND_FORMATTING_ERROR_MSG="Frontend formatting issues! Make sure to stage the formatted files before committing."
AUTO_FORMAT_FRONTEND_AND_STAGE=false

# OpenAPI spec validation - optional helper (PR check is primary guard)
OPENAPI_CHECK_ENABLED=false
OPENAPI_CHECK_COMMAND="git diff --cached --name-only | grep -E '^(src/|Cargo.toml)' >/dev/null && (if [ -f docs/openapi/main.json ]; then ./scripts/generate-openapi.sh 2>/dev/null && (git diff --quiet docs/openapi/main.json || (echo '⚠️  OpenAPI spec may be out of date!' && echo '💡 PR will fail if spec is not updated' && echo 'Run: ./scripts/generate-openapi.sh && git add docs/openapi/main.json')); else echo 'ℹ️  No OpenAPI spec exists yet'; fi) || echo 'No Rust changes detected'"
OPENAPI_CHECK_REQUIRED=false
OPENAPI_CHECK_ERROR_MSG="OpenAPI spec may be out of date! PR check will enforce this."

# Tool requirements
CARGO_REQUIRED=true
RUSTFMT_REQUIRED=true
CLIPPY_REQUIRED=true
NODE_REQUIRED=true
PRETTIER_REQUIRED=true
ESLINT_REQUIRED=true

# Tool installation instructions
CARGO_INSTALL_MSG="Install Rust: https://rustup.rs/"
RUSTFMT_INSTALL_MSG="Install rustfmt: rustup component add rustfmt"
CLIPPY_INSTALL_MSG="Install clippy: rustup component add clippy"
NODE_INSTALL_MSG="Install Node.js: https://nodejs.org/ or use your system package manager"
PRETTIER_INSTALL_MSG="Install Prettier: cd frontend && npm install"
ESLINT_INSTALL_MSG="Install ESLint: cd frontend && npm install"

# Hook settings
PRE_COMMIT_TIMEOUT_SECONDS=300
PRE_COMMIT_VERBOSE=true
