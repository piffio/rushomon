# Rushomon - URL Shortener

A self-hostable URL shortener built for Cloudflare Workers with Rust (WebAssembly), designed for personal/family use with multi-tenant capability.

## Why Rushomon?

The name *Rushomon* is both descriptive and conceptual. In its basic sense, the "Rush" part stands for Rust Url Shortener, while the "Mon" part stands for Monitoring as the application includes analytics capabilities.

Conceptually, it's a homage to Akira Kurosawa's film *Rashomon*, which explores the idea of multiple perspectives of the same event. In the same way as an event can be perceived differently by different people, the same URL can be accessed from different angles, e.g. different short links.

Never forget that naming things is a hard problem to solve. Nevertheless, I'm glad I came up with such a cool name for this project.

## Features

- **Fast Edge Redirects**: Sub-millisecond URL resolution via Cloudflare KV
- **Custom Redirect Codes**: User-chosen slugs with random fallback
- **Per-Link Analytics**: Detailed analytics data for individual links with filtering
- **OAuth Authentication**: GitHub and Google OAuth with secure JWT sessions, provider opt-in via env vars, and account linking across providers
- **Instance Admin**: First user becomes admin; admin dashboard for user management and settings
- **Signup Control**: Admins can disable new signups to lock down the instance
- **Multi-tenant Ready**: Organization/team model from day one
- **Self-hostable**: Run on your own custom domain, with straightforward deployment in Cloudflare's free tier
- **Abuse Reporting**: Users can report abusive links
- **Tagging**: Links can be tagged for better organization and filtering
- **Link Status Management**: Active/Disabled/Deleted states with soft delete functionality
- **Usage Tracking & Limits**: Monthly counters with tier-based limits (free/unlimited tiers)
- **Advanced Security**: Destination blacklist and user suspension capabilities
- **Admin Moderation**: Link review workflow and comprehensive abuse report management
- **Title Fetching**: Automatic title extraction for URLs during link creation
- **Rate Limiting**: Comprehensive IP, user, and session-based rate limiting
- **Instance Settings**: Configurable admin settings including signup control and default tiers

## Planned Features

- **Analytics aggregation**: Advanced queries and dashboard UI
- **More OAuth providers**: GitLab and other providers beyond GitHub/Google
- **Team/organization management**: Enhanced collaborative features and permissions
- **QR Codes Generation**: Generate QR codes for links
- **Bulk link operations**: Import/export and batch management
- **Custom domains per organization**: Organization-specific branded domains

## How to try it out

Rushomon can be used in two ways:

1. **Self-hosted**: Deploy your own instance on Cloudflare Workers. Check the [SELF_HOSTING.md](./docs/SELF_HOSTING.md) file for detailed instructions. You will need to deploy your own domain and ensure the whole configuration is working.

2. **Managed service**: Use the public instance at https://rushomon.cc. It's currently in beta and you can sign up to try the free tier via GitHub or Google OAuth. Paid tiers will be made available in the near future. Signing up for a Rushomon subscription is the best way to support the open-source development of this project.

## Version Management

Rushomon uses a robust versioning system with `Cargo.toml` as the single source of truth. See [docs/VERSIONING.md](./docs/VERSIONING.md) for complete version management instructions using the provided Makefile targets.

For self-hosted users:
- **Stable releases**: Use version tags (e.g., `git checkout v0.1.0`)
- **Latest features**: Pull from `main` branch (e.g., `git pull origin main`)


## Tech Stack

The project is designed to be deployed on a single Cloudflare worker, bundling both the backend and frontend. In future iterations we'll add support for running the public service managing redirects on a separate worker instance.

- **Backend**: Rust compiled to WebAssembly for Cloudflare Workers
- **Frontend**: SvelteKit + Tailwind CSS v4
- **Storage**: Cloudflare KV (URL mappings) + D1 (metadata & analytics)

## Setup Instructions

### Prerequisites

1. **Rust**: Install via [rustup](https://rustup.rs/)
2. **Wasm target**: `rustup target add wasm32-unknown-unknown`
3. **worker-build**: `cargo install worker-build`
4. **Cloudflare account**: Sign up at [cloudflare.com](https://cloudflare.com)
5. **GitHub or Google account**: For OAuth authentication (at least one provider required)
6. **Wrangler CLI**: `npm install -g wrangler` or `cargo install wrangler`
7. **Node.js**: For frontend development (v20+ recommended)
8. **expect** (macOS only): For colored output in development script - `brew install expect`

### Step 1: Clone and Install

```bash
git clone git@github.com:piffio/rushomon.git
cd rushomon

# Set up development hooks and configuration
./repo-config/scripts/setup.sh
```

### Step 2: Create Cloudflare Resources

```bash
# Authenticate with Cloudflare
wrangler login

# Create KV namespace for URL mappings
wrangler kv namespace create "URL_MAPPINGS"
# Save the returned 'id' for wrangler.toml

# Create D1 database
wrangler d1 create rushomon
# Save the returned 'database_id' for wrangler.toml

# Apply database migrations
wrangler d1 migrations apply rushomon --local
wrangler d1 migrations apply rushomon --remote
```

### Step 3: Configure Environment

⚠️ **SECURITY WARNING**: Never commit secrets to version control. This repository includes `.dev.vars` in `.gitignore` to prevent accidental exposure.

1. **Set up local development environment**:
   ```bash
   # Copy the example files
   cp .dev.vars.example .dev.vars
   cp wrangler.example.toml wrangler.toml

   # Edit .dev.vars and wrangler.toml with your development credentials
   # Use DIFFERENT credentials than production!
   ```

2. **Configure `wrangler.toml`**:
   - Replace `your-kv-namespace-id-here` with KV namespace ID
   - Replace `your-preview-kv-id-here` with preview KV namespace ID
   - Replace `your-database-id-here` with D1 database ID
   - Set your domain in `DOMAIN` variable
   - Update `ALLOWED_ORIGINS` with your frontend URLs (comma-separated)

3. **Set up GitHub OAuth App** (create separate apps for dev and production):

   **For Development:**
   - Go to GitHub Settings → Developer settings → OAuth Apps → New OAuth App
   - Application name: "Rushomon URL Shortener (Dev)"
   - Homepage URL: `http://localhost:5173`
   - Authorization callback URL: `http://localhost:8787/api/auth/callback`
   - Save Client ID and Client Secret to `.dev.vars`

   **For Production:**
   - Create a NEW OAuth App (do not reuse development app)
   - Application name: "Rushomon URL Shortener"
   - Homepage URL: `https://yourdomain.com`
   - Authorization callback URL: `https://yourdomain.com/api/auth/callback`
   - Store secrets via Wrangler (see below)

4. **Store production secrets** (NEVER in wrangler.toml):
   ```bash
   # Store GitHub OAuth client secret
   wrangler secret put GITHUB_CLIENT_SECRET

   # Generate and store JWT secret (minimum 32 characters required)
   # Generate secure random string:
   openssl rand -base64 32

   # Store it:
   wrangler secret put JWT_SECRET
   ```

   🔒 **Important**: Production secrets are stored via Cloudflare Workers Secrets API and are never visible in your codebase or dashboard.

### Step 4: Local Development

#### Quick Start (Recommended)

```bash
# Start both backend and frontend with colored output
./scripts/start-local-environment.sh

# Backend: http://localhost:8787
# Frontend: http://localhost:5173
# Press Ctrl+C to stop both services
```

#### Manual Setup

**Backend (Rust Worker)**

```bash
# Start the Worker locally
wrangler dev

# The Worker will be available at http://localhost:8787
```

**Frontend (SvelteKit)**

```bash
# Navigate to frontend directory
cd frontend

# Install dependencies
npm install

# Start development server
npm run dev

# Frontend will be available at http://localhost:5173
# Configure .env for local backend: VITE_API_URL=http://localhost:8787
```

**Note**: For local development, run both the backend (wrangler dev) and frontend (npm run dev) simultaneously. The convenience script handles both services with proper color output and logging.

### Step 5: Deploy to Production

For a complete self-hosting guide with custom domains, see **[docs/SELF_HOSTING.md](docs/SELF_HOSTING.md)**.

#### Quick Manual Deploy

```bash
# Backend: Build and deploy the Worker
worker-build --release
wrangler d1 migrations apply rushomon --remote -c wrangler.production.toml
wrangler deploy -c wrangler.production.toml

# Frontend: Build and deploy to Cloudflare Pages
cd frontend
npm ci
PUBLIC_VITE_API_BASE_URL=https://your-api-domain.com npm run build
npx wrangler pages deploy build --project-name=rushomon-ui --branch=main
```

## Development

### Repository Configuration

This project uses a sophisticated repository configuration system for consistent development experience. See [repo-config/README.md](repo-config/README.md) for complete setup and customization instructions.

**Quick Setup:**
```bash
./repo-config/scripts/setup.sh
```

This installs:
- **Pre-commit hooks** with unit tests, formatting, and linting
- **Configurable checks** (personalize via `repo-config/config/user.sh`)

### Project Structure

```
rushomon/
├── src/                    # Backend (Rust Worker)
│   ├── lib.rs              # Wasm entry point
│   ├── router.rs           # Route handlers
│   ├── models/             # Data models
│   ├── auth/               # OAuth & session management
│   ├── api/                # API endpoints
│   ├── db/                 # D1 queries
│   ├── kv/                 # KV operations
│   ├── utils/              # Utilities (short codes, validation)
│   └── bin/                # Binary utilities
├── frontend/               # Frontend (SvelteKit)
│   ├── src/
│   │   ├── routes/         # SvelteKit routes
│   │   └── lib/            # Shared components and utilities
├── docs/                   # Documentation
├── migrations/             # D1 schema migrations
├── scripts/                # Utility scripts
├── tests/                  # Test suite
├── repo-config/            # Repository configuration system
├── .github/                # GitHub workflows
├── Cargo.toml              # Rust dependencies
├── wrangler.example.toml   # Cloudflare config template
└── README.md               # This file
```

### Running Tests

#### Backend Tests

Rushomon comes with an extensive test suite including unit tests and integration tests for the Rust backend.

```bash
# Run unit tests
cargo test

# Run integration tests (includes mock OAuth server)
./scripts/run-integration-tests.sh
```

**Integration Tests**: The project includes a mock OAuth server for testing the complete authentication flow without requiring real GitHub OAuth credentials. The integration test script automatically starts the mock server, runs all tests, and cleans up afterward.

#### Frontend Tests

Currently there aren't dedicated unit tests for the frontend, but you should at least run the type checker and build test to ensure the frontend is working correctly.

```bash
# Navigate to frontend directory
cd frontend

# Run type checking
npm run check

# Run build test
npm run build
```

## Data Model

### D1 Tables

- **organizations**: Multi-tenant org structure with tier system
- **users**: OAuth user accounts with suspension support
- **links**: Link metadata with status, tags, and click tracking
- **analytics_events**: Detailed click tracking with geo data
- **settings**: Instance-level configuration key-value store
- **destination_blacklist**: Blocked URLs/domains for security
- **link_reports**: User-reported abusive links with review workflow
- **link_tags**: Many-to-many relationship for link organization
- **monthly_counters**: Usage tracking for tier limits

### KV Storage

- **URL mappings**: `{short_code}` → `{destination_url, link_id, expires_at, is_active}` (global namespace)
- **Sessions**: `session:{jwt_token}` → `{user_id, org_id, created_at}` (JWT-based with refresh tokens)
- **OAuth state**: `oauth_state:{random}` → `{redirect_uri, created_at}` (secure CSRF protection)
- **Rate limiting**: `ratelimit:{type}:{identifier}` → `{count, window_start}` (IP, user, session-based)

## Architecture Decisions

### Why Global Namespace for Short Codes?

Phase 1 uses a global namespace (short codes are unique across all orgs) for simplicity and performance. This enables:
- Single KV lookup for redirects (no org resolution needed)
- Best performance for single custom domain deployments

Future enhancement: Add per-org custom domains and/or org-prefixed keys.

### Why 301 Permanent Redirects?

- Better for SEO (link equity transfer)
- Browser/proxy caching reduces server load
- Acceptable trade-off: harder to change destination

### Why Rust?

- One of the reasons why I started this project was to learn Rust
- Type safety and compile-time guarantees
- Good performance
- Growing ecosystem for Cloudflare Workers

## License

AGPL-3.0

## Contributing

Contributions welcome! Please open an issue first to discuss proposed changes.

## Support

For issues or questions, please open a GitHub issue.