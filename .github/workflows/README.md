# GitHub Actions Workflows

This directory contains all CI/CD workflows for the Rushomon project.

## Workflows Overview

### 1. test.yml
**Purpose**: Run unit and integration tests on all branches

**Trigger**: 
- Push to any branch
- Pull requests

**Jobs**:
- `unit-tests`: Runs `cargo test --lib`
- `integration-tests`: Runs `cargo test --test '*'` with local D1

**Duration**: ~15 minutes

**Status**: Required for PR merge

**Key Features**:
- Rust caching for faster builds
- npm caching for dependencies
- Local D1 database for integration tests
- Sequential test execution (`--test-threads=1`)

### 2. deploy-ephemeral.yml
**Purpose**: Deploy PR changes to ephemeral staging environment

**Trigger**:
- Pull requests (with smart conditions)
- Only when code files change (not docs)
- Only for ready (non-draft) PRs
- Only for PRs from this repo (not forks)
- Unless `skip-preview` label is present

**Jobs**:
- Determines PR number from branch/commit
- Creates D1 database: `rushomon-pr-{PR_NUMBER}`
- Creates KV namespace: `URL_MAPPINGS-pr-{PR_NUMBER}`
- Deploys backend to: `rushomon-pr-{PR_NUMBER}.workers.dev`
- Deploys frontend to: `pr-{PR_NUMBER}.rushomon-ui.pages.dev`
- Runs smoke tests on both frontend and backend
- Posts deployment URLs in PR comments
- Stores deployment info for cleanup

**Duration**: ~10 minutes

**Status**: Informational (doesn't block merge)

**Key Features**:
- Isolated D1 database per PR
- Isolated KV namespace per PR
- Isolated Cloudflare Pages deployment per PR
- Frontend and backend fully integrated with CORS
- Automatic PR comments with both frontend and backend URLs
- Smoke tests verify both deployments
- Smart deployment controls (draft, labels, file filters)
- Fork protection for security
- Deployment info stored for cleanup

**How to Control Deployment**:

✅ **Enable Deployment** (automatic when):
- PR is from this repository (not a fork)
- PR is marked as "Ready for review" (not draft)
- PR doesn't have `skip-preview` label
- Changes include code files (src/, tests/, Cargo.toml, etc.)

⏭️ **Skip Deployment** (use any method):
1. **Draft Mode**: Create PR as draft or mark as draft later
   ```bash
   gh pr ready --undo <pr-number>
   ```
2. **Add Label**: Add `skip-preview` label to PR
   ```bash
   gh pr edit <pr-number> --add-label "skip-preview"
   ```
3. **Docs Only**: If you only change `.md` files, deployment is auto-skipped

🔄 **Re-enable Deployment**:
- Mark PR as "Ready for review" if draft
- Remove `skip-preview` label if present
- Add code changes if only docs were modified

### 3. deploy-production.yml
**Purpose**: Deploy main branch to production with custom domains

**Trigger**:
- Push to main branch

**Jobs**:
1. `wait-for-tests`: Waits for test workflow to pass
2. `deploy-backend`: Builds Worker, generates `wrangler.production.toml`, applies D1 migrations to `rushomon`, deploys to `rush.mn`, sets worker secrets
3. `deploy-frontend`: Builds SvelteKit frontend, deploys to Cloudflare Pages, attaches custom domain `rushomon.cc`
4. `smoke-tests`: Read-only health checks (no DB/KV mutations)
5. `notifications`: Commit comments with success/failure status

**Duration**: ~15 minutes

**Status**: Gated by `production` environment protection rules

**Key Features**:
- Waits for tests to pass before deploying
- Separate backend and frontend deployment jobs
- Generates `wrangler.production.toml` at deploy time from environment-scoped secrets
- Custom domain `rush.mn` auto-attached to Worker via `[[routes]]` config
- Custom domain `rushomon.cc` auto-attached to Pages via Cloudflare API (idempotent)
- Read-only smoke tests (worker health, frontend accessibility, auth enforcement)
- No database mutations in production smoke tests
- Worker secrets set via Cloudflare API
- Deployment notifications via commit comments
- Concurrency control prevents overlapping deploys

### 4. cleanup-ephemeral.yml
**Purpose**: Clean up ephemeral resources when PR closes

**Trigger**:
- Pull request closed event

**Jobs**:
- Extracts PR number
- Deletes Cloudflare Pages deployment
- Deletes Worker
- Deletes KV namespace
- Deletes D1 database
- Posts cleanup notification with all resource statuses

**Duration**: ~3 minutes

**Status**: Automatic (no approval needed)

**Key Features**:
- Automatic cleanup on PR close
- Handles already-deleted resources gracefully
- Posts confirmation in PR comments
- Cost-effective (no orphaned resources)

## Secrets Required

Secrets are scoped to GitHub Environments. Each environment has its own set of secrets with the **same variable names** but different values.

### Ephemeral Environment (`ephemeral`)

| Secret | Purpose |
|--------|--------|
| `CLOUDFLARE_API_TOKEN` | Authenticate with Cloudflare API (ephemeral account) |
| `CLOUDFLARE_ACCOUNT_ID` | Cloudflare account ID (ephemeral account) |
| `WORKERS_DOMAIN` | Workers subdomain for ephemeral deployments |
| `GH_CLIENT_ID` | GitHub OAuth app client ID |
| `GH_CLIENT_SECRET` | GitHub OAuth app client secret |
| `JWT_SECRET` | JWT signing secret (32+ chars) |

### Production Environment (`production`)

| Secret | Purpose |
|--------|--------|
| `CLOUDFLARE_API_TOKEN` | Authenticate with Cloudflare API (production account) |
| `CLOUDFLARE_ACCOUNT_ID` | Cloudflare account ID (production account) |
| `D1_DATABASE_ID` | Pre-created D1 database ID (`rushomon`) |
| `KV_NAMESPACE_ID` | Pre-created KV namespace ID |
| `GH_CLIENT_ID` | GitHub OAuth app client ID (production) |
| `GH_CLIENT_SECRET` | GitHub OAuth app client secret (production) |
| `JWT_SECRET` | JWT signing secret (32+ chars) |
| `DOMAIN` | API/short domain (e.g., `rush.mn`) |
| `FRONTEND_URL` | Frontend URL (e.g., `https://rushomon.cc`) |

See `docs/SELF_HOSTING.md` for detailed setup instructions.

## Workflow Execution Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Push to any branch                                          │
└─────────────────────────────────────────────────────────────┘
                              ↓
                    ┌─────────────────┐
                    │  test.yml       │
                    │  (all branches) │
                    └─────────────────┘
                              ↓
                    ┌─────────────────────────────────────┐
                    │ Tests pass?                         │
                    └─────────────────────────────────────┘
                         ↙              ↘
                    YES                  NO
                     ↓                    ↓
        ┌──────────────────────┐  ❌ Merge blocked
        │ Is main branch?      │
        └──────────────────────┘
             ↙              ↘
           YES              NO
            ↓                ↓
   ┌──────────────────┐  ┌──────────────────────┐
   │ deploy-          │  │ deploy-ephemeral.yml │
   │ production.yml   │  │ (PR staging env)     │
   │ (with approval)  │  └──────────────────────┘
   └──────────────────┘           ↓
            ↓              ┌──────────────────────┐
   ✅ Production live     │ PR comment with URL  │
                          └──────────────────────┘
                                   ↓
                          ┌──────────────────────┐
                          │ PR closed?           │
                          └──────────────────────┘
                                   ↓
                          ┌──────────────────────┐
                          │ cleanup-             │
                          │ ephemeral.yml        │
                          └──────────────────────┘
                                   ↓
                          ✅ Resources deleted
```

## Monitoring Workflows

### View Workflow Status
1. Go to **Actions** tab on GitHub
2. Click workflow name to see all runs
3. Click specific run to see details

### View Workflow Logs
1. Click workflow run
2. Click job name
3. Click step to expand logs

### View Deployment URLs
- **PR Deployments**: Check PR comments for:
  - Frontend: `https://pr-{PR_NUMBER}.rushomon-ui.pages.dev`
  - Backend: `https://rushomon-pr-{PR_NUMBER}.workers.dev`
- **Production Deployments**: Check commit comments for status
  - Backend: `https://rush.mn`
  - Frontend: `https://rushomon.cc`

## Troubleshooting

See `.github/TROUBLESHOOTING.md` for common issues and solutions.

## Environment Variables

Workflows use these environment variables:

| Variable | Source | Used By |
|----------|--------|--------|
| `CLOUDFLARE_API_TOKEN` | GitHub Secret (env-scoped) | All workflows |
| `CLOUDFLARE_ACCOUNT_ID` | GitHub Secret (env-scoped) | All workflows |
| `D1_DATABASE_ID` | GitHub Secret (production) | Production deploy |
| `KV_NAMESPACE_ID` | GitHub Secret (production) | Production deploy |
| `DOMAIN` | GitHub Secret (production) | Production deploy |
| `FRONTEND_URL` | GitHub Secret (production) | Production deploy |
| `GH_CLIENT_ID` | GitHub Secret (env-scoped) | All deployments |
| `GH_CLIENT_SECRET` | GitHub Secret (env-scoped) | All deployments |
| `JWT_SECRET` | GitHub Secret (env-scoped) | All deployments |
| `PR_NUMBER` | Computed | Ephemeral/cleanup |

## Customization

### Change Test Timeout
Edit `.github/workflows/test.yml`:
```yaml
timeout-minutes: 20  # Increase from 15
```

### Change Ephemeral Domain
Edit `.github/workflows/deploy-ephemeral.yml`:
```yaml
route = "pr-${{ env.PR_NUMBER }}.your-domain.workers.dev/*"
```

### Add Slack Notifications
Add to any workflow:
```yaml
- name: Notify Slack
  uses: slackapi/slack-github-action@v1
  with:
    webhook-url: ${{ secrets.SLACK_WEBHOOK }}
    payload: |
      {
        "text": "Deployment successful!"
      }
```

### Disable Production Approval
Remove the `environment` block from each production job in `.github/workflows/deploy-production.yml`:
```yaml
environment:
  name: production
  url: https://rush.mn
```

## Best Practices

1. **Always run tests locally before pushing**:
   ```bash
   cargo test
   ```

2. **Use feature branches for development**:
   ```bash
   git checkout -b feature/my-feature
   ```

3. **Create PRs for code review**:
   - Allows ephemeral deployment
   - Enables team feedback
   - Prevents accidental main branch pushes

4. **Review deployment URLs before merging**:
   - Test ephemeral environment
   - Verify functionality
   - Check for errors

5. **Approve production deployments carefully**:
   - Review changes in PR
   - Verify tests passed
   - Check smoke test results

## Performance Tips

1. **Cache Rust builds**:
   - Already configured in workflows
   - Saves ~5 minutes per run

2. **Use local D1 for integration tests**:
   - Already configured with `--local`
   - Faster than remote database

3. **Run tests in parallel**:
   - Unit tests run in parallel
   - Integration tests run sequentially (to avoid race conditions)

4. **Reuse built artifacts**:
   - Workflows cache build outputs
   - Reduces rebuild time

## Security Considerations

1. **Fork Protection** 🔒:
   - Workflows with secrets only run for PRs from this repository
   - Fork PRs cannot access `CLOUDFLARE_API_TOKEN` or other secrets
   - Prevents secret exfiltration attacks
   - Condition: `github.event.pull_request.head.repo.full_name == github.repository`

2. **Secrets are never logged**:
   - GitHub automatically masks secrets in logs
   - Never print secrets in workflow steps

3. **API tokens are scoped**:
   - Use least-privilege tokens
   - Regenerate if compromised

4. **Ephemeral databases are isolated**:
   - Each PR gets its own database
   - No cross-PR data leakage

5. **Production requires approval**:
   - Manual review before deployment
   - Prevents accidental deployments

6. **Minimal Permissions**:
   - Workflows only request `contents: read` and `pull-requests: write`
   - No unnecessary access to repository settings or secrets

## References

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Cloudflare API Documentation](https://developers.cloudflare.com/api/)
- [Wrangler Documentation](https://developers.cloudflare.com/workers/wrangler/)
- [Self-Hosting Guide](docs/SELF_HOSTING.md)
- [Setup Guide](.github/SETUP_CICD.md)
- [Troubleshooting Guide](.github/TROUBLESHOOTING.md)
