/// OAuth handlers
///
/// GET /api/auth/github    - Initiate GitHub OAuth
/// GET /api/auth/google    - Initiate Google OAuth
/// GET /api/auth/callback  - OAuth provider callback
use crate::auth;
use crate::middleware::{RateLimitConfig, RateLimiter};
use crate::services::OAuthService;
use worker::*;

/// Extract client IP from Cloudflare headers
fn get_client_ip(req: &Request) -> String {
    if let Ok(Some(ip)) = req.headers().get("CF-Connecting-IP") {
        return ip;
    }
    // Fallback to X-Forwarded-For
    if let Ok(Some(forwarded)) = req.headers().get("X-Forwarded-For")
        && let Some(ip) = forwarded.split(',').next()
    {
        return ip.trim().to_string();
    }
    // Fallback to X-Real-IP
    if let Ok(Some(ip)) = req.headers().get("X-Real-IP") {
        return ip;
    }
    "unknown".to_string()
}

/// Hash an IP address for logging to avoid storing raw IPs
fn hash_ip(ip: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    ip.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Helper function to extract query parameters
fn extract_query_param(query: &str, name: &str) -> Result<String> {
    query
        .split('&')
        .find_map(|pair| {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 && parts[0] == name {
                // URL-decode the parameter value
                let decoded = urlencoding::decode(parts[1]).ok()?;
                Some(decoded.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| Error::RustError(format!("Missing {} parameter", name)))
}

/// Get frontend URL from environment
fn get_frontend_url(env: &Env) -> String {
    env.var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
}

#[utoipa::path(
    get,
    path = "/api/auth/github",
    tag = "Authentication",
    summary = "Initiate GitHub OAuth",
    description = "Redirects the user to GitHub to begin the OAuth 2.0 authorization flow",
    responses(
        (status = 302, description = "Redirect to GitHub authorization URL"),
        (status = 404, description = "GitHub OAuth not configured"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn handle_github_login(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Rate limiting: 20 requests per 15 minutes per IP
    let client_ip = get_client_ip(&req);
    let rate_limit_key = RateLimiter::ip_key("oauth", &client_ip);
    let rate_limit_config = RateLimitConfig::oauth();

    // Check if KV rate limiting is enabled (default false)
    let kv_rate_limiting_enabled = ctx
        .env
        .var("ENABLE_KV_RATE_LIMITING")
        .map(|v| v.to_string() == "true")
        .unwrap_or(false);

    if let Err(err) = RateLimiter::check(
        &kv,
        &rate_limit_key,
        &rate_limit_config,
        kv_rate_limiting_enabled,
    )
    .await
    {
        let ip_hash = hash_ip(&client_ip);
        console_log!(
            "{}",
            serde_json::json!({
                "event": "rate_limit_hit",
                "endpoint": "oauth_login",
                "limit_type": "ip",
                "ip_hash": ip_hash,
                "level": "warn"
            })
        );
        let mut response = Response::error(err.to_error_response(), 429)?;
        if let Some(retry_after) = err.retry_after() {
            response
                .headers_mut()
                .set("Retry-After", &retry_after.to_string())?;
        }
        return Ok(response);
    }

    let service = OAuthService::new();
    service.initiate_github_login(&req, &ctx).await
}

#[utoipa::path(
    get,
    path = "/api/auth/google",
    tag = "Authentication",
    summary = "Initiate Google OAuth",
    description = "Redirects the user to Google to begin the OAuth 2.0 authorization flow",
    responses(
        (status = 302, description = "Redirect to Google authorization URL"),
        (status = 404, description = "Google OAuth not configured"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn handle_google_login(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Rate limiting: 20 requests per 15 minutes per IP
    let client_ip = get_client_ip(&req);
    let rate_limit_key = RateLimiter::ip_key("oauth", &client_ip);
    let rate_limit_config = RateLimitConfig::oauth();

    let kv_rate_limiting_enabled = ctx
        .env
        .var("ENABLE_KV_RATE_LIMITING")
        .map(|v| v.to_string() == "true")
        .unwrap_or(false);

    if let Err(err) = RateLimiter::check(
        &kv,
        &rate_limit_key,
        &rate_limit_config,
        kv_rate_limiting_enabled,
    )
    .await
    {
        let ip_hash = hash_ip(&client_ip);
        console_log!(
            "{}",
            serde_json::json!({
                "event": "rate_limit_hit",
                "endpoint": "oauth_login",
                "limit_type": "ip",
                "ip_hash": ip_hash,
                "level": "warn"
            })
        );
        let mut response = Response::error(err.to_error_response(), 429)?;
        if let Some(retry_after) = err.retry_after() {
            response
                .headers_mut()
                .set("Retry-After", &retry_after.to_string())?;
        }
        return Ok(response);
    }

    let service = OAuthService::new();
    service.initiate_google_login(&req, &ctx).await
}

#[utoipa::path(
    get,
    path = "/api/auth/callback",
    tag = "Authentication",
    summary = "OAuth callback",
    description = "Handles the OAuth provider callback. Validates the state parameter, exchanges the authorization code for tokens, creates or updates the user, issues a session cookie, and redirects to the dashboard",
    params(
        ("code" = String, Query, description = "Authorization code from the OAuth provider"),
        ("state" = String, Query, description = "OAuth state parameter for CSRF protection"),
    ),
    responses(
        (status = 302, description = "Redirect to dashboard with session cookie set"),
        (status = 400, description = "Missing or invalid parameters"),
        (status = 401, description = "OAuth state validation failed"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn handle_oauth_callback(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Rate limiting: 20 requests per 15 minutes per IP (same as OAuth initiation)
    let client_ip = get_client_ip(&req);
    let rate_limit_key = RateLimiter::ip_key("oauth", &client_ip);
    let rate_limit_config = RateLimitConfig::oauth();

    let kv_rate_limiting_enabled = ctx
        .env
        .var("ENABLE_KV_RATE_LIMITING")
        .map(|v| v.to_string() == "true")
        .unwrap_or(false);

    if let Err(err) = RateLimiter::check(
        &kv,
        &rate_limit_key,
        &rate_limit_config,
        kv_rate_limiting_enabled,
    )
    .await
    {
        let ip_hash = hash_ip(&client_ip);
        console_log!(
            "{}",
            serde_json::json!({
                "event": "rate_limit_hit",
                "endpoint": "oauth_callback",
                "limit_type": "ip",
                "ip_hash": ip_hash,
                "level": "warn"
            })
        );
        let mut response = Response::error(err.to_error_response(), 429)?;
        if let Some(retry_after) = err.retry_after() {
            response
                .headers_mut()
                .set("Retry-After", &retry_after.to_string())?;
        }
        return Ok(response);
    }

    // Extract code and state from query params
    let url = req.url()?;
    let query = url
        .query()
        .ok_or_else(|| Error::RustError("Missing query parameters".to_string()))?;

    let code = extract_query_param(query, "code")?;
    let state = extract_query_param(query, "state")?;

    let service = OAuthService::new();
    let result = match service.handle_callback(&req, &code, &state, &ctx).await {
        Ok(result) => result,
        Err(e) => {
            // Check if signups are disabled
            let error_msg = format!("{:?}", e);
            if error_msg.contains("SIGNUPS_DISABLED") {
                let frontend_url = get_frontend_url(&ctx.env);
                let redirect_url = format!("{}/?error=signups_disabled", frontend_url);
                let headers = Headers::new();
                headers.set("Location", &redirect_url)?;
                return Ok(Response::empty()?.with_status(302).with_headers(headers));
            }

            // Check if email is already used by different provider
            if error_msg.contains("EMAIL_ALREADY_USED") {
                let frontend_url = get_frontend_url(&ctx.env);
                let redirect_url = format!("{}/login?error=email_already_used", frontend_url);
                let headers = Headers::new();
                headers.set("Location", &redirect_url)?;
                return Ok(Response::empty()?.with_status(302).with_headers(headers));
            }

            console_log!(
                "{}",
                serde_json::json!({
                    "event": "oauth_callback_failed",
                    "error": error_msg,
                    "level": "error"
                })
            );
            return Response::error("OAuth callback failed", 500);
        }
    };

    // Extract session ID from access token claims and store session in KV
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let claims = auth::session::validate_jwt(&result.tokens.access_token, &jwt_secret)?;

    // Store session in KV
    auth::session::store_session(
        &kv,
        &claims.session_id,
        &result.user.id,
        &result.user.org_id,
    )
    .await?;

    // Get frontend URL and determine scheme
    let frontend_url = get_frontend_url(&ctx.env);
    let domain = ctx
        .env
        .var("DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "localhost:8787".to_string());

    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };

    // Set both access and refresh tokens as httpOnly cookies (no token in URL)
    let access_cookie =
        auth::session::create_access_cookie_with_scheme(&result.tokens.access_token, scheme);
    let refresh_cookie =
        auth::session::create_refresh_cookie_with_scheme(&result.tokens.refresh_token, scheme);

    // Redirect to frontend WITHOUT token in URL (cookies set automatically)
    // Use stored redirect from OAuth state if present, otherwise default to auth callback
    let redirect_url = if let Some(redirect_path) = result.redirect {
        format!("{}{}", frontend_url, redirect_path)
    } else {
        format!("{}/auth/callback", frontend_url)
    };

    // Build redirect response with both cookies
    let headers = Headers::new();
    headers.set("Location", &redirect_url)?;
    // Note: Multiple Set-Cookie headers need to be appended separately
    headers.append("Set-Cookie", &access_cookie)?;
    headers.append("Set-Cookie", &refresh_cookie)?;

    Ok(Response::empty()?.with_status(302).with_headers(headers))
}
