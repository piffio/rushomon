# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rushomon is a self-hostable URL shortener built for Cloudflare Workers using Rust compiled to WebAssembly. It uses Cloudflare KV for fast edge redirects and D1 (SQLite) for metadata and analytics.

**Current Status**: Core infrastructure complete. Authentication system pending. No frontend yet.

## Build & Development Commands

### Building

```bash
# Standard build (will fail with worker-build not installed)
cargo build

# Build for Cloudflare Workers (requires worker-build)
cargo install worker-build
worker-build --release

# Local development server
wrangler dev
```

### Testing

```bash
# Run unit tests
cargo test

# Run specific test
cargo test test_name
```

### Database Migrations

```bash
# Apply migrations locally (for development)
wrangler d1 migrations apply rushomon --local

# Apply migrations to production
wrangler d1 migrations apply rushomon --remote

# Query local D1 database
wrangler d1 execute rushomon --local --command "SELECT * FROM links"
```

### Deployment

```bash
# Deploy to Cloudflare
wrangler deploy

# Set secrets (required before first deploy)
wrangler secret put GITHUB_CLIENT_SECRET
wrangler secret put JWT_SECRET
```

## Architecture

### Storage Strategy (Critical)

**Dual Storage Pattern**: Links are stored in BOTH D1 and KV for different purposes:

- **KV (Cloudflare Key-Value)**: Fast edge lookups for redirects
  - Key format: `{short_code}` (global namespace in Phase 1)
  - Value: `LinkMapping` struct with `destination_url`, `link_id`, `expires_at`, `is_active`
  - Used by: `GET /{short_code}` redirect handler

- **D1 (SQLite)**: Full metadata, relationships, and analytics
  - Tables: `organizations`, `users`, `links`, `analytics_events`
  - Used by: All API endpoints, user management, analytics queries

**Why both?** KV provides sub-millisecond global edge reads but limited query capability. D1 provides relational queries but is region-based. Links must be created/updated/deleted in BOTH stores atomically.

### Multi-tenant Design

Despite being single-user focused, the data model is multi-tenant from day one:
- Every user belongs to an `organization`
- Links are scoped to organizations (`org_id` foreign key)
- First user in an org becomes admin
- Phase 1 uses global short code namespace (no org prefix for simplicity)
- Future: Per-org custom domains would require org-prefixed KV keys

### Request Flow

**Redirect Path** (performance critical):
1. `GET /{short_code}` → router matches catch-all route
2. KV lookup: `kv.get(short_code).json::<LinkMapping>()`
3. Validate: check `is_active` and `expires_at`
4. Async: Log analytics event to D1 + increment counter
5. Return: 301 redirect to `destination_url`

**API Path** (authenticated - TODO):
1. Extract JWT from cookie (not implemented yet)
2. Validate session in KV
3. Get user/org context from D1
4. Execute operation (create/read/update/delete)
5. Update both KV and D1 atomically

## Module Structure

- **`src/lib.rs`**: Wasm entry point with `#[event(fetch)]`, router setup
- **`src/router.rs`**: All route handlers (redirect, CRUD operations)
- **`src/db/queries.rs`**: D1 database operations (SQL queries)
- **`src/kv/links.rs`**: KV operations for link mappings
- **`src/models/`**: Data structures (User, Link, Organization, etc.)
- **`src/utils/`**: Short code generation (base62), URL validation
- **`src/auth/`**: OAuth & JWT (placeholder - not implemented)

## Worker 0.7 API Quirks

### Accessing Bindings

The worker 0.7.x API has changed from previous versions:

```rust
// D1 Database access
let db = ctx.env.get_binding::<D1Database>("rushomon")?;

// KV access
let kv = ctx.kv("URL_MAPPINGS")?;

// Import required
use worker::D1Database;  // Note: not in worker:: root, imported from worker crate
```

### D1 Query Pattern

```rust
let stmt = db.prepare("SELECT * FROM links WHERE id = ?1");
let result = stmt
    .bind(&[link_id.into()])?  // stmt.bind returns Result, must unwrap with ?
    .first::<Link>(None)        // first() takes Option<&str> for column name
    .await?;
```

### Known Issues

- `D1Database` import path may cause compiler errors - it's re-exported but not always found
- D1 `bind()` requires `JsValue` array, use `.into()` on each parameter
- KV `json()` returns `KvError` that must be mapped to `worker::Error`

## Configuration Files

### `wrangler.toml`

- **binding name**: Must match code exactly (`"rushomon"` for D1, `"URL_MAPPINGS"` for KV)
- **compatibility_date**: Set to "2024-01-31" for worker 0.7 compatibility
- **build command**: Uses `worker-build` tool (must be installed separately)

### `Cargo.toml`

- **crate-type**: MUST be `["cdylib", "rlib"]` for Wasm
- **edition**: Set to "2024" (latest)
- **worker version**: Currently 0.7.4
- **getrandom**: Requires `features = ["std"]` for Wasm compatibility
- **rand**: Version 0.9.2 uses updated API (`rand::rng()`, not `thread_rng()`)

## Data Model Relationships

```
organizations (1) ──→ (*) users
                 ↓
                 └──→ (*) links ──→ (*) analytics_events
                          ↓
                          └──→ (1) user (created_by)
```

- Organization is created on first user signup
- Links belong to org, not directly to user (enables team sharing)
- `created_by` tracks link creator but doesn't restrict access
- Analytics events duplicate `org_id` for efficient queries

## Short Code Generation

- **Algorithm**: Random base62 (0-9, A-Z, a-z)
- **Length**: 6 characters = 56.8 billion combinations
- **Collision handling**: Check KV before confirming, retry up to 10 times
- **Reserved codes**: `api`, `auth`, `login`, `dashboard`, etc. (see `utils/validation.rs`)
- **Custom codes**: User can specify 4-10 alphanumeric characters

## Security Considerations

### URL Validation

All destination URLs MUST be validated:
- Only `http://` or `https://` schemes allowed
- Prevents XSS via `javascript:` URLs
- See `utils/validation.rs::validate_url()`

### Soft Delete Pattern

Links are soft-deleted (not hard-deleted):
- D1: Set `is_active = 0` (preserves analytics history)
- KV: Hard delete (stops redirects immediately)
- Rationale: Analytics depend on link metadata

### Future Auth (Not Implemented)

Planned authentication flow:
1. OAuth state stored in KV with 10min TTL (CSRF protection)
2. JWT tokens issued after OAuth callback
3. Sessions stored in KV with 7-day TTL
4. httpOnly cookies for JWT transport

## Troubleshooting

### Build Errors

**"no `D1Database` in the root"**: Import path issue with worker crate
- Solution: `use worker::D1Database;` (not `use worker::*;`)

**"method `d1` not found for `Env`"**: API changed in worker 0.7
- Solution: Use `ctx.env.get_binding::<D1Database>("rushomon")?`

**rand deprecation warnings**: Using old rand API
- Solution: `rand::rng()` instead of `thread_rng()`, `random_range()` instead of `gen_range()`

### Runtime Errors

**"Binding not found"**: Check wrangler.toml binding names match code exactly

**D1 errors**: Ensure migrations are applied (`wrangler d1 migrations apply rushomon --local`)

**KV not found**: Create namespaces with `wrangler kv namespace create "URL_MAPPINGS"`

## Future Work

### Phase 3: Authentication (Next)
- Implement `src/auth/oauth.rs` - GitHub OAuth flow
- Implement `src/auth/session.rs` - JWT generation/validation
- Add auth middleware to router for API routes
- Store sessions in KV

### Phase 6-7: Analytics & Frontend
- Analytics aggregation queries in `src/api/analytics.rs`
- SvelteKit frontend in `frontend/` directory
- Dashboard UI for link management

### Known TODOs in Code
- Router uses placeholder user/org IDs (search for "placeholder-")
- Analytics logging lacks org_id (would need to store in KV mapping)
- No authentication middleware on API routes yet
- Error handling could be more specific (using generic `Error::RustError`)
