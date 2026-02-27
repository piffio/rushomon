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

Rushomon uses a multi-stage deployment pipeline:

### Development Phase

1. **Create feature branch**:
   ```bash
   git checkout -b feat/my-feature
   ```

2. **Develop and commit**:
   ```bash
   git add .
   git commit -m "Add feature"
   ```

3. **Create Pull Request**:
   ```bash
   git push origin feat/my-feature
   ```
   - Ephemeral environment created for testing
   - Review and test changes

4. **Merge to main**:
   - After approval, merge PR to main
   - Staging deployment triggered automatically

### Staging Validation

When you push to `main`:
1. Tests run automatically
2. On success, staging environment is deployed
3. Database backup created before deployment
4. Changes go live at staging URL

**Validate on staging**:
- Test all new features thoroughly
- Verify database migrations applied correctly
- Check OAuth flows work
- Test on multiple devices/browsers

### Production Release

When staging validation is complete:

1. **Bump version** (on main branch):
   ```bash
   make version-bump-patch  # 0.1.0 → 0.1.1
   # or
   make version-bump-minor  # 0.1.0 → 0.2.0
   # or
   make version-bump-major  # 0.1.0 → 1.0.0
   ```

2. **Push changes and tags**:
   ```bash
   git push origin main
   git push origin --tags
   ```

3. **Production deployment** triggers automatically:
   - Triggered by the version tag (e.g., `v0.2.0`)
   - Deploys to production environment
   - Smoke tests run automatically

**Quick release** (combines bump + tag + push):
```bash
make release
```

### Deployment Pipeline Summary

```
Development → Pull Request → Main → Staging → Tag → Production
                   ↓           ↓        ↓        ↓
              Ephemeral    Staging  Validate  Prod
```

- **Ephemeral**: Created per PR, destroyed when closed
- **Staging**: Deployed on push to main (after tests)
- **Production**: Deployed on version tags only

### Important Notes

- **Production** only deploys on release tags (v*.*.*)
- **Never** push directly to production without staging validation
- **Always** test on staging before creating a release tag
- Create **backups** before major releases:
  ```bash
  ./scripts/backup.sh -c wrangler.production.toml -r rushomon-backups
  ```

## Self-Hosting Updates

For self-hosted users:
- **Stable releases**: `git checkout v0.1.0` (use version tags)
- **Latest features**: `git pull origin main` (use main branch)

Always verify version after updates using the `/api/version` endpoint.
