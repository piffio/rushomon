# Rushomon Scripts Documentation

This directory contains automation scripts for Rushomon deployment, backup, and management.

## Table of Contents

- [Overview](#overview)
- [Setup Script](#setup-script)
- [Backup Script](#backup-script)
- [Restore Script](#restore-script)
- [Library Functions](#library-functions)
- [Configuration Files](#configuration-files)
- [CI/CD Integration](#cicd-integration)
- [Troubleshooting](#troubleshooting)

## Overview

Rushomon includes several scripts to simplify deployment and maintenance:

| Script | Purpose | Use Case |
|--------|---------|----------|
| `setup.sh` | Interactive deployment wizard | Deploy new instance or update existing |
| `backup.sh` | Database backup utility | Manual backups, scheduled backups |
| `restore.sh` | Database restore utility | Disaster recovery, environment cloning |
| `lib/*.sh` | Shared library functions | Used by other scripts |

## Setup Script

### Description

`setup.sh` is an interactive wizard that guides you through deploying Rushomon to Cloudflare Workers.

### Usage

**Interactive mode** (prompts for all configuration):
```bash
./scripts/setup.sh
```

**Config file mode** (non-interactive):
```bash
./scripts/setup.sh --config config/production.yaml
```

**Update existing deployment**:
```bash
./scripts/setup.sh --config config/production.yaml --update
```

**Dry run** (preview without deploying):
```bash
./scripts/setup.sh --config config/production.yaml --dry-run
```

**Help**:
```bash
./scripts/setup.sh --help
```

### Features

The setup script handles:
- ✓ Prerequisites checking (wrangler, Node.js, Rust, worker-build)
- ✓ Domain configuration (single, multi, or custom strategy)
- ✓ OAuth provider setup (GitHub, Google)
- ✓ Mailgun email setup (team invitations)
- ✓ Cloudflare resource creation (D1 database, KV namespace)
- ✓ Wrangler configuration generation
- ✓ Backend and frontend build
- ✓ Database migration
- ✓ Worker secrets configuration
- ✓ Worker deployment
- ✓ Smoke tests
- ✓ Configuration file saving

### Domain Strategies

**Single domain** (simplest):
- All services on one domain
- Example: `myapp.com` for everything

**Multi-domain** (recommended):
- Separate domains for different services
- Example: `myapp.com`, `api.myapp.com`, `go.myapp.com`

**Custom subdomains**:
- All subdomains of one parent domain
- Example: `app.myapp.com`, `api.myapp.com`, `links.myapp.com`

### Example: Staging Setup

```bash
# Create staging config
cat > config/staging.yaml <<EOF
domains:
  strategy: "single"
  main: "rushomon-staging.mydomain.com"
  api: "rushomon-staging.mydomain.com"
  redirect: "rushomon-staging.mydomain.com"

deployment:
  environment_name: "staging"
  worker_name: "rushomon-staging"

oauth:
  github:
    enabled: true
    client_id: "Iv1.abc123..."
    client_secret: "\${GITHUB_CLIENT_SECRET}"
EOF

# Set secrets
export GITHUB_CLIENT_SECRET="your-staging-secret"
export JWT_SECRET="$(openssl rand -base64 32)"

# Deploy
./scripts/setup.sh --config config/staging.yaml
```

### Example: Production Setup with Mailgun

```bash
# Create production config with Mailgun
cat > config/production.yaml <<EOF
domains:
  strategy: "multi"
  main: "myapp.com"
  api: "api.myapp.com"
  redirect: "go.myapp.com"

deployment:
  environment_name: "production"
  worker_name: "rushomon-production"

oauth:
  github:
    enabled: true
    client_id: "Iv1.abc123..."
    client_secret: "\${GITHUB_CLIENT_SECRET}"

mailgun:
  domain: "mg.myapp.com"
  base_url: "https://api.mailgun.net"
  from: "invites@mg.myapp.com"
  api_key: "\${MAILGUN_API_KEY}"
EOF

# Set secrets
export GITHUB_CLIENT_SECRET="your-production-secret"
export JWT_SECRET="$(openssl rand -base64 32)"
export MAILGUN_API_KEY="key-your-mailgun-api-key"

# Deploy
./scripts/setup.sh --config config/production.yaml
```

### MAILGUN Configuration

**Optional** - Required only for team invitation emails.

**Setup Process:**
1. Sign up at [mailgun.com](https://www.mailgun.com/) (free Flex plan works)
2. Add and verify your sending domain (e.g., `mg.myapp.com`)
3. Create an API key (starts with `key-`)
4. Configure during setup or in config file

**Config File Variables:**
```yaml
mailgun:
  domain: "mg.myapp.com"           # Verified sending domain
  base_url: "https://api.mailgun.net"  # Or EU: https://api.eu.mailgun.net
  from: "invites@mg.myapp.com"    # From address for emails
  api_key: "${MAILGUN_API_KEY}"   # API key secret
```

**Environment Variables:**
- `MAILGUN_API_KEY`: Your Mailgun API key (starts with `key-`)

**Note**: If Mailgun is not configured, team invitations will be created but email delivery will fail silently.

## Backup Script

### Description

`backup.sh` exports a D1 database to SQL format with optional compression and R2 upload.

### Usage

**Basic backup**:
```bash
./scripts/backup.sh
```

**Specify config file**:
```bash
./scripts/backup.sh -c wrangler.staging.toml
```

**Specify output file**:
```bash
./scripts/backup.sh -o my_backup.sql
```

**Compress with gzip**:
```bash
./scripts/backup.sh -z
```

**Upload to R2**:
```bash
./scripts/backup.sh -r rushomon-backups
```

**Generate metadata**:
```bash
./scripts/backup.sh -m
```

**Quiet mode** (for piping):
```bash
./scripts/backup.sh -q > backup.sql
```

**Combine options**:
```bash
./scripts/backup.sh -c wrangler.production.toml -z -r rushomon-backups -m
```

### Options

| Option | Description |
|--------|-------------|
| `-c, --config FILE` | Wrangler config file (default: wrangler.toml) |
| `-o, --output FILE` | Output SQL file (default: auto-generated timestamp) |
| `-r, --r2-bucket BUCKET` | Upload backup to R2 bucket |
| `-z, --compress` | Compress backup with gzip |
| `-q, --quiet` | Quiet mode (no progress, output to stdout) |
| `-m, --metadata` | Generate metadata file |
| `-h, --help` | Show help message |

### Output Format

**Filename format**: `rushomon_{env}_{YYYYMMDD_HHMMSS}.sql`

Examples:
- `rushomon_production_20260226_143022.sql`
- `rushomon_staging_20260226_143022.sql.gz` (compressed)

**Metadata file** (when `-m` is used):
```json
{
  "backup_file": "rushomon_production_20260226_143022.sql",
  "timestamp": "2026-02-26T14:30:22Z",
  "database_name": "rushomon",
  "config_file": "wrangler.production.toml",
  "file_size": 12345678,
  "row_counts": {
    "links": 1234,
    "users": 56,
    "organizations": 12
  }
}
```

### Examples

**Daily production backup**:
```bash
#!/bin/bash
# daily-backup.sh
DATE=$(date +%Y%m%d)
./scripts/backup.sh \
  -c wrangler.production.toml \
  -o "backups/prod_${DATE}.sql" \
  -z \
  -r rushomon-backups \
  -m
```

**Backup for migration**:
```bash
# Before running migrations
./scripts/backup.sh -o pre_migration_backup.sql
wrangler d1 migrations apply rushomon --remote
```

## Restore Script

### Description

`restore.sh` restores a D1 database from SQL backup with safety checks.

### Usage

**Interactive restore** (with confirmation):
```bash
./scripts/restore.sh backup.sql
```

**Force restore** (skip confirmation):
```bash
./scripts/restore.sh -f backup.sql
```

**Dry run** (preview only):
```bash
./scripts/restore.sh --dry-run backup.sql
```

**Restore with verification**:
```bash
./scripts/restore.sh -v backup.sql
```

**Restore to staging**:
```bash
./scripts/restore.sh -c wrangler.staging.toml staging_backup.sql
```

**Skip safety backup**:
```bash
./scripts/restore.sh --no-backup backup.sql
```

### Options

| Option | Description |
|--------|-------------|
| `-c, --config FILE` | Wrangler config file (default: wrangler.toml) |
| `-f, --force` | Force restore without confirmation |
| `-n, --dry-run` | Preview what would be restored |
| `-b, --backup` | Create safety backup before restore (default: true) |
| `--no-backup` | Skip safety backup creation |
| `-v, --verify` | Verify restoration by comparing row counts |
| `-h, --help` | Show help message |

### Safety Features

1. **Confirmation prompt**: Requires "yes" before proceeding (unless `-f`)
2. **Safety backup**: Creates backup of current state before restore
3. **Verification**: Compares row counts before/after (with `-v`)
4. **Dry run**: Preview restore without making changes
5. **Auto-decompress**: Handles `.gz` files automatically

### Examples

**Disaster recovery**:
```bash
# Restore from latest backup
./scripts/restore.sh -f latest_backup.sql
```

**Clone production to staging**:
```bash
# Backup production
./scripts/backup.sh -c wrangler.production.toml -o prod_clone.sql

# Restore to staging
./scripts/restore.sh -c wrangler.staging.toml -f prod_clone.sql
```

**Test restore**:
```bash
# Preview what would be restored
./scripts/restore.sh --dry-run backup.sql

# Restore with verification
./scripts/restore.sh -v backup.sql
```

## Library Functions

The `lib/` directory contains shared functions used by scripts:

### common.sh

Shared utilities:
- Color output functions (`info`, `success`, `error`, `warning`)
- User prompts (`prompt_yes_no`, `prompt_input`, `prompt_secret`)
- Config file loading/saving
- Logging utilities
- Error handling

### cloudflare.sh

Cloudflare API wrappers:
- Authentication checking (`check_wrangler_auth`)
- Account ID retrieval (`get_account_id`)
- D1 operations (`create_or_get_d1_database`)
- KV operations (`create_or_get_kv_namespace`)
- Worker deployment (`deploy_worker`)
- R2 operations (`create_or_get_r2_bucket`, `upload_to_r2`)

### validation.sh

Input validation:
- Domain format validation
- OAuth credential validation
- Secret strength validation
- URL reachability testing
- UUID validation

### oauth.sh

OAuth configuration:
- GitHub OAuth instructions
- Google OAuth instructions
- Interactive OAuth setup
- Credential validation

### deployment.sh

Deployment helpers:
- Wrangler config generation
- Backend/frontend build
- Migration application
- Secret management
- Smoke tests
- Deployment orchestration

## Configuration Files

### Setup Configuration

**Location**: `config/setup.yaml`

**Example** (see `config/setup.example.yaml`):
```yaml
domains:
  strategy: "multi"
  main: "myapp.com"
  api: "api.myapp.com"
  redirect: "go.myapp.com"

oauth:
  github:
    enabled: true
    client_id: "Iv1.abc123..."
    client_secret: "${GITHUB_CLIENT_SECRET}"

mailgun:
  domain: "mg.myapp.com"
  base_url: "https://api.mailgun.net"
  from: "invites@mg.myapp.com"
  api_key: "${MAILGUN_API_KEY}"

deployment:
  environment_name: "production"
  worker_name: "rushomon-production"

secrets:
  jwt_secret: "${JWT_SECRET}"
```

**Environment Variables**:
- `GITHUB_CLIENT_SECRET`: GitHub OAuth secret
- `GOOGLE_CLIENT_SECRET`: Google OAuth secret (if enabled)
- `JWT_SECRET`: JWT signing secret
- `MAILGUN_API_KEY`: Mailgun API key (for team invitations)

**Generate JWT secret**:
```bash
openssl rand -base64 32
```

### Wrangler Configuration

Generated by `setup.sh` or manually created.

**Location**: `wrangler.{environment}.toml`

Examples:
- `wrangler.staging.toml` - Staging environment
- `wrangler.production.toml` - Production environment

## CI/CD Integration

### GitHub Actions Usage

**Backup before staging deployment**:
```yaml
- name: Create backup
  run: |
    ./scripts/backup.sh -c wrangler.staging.toml -o backup.sql

- name: Upload backup artifact
  uses: actions/upload-artifact@v4
  with:
    name: staging-backup
    path: backup.sql
```

**Automated setup** (example):
```yaml
- name: Deploy to staging
  env:
    GITHUB_CLIENT_SECRET: ${{ secrets.GH_CLIENT_SECRET_STAGING }}
    JWT_SECRET: ${{ secrets.JWT_SECRET_STAGING }}
  run: |
    ./scripts/setup.sh --config config/staging.yaml
```

### Scheduled Backups

**GitHub Actions cron**:
```yaml
name: Daily Production Backup

on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM daily

jobs:
  backup:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Create backup
        run: |
          ./scripts/backup.sh \
            -c wrangler.production.toml \
            -z \
            -r rushomon-backups
```

## Troubleshooting

### Setup Script Issues

**Script fails to authenticate**:
```bash
wrangler logout
wrangler login
./scripts/setup.sh
```

**Missing prerequisites**:
```bash
# Check what's missing
which wrangler node cargo worker-build

# Install missing tools
npm install -g wrangler
cargo install worker-build
```

**Configuration not loading**:
```bash
# Verify YAML syntax
cat config/production.yaml | yq '.'

# Install yq if needed
brew install yq  # macOS
snap install yq  # Linux
```

**Domain validation fails**:
```bash
# Check if domain is in Cloudflare
wrangler zones list

# Verify domain format
echo "myapp.com" | grep -E '^[a-z0-9.-]+\.[a-z]{2,}$'
```

### Backup Script Issues

**Backup fails**:
```bash
# Check wrangler auth
wrangler whoami

# Verify config file
ls -la wrangler.toml

# Test database connection
wrangler d1 list
```

**R2 upload fails**:
```bash
# Check R2 bucket exists
wrangler r2 bucket list

# Create bucket if needed
wrangler r2 bucket create rushomon-backups
```

### Restore Script Issues

**Restore fails**:
```bash
# Check backup file
file backup.sql
head -10 backup.sql

# Test SQL syntax
sqlite3 :memory: < backup.sql
```

**Restore times out**:
```bash
# Split large backup into chunks
split -l 10000 backup.sql backup_chunk_

# Restore each chunk
for chunk in backup_chunk_*; do
  wrangler d1 execute rushomon --remote --file $chunk
done
```

### Library Function Issues

**Function not found**:
```bash
# Ensure libraries are sourced
source scripts/lib/common.sh
source scripts/lib/cloudflare.sh
```

**Permission denied**:
```bash
# Make scripts executable
chmod +x scripts/*.sh
chmod +x scripts/lib/*.sh
```

## Best Practices

1. **Test scripts locally** before using in CI/CD
2. **Use config files** for consistent deployments
3. **Store secrets securely** - use environment variables, never commit
4. **Create backups** before major changes
5. **Test restores regularly** to verify backup integrity
6. **Document customizations** if you modify scripts
7. **Use version control** for config files (without secrets)
8. **Monitor script output** for errors and warnings

## Contributing

When modifying scripts:

1. **Maintain backward compatibility** when possible
2. **Add help text** for new options
3. **Update documentation** in this file
4. **Test on multiple environments** (staging, production)
5. **Follow existing code style** (bash best practices)
6. **Add error handling** for edge cases
7. **Use existing library functions** instead of duplicating code

## Additional Resources

- [Deployment Guide](../docs/DEPLOYMENT.md)
- [Backup & Restore Guide](../docs/BACKUP_RESTORE.md)
- [Self-Hosting Guide](../docs/SELF_HOSTING.md)
- [Versioning Guide](../docs/VERSIONING.md)

## Getting Help

If you encounter issues with scripts:

1. Check the [Troubleshooting](#troubleshooting) section above
2. Review script help: `./scripts/script-name.sh --help`
3. Check [GitHub Issues](https://github.com/rushomon/rushomon/issues)
4. Enable debug mode: `DEBUG=true ./scripts/setup.sh`
