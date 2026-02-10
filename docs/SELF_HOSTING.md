# Self-Hosting Rushomon

Step-by-step guide to deploy your own Rushomon URL shortener instance on Cloudflare. This guide covers manual setup — for automated CI/CD, see the GitHub Actions workflow in `.github/workflows/deploy-production.yml`.

## Prerequisites

- **Cloudflare account** — [Sign up](https://dash.cloudflare.com/sign-up) (free plan works)
- **Two domains** — One for the frontend/UI, one for the API/short URLs (e.g., `myapp.com` and `short.io`). You can also use two subdomains of the same domain (e.g., `myapp.com` and `short.myapp.com`) if you prefer. 
- **Rust toolchain** — Install via [rustup](https://rustup.rs/)
- **wasm32 target** — `rustup target add wasm32-unknown-unknown`
- **worker-build** — `cargo install worker-build`
- **Node.js 20+** — For frontend build. I recommend using [nvm](https://github.com/nvm-sh/nvm) to manage Node.js versions.
- **Wrangler CLI** — `npm install -g wrangler`
- **GitHub account** — For OAuth authentication

## Overview

Rushomon consists of two components:

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Backend (Worker)** | Rust → WebAssembly on Cloudflare Workers | API, URL redirects, OAuth |
| **Frontend (UI)** | SvelteKit on Cloudflare Pages | Dashboard, link management |

The backend runs on your **short domain** (e.g., `short.io`) — this is where redirect URLs point.
The frontend runs on your **main domain** (e.g., `myapp.com`) — this is the user-facing UI.

---

## Step 1: Add Domains to Cloudflare

Both domains must be added as zones in your Cloudflare account.

1. Go to [Cloudflare Dashboard](https://dash.cloudflare.com/)
2. Click **Add a site** → enter your API/short domain (e.g., `short.io`)
3. Select the **Free** plan
4. Update your domain registrar's nameservers to the ones Cloudflare provides
5. Wait for the zone to become **Active** (can take up to 24 hours, usually minutes)
6. Repeat for your frontend domain (e.g., `myapp.com`)

> **Note**: Both zones - or the zone in case of using subdomains -must be **Active** before proceeding with custom domain attachment.

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
   - **Homepage URL**: `https://myapp.com` (your frontend domain)
   - **Authorization callback URL**: `https://short.io/api/auth/callback` (your API domain)
4. Click **Register application**
5. Save the **Client ID**
6. Click **Generate a new client secret** and save the **Client Secret**

> **Important**:
> - The callback URL must use your API/short domain, not the frontend domain
> - Never commit OAuth secrets to version control
> - Use different OAuth apps for development (`http://localhost:8787/api/auth/callback`) and production
> - Store production secrets via `wrangler secret put` (see Step 5)

---

## Step 4: Create Production Wrangler Configuration

⚠️ **SECURITY WARNING**: This file contains configuration data but should NOT contain secrets. Secrets are stored separately via `wrangler secret put` (see Step 5).

Create a file called `wrangler.production.toml` in the project root. This file can be committed to version control as long as you follow Step 5 correctly.

```toml
name = "rushomon-api"
main = "build/worker/shim.mjs"
compatibility_date = "2024-01-31"
workers_dev = false
preview_urls = false

# Custom domain — routes all traffic on your short domain to this Worker
[[routes]]
pattern = "short.io"
custom_domain = true

# D1 Database
[[d1_databases]]
binding = "rushomon"
database_name = "rushomon"
database_id = "YOUR_D1_DATABASE_ID"

# KV Namespace
[[kv_namespaces]]
binding = "URL_MAPPINGS"
id = "YOUR_KV_NAMESPACE_ID"

# Environment variables
[vars]
GITHUB_CLIENT_ID = "YOUR_GITHUB_CLIENT_ID"
DOMAIN = "short.io"
FRONTEND_URL = "https://myapp.com"
ALLOWED_ORIGINS = "https://myapp.com"
```

Replace the placeholder values:
- `YOUR_D1_DATABASE_ID` — from Step 2
- `YOUR_KV_NAMESPACE_ID` — from Step 2
- `YOUR_GITHUB_CLIENT_ID` — from Step 3
- `short.io` — your API/short domain
- `myapp.com` — your frontend domain

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

## Step 6: Deploy the Backend (Worker)

### Build the Worker

```bash
worker-build --release
```

### Apply Database Migrations

```bash
wrangler d1 migrations apply rushomon --remote -c wrangler.production.toml
```

### Clear Conflicting DNS Records

Before deploying, delete any existing A, AAAA, or CNAME records on your short domain's apex in the Cloudflare DNS dashboard. The Worker custom domain needs to create its own DNS record and will fail if conflicting records exist.

> **Note**: This is only needed on the first deploy. Subsequent deploys will reuse the existing Worker custom domain record.

### Deploy

```bash
wrangler deploy -c wrangler.production.toml
```

This will deploy the Worker and automatically attach the custom domain specified in `[[routes]]`.

---

## Step 7: Deploy the Frontend (Pages)

### Build the Frontend

```bash
cd frontend
npm ci

# Set the API base URL to your short domain
PUBLIC_VITE_API_BASE_URL=https://short.io npm run build
```

> **Important**: `PUBLIC_VITE_API_BASE_URL` must be set at **build time** — it's baked into the static output by SvelteKit's `adapter-static`.

### Deploy to Cloudflare Pages

```bash
npx wrangler pages deploy build --project-name=rushomon-ui --branch=main
```

If this is your first deployment, Wrangler will create the Pages project automatically.

---

## Step 8: Attach Custom Domain to Pages

### Option A: Via Cloudflare Dashboard (Recommended)

1. Go to [Workers & Pages](https://dash.cloudflare.com/?to=/:account/workers-and-pages)
2. Select **rushomon-ui**
3. Go to **Custom domains** tab
4. Click **Set up a custom domain**
5. Enter your frontend domain (e.g., `myapp.com`)
6. Click **Activate domain**

### Option B: Via Cloudflare API

```bash
curl -X POST \
  "https://api.cloudflare.com/client/v4/accounts/YOUR_ACCOUNT_ID/pages/projects/rushomon-ui/domains" \
  -H "Authorization: Bearer YOUR_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "myapp.com"}'
```

> **Note**: The domain must be an active zone in your Cloudflare account. DNS records will be created automatically.

---

## Step 9: Verify Your Deployment

### Check the Worker

```bash
# Should return 200 with a welcome message
curl -s https://short.io/

# Should return 401 (auth required)
curl -s -o /dev/null -w "%{http_code}" https://short.io/api/auth/me

# Should return 401 (auth required)
curl -s -o /dev/null -w "%{http_code}" https://short.io/api/links
```

### Check the Frontend

Open `https://myapp.com` in your browser — you should see the Rushomon landing page.

### Test OAuth Flow

1. Click **Sign in with GitHub** on the landing page
2. Authorize the OAuth App
3. You should be redirected back to the dashboard
4. The first user to sign in becomes the **instance admin**

---

## Environment Variables Reference

### Worker Variables (`[vars]` in wrangler.toml)

| Variable | Description | Example |
|----------|-------------|---------|
| `GITHUB_CLIENT_ID` | GitHub OAuth App client ID | `Iv1.abc123def456` |
| `DOMAIN` | Your API/short domain (no protocol) | `short.io` |
| `FRONTEND_URL` | Your frontend URL (with protocol) | `https://myapp.com` |
| `ALLOWED_ORIGINS` | Comma-separated CORS origins | `https://myapp.com` |
| `EPHEMERAL_ORIGIN_PATTERN` | Pattern for ephemeral envs (optional) | `https://pr-{}.rushomon-ui.pages.dev` |

### Worker Secrets (set via `wrangler secret put`)

| Secret | Description |
|--------|-------------|
| `GITHUB_CLIENT_SECRET` | GitHub OAuth App client secret |
| `JWT_SECRET` | JWT signing key (32+ random characters) |

### Frontend Build-Time Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `PUBLIC_VITE_API_BASE_URL` | Backend API URL (with protocol) | `https://short.io` |

> **Important**: Frontend variables are baked in at build time. Changing them requires rebuilding and redeploying the frontend.

---

## Updating Your Instance

To update to a newer version of Rushomon:

```bash
# Pull latest changes
git pull origin main

# Rebuild and deploy backend
worker-build --release
wrangler d1 migrations apply rushomon --remote -c wrangler.production.toml
wrangler deploy -c wrangler.production.toml

# Rebuild and deploy frontend
cd frontend
npm ci
PUBLIC_VITE_API_BASE_URL=https://short.io npm run build
npx wrangler pages deploy build --project-name=rushomon-ui --branch=main
```

---

## Troubleshooting

### Custom domain not working
- Ensure the zone is **Active** in Cloudflare (check DNS tab)
- DNS propagation can take up to 24 hours (usually minutes)
- SSL certificates are provisioned automatically but may take a few minutes

### OAuth callback fails
- Verify the callback URL in your GitHub OAuth App matches exactly: `https://YOUR_API_DOMAIN/api/auth/callback`
- Ensure `DOMAIN` in wrangler.toml matches the domain in the OAuth callback URL
- Check that `GITHUB_CLIENT_SECRET` is set correctly via `wrangler secret put`

### CORS errors in browser console
- Ensure `ALLOWED_ORIGINS` in wrangler.toml includes your frontend URL (with `https://`)
- The value must match exactly — no trailing slashes

### Frontend shows "localhost" URLs
- `PUBLIC_VITE_API_BASE_URL` was not set at build time
- Rebuild the frontend with the correct value: `PUBLIC_VITE_API_BASE_URL=https://short.io npm run build`
