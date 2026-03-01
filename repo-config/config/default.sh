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
SVELTE_CHECK_COMMAND="cd frontend && (git diff --cached --name-only | grep -E '^frontend/' >/dev/null && npm run check || echo 'No frontend changes detected')"
SVELTE_CHECK_REQUIRED=true
SVELTE_CHECK_ERROR_MSG="Svelte/TypeScript issues found! Please fix errors and warnings before committing."

# Tool requirements
CARGO_REQUIRED=true
RUSTFMT_REQUIRED=true
CLIPPY_REQUIRED=true
NODE_REQUIRED=true

# Tool installation instructions
CARGO_INSTALL_MSG="Install Rust: https://rustup.rs/"
RUSTFMT_INSTALL_MSG="Install rustfmt: rustup component add rustfmt"
CLIPPY_INSTALL_MSG="Install clippy: rustup component add clippy"
NODE_INSTALL_MSG="Install Node.js: https://nodejs.org/ or use your system package manager"

# Hook settings
PRE_COMMIT_TIMEOUT_SECONDS=300
PRE_COMMIT_VERBOSE=true
