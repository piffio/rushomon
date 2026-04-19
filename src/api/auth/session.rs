/// Auth session handlers
///
/// GET  /api/auth/me      — get current user
/// POST /api/auth/refresh — refresh access token
/// POST /api/auth/logout  — logout and clear cookies
use crate::auth;
use crate::middleware::{RateLimitConfig, RateLimiter, is_kv_rate_limiting_enabled};
use crate::services::AuthService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

// ── GET /api/auth/me ─────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "Authentication",
    summary = "Get current user",
    description = "Returns information about the currently authenticated user",
    responses(
        (status = 200, description = "User information", body = crate::models::user::User),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_current_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_get_current_user(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_get_current_user(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    // Rate limiting: 100 requests per minute per session
    let kv = ctx.kv("URL_MAPPINGS")?;
    let rate_limit_key = RateLimiter::session_key("auth_check", &user_ctx.session_id);
    let rate_limit_config = RateLimitConfig::auth_check();

    if let Err(err) = RateLimiter::check(
        &kv,
        &rate_limit_key,
        &rate_limit_config,
        is_kv_rate_limiting_enabled(&ctx.env),
    )
    .await
    {
        console_log!(
            "{}",
            serde_json::json!({
                "event": "rate_limit_hit",
                "endpoint": "auth_me",
                "limit_type": "session",
                "session_id": user_ctx.session_id,
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

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let auth_service = AuthService::new();
    let user = auth_service.get_user_by_id(&db, &user_ctx.user_id).await?;

    match user {
        Some(user) => Ok(Response::from_json(&user)?),
        None => Err(AppError::NotFound("User not found".to_string())),
    }
}

// ── POST /api/auth/refresh ────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "Authentication",
    summary = "Refresh access token",
    description = "Validates the refresh token from the httpOnly cookie and issues a new short-lived access token. Rate limited to 30 requests per hour per session",
    responses(
        (status = 200, description = "Access token refreshed, new cookie set"),
        (status = 401, description = "Missing, invalid, or expired refresh token"),
        (status = 429, description = "Rate limit exceeded"),
    ),
    security(
        ("session_cookie" = [])
    )
)]
pub async fn handle_token_refresh(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Extract refresh token from cookie
    let cookie_header = match req.headers().get("Cookie") {
        Ok(Some(header)) => header,
        Ok(None) => return Response::error("Missing refresh token", 401),
        Err(_) => return Response::error("Failed to read cookies", 500),
    };

    let refresh_token = match auth::session::parse_refresh_cookie_header(&cookie_header) {
        Some(token) => token,
        None => return Response::error("Missing refresh token", 401),
    };

    // Validate refresh token JWT
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let claims = match auth::session::validate_jwt(&refresh_token, &jwt_secret) {
        Ok(claims) => claims,
        Err(_) => return Response::error("Invalid or expired refresh token", 401),
    };

    // Verify it's a refresh token
    if claims.token_type != "refresh" {
        return Response::error("Invalid token type", 401);
    }

    // Rate limiting: 30 requests per hour per session
    let rate_limit_key = RateLimiter::session_key("token_refresh", &claims.session_id);
    let rate_limit_config = RateLimitConfig::token_refresh();

    if let Err(err) = RateLimiter::check(
        &kv,
        &rate_limit_key,
        &rate_limit_config,
        is_kv_rate_limiting_enabled(&ctx.env),
    )
    .await
    {
        console_log!(
            "{}",
            serde_json::json!({
                "event": "rate_limit_hit",
                "endpoint": "token_refresh",
                "limit_type": "session",
                "session_id": claims.session_id,
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

    // Verify session still exists in KV
    let session = match auth::session::get_session(&kv, &claims.session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => return Response::error("Session expired or invalid", 401),
        Err(_) => return Response::error("Failed to validate session", 500),
    };

    // Verify user_id matches (constant-time comparison to prevent timing attacks)
    use subtle::ConstantTimeEq;
    let session_user_id_bytes = session.user_id.as_bytes();
    let claims_user_id_bytes = claims.sub.as_bytes();

    let max_len = session_user_id_bytes.len().max(claims_user_id_bytes.len());
    let mut session_padded = vec![0u8; max_len];
    let mut claims_padded = vec![0u8; max_len];

    session_padded[..session_user_id_bytes.len()].copy_from_slice(session_user_id_bytes);
    claims_padded[..claims_user_id_bytes.len()].copy_from_slice(claims_user_id_bytes);

    let is_equal: bool = session_padded.ct_eq(&claims_padded).into();
    if !is_equal {
        return Response::error("Session mismatch", 401);
    }

    // Get fresh user data from database to get current role
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let auth_service = AuthService::new();
    let user = match auth_service.get_user_by_id(&db, &claims.sub).await? {
        Some(user) => user,
        None => return Response::error("User not found", 404),
    };

    // Generate new access token (1 hour) with fresh role from database
    let new_access_token = auth::session::create_access_token(
        &claims.sub,
        &claims.org_id,
        &claims.session_id,
        &user.role,
        &jwt_secret,
    )?;

    // Determine scheme for cookie (secure flag)
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

    let access_cookie = auth::session::create_access_cookie_with_scheme(&new_access_token, scheme);

    let mut response = Response::ok("Token refreshed successfully")?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;

    Ok(response)
}

// ── POST /api/auth/logout ─────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "Authentication",
    summary = "Logout",
    description = "Deletes the server-side session and clears all auth cookies (access token, refresh token, legacy session)",
    responses(
        (status = 200, description = "Logged out successfully"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_logout(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_logout(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_logout(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let kv = ctx.kv("URL_MAPPINGS")?;

    let auth_service = AuthService::new();
    auth_service.logout(&kv, &user_ctx.session_id).await?;

    // Clear all three cookies: access token, refresh token, and legacy session
    let access_cookie = auth::session::create_access_logout_cookie();
    let refresh_cookie = auth::session::create_refresh_logout_cookie();
    let session_cookie = auth::session::create_logout_cookie();

    let mut response = Response::ok("Logged out successfully")?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    response
        .headers_mut()
        .append("Set-Cookie", &refresh_cookie)?;
    response
        .headers_mut()
        .append("Set-Cookie", &session_cookie)?;

    Ok(response)
}
