---
sidebar_position: 1
---

# Getting Started

Welcome to the **Rushomon API documentation**. Rushomon is a self-hosted URL shortener built on Cloudflare Workers. This reference covers all available API endpoints, request/response schemas, and authentication.

## Base URLs

| Environment | URL |
|---|---|
| Production | `https://api.rushomon.cc` |
| Local dev | `http://localhost:8787` |

## Authentication

Most API endpoints require authentication. Rushomon supports two authentication methods:

### Session Cookies (browser)

When using the web dashboard, authentication is handled automatically via an `httpOnly` session cookie set after OAuth login.

### API Keys (programmatic access)

Create an API key in the dashboard under **Account Settings → API Keys** (requires **Pro tier or higher**).

Include the key in the `Authorization` header:

```http
Authorization: Bearer ro_pat_your_key_here
```

## Response Format

All responses are JSON. Errors follow this structure:

```json
{
  "error": "Description of what went wrong"
}
```

## Pagination

List endpoints accept `page` (1-indexed, default `1`) and `limit` (1–100, default `20`) query parameters:

```
GET /api/links?page=2&limit=50
```

## Versioning

This documentation is versioned to match Rushomon releases. Use the dropdown in the top navigation bar to switch between versions. The **main** version reflects what is currently deployed to staging and will be included in the next release.
