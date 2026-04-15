use crate::auth;
use crate::db;
use crate::middleware::{RateLimitConfig, RateLimiter};
use chrono::{Datelike, TimeZone};
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

#[utoipa::path(
    get,
    path = "/api/admin/billing-accounts",
    tag = "Admin",
    summary = "List billing accounts",
    params(
        ("page" = Option<i64>, Query, description = "Page number"),
        ("limit" = Option<i64>, Query, description = "Items per page"),
        ("search" = Option<String>, Query, description = "Search by email or org name"),
        ("tier" = Option<String>, Query, description = "Filter by tier"),
    ),
    responses(
        (status = 200, description = "Paginated list of billing accounts"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_list_billing_accounts(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let url = req.url()?;

    // Parse query parameters
    let page = url
        .query()
        .and_then(|q| extract_query_param(q, "page").ok())
        .and_then(|p| p.parse().ok())
        .unwrap_or(1);
    let limit = url
        .query()
        .and_then(|q| extract_query_param(q, "limit").ok())
        .and_then(|l| l.parse().ok())
        .unwrap_or(50);
    let search = url
        .query()
        .and_then(|q| extract_query_param(q, "search").ok());
    let tier_filter = url
        .query()
        .and_then(|q| extract_query_param(q, "tier").ok());

    match db::list_billing_accounts_for_admin(
        &db,
        page,
        limit,
        search.as_deref(),
        tier_filter.as_deref(),
    )
    .await
    {
        Ok((accounts, total)) => {
            // Calculate next reset time (first day of next month at midnight UTC)
            let now = chrono::Utc::now();
            let next_reset = chrono::Utc
                .with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
                .single()
                .unwrap_or_else(chrono::Utc::now);
            let next_reset_timestamp = next_reset.timestamp();

            Response::from_json(&serde_json::json!({
                "accounts": accounts,
                "total": total,
                "page": page,
                "limit": limit,
                "next_reset": {
                    "utc": next_reset.to_rfc3339(),
                    "timestamp": next_reset_timestamp,
                }
            }))
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "list_billing_accounts_failed",
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to list billing accounts", 500)
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/billing-accounts/{id}",
    tag = "Admin",
    summary = "Get billing account details",
    params(("id" = String, Path, description = "Billing account ID")),
    responses(
        (status = 200, description = "Billing account details"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "Not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_get_billing_account(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let billing_account_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing billing account ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match db::get_billing_account_details(&db, billing_account_id).await {
        Ok(Some(details)) => Response::from_json(&details),
        Ok(None) => Response::error("Billing account not found", 404),
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "get_billing_account_failed",
                    "billing_account_id": billing_account_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to get billing account details", 500)
        }
    }
}

#[utoipa::path(
    put,
    path = "/api/admin/billing-accounts/{id}/tier",
    tag = "Admin",
    summary = "Update billing account tier",
    params(("id" = String, Path, description = "Billing account ID")),
    responses(
        (status = 200, description = "Tier updated"),
        (status = 400, description = "Invalid tier"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_update_billing_account_tier(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let billing_account_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing billing account ID".to_string()))?;

    #[derive(serde::Deserialize)]
    struct UpdateTierRequest {
        tier: String,
    }

    let body: UpdateTierRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    // Validate tier value
    if !matches!(
        body.tier.as_str(),
        "free" | "pro" | "business" | "unlimited"
    ) {
        return Response::error(
            "Invalid tier. Must be: free, pro, business, or unlimited",
            400,
        );
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match db::update_billing_account_tier(&db, billing_account_id, &body.tier).await {
        Ok(_) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "billing_account_tier_updated",
                    "billing_account_id": billing_account_id,
                    "new_tier": body.tier,
                    "admin_user_id": user_ctx.user_id,
                    "level": "info"
                })
            );
            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Billing account tier updated successfully",
                "tier": body.tier
            }))
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "update_billing_account_tier_failed",
                    "billing_account_id": billing_account_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to update billing account tier", 500)
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/admin/billing-accounts/{id}/reset-counter",
    tag = "Admin",
    summary = "Reset billing account monthly counter",
    params(("id" = String, Path, description = "Billing account ID")),
    responses(
        (status = 200, description = "Counter reset"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_reset_billing_account_counter(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let billing_account_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing billing account ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let current_month = chrono::Utc::now().format("%Y-%m").to_string();

    match db::reset_monthly_counter_for_billing_account(&db, billing_account_id, &current_month)
        .await
    {
        Ok(_) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "billing_account_counter_reset",
                    "billing_account_id": billing_account_id,
                    "year_month": current_month,
                    "admin_user_id": user_ctx.user_id,
                    "level": "info"
                })
            );
            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Counter reset successfully",
                "year_month": current_month
            }))
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "reset_billing_account_counter_failed",
                    "billing_account_id": billing_account_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to reset counter", 500)
        }
    }
}

#[utoipa::path(
    put,
    path = "/api/admin/billing-accounts/{id}/subscription",
    tag = "Admin",
    summary = "Update subscription status",
    params(("id" = String, Path, description = "Billing account ID")),
    responses(
        (status = 200, description = "Subscription status updated"),
        (status = 400, description = "Missing status"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "No subscription found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_update_subscription_status(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let billing_account_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing billing account ID".to_string()))?;

    let body: serde_json::Value = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            return Response::error("Invalid request body", 400);
        }
    };

    let status = match body["status"].as_str() {
        Some(s) => s.to_string(),
        None => return Response::error("status is required", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get the subscription for this billing account
    match db::get_subscription_for_billing_account(&db, billing_account_id).await? {
        Some(subscription) => {
            let subscription_id = subscription["id"].as_str().unwrap_or("");

            // Update subscription status
            match db::update_subscription_status(
                &db,
                subscription_id,
                &status,
                chrono::Utc::now().timestamp(),
            )
            .await
            {
                Ok(_) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "subscription_status_updated",
                            "billing_account_id": billing_account_id,
                            "subscription_id": subscription_id,
                            "new_status": status,
                            "admin_user_id": user_ctx.user_id,
                            "level": "info"
                        })
                    );

                    // Also update billing account tier if subscription is canceled
                    if status == "canceled" {
                        db::update_billing_account_tier(&db, billing_account_id, "free").await?;
                    }

                    Response::from_json(&serde_json::json!({
                        "success": true,
                        "message": "Subscription status updated successfully",
                        "subscription_id": subscription_id,
                        "new_status": status
                    }))
                }
                Err(e) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "update_subscription_status_failed",
                            "billing_account_id": billing_account_id,
                            "subscription_id": subscription_id,
                            "error": e.to_string(),
                            "level": "error"
                        })
                    );
                    Response::error("Failed to update subscription status", 500)
                }
            }
        }
        None => Response::error("No subscription found for this billing account", 404),
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
