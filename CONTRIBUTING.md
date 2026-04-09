# Contributing to Rushomon

Thank you for your interest in contributing to Rushomon! This document outlines how to set up your development environment, our code style guidelines, and how to submit contributions.

## Table of Contents

- [Development Setup](#development-setup)
- [Code Style](#code-style)
  - [Backend (Rust)](#backend-rust)
  - [Frontend (SvelteKit)](#frontend-sveltekit)
- [Pre-commit Hook](#pre-commit-hook)
- [Submitting Pull Requests](#submitting-pull-requests)
- [IDE Setup](#ide-setup)

## Development Setup

For the full development environment setup, see the [README.md](./README.md). Quick start:

```bash
# Clone and setup
git clone git@github.com:piffio/rushomon.git
cd rushomon
./repo-config/scripts/setup.sh

# Backend (Rust)
rustup target add wasm32-unknown-unknown
cargo install worker-build

# Frontend (SvelteKit)
cd frontend && npm install
```

## Code Style

### Backend (Rust)

We use standard Rust tooling for code quality:

- **Formatting**: `cargo fmt`
- **Linting**: `cargo clippy -- -D warnings`

All warnings are treated as errors in CI.

### Frontend (SvelteKit)

We use ESLint + Prettier with the following configuration:

**ESLint configuration** (`frontend/eslint.config.js`):
- Base: ESLint recommended rules
- TypeScript: `@typescript-eslint/recommended`
- Svelte: `eslint-plugin-svelte` with `flat/recommended` (Svelte 5 runes support)
- Prettier integration: `eslint-config-prettier` + `eslint-plugin-prettier`
- Custom rules:
  - `prefer-const`: error
  - `no-var`: error
  - `@typescript-eslint/no-explicit-any`: warn
  - `@typescript-eslint/no-unused-vars`: error (args starting with `_` allowed)
  - `svelte/no-at-html-tags`: off

**Prettier configuration** (`frontend/.prettierrc`):
- 2-space indentation
- Semicolons enabled
- Double quotes
- No trailing commas
- 80-character print width
- Svelte plugin with sorting: options → scripts → markup → styles

**NPM scripts** (run from `frontend/`):

```bash
# Check for issues
npm run lint          # Run ESLint
npm run format:check  # Check Prettier formatting

# Fix issues
npm run lint:fix      # Auto-fix ESLint issues
npm run format        # Format with Prettier
```

## Pre-commit Hook

The pre-commit hook runs automatically on every commit. To install/update it:

```bash
./repo-config/scripts/setup.sh
```

### What the Hook Checks

1. **Unit tests**: `cargo test --lib`
2. **Rust formatting**: `cargo fmt --check`
3. **Clippy linting**: `cargo clippy -- -D warnings`
4. **Svelte/TypeScript checking**: `svelte-check` (only if frontend files changed)
5. **ESLint checking**: `npm run lint` (only if frontend files changed)
6. **Frontend formatting**: `npm run format:check` (only if frontend files changed)

### Auto-fix Options

You can enable auto-fix and auto-stage in `repo-config/config/user.sh`:

```bash
# Auto-format Rust and stage changes
AUTO_FORMAT_AND_STAGE=true

# Auto-format frontend and stage changes
AUTO_FORMAT_FRONTEND_AND_STAGE=true

# Auto-fix ESLint and stage changes
AUTO_FORMAT_ESLINT_AND_STAGE=true
```

Copy from the example file to create your user config:

```bash
cp repo-config/config/user.sh.example repo-config/config/user.sh
```

## Submitting Pull Requests

1. **Discuss first**: Before starting work, either:
   - Find an existing issue and ask for it to be assigned to you, or
   - Create a new issue with a detailed proposal for discussion

   PRs without a confirmed linked issue may be discarded to avoid wasted effort.

2. Create a new branch for your feature/fix
3. Make your changes with clear, focused commits
4. Ensure the pre-commit hook passes locally
5. Push your branch and open a PR against `main`

The CI workflow (`.github/workflows/test.yml`) will run:
- Rustfmt check
- Clippy linting
- Unit tests
- **Frontend ESLint + Prettier** (new!)
- Integration tests

All checks must pass before merging.

## IDE Setup

While `.vscode` is gitignored, we recommend the following VSCode/Windsurf configuration:

### Extensions

Install these extensions for the best development experience:

- **Svelte**: `svelte.svelte-vscode`
- **ESLint**: `dbaeumer.vscode-eslint`
- **Prettier**: `esbenp.prettier-vscode`

### Settings

Add to your `.vscode/settings.json`:

```json
{
  "editor.defaultFormatter": "vscode.json-language-features",
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll.eslint": "explicit",
    "source.organizeImports": "explicit"
  },
  "eslint.validate": [
    "javascript",
    "javascriptreact",
    "typescript",
    "typescriptreact",
    "svelte"
  ],
  "eslint.workingDirectories": ["frontend"],
  "svelte.enable-ts-plugin": true,
  "[svelte]": {
    "editor.defaultFormatter": "svelte.svelte-vscode"
  },
  "[javascript]": {
    "editor.defaultFormatter": "vscode.json-language-features"
  },
  "[typescript]": {
    "editor.defaultFormatter": "vscode.json-language-features"
  },
  "[json]": {
    "editor.defaultFormatter": "vscode.json-language-features"
  }
}
```

This configuration ensures:
- Files are formatted on save using appropriate formatters
- ESLint auto-fixes issues on save
- Import statements are organized automatically
- Svelte files get proper TypeScript support
- Each file type uses the best available formatter

## Questions?

Open an issue on GitHub if you have questions about contributing.
