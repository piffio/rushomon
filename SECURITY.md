# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Rushomon, please report it responsibly:

**Email**: security@rushomon.cc (or open a private security advisory on GitHub)

**Please do NOT**:
- Open a public GitHub issue for security vulnerabilities
- Disclose the vulnerability publicly before we've had a chance to address it

**What to include**:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)

We aim to respond to security reports within 48 hours and will work with you to understand and address the issue promptly.

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| main    | :white_check_mark: |
| < 1.0   | :x:                |

Currently, only the `main` branch receives security updates. Once version 1.0 is released, we will provide security updates for the latest stable version.

## Security Features

Rushomon implements multiple layers of security:

### Authentication & Authorization
- **GitHub OAuth 2.0** - Secure authentication flow with CSRF protection
- **JWT-based sessions** - Cryptographically signed tokens (HS256)
- **httpOnly cookies** - Access and refresh tokens protected from XSS
- **Token separation** - Distinct access (1 hour) and refresh (7 days) tokens
- **Session management** - Server-side session validation with KV storage
- **Role-based access control** - Instance-level admin and member roles

### Input Validation
- **URL validation** - Only `http://` and `https://` schemes allowed (prevents XSS via `javascript:` URLs)
- **Short code validation** - Alphanumeric only, 4-10 characters
- **Reserved codes** - System routes (`api`, `auth`, `admin`) cannot be used as short codes
- **Pagination limits** - Maximum 100 items per page (DoS prevention)

### Cryptographic Security
- **JWT secret requirements** - Minimum 32 characters enforced
- **Constant-time comparison** - Prevents timing attacks on session/state validation
- **Secure random generation** - UUIDv4 for session IDs and OAuth states
- **HTTPS enforcement** - Strict-Transport-Security header (production)

### Attack Prevention
- **CSRF protection** - OAuth state validation, SameSite cookies
- **XSS mitigation** - httpOnly cookies, Content-Security-Policy, input sanitization
- **Clickjacking prevention** - X-Frame-Options: DENY
- **MIME sniffing prevention** - X-Content-Type-Options: nosniff
- **Session fixation prevention** - Session IDs generated after authentication
- **Rate limiting** - (TODO: Not yet implemented, see roadmap)

### Operational Security
- **Soft delete pattern** - Links deactivated (not deleted) to preserve analytics
- **Secrets management** - Production secrets via Cloudflare Workers Secrets API
- **Environment separation** - Distinct credentials for development and production
- **Audit logging** - (TODO: Not yet implemented, see roadmap)

### Infrastructure Security
- **Cloudflare Workers** - Runs on Cloudflare's global edge network
- **D1 (SQLite)** - Parameterized queries prevent SQL injection
- **KV Storage** - Encrypted at rest, TTL-based expiration
- **Edge computing** - No traditional servers to maintain or patch

## Security Best Practices for Self-Hosting

If you're self-hosting Rushomon, follow these security guidelines:

### Required Configuration

1. **Use strong secrets**:
   ```bash
   # Generate a secure JWT secret (minimum 32 characters)
   openssl rand -base64 32

   # Set via Cloudflare Workers Secrets API
   wrangler secret put JWT_SECRET
   wrangler secret put GITHUB_CLIENT_SECRET
   ```

2. **Separate development and production credentials**:
   - Use different GitHub OAuth apps for dev and production
   - Never commit `.dev.vars` to version control (already in `.gitignore`)
   - Use `.dev.vars.example` as a template

3. **Configure CORS carefully**:
   - Set `ALLOWED_ORIGINS` to only your production domains
   - Never use wildcard `*` origins with credentials
   - Localhost is automatically disabled in production builds

4. **Enable HTTPS**:
   - Always use HTTPS in production (Cloudflare provides SSL)
   - HSTS header is automatically enabled for HTTPS requests
   - Configure GitHub OAuth callback URL with `https://`

### Recommended Practices

- **Regular updates**: Keep dependencies up to date (`cargo update`, `npm update`)
- **Dependency scanning**: Use `cargo audit` to check for known vulnerabilities
- **Secret rotation**: Rotate JWT_SECRET and OAuth secrets periodically
- **Monitoring**: Monitor error logs for suspicious activity
- **Backup**: Regular backups of D1 database (links and user data)

### What NOT to Do

- ❌ Never commit `.dev.vars` or `.env` files
- ❌ Never use development secrets in production
- ❌ Never expose admin endpoints publicly without authentication
- ❌ Never disable CORS protections in production
- ❌ Never use HTTP in production (always HTTPS)
- ❌ Never reuse JWT secrets across environments

## Known Limitations

### Current Limitations

1. **No rate limiting** (planned): All endpoints are vulnerable to abuse without external rate limiting
2. **No audit logging** (planned): Limited visibility into security events
3. **No multi-session management**: Cannot view or revoke specific sessions
4. **No 2FA/MFA**: Only GitHub OAuth authentication available
5. **No API keys**: Programmatic access requires OAuth flow

### Mitigations

- Deploy behind Cloudflare's DDoS protection (included)
- Monitor Worker invocations for unusual patterns
- Use short-lived access tokens (1 hour) to limit exposure
- Enable GitHub organization restrictions on OAuth app

## Security Roadmap

Planned security improvements:

- [ ] **Rate limiting** - KV-based rate limiting middleware (high priority)
- [ ] **Audit logging** - Security event logging to D1
- [ ] **CSRF tokens** - Double-submit cookie pattern for API endpoints
- [ ] **Enhanced JWT claims** - Add `iss`, `aud`, `nbf` claims
- [ ] **Multi-session logout** - View and revoke all active sessions
- [ ] **URL re-validation** - Re-check destination URLs on redirect
- [ ] **Secret rotation** - Automated JWT secret rotation support
- [ ] **2FA/TOTP** - Two-factor authentication option
- [ ] **API keys** - Programmatic access with scoped permissions
- [ ] **Webhook signatures** - HMAC signatures for webhooks

## Security Testing

We encourage security researchers to test Rushomon responsibly:

### In Scope
- Authentication and authorization bypasses
- XSS, CSRF, and injection vulnerabilities
- Session management issues
- Access control vulnerabilities
- Cryptographic weaknesses

### Out of Scope
- Social engineering attacks
- Physical attacks
- Denial of service attacks
- Issues in third-party dependencies (report to maintainers directly)

### Rules of Engagement
- Only test against your own Rushomon instance
- Do not access other users' data
- Do not perform attacks that degrade service
- Report findings privately before public disclosure

## Responsible Disclosure

We follow a coordinated disclosure process:

1. **Report received** - We acknowledge receipt within 48 hours
2. **Validation** - We validate the vulnerability (1-5 days)
3. **Fix development** - We develop and test a fix (1-2 weeks)
4. **Fix deployment** - We deploy the fix to production
5. **Public disclosure** - We coordinate public disclosure with the reporter (typically 90 days after fix)

We may provide credit to reporters in our release notes (if desired).

## Security Updates

Security updates are announced via:
- GitHub Security Advisories
- Git commit messages (tagged with `[SECURITY]`)
- Release notes

Subscribe to repository notifications to stay informed.

## Contact

- **Security issues**: security@rushomon.cc
- **General questions**: Open a GitHub issue
- **Website**: https://rushomon.cc

## Additional Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Cloudflare Workers Security](https://developers.cloudflare.com/workers/platform/security/)
- [JWT Best Practices](https://datatracker.ietf.org/doc/html/rfc8725)

---

*Last updated: 2026-02-10*
