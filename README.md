# Rushomon - URL Shortener

A self-hostable URL shortener built for Cloudflare Workers with Rust (WebAssembly), designed for personal/family use with multi-tenant capability.

## Features

- **Fast Edge Redirects**: Sub-millisecond URL resolution via Cloudflare KV
- **Custom Short URLs**: User-chosen slugs with random fallback
- **Analytics**: Detailed click tracking with referrer, geo, and user-agent data
- **OAuth Authentication**: GitHub OAuth (Google coming soon)
- **Multi-tenant Ready**: Organization/team model from day one
- **Self-hostable**: Run on your own custom domain

## Tech Stack

- **Backend**: Rust + Cloudflare Workers (WebAssembly)
- **Frontend**: SvelteKit + Cloudflare Pages (coming soon)
- **Storage**: Cloudflare KV (URL mappings) + D1 (metadata & analytics)
- **Authentication**: OAuth 2.0 with JWT sessions

## Project Status

✅ **Phase 1-2 Complete**: Core infrastructure, data models, KV operations
✅ **Phase 4-5 Complete**: Link management API, URL redirection
🚧 **Phase 3 In Progress**: Authentication system
⏳ **Phase 6-7 Pending**: Analytics, Frontend
⏳ **Phase 8 Pending**: Testing & Deployment

## Setup Instructions

### Prerequisites

1. **Rust**: Install via [rustup](https://rustup.rs/)
2. **Wasm target**: `rustup target add wasm32-unknown-unknown`
3. **worker-build**: `cargo install worker-build`
4. **Cloudflare account**: Sign up at [cloudflare.com](https://cloudflare.com)
5. **Wrangler CLI**: `npm install -g wrangler` or `cargo install wrangler`

### Step 1: Clone and Install

```bash
git clone <your-repo>
cd rushomon
```

### Step 2: Create Cloudflare Resources

```bash
# Authenticate with Cloudflare
wrangler login

# Create KV namespace for URL mappings
wrangler kv namespace create "URL_MAPPINGS"
# Save the returned 'id' for wrangler.toml

# Create KV namespace for preview
wrangler kv namespace create "URL_MAPPINGS" --preview
# Save the returned 'id' for wrangler.toml preview_id

# Create D1 database
wrangler d1 create rushomon
# Save the returned 'database_id' for wrangler.toml

# Apply database migrations
wrangler d1 migrations apply rushomon --local
wrangler d1 migrations apply rushomon --remote
```

### Step 3: Configure Environment

1. Update `wrangler.toml`:
   - Replace `your-kv-namespace-id-here` with KV namespace ID
   - Replace `your-preview-kv-id-here` with preview KV namespace ID
   - Replace `your-database-id-here` with D1 database ID
   - Set your domain in `DOMAIN` variable

2. Set up GitHub OAuth App:
   - Go to GitHub Settings → Developer settings → OAuth Apps → New OAuth App
   - Application name: "Rushomon URL Shortener"
   - Homepage URL: `https://yourdomain.com`
   - Authorization callback URL: `https://yourdomain.com/api/auth/callback`
   - Save Client ID and generate Client Secret
   - Update `GITHUB_CLIENT_ID` in `wrangler.toml`

3. Store secrets:
```bash
# Store GitHub OAuth client secret
wrangler secret put GITHUB_CLIENT_SECRET

# Generate and store JWT secret (use a random 32+ character string)
wrangler secret put JWT_SECRET
```

### Step 4: Local Development

```bash
# Start the Worker locally
wrangler dev

# The Worker will be available at http://localhost:8787
```

### Step 5: Deploy to Production

```bash
# Deploy the Worker
wrangler deploy

# Your Worker will be live at https://rushomon.<your-subdomain>.workers.dev
# Configure a custom domain in the Cloudflare dashboard
```

## API Endpoints

### Public Routes

- `GET /{short_code}` - Redirect to destination URL

### API Routes (Authentication Required - Coming Soon)

- `POST /api/links` - Create a new short link
- `GET /api/links` - List all links (paginated)
- `GET /api/links/{id}` - Get link details
- `DELETE /api/links/{id}` - Delete a link

### Authentication (Coming Soon)

- `GET /api/auth/github` - Initiate GitHub OAuth
- `GET /api/auth/callback` - OAuth callback
- `POST /api/auth/logout` - Logout

## Development

### Project Structure

```
rushomon/
├── src/
│   ├── lib.rs              # Wasm entry point
│   ├── router.rs           # Route handlers
│   ├── models/             # Data models
│   ├── auth/               # OAuth & session (WIP)
│   ├── api/                # API endpoints
│   ├── db/                 # D1 queries
│   ├── kv/                 # KV operations
│   └── utils/              # Utilities (short codes, validation)
├── migrations/             # D1 schema
├── Cargo.toml              # Rust dependencies
└── wrangler.toml           # Cloudflare config
```

### Running Tests

```bash
cargo test
```

## Data Model

### D1 Tables

- **organizations**: Multi-tenant org structure
- **users**: OAuth user accounts
- **links**: Link metadata (short code, destination, etc.)
- **analytics_events**: Click tracking data

### KV Storage

- **URL mappings**: `{short_code}` → `{destination_url, link_id, expires_at, is_active}`
- **Sessions**: `session:{jwt_token}` → `{user_id, org_id, created_at}` (Coming Soon)
- **OAuth state**: `oauth_state:{random}` → `{redirect_uri, created_at}` (Coming Soon)

## Architecture Decisions

### Why Global Namespace for Short Codes?

Phase 1 uses a global namespace (short codes are unique across all orgs) for simplicity and performance. This enables:
- Single KV lookup for redirects (no org resolution needed)
- Best performance for single custom domain deployments

Future enhancement: Add per-org custom domains with org-prefixed keys.

### Why 301 Permanent Redirects?

- Better for SEO (link equity transfer)
- Browser/proxy caching reduces server load
- Acceptable trade-off: harder to change destination (use soft delete/recreate pattern)

### Why Rust?

- Type safety and compile-time guarantees
- Learning opportunity
- Good performance (though not critical for this workload)
- Growing ecosystem for Cloudflare Workers

## Roadmap

- [x] Core Worker infrastructure
- [x] Data models and validation
- [x] KV operations
- [x] Link management API
- [x] URL redirection handler
- [ ] GitHub OAuth authentication
- [ ] Session management (JWT)
- [ ] Analytics collection
- [ ] Analytics query API
- [ ] SvelteKit frontend
- [ ] Dashboard UI
- [ ] E2E tests
- [ ] Production deployment
- [ ] Google OAuth support
- [ ] Multi-domain support
- [ ] API keys for programmatic access
- [ ] Webhooks

## License

AGPL-3.0

## Contributing

Contributions welcome! Please open an issue first to discuss proposed changes.

## Support

For issues or questions, please open a GitHub issue.
