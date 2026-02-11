# Self-Hosting Rushomon

Step-by-step guide to deploy your own Rushomon URL shortener instance on Cloudflare. This guide covers manual setup — for automated CI/CD, see the GitHub Actions workflow in `.github/workflows/deploy-production.yml`.

## Prerequisites

- **Cloudflare account** — [Sign up](https://dash.cloudflare.com/sign-up) (free plan works)
- **Domain(s)** — At minimum, one domain for the main application. Optionally, separate domains for different functions (see Architecture section below)
- **Rust toolchain** — Install via [rustup](https://rustup.rs/)
- **wasm32 target** — `rustup target add wasm32-unknown-unknown`
- **worker-build** — `cargo install worker-build`
- **Node.js 20+** — For frontend build. I recommend using [nvm](https://github.com/nvm-sh/nvm) to manage Node.js versions.
- **Wrangler CLI** — `npm install -g wrangler`
- **GitHub account** — For OAuth authentication

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
- Cons: Short URLs are longer (e.g., `myapp.com/abc123`)

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

## Step 3: Create GitHub OAuth App

🔒 **SECURITY NOTE**: Create separate OAuth apps for development and production environments. Never reuse development credentials in production.

Rushomon uses GitHub OAuth for authentication. You need to create an OAuth App:

1. Go to [GitHub Developer Settings](https://github.com/settings/developers)
2. Click **OAuth Apps** → **New OAuth App**
3. Fill in:
   - **Application name**: `Rushomon` (or your preferred name)
   - **Homepage URL**: `https://myapp.com` (your main web domain)
   - **Authorization callback URL**: `https://api.myapp.com/api/auth/callback` (your API domain/subdomain)
4. Click **Register application**
5. Save the **Client ID**
6. Click **Generate a new client secret** and save the **Client Secret**

> **Important**:
> - The callback URL must point to where your API endpoints are served
>   - If using subdomain: `https://api.myapp.com/api/auth/callback`
>   - If using single domain: `https://myapp.com/api/auth/callback`
> - The callback URL must match the `DOMAIN` variable in your Worker configuration
> - Never commit OAuth secrets to version control
> - Use different OAuth apps for development (`http://localhost:8787/api/auth/callback`) and production
> - Store production secrets via `wrangler secret put` (see Step 5)

---

## Step 4: Create Production Wrangler Configuration

⚠️ **SECURITY WARNING**: This file contains configuration data but should NOT contain secrets. Secrets are stored separately via `wrangler secret put` (see Step 5).

Create a file called `wrangler.production.toml` in the project root. This file can be committed to version control as long as you follow Step 5 correctly.

```toml
name = "rushomon-production"
main = "build/worker/shim.mjs"
compatibility_date = "2026-02-11"
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

# Environment variables
[vars]
GITHUB_CLIENT_ID = "YOUR_GITHUB_CLIENT_ID"
GITHUB_AUTHORIZE_URL = "https://github.com/login/oauth/authorize"
GITHUB_TOKEN_URL = "https://github.com/login/oauth/access_token"
GITHUB_USER_URL = "https://api.github.com/user"
DOMAIN = "api.myapp.com"  # Where OAuth callbacks go (your API domain)
FRONTEND_URL = "https://myapp.com"  # Main web interface URL
ALLOWED_ORIGINS = "https://myapp.com,https://api.myapp.com"  # CORS allowed origins
```

Replace the placeholder values:
- `YOUR_D1_DATABASE_ID` — from Step 2
- `YOUR_KV_NAMESPACE_ID` — from Step 2
- `YOUR_GITHUB_CLIENT_ID` — from Step 3
- `api.myapp.com` — your API domain/subdomain (must match OAuth callback URL)
- `myapp.com` — your main web domain
- Adjust `ALLOWED_ORIGINS` to match your domain setup

---

## Step 5: Set Worker Secrets

🔒 **CRITICAL SECURITY STEP**: Secrets must be stored via Cloudflare Workers Secrets API, NOT in wrangler.toml files.

Set these secrets in your Cloudflare account:

```bash
# GitHub OAuth client secret (from Step 3)
wrangler secret put GITHUB_CLIENT_SECRET -c wrangler.production.toml

# JWT signing secret (MUST be at least 32 characters)
# Generate a secure random string:
openssl rand -base64 32

# Then store it:
wrangler secret put JWT_SECRET -c wrangler.production.toml
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
wrangler d1 migrations apply rushomon --remote -c wrangler.production.toml
```

### Deploy Worker

```bash
wrangler deploy -c wrangler.production.toml
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

## Step 8: Verify Your Deployment

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

### Test Short Link Redirects

After creating a link in the dashboard:

```bash
# Should redirect to the destination URL
curl -I https://short.io/abc123
```

> **Note**: Short links will only work after you've created at least one link via the dashboard

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

```bash
# Pull latest changes
git pull origin main

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

> **Note**: Because frontend assets are included in the Worker deployment, you only need one `wrangler deploy` command.

---

## Troubleshooting

### Custom domain not working
- Ensure the zone is **Active** in Cloudflare (check DNS tab)
- DNS propagation can take up to 24 hours (usually minutes)
- SSL certificates are provisioned automatically but may take a few minutes
- Check that custom domains are correctly attached in Workers dashboard

### OAuth callback fails with "redirect_uri_mismatch"
- Verify the callback URL in your GitHub OAuth App matches exactly: `https://YOUR_API_DOMAIN/api/auth/callback`
- Ensure `DOMAIN` in wrangler.toml matches your API domain (no `https://` prefix, no trailing slash)
- Check for hidden characters (tabs, newlines) in the `DOMAIN` value when setting it
- The `DOMAIN` must match the domain where `/api/auth/callback` is served

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
