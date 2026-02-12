#!/bin/bash

# Default configuration for Rushomon repo hooks
# This file contains team-wide default settings
# Copy to user.sh to override for personal preferences

# Unit tests - always required for code quality
UNIT_TESTS_ENABLED=true
UNIT_TESTS_COMMAND="cargo test --lib"
UNIT_TESTS_REQUIRED=true
UNIT_TESTS_ERROR_MSG="Unit tests failed! Please fix failing tests before committing."

# Code formatting - optional but recommended
CODE_FORMATTING_ENABLED=true
CODE_FORMATTING_COMMAND="cargo fmt -- --check"
CODE_FORMATTING_REQUIRED=false
CODE_FORMATTING_ERROR_MSG="Code formatting issues! Run 'cargo fmt' to fix."
AUTO_FORMAT_AND_STAGE=false

# Clippy linting - optional but recommended
CLIPPY_ENABLED=true
CLIPPY_COMMAND="cargo clippy -- -D warnings"
CLIPPY_REQUIRED=true
CLIPPY_ERROR_MSG="Clippy found issues! Address warnings before committing."

# Tool requirements
CARGO_REQUIRED=true
RUSTFMT_REQUIRED=false
CLIPPY_REQUIRED=true

# Tool installation instructions
CARGO_INSTALL_MSG="Install Rust: https://rustup.rs/"
RUSTFMT_INSTALL_MSG="Install rustfmt: rustup component add rustfmt"
CLIPPY_INSTALL_MSG="Install clippy: rustup component add clippy"

# Hook settings
PRE_COMMIT_TIMEOUT_SECONDS=300
PRE_COMMIT_VERBOSE=true
