use crate::auth;
use crate::db;
use crate::middleware::{RateLimitConfig, RateLimiter};
use chrono::Datelike;
use worker::d1::D1Database;
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
        // Take first IP in the list
        return ip.trim().to_string();
    }
    // Fallback to X-Real-IP
    if let Ok(Some(ip)) = req.headers().get("X-Real-IP") {
        return ip;
    }
    // Last resort: use a placeholder (should never happen with Cloudflare)
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

/// Shared rate-limit + redirect_uri setup for OAuth initiation handlers
fn oauth_redirect_uri(ctx: &RouteContext<()>) -> Result<(String, String)> {
    let domain = ctx.env.var("DOMAIN")?.to_string();
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let redirect_uri = format!("{}://{}/api/auth/callback", scheme, domain);
    Ok((scheme.to_string(), redirect_uri))
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

    if !auth::providers::GITHUB.is_enabled(&ctx.env) {
        return Response::error("GitHub OAuth is not configured", 404);
    }

    let (_, redirect_uri) = oauth_redirect_uri(&ctx)?;
    auth::oauth::initiate_oauth(&req, &kv, &auth::providers::GITHUB, &redirect_uri, &ctx.env).await
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

    if !auth::providers::GOOGLE.is_enabled(&ctx.env) {
        return Response::error("Google OAuth is not configured", 404);
    }

    let (_, redirect_uri) = oauth_redirect_uri(&ctx)?;
    auth::oauth::initiate_oauth(&req, &kv, &auth::providers::GOOGLE, &redirect_uri, &ctx.env).await
}

#[utoipa::path(
    get,
    path = "/api/auth/callback",
    tag = "Authentication",
    summary = "OAuth callback",
    description = "Handles the OAuth provider callback. Validates the state parameter, exchanges the authorization code for tokens, creates or updates the user, issues a session cookie, and redirects to the dashboard",
    params(
        ("code" = String, Query, description = "Authorization code from the OAuth provider"),
        ("state" = String, Query, description = "CSRF state token"),
    ),
    responses(
        (status = 302, description = "Redirect to dashboard on success or home on failure"),
        (status = 429, description = "Rate limit exceeded"),
    )
)]
pub async fn handle_oauth_callback(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Rate limiting: 20 requests per 15 minutes per IP (same as OAuth initiation)
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

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Handle OAuth callback - returns both access and refresh tokens, plus optional redirect
    let (user, _org, tokens, redirect) =
        match auth::oauth::handle_oauth_callback(&req, code, state, &kv, &db, &ctx.env).await {
            Ok(result) => result,
            Err(e) => {
                // Check if signups are disabled
                let error_msg = format!("{:?}", e);
                if error_msg.contains("SIGNUPS_DISABLED") {
                    let frontend_url = ctx
                        .env
                        .var("FRONTEND_URL")
                        .map(|v| v.to_string())
                        .unwrap_or_else(|_| "http://localhost:5173".to_string());
                    let redirect_url = format!("{}/?error=signups_disabled", frontend_url);
                    let headers = Headers::new();
                    headers.set("Location", &redirect_url)?;
                    return Ok(Response::empty()?.with_status(302).with_headers(headers));
                }

                // Check if email is already used by different provider
                if error_msg.contains("EMAIL_ALREADY_USED") {
                    let frontend_url = ctx
                        .env
                        .var("FRONTEND_URL")
                        .map(|v| v.to_string())
                        .unwrap_or_else(|_| "http://localhost:5173".to_string());

                    let redirect_url = format!("{}/login?error=email_already_used", frontend_url);
                    let headers = Headers::new();
                    headers.set("Location", &redirect_url)?;
                    return Ok(Response::empty()?.with_status(302).with_headers(headers));
                }

                return Err(e);
            }
        };

    // Extract session ID from access token claims
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let claims = auth::session::validate_jwt(&tokens.access_token, &jwt_secret)?;

    // Store session in KV
    auth::session::store_session(&kv, &claims.session_id, &user.id, &user.org_id).await?;

    // Get frontend URL and determine scheme
    let frontend_url = ctx
        .env
        .var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string());

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
        auth::session::create_access_cookie_with_scheme(&tokens.access_token, scheme);
    let refresh_cookie =
        auth::session::create_refresh_cookie_with_scheme(&tokens.refresh_token, scheme);

    // Redirect to frontend WITHOUT token in URL (cookies set automatically)
    // Use stored redirect from OAuth state if present, otherwise default to auth callback
    let redirect_url = if let Some(redirect_path) = redirect {
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

// handle_admin_list_users and handle_admin_get_user moved to api/admin/users.rs

#[utoipa::path(
    post,
    path = "/api/admin/orgs/{id}/reset-counter",
    tag = "Admin",
    summary = "Reset org monthly counter",
    params(("id" = String, Path, description = "Org ID")),
    responses(
        (status = 200, description = "Counter reset"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_reset_monthly_counter(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    console_log!(
        "{}",
        serde_json::json!({
            "event": "admin_reset_counter_called",
            "level": "info"
        })
    );

    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    // Check if user is admin
    if user_ctx.role != "admin" {
        return Response::error("Admin access required", 403);
    }

    // Extract organization ID from route
    let org_id = match ctx.param("id") {
        Some(id) => id.to_string(),
        None => return Response::error("Missing organization ID", 400),
    };

    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(_) => return Response::error("Database not available", 500),
    };

    // Get the organization's billing account ID
    let billing_account_id = match db::get_org_billing_account(&db, &org_id).await? {
        Some(id) => id,
        None => return Response::error("Organization has no billing account", 500),
    };

    let now = chrono::Utc::now();
    let year_month = format!("{}-{:02}", now.year(), now.month());

    // Reset the monthly counter for the billing account
    match db::reset_monthly_counter_for_billing_account(&db, &billing_account_id, &year_month).await
    {
        Ok(_) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "admin_reset_counter_success",
                    "org_id": org_id,
                    "billing_account_id": billing_account_id,
                    "level": "info"
                })
            );
            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Monthly counter reset for billing account"
            }))
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "admin_reset_counter_failed",
                    "org_id": org_id,
                    "billing_account_id": billing_account_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to reset monthly counter", 500)
        }
    }
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
