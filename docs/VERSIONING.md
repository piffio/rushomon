# Version Management

This document explains how to manage versions in Rushomon using the provided Makefile targets.

## Overview

Rushomon uses `Cargo.toml` as the single source of truth for version numbers. The version is automatically synchronized to `frontend/package.json` during build.

## Makefile Targets

### View Current Version
```bash
make version
```
Shows the current version from Cargo.toml.

### Bump Versions
```bash
# Patch version (0.1.0 → 0.1.1)
make version-bump-patch

# Minor version (0.1.0 → 0.2.0)
make version-bump-minor

# Major version (0.1.0 → 1.0.0)
make version-bump-major
```

Each bump command:
- Updates Cargo.toml version
- Synchronizes frontend/package.json
- Shows the new version

### Manual Sync
```bash
make version-sync
```
Manually synchronize Cargo.toml version to frontend/package.json.

### Create Release Tag
```bash
make version-tag
```
Creates a git tag for the current version (e.g., `v0.1.0`).

### Full Release Process
```bash
make release
```
Performs a complete release:
1. Bumps patch version
2. Synchronizes frontend version
3. Creates git tag
4. Pushes changes and tags to remote

## Version Information

### Runtime Version API
```bash
curl https://your-domain.com/api/version
```

Returns:
```json
{
  "version": "0.1.0",
  "name": "rushomon",
  "build_timestamp": "2024-02-23T22:30:00Z",
  "git_commit": "abc123def"
}
```

### Frontend Version
The frontend version utility is available in `src/lib/version.ts`:
```typescript
import { getVersionInfo } from '$lib/version';

console.log(getVersionInfo());
// { version: "0.1.0", name: "Rushomon", isProduction: true }
```

## Release Workflow

1. **Development**: Work on `main` branch
2. **Release**: Run `make release` or manual steps:
   - `make version-bump-patch` (or minor/major)
   - `make version-tag`
   - `git push && git push --tags`
3. **Deployment**: GitHub Actions deploy from `main` automatically

## Self-Hosting Updates

For self-hosted users:
- **Stable releases**: `git checkout v0.1.0` (use version tags)
- **Latest features**: `git pull origin main` (use main branch)

Always verify version after updates using the `/api/version` endpoint.
