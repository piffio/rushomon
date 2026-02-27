# Self-Hosting Rushomon

Step-by-step guide to deploy your own Rushomon URL shortener instance on Cloudflare.

## Quick Setup (Recommended)

The fastest way to deploy Rushomon is using our **automated setup script**. However, you must complete the prerequisites below **before** running the script.

### Prerequisites (Complete These First)

Before running the setup script, you need to prepare:

#### 1. Install Required Tools

```bash
# wrangler CLI (for Cloudflare deployment)
# You can use homebrew on MacOs or npm on other platforms

brew install wrangler # on MacOS
npm install -g wrangler # on other platforms

# jq (for JSON parsing in setup scripts)
brew install jq # on MacOS
apt-get install jq # on Ubuntu/Debian
# For other platforms: https://stedolan.github.io/jq/download/

# Rust toolchain (for backend compilation)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # on all platforms
brew install rustup # alternative on MacOS

rustup target add wasm32-unknown-unknown

# worker-build (for Worker compilation)
cargo install worker-build

# Node.js 20+ (for frontend build)
# Use your system's package manager, NVM or https://nodejs.org/
node --version # Check the version after installation
```

#### 2. Authenticate with Cloudflare

```bash
wrangler login
```

This opens your browser to authenticate. Verify it worked:

```bash
wrangler whoami
```

#### 3. Add Domain(s) to Cloudflare

If you already have a domain setup in Cloudflare, you can skip this part.
If you're setting up a new domain for it, then follow the next steps.

⚠️ **IMPORTANT**: Your domain(s) must be added to Cloudflare and have **Active** status before running the setup script.

1. Go to [Cloudflare Dashboard](https://dash.cloudflare.com/)
2. Click **Add a site** → enter your domain (e.g., `myapp.com`)
3. Select the **Free** plan
4. Update your domain registrar's nameservers to Cloudflare's nameservers
5. Wait for status to become **Active** (usually takes a few minutes, max 24 hours)

**For subdomains**: If using `api.myapp.com`, you only need to add the parent domain `myapp.com` to Cloudflare.

**For separate short domain**: If using a different domain for short links (e.g., `short.com`), add it as a separate site.

Verify in the Cloudflare dashboard that your domain is active and has the correct nameservers configured.

#### 4. Create OAuth Application(s)

**You must have at least ONE OAuth provider configured.** You can choose GitHub, Google, or both.

**Option A: GitHub OAuth (Recommended)**

1. Go to [GitHub Developer Settings](https://github.com/settings/developers)
2. Click **OAuth Apps** → **New OAuth App**
3. Fill in:
   - **Application name**: `Rushomon` (or your preferred name)
   - **Homepage URL**: `https://myapp.com` (your main domain)
   - **Authorization callback URL**: `https://myapp.com/api/auth/callback` (or your API domain)
4. Click **Register application**
5. Save the **Client ID** (you'll enter this in the setup script)
6. Click **Generate a new client secret** and save the **Client Secret** securely

**Option B: Google OAuth (Optional)**

1. Go to [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Select or create a project
3. Click **Create Credentials** → **OAuth client ID**
4. Configure OAuth consent screen if prompted:
   - User Type: **External**
   - App name: `Rushomon`
   - Scopes: `openid`, `email`, `profile`
5. Create OAuth 2.0 Client ID:
   - Application type: **Web application**
   - **Authorized redirect URIs**: `https://myapp.com/api/auth/callback` (or your API domain)
6. Click **Create** and save both **Client ID** and **Client Secret**

⚠️ **Important**: Never commit OAuth secrets to version control. The setup script will prompt you for these securely, and you might want to store them in a secure password manager.

### Running the Setup Script

Once all prerequisites are complete, run:

```bash
./scripts/setup.sh
```

The interactive wizard will guide you through:
- ✓ Domain configuration (single or multi-domain)
- ✓ OAuth credential entry (GitHub and/or Google)
- ✓ Cloudflare resource creation (D1 database, KV namespace)
- ✓ JWT secret generation
- ✓ Backend and frontend compilation
- ✓ Database migrations
- ✓ Worker deployment
- ✓ Smoke tests

### Using a Configuration File (Non-Interactive)

For automated/repeatable deployments:

```bash
# Copy the example
cp config/setup.example.yaml config/production.yaml

# Edit with your values
vim config/production.yaml

# Set secrets as environment variables
export GITHUB_CLIENT_SECRET="your-github-secret"
export GOOGLE_CLIENT_SECRET="your-google-secret"  # Optional
export JWT_SECRET="$(openssl rand -base64 32)"

# Run setup
./scripts/setup.sh --config config/production.yaml
```

### After Setup Completes

The script will display a summary with your deployment URLs. You still need to:

1. **Configure custom domains** in Cloudflare Dashboard:
   - Go to **Workers & Pages** → Your worker → **Settings** → **Domains**
   - Add your custom domain(s) (the script will remind you)

2. **Configure rate limiting** (recommended):
   - See [Step 8](#step-8-configure-rate-limiting-important) below for setup instructions

3. **Test your deployment**:
   ```bash
   # Should return your landing page
   curl https://myapp.com/

   # Should return 401 (authentication required)
   curl https://myapp.com/api/links
   ```

4. **Sign in and upgrade your admin account**:
   - Visit your domain and sign in with OAuth
   - The first user becomes the admin automatically
   - Go to `/admin` and upgrade yourself to "unlimited" tier

5. **Turn off signup** (optional):
   - Go to `https://myapp.com/admin/settings`
   - Turn off **Allow new sign ups** to prevent other people to create accounts on your instance.

For more details on the setup script, see [scripts/README.md](../scripts/README.md).

---

## Manual Setup

If you prefer manual setup or need to troubleshoot, follow the detailed steps below. The automated script performs these same steps automatically.

## Prerequisites

- **Cloudflare account** — [Sign up](https://dash.cloudflare.com/sign-up) (free plan works)
- **Domain(s)** — At minimum, one domain for the main application. Optionally, separate domains for different functions (see Architecture section below)
- **Rust toolchain** — Install via [rustup](https://rustup.rs/)
- **wasm32 target** — `rustup target add wasm32-unknown-unknown`
- **worker-build** — `cargo install worker-build`
- **Node.js 20+** — For frontend build. I recommend using [nvm](https://github.com/nvm-sh/nvm) to manage Node.js versions.
- **Wrangler CLI** — `npm install -g wrangler`
- **GitHub or Google account** — For OAuth authentication (at least one provider must be configured)

## Architecture Overview

Rushomon uses a **unified Worker architecture** where a single Cloudflare Worker serves:
- **Frontend (static assets)** — SvelteKit app via Workers Assets binding
- **API endpoints** — Authentication, link management (`/api/*`)
- **URL redirects** — Short link resolution (`/:code`)

### Domain Strategy

You have flexibility in how you configure domains:

**Option A: Single Domain (Simplest)**
- `myapp.com` — Serves everything (UI, API, redirects)
- Pros: Simplest setup, no CORS complexity
- Cons: Short URLs can be longer (e.g., `myapp.com/abc123`)

**Option B: Multi-Domain (Recommended)**
- `myapp.com` — Main web interface
- `api.myapp.com` — API subdomain (optional, same Worker)
- `short.io` — Short branded URLs for redirects
- Pros: Clean short URLs (`short.io/abc123`), professional separation
- Cons: Requires multiple domain configurations

This guide uses **Option B** for demonstration, but you can adapt it for a single domain.

---

## Step 1: Add Domains to Cloudflare

All domains/subdomains must be added as zones in your Cloudflare account.

1. Go to [Cloudflare Dashboard](https://dash.cloudflare.com/)
2. Click **Add a site** → enter your primary domain (e.g., `myapp.com`)
3. Select the **Free** plan
4. Update your domain registrar's nameservers to the ones Cloudflare provides
5. Wait for the zone to become **Active** (can take up to 24 hours, usually minutes)
6. If using a separate short domain (e.g., `short.io`), repeat the above steps

> **Note**:
> - If using subdomains (e.g., `api.myapp.com`), you only need to add the parent domain (`myapp.com`)
> - All zones must be **Active** before proceeding with custom domain attachment
> - DNS records for custom domains will be created automatically by Cloudflare Workers

---

## Step 2: Create Cloudflare Resources

All the following actions can either be performed via `wrangler` CLI or via the Cloudflare dashboard. That's entirely up to you.

Authenticate with Cloudflare:

```bash
wrangler login
```

### Create D1 Database

```bash
wrangler d1 create rushomon
```

Save the returned `database_id` — you'll need it in Step 4.

### Create KV Namespace

```bash
wrangler kv namespace create "URL_MAPPINGS"
```

Save the returned `id` — you'll need it in Step 4.

### Note Your Account ID

Find your Account ID in the Cloudflare dashboard: click on any zone → **Overview** → right sidebar shows **Account ID**. Save this value.

---

## Step 3: Configure OAuth Providers

🔒 **SECURITY NOTE**: Create separate OAuth apps for development and production environments. Never reuse development credentials in production.

Rushomon supports multiple OAuth providers. A provider is only shown to users if its `CLIENT_ID` environment variable is set. You can enable one or both.

**Both providers use the same callback URL**: `https://api.myapp.com/api/auth/callback`

### 3a: GitHub OAuth App

1. Go to [GitHub Developer Settings](https://github.com/settings/developers)
2. Click **OAuth Apps** → **New OAuth App**
3. Fill in:
   - **Application name**: `Rushomon` (or your preferred name)
   - **Homepage URL**: `https://myapp.com` (your main web domain)
   - **Authorization callback URL**: `https://api.myapp.com/api/auth/callback`
4. Click **Register application**
5. Save the **Client ID**
6. Click **Generate a new client secret** and save the **Client Secret**

### 3b: Google OAuth App (optional)

1. Go to [Google Cloud Console](https://console.cloud.google.com/apis/credentials)
2. Select or create a project
3. Click **Create Credentials** → **OAuth client ID**
4. If prompted, configure the **OAuth consent screen** first:
   - User type: **External** (for public instances) or **Internal** (for G Suite/Workspace)
   - Add scopes: `openid`, `email`, `profile`
   - Add your domain to **Authorised domains**
5. Back in **Create OAuth client ID**:
   - Application type: **Web application**
   - Name: `Rushomon`
   - **Authorised redirect URIs**: `https://api.myapp.com/api/auth/callback`
6. Click **Create** and save the **Client ID** and **Client Secret**

> **Google-specific notes**:
> - For development, add `http://localhost:8787/api/auth/callback` as an additional redirect URI
> - Google requires your app to be verified for production use with external users; internal (Workspace) apps do not require verification
> - If you see `Error 400: redirect_uri_mismatch`, double-check the redirect URI matches exactly (no trailing slash)

> **Applies to both providers**:
> - The callback URL must match the `DOMAIN` variable in your Worker configuration
> - Never commit OAuth secrets to version control
> - Use different OAuth apps for development and production
> - Store production secrets via `wrangler secret put` (see Step 5)

---

## Step 4: Create Production Wrangler Configuration

⚠️ **SECURITY WARNING**: This file contains configuration data but should NOT contain secrets. Secrets are stored separately via `wrangler secret put` (see Step 5).

Copy the example configuration file and customize it for production:

```bash
# Copy the example configuration
cp wrangler.example.toml wrangler.toml
```

Now edit `wrangler.toml` and update the following values:

```toml
name = "rushomon-production"
main = "build/worker/shim.mjs"
compatibility_date = "2026-02-10"
workers_dev = false

# Custom domains (configure these after first deployment via Cloudflare Dashboard)
# These are the domains that will route to your Worker:
# - myapp.com (main web interface)
# - api.myapp.com (API subdomain)
# - short.io (short link redirects)

# D1 Database
[[d1_databases]]
binding = "rushomon"
database_name = "rushomon"
database_id = "YOUR_D1_DATABASE_ID"

# KV Namespace
[[kv_namespaces]]
binding = "URL_MAPPINGS"
id = "YOUR_KV_NAMESPACE_ID"

# Static Assets (Frontend)
[assets]
directory = "./frontend/build"
binding = "ASSETS"
run_worker_first = true
not_found_handling = "none"

# Environment variables
[vars]
# GitHub OAuth (omit GITHUB_CLIENT_ID to disable GitHub login)
GITHUB_CLIENT_ID = "YOUR_GITHUB_CLIENT_ID"
GITHUB_AUTHORIZE_URL = "https://github.com/login/oauth/authorize"
GITHUB_TOKEN_URL = "https://github.com/login/oauth/access_token"
GITHUB_USER_URL = "https://api.github.com/user"

# Google OAuth (omit GOOGLE_CLIENT_ID to disable Google login)
GOOGLE_CLIENT_ID = "YOUR_GOOGLE_CLIENT_ID"
GOOGLE_AUTHORIZE_URL = "https://accounts.google.com/o/oauth2/v2/auth"
GOOGLE_TOKEN_URL = "https://oauth2.googleapis.com/token"
GOOGLE_USER_URL = "https://openidconnect.googleapis.com/v1/userinfo"

DOMAIN = "api.myapp.com"  # Where OAuth callbacks go (your API domain)
FRONTEND_URL = "https://myapp.com"  # Main web interface URL
ALLOWED_ORIGINS = "https://myapp.com,https://api.myapp.com"  # CORS allowed origins
# KV-based rate limiting is disabled by default in favor of Cloudflare rate limiting rules
# Set to "true" to re-enable KV-based rate limiting for specific use cases
ENABLE_KV_RATE_LIMITING = "false"
```

Replace the placeholder values:
- `YOUR_D1_DATABASE_ID` — from Step 2
- `YOUR_KV_NAMESPACE_ID` — from Step 2
- `YOUR_GITHUB_CLIENT_ID` — from Step 3a (omit key entirely to disable GitHub login)
- `YOUR_GOOGLE_CLIENT_ID` — from Step 3b (omit key entirely to disable Google login)
- `api.myapp.com` — your API domain/subdomain (must match OAuth callback URL)
- `myapp.com` — your main web domain
- Adjust `ALLOWED_ORIGINS` to match your domain setup

---

## Step 5: Set Worker Secrets

🔒 **CRITICAL SECURITY STEP**: Secrets must be stored via Cloudflare Workers Secrets API, NOT in wrangler.toml files.

The configuration file (`wrangler.toml`) should already be created from Step 4. Now set the required secrets:

Set these secrets in your Cloudflare account:

```bash
# GitHub OAuth client secret (from Step 3a) — skip if not using GitHub
wrangler secret put GITHUB_CLIENT_SECRET -c wrangler.toml

# Google OAuth client secret (from Step 3b) — skip if not using Google
wrangler secret put GOOGLE_CLIENT_SECRET -c wrangler.toml

# JWT signing secret (MUST be at least 32 characters)
# Generate a secure random string:
openssl rand -base64 32

# Then store it:
wrangler secret put JWT_SECRET -c wrangler.toml
```

**Security Requirements**:
- JWT_SECRET must be at least 32 characters (enforced by application)
- Never commit secrets to version control
- Never embed secrets in environment variables (use Workers Secrets API)
- Use different secrets for development and production
- Store secrets securely (password manager recommended)

**For CI/CD**: Add secrets as GitHub Secrets and use `wrangler secret put` in your deployment workflow.

---

## Step 6: Build and Deploy the Unified Worker

### Build Frontend

First, build the frontend with the correct API URL:

```bash
cd frontend
npm ci

# Build with production API URL
PUBLIC_VITE_API_BASE_URL=https://api.myapp.com \
PUBLIC_VITE_SHORT_LINK_BASE_URL=https://short.io \
npm run build

cd ..
```

> **Important**:
> - `PUBLIC_VITE_API_BASE_URL` — Where API calls go (your API domain)
> - `PUBLIC_VITE_SHORT_LINK_BASE_URL` — Domain shown in short URLs (your redirect domain)
> - These are baked in at build time and cannot be changed without rebuilding

### Build Backend

```bash
worker-build --release
```

### Apply Database Migrations

```bash
wrangler d1 migrations apply rushomon --remote -c wrangler.toml
```

### Deploy Worker

```bash
wrangler deploy -c wrangler.toml
```

This deploys the unified Worker with both frontend assets and backend API.

---

## Step 7: Configure Custom Domains

After deployment, attach custom domains to your Worker via the Cloudflare Dashboard.

1. Go to [Workers & Pages](https://dash.cloudflare.com/?to=/:account/workers-and-pages)
2. Select **rushomon-production**
3. Go to **Settings** → **Domains & Routes**
4. Click **Add** under Custom Domains
5. Add each domain you want to use:
   - `myapp.com` (main web interface)
   - `api.myapp.com` (API subdomain)
   - `short.io` (short link redirects)
6. Cloudflare will automatically create DNS records and provision SSL certificates

> **Note**:
> - DNS propagation usually takes a few minutes
> - SSL certificates are provisioned automatically
> - All domains route to the same Worker but serve different content based on the request path

---

## Step 8: Configure Rate Limiting (Important)

Rushomon can use Cloudflare's built-in rate limiting instead of KV-based rate limiting to reduce costs and improve performance. If you chose this option, you will have to configure rate limiting rules after deployment.

### Option A: Free Tier Setup (1 Rule)

For basic protection, use one comprehensive rule:

1. Go to [Cloudflare Dashboard](https://dash.cloudflare.com/)
2. Select your domain → **Security** → **WAF** → **Rate limiting rules**
3. Click **Create rule**
4. **Rule name**: `Comprehensive API and Redirect Protection`
5. **Expression**: `(http.request.uri.path starts_with "/api/") or (http.request.uri.path matches "^/[a-zA-Z0-9_-]+$")`
6. **Rate limit settings**:
   - **Requests per period**: 200
   - **Period**: 1 minute
   - **Action**: Challenge
7. Click **Deploy**

### Option B: Pro Tier Setup (3 Rules)

For production, upgrade to Pro plan ($20/month) and configure granular rules:

1. **Redirect Protection**:
   - **Expression**: `not (http.request.uri.path starts_with "/api/") and not (http.request.uri.path starts_with "/admin/")`
   - **Requests per period**: 300
   - **Period**: 1 minute
   - **Action**: Challenge

2. **API Protection**:
   - **Expression**: `http.request.uri.path starts_with "/api/"`
   - **Requests per period**: 100
   - **Period**: 1 minute
   - **Action**: Challenge

3. **Auth Protection**:
   - **Expression**: `http.request.uri.path starts_with "/api/auth/"`
   - **Requests per period**: 20
   - **Period**: 15 minutes
   - **Action**: Challenge

### Testing Rate Limiting

```bash
# Test API rate limiting (should work for first 200 requests, then show challenge)
for i in {1..250}; do
  curl -s -o /dev/null -w "%{http_code}\n" https://api.myapp.com/api/links
done

# Test redirect rate limiting
for i in {1..250}; do
  curl -s -o /dev/null -w "%{http_code}\n" https://short.io/nonexistent
done
```

### Optional: Re-enable KV Rate Limiting

If you need a more fine-grained rate limiting and are not concerned about the extra reads and writes on the KV store, you can use the following command to enable it:

```bash
wrangler secret put ENABLE_KV_RATE_LIMITING -c wrangler.toml
# Enter: true
```

**Note**: This will incur KV write costs or will exhaust your free tier allowance, and is only recommended for specific use cases.

---

## Step 9: Configure Observability and Logging

Rushomon includes comprehensive observability features using Cloudflare Workers' built-in capabilities. These are already configured in the sample `wrangler.example.toml` and should be included in production deployments.

### What's Included

**Built-in Observability Features:**
- **Source Maps** - Deobfuscated stack traces for debugging
- **Workers Logs** - All application logs captured centrally
- **Traces** - End-to-end request tracing (10% sampling by default, configurable via `traces_sample_rate` in `wrangler.toml`)
- **Structured JSON Logging** - Consistent, searchable log format

### Accessing Logs and Traces

1. **View Logs in Cloudflare Dashboard**:
   - Go to [Cloudflare Dashboard](https://dash.cloudflare.com/)
   - Navigate to **Workers & Pages** → **rushomon**
   - Click the **Logs** tab to view real-time logs
   - Click the **Traces** tab to view request traces

2. **Search Logs**:
   Use JSON syntax to filter logs:
   ```
   event:"rate_limit_hit"
   event:"auth_*" AND level:"warn"
   level:"error"
   ```

---

## Step 10: Verify Your Deployment

### Check the Frontend

Open `https://myapp.com` in your browser — you should see the Rushomon landing page with static assets loading correctly.

### Check API Endpoints

```bash
# Health check (should return 200)
curl -s https://myapp.com/

# Protected endpoint (should return 401 - auth required)
curl -s -o /dev/null -w "%{http_code}" https://api.myapp.com/api/auth/me

# Protected endpoint (should return 401 - auth required)
curl -s -o /dev/null -w "%{http_code}" https://api.myapp.com/api/links
```

### Test OAuth Flow

1. Click **Sign in with GitHub** on the landing page
2. Authorize the OAuth App
3. You should be redirected back to the dashboard at `https://myapp.com/dashboard`
4. The first user to sign in becomes the **instance admin**

### Test Admin Console

1. Navigate to `https://myapp.com/admin`
2. You should see the admin console (only accessible to the first user)
3. Verify you can see your user account with "free" tier
4. **Important**: See Step 10 to upgrade your admin account and configure default tiers

### Test Short Link Redirects

After creating a link in the dashboard:

```bash
# Should redirect to the destination URL
curl -I https://short.io/abc123
```

> **Note**: Short links will only work after you've created at least one link via the dashboard

---

## Step 10: Configure User Tiers (Important for Self-Hosting)

Rushomon includes a tier system with **Free** and **Unlimited** plans. As a self-hosted instance administrator, you need to configure tiers appropriately for your use case.

### Understanding the Tier System

**Free Tier (Default)**:
- 15 links per month
- 7 days analytics retention
- Links continue to work after limits are reached

**Unlimited Tier**:
- Unlimited links and tracked clicks
- Full analytics retention (all time)
- Complete feature access

### First User Setup

The first user to sign in to your instance automatically becomes the **instance admin**. However, they start on the **Free tier** by default. You should upgrade them:

1. **Access the Admin Console**:
   - Navigate to `https://myapp.com/admin`
   - Only the first user (admin) can access this page

2. **Upgrade Your Admin Account**:
   - Find your user in the admin console
   - Change your tier from "free" to "unlimited" by clicking on the `Free` label
   - Click **Change Tier** to save changes

### Setting Default Tier for New Users

You can configure what tier new users receive by default:

1. **In the Admin Console** (`/admin`):
   - Look for the "Default New User Tier" setting
   - Choose between "free" or "unlimited"
   - Click **Update Default Tier**

2. **Recommended Settings**:
   - **Personal instance**: Set to "unlimited" (no restrictions)
   - **Public service**: Set to "free" (with upgrade path)
   - **Team/Company**: Set to "unlimited" (internal use)

### Managing Existing Users

As an admin, you can:
- **View all users** and their current tiers
- **Upgrade individual users** from free to unlimited
- **Downgrade users** (if needed)
- **See usage statistics** for each user

> **Important**: The admin console is only accessible to the first user who signed in. If you lose access to the admin account, you'll need to run queries via the Cloudflare D1 console to change user's roles.

---

## Environment Variables Reference

### Worker Variables (`[vars]` in wrangler.toml)

| Variable | Description | Example |
|----------|-------------|---------|
| `GITHUB_CLIENT_ID` | GitHub OAuth App client ID | `Iv1.abc123def456` |
| `GITHUB_AUTHORIZE_URL` | GitHub OAuth authorize endpoint | `https://github.com/login/oauth/authorize` |
| `GITHUB_TOKEN_URL` | GitHub OAuth token endpoint | `https://github.com/login/oauth/access_token` |
| `GITHUB_USER_URL` | GitHub user API endpoint | `https://api.github.com/user` |
| `DOMAIN` | Domain where OAuth callbacks go (no protocol) | `api.myapp.com` |
| `FRONTEND_URL` | Main web interface URL (with protocol) | `https://myapp.com` |
| `ALLOWED_ORIGINS` | Comma-separated CORS origins | `https://myapp.com,https://api.myapp.com` |
| `ENABLE_KV_RATE_LIMITING` | Enable KV-based rate limiting (default: false) | `false` |

### Worker Secrets (set via `wrangler secret put`)

| Secret | Description |
|--------|-------------|
| `GITHUB_CLIENT_SECRET` | GitHub OAuth App client secret |
| `JWT_SECRET` | JWT signing key (32+ random characters) |

### Frontend Build-Time Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `PUBLIC_VITE_API_BASE_URL` | Where API calls go (with protocol) | `https://api.myapp.com` |
| `PUBLIC_VITE_SHORT_LINK_BASE_URL` | Domain shown in short URLs (with protocol) | `https://short.io` |

> **Important**: Frontend variables are baked in at build time. Changing them requires rebuilding the frontend and redeploying the Worker.

---

## Updating Your Instance

To update to a newer version of Rushomon:

### Option A: Stable Releases (Recommended)

For production deployments, use version tags for stable releases:

```bash
# Check available versions
git tag --list 'v*'

# Checkout a specific stable version
git checkout v0.1.0

# Then follow the build and deploy steps below
```

### Option B: Latest Features

For development or to get the latest features:

```bash
# Pull latest changes from main branch
git pull origin main
```

### Build and Deploy

After checking out your desired version:

```bash
# Rebuild frontend
cd frontend
npm ci
PUBLIC_VITE_API_BASE_URL=https://api.myapp.com \
PUBLIC_VITE_SHORT_LINK_BASE_URL=https://short.io \
npm run build
cd ..

# Rebuild backend
worker-build --release

# Apply any new database migrations
wrangler d1 migrations apply rushomon --remote -c wrangler.production.toml

# Deploy unified Worker (includes both frontend and backend)
wrangler deploy -c wrangler.production.toml
```

### Verify Version

After updating, verify your version:

```bash
# Check the API version endpoint
curl https://api.myapp.com/api/version
```

This returns version information including the current version, build timestamp, and git commit.

> **Note**: Because frontend assets are included in the Worker deployment, you only need one `wrangler deploy` command.

---

## Troubleshooting

### Custom domain not working
- Ensure the zone is **Active** in Cloudflare (check DNS tab)
- DNS propagation can take up to 24 hours (usually minutes)
- SSL certificates are provisioned automatically but may take a few minutes
- Check that custom domains are correctly attached in Workers dashboard

### OAuth callback fails with "redirect_uri_mismatch"
- Verify the callback URL in your GitHub or Google OAuth App matches exactly: `https://YOUR_API_DOMAIN/api/auth/callback`
- Ensure `DOMAIN` in wrangler.toml matches your API domain (no `https://` prefix, no trailing slash)
- Check for hidden characters (tabs, newlines) in the `DOMAIN` value when setting it
- The `DOMAIN` must match the domain where `/api/auth/callback` is served
- For Google: the redirect URI must be listed under **Authorised redirect URIs** in the Google Cloud Console (not just the consent screen)

### Google OAuth shows "access_denied"
- Check the OAuth consent screen configuration in Google Cloud Console
- If in "Testing" mode, only listed test users can sign in — publish the app or add the user to the test list
- Ensure the `email` and `profile` scopes are included in the consent screen
- Verify the Google account has a verified email address

### Login page shows no providers
- Ensure at least one provider's `CLIENT_ID` variable is set in your `wrangler.toml` `[vars]` section
- Check that the corresponding secret (`GITHUB_CLIENT_SECRET` or `GOOGLE_CLIENT_SECRET`) is also set via `wrangler secret put`
- Verify the Worker was redeployed after adding variables

### CORS errors in browser console
- Ensure `ALLOWED_ORIGINS` in wrangler.toml includes all domains that need API access
- Format: `https://myapp.com,https://api.myapp.com` (comma-separated, no spaces)
- Values must match exactly — no trailing slashes
- Include both main domain and API subdomain if using separate domains

### Frontend shows "localhost" URLs
- `PUBLIC_VITE_API_BASE_URL` was not set at build time
- Rebuild the frontend with both environment variables set correctly
- Redeploy the Worker to pick up the new frontend build

### Short links show wrong domain
- Verify `PUBLIC_VITE_SHORT_LINK_BASE_URL` was set correctly during frontend build
- Rebuild frontend with correct value and redeploy Worker

### Static assets (CSS/JS) not loading
- Check that `[assets]` binding is configured in wrangler.toml
- Verify `directory = "./frontend/build"` points to the correct build output
- Ensure frontend was built before deploying Worker

### Admin console not accessible
- Only the first user to sign in can access `/admin`
- If you lost access to the admin account, you'll need database access to manually update the `organizations` table
- Check that you're signed in as the correct user (the first one who registered)

### Users hitting limits unexpectedly
- Check the default tier setting in the admin console
- Verify individual user tiers in the admin console
- For personal instances, consider setting default tier to "unlimited"
- Remember: Free tier has 15 links/month and 7-day analytics retention

### Analytics showing "upgrade" messages
- Free tier users only see 7 days of analytics data
- Upgrade users to "unlimited" tier for full analytics access
- Check retention limits in the tier system (Free: 7 days, Unlimited: unlimited)

### Observability not working
- Verify observability is enabled in `wrangler.toml` (should be by default)
- Check that `upload_source_maps = true` for deobfuscated stack traces
- Ensure `[observability]` and `[observability.traces]` sections are present
- Logs may take a few minutes to appear in the Cloudflare dashboard

### Logs not appearing in Cloudflare Dashboard
- Navigate to **Workers & Pages** → **rushomon** → **Logs** tab
- Check the time range filter (logs are retained for 7 days on free plan)
- Try searching for a specific event: `event:"rate_limit_hit"`
- Verify your Worker is receiving traffic (check the Analytics tab)

### Traces not visible
- Traces are sampled at 10% by default - you may need more traffic to see traces
- Check the **Traces** tab in Workers dashboard
- For debugging, temporarily increase sampling to 100% in `wrangler.toml`
- Remember to reset sampling to 0.1 after debugging

### High error rate in logs
- Search for `level:"error"` to identify system issues
- Check `analytics_event_failed` events for database connectivity issues
- Monitor `click_count_failed` events for D1 performance problems
- Use `level:"critical"` to find data integrity issues

### Rate limiting not working
- Verify Cloudflare rate limiting rules are deployed and enabled
- Check rule expressions match your domain structure
- Monitor Cloudflare Security dashboard for rate limiting events
- Ensure rules are attached to the correct domain/zone

### Users getting rate limited too quickly
- Check your rate limiting limits (200/min for comprehensive rule)
- Consider upgrading to Pro tier for granular rules
- Monitor for abuse patterns vs legitimate traffic
- Adjust limits based on your usage patterns

### High KV costs (if KV rate limiting enabled)
- Ensure `ENABLE_KV_RATE_LIMITING` is set to "false"
- Check Cloudflare KV usage dashboard
- Consider using Cloudflare rate limiting instead
- Monitor KV write operations in analytics

### Setup Script Issues

If you encounter issues with the automated setup script:

**Script fails to authenticate**:
```bash
# Re-authenticate with Cloudflare
wrangler logout
wrangler login
```

**Missing prerequisites**:
```bash
# Install all prerequisites
npm install -g wrangler
cargo install worker-build
```

**Configuration file not loading**:
```bash
# Verify YAML syntax
cat config/production.yaml

# Install yq for better YAML parsing (optional but recommended)
brew install yq  # macOS
snap install yq  # Linux
```

**Deployment validation fails**:
```bash
# Run setup in dry-run mode to check configuration
./scripts/setup.sh --config config/production.yaml --dry-run

# Check wrangler configuration
wrangler whoami
wrangler d1 list
wrangler kv namespace list
```

**Want to update existing deployment**:
```bash
# Use update mode to modify existing resources
./scripts/setup.sh --config config/production.yaml --update
```

For more troubleshooting help with the setup script, see [scripts/README.md](../scripts/README.md).
