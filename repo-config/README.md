# Rushomon Repo Configuration

This directory contains a shareable, version-controlled system for managing git hooks and repository configuration.

## 🚀 Quick Start

### For New Developers
```bash
# Clone the repository
git clone <repository-url>
cd rushomon

# One-time setup
./repo-config/scripts/setup.sh

# Done! Your hooks are now active
git add .
git commit -m "my first commit"  # Tests run automatically
```

### For Updates
```bash
# When hooks are updated in the repo
./repo-config/scripts/setup.sh --update
```

## 📁 Directory Structure

```
repo-config/
├── hooks/
│   └── pre-commit          # Enhanced pre-commit hook
├── scripts/
│   └── setup.sh            # Installation and management script
├── config/
│   ├── default.sh          # Team-wide default settings
│   └── user.sh.example     # Template for personal overrides
└── README.md               # This file
```

## ⚙️ Configuration

### Default Configuration (`config/default.sh`)
Contains team-wide settings that apply to everyone:

- **Unit Tests**: Always required (`cargo test --lib`)
- **Code Formatting**: Optional but recommended (`cargo fmt -- --check`)
- **Clippy Linting**: Optional but recommended (`cargo clippy -- -D warnings`)
- **Tool Requirements**: Which tools are required vs optional
- **Timeout Settings**: How long checks can run
- **Error Messages**: Customizable error messages

### Personal Configuration (`config/user.sh`)
Create your own overrides:

```bash
# Copy the template
cp repo-config/config/user.sh.example repo-config/config/user.sh

# Edit your preferences
nano repo-config/config/user.sh
```

#### Common Customizations

**Disable formatting checks:**
```bash
CODE_FORMATTING_ENABLED=false
CODE_FORMATTING_REQUIRED=false
```

**Make clippy block commits:**
```bash
CLIPPY_REQUIRED=true
```

**Use faster tests for development:**
```bash
UNIT_TESTS_COMMAND="cargo test --lib --release"
```

**Add strict clippy rules:**
```bash
CLIPPY_COMMAND="cargo clippy -- -D warnings -W clippy::unwrap_used"
```

## 🎯 Pre-commit Hook Features

### Checks Performed

1. **Unit Tests** (`cargo test --lib`)
   - Always required by default
   - Ensures code compiles and tests pass
   - Fast feedback on breaking changes

2. **Code Formatting** (`cargo fmt -- --check`)
   - Optional by default
   - Ensures consistent code style
   - Can be disabled if you format manually

3. **Clippy Linting** (`cargo clippy -- -D warnings`)
   - Optional by default
   - Catches common Rust issues
   - Configurable warning levels

### Behavior

- **Required checks**: Block commit if they fail
- **Optional checks**: Show warning but allow commit
- **Missing tools**: Clear installation instructions
- **Verbose output**: Detailed progress information

## 🔧 Setup Script Options

```bash
./repo-config/scripts/setup.sh          # Standard setup
./repo-config/scripts/setup.sh --check  # Verify installation
./repo-config/scripts/setup.sh --update # Force update hooks
./repo-config/scripts/setup.sh --help   # Show help
```

### What Setup Does

1. **Validates Environment**: Checks git repository, file structure
2. **Backs Up Existing Hooks**: Preserves any existing hooks
3. **Installs New Hook**: Copies enhanced pre-commit hook
4. **Sets Permissions**: Makes hook executable
5. **Tests Installation**: Verifies hook works
6. **Shows Configuration**: Displays current setup

## 🚨 Troubleshooting

### Hook Not Running
```bash
# Check if hook is installed
./repo-config/scripts/setup.sh --check

# Manual verification
ls -la .git/hooks/pre-commit
```

### Permission Errors
```bash
# Fix permissions
chmod +x .git/hooks/pre-commit
chmod +x repo-config/scripts/setup.sh
```

### Missing Tools
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install components
rustup component add rustfmt clippy
```

### Configuration Issues
```bash
# Test configuration loading
source repo-config/config/default.sh
echo $UNIT_TESTS_ENABLED
```

### Hook Fails Unexpectedly
```bash
# Run hook manually for debugging
.git/hooks/pre-commit

# Skip hook temporarily (not recommended)
git commit --no-verify -m "emergency commit"
```

## 🔄 Updating Hooks

When the repository updates its hooks:

1. **Pull latest changes**: `git pull`
2. **Re-run setup**: `./repo-config/scripts/setup.sh`
3. **Review config**: Check if any new settings were added
4. **Test commit**: Verify everything works

## 🎨 Customization Examples

### Strict Development Setup
```bash
# user.sh
CLIPPY_REQUIRED=true
CLIPPY_COMMAND="cargo clippy -- -D warnings -W clippy::unwrap_used -W clippy::expect_used"
CODE_FORMATTING_REQUIRED=true
```

### Fast Development Setup
```bash
# user.sh
UNIT_TESTS_COMMAND="cargo test --lib --release --quiet"
CODE_FORMATTING_ENABLED=false
CLIPPY_ENABLED=false
PRE_COMMIT_VERBOSE=false
```

### CI/CD Preparation Setup
```bash
# user.sh
CLIPPY_REQUIRED=true
CODE_FORMATTING_REQUIRED=true
UNIT_TESTS_COMMAND="cargo test --lib --all-features"
```

## 📝 Contributing

When modifying the repo configuration:

1. **Test changes**: Verify hooks work with different configurations
2. **Update documentation**: Keep this README current
3. **Consider defaults**: Choose sensible team-wide defaults
4. **Maintain compatibility**: Don't break existing user configurations

## 🆘 Getting Help

- **Setup issues**: Run `./repo-config/scripts/setup.sh --help`
- **Configuration**: Check `config/default.sh` for available options
- **Hook problems**: Look at hook output for specific error messages
- **Tool issues**: Follow installation instructions in error messages

## 🎯 Best Practices

1. **Commit the configuration**: All changes to hooks should be committed
2. **Document customizations**: Comment your personal configuration changes
3. **Test regularly**: Verify hooks work after major changes
4. **Keep tools updated**: Maintain Rust toolchain components
5. **Share improvements**: Contribute useful configuration changes back to the team
