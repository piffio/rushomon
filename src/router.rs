use crate::auth;
use crate::db;
use crate::kv;
use crate::models::{
    Link,
    link::{CreateLinkRequest, LinkStatus, UpdateLinkRequest},
};
use crate::utils::{generate_short_code, now_timestamp, validate_short_code, validate_url};
use worker::d1::D1Database;
use worker::*;

/// Handle public short code redirects: GET /{short_code}
pub async fn handle_redirect(
    req: Request,
    ctx: RouteContext<()>,
    short_code: String,
) -> Result<Response> {
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Look up the link mapping in KV
    let mapping = kv::get_link_mapping(&kv, &short_code).await?;

    let Some(mapping) = mapping else {
        return Response::error("Link not found", 404);
    };

    // Check if link is active
    if !matches!(mapping.status, LinkStatus::Active) {
        return Response::error("Link not found", 404);
    }

    // Check if expired
    if let Some(expires_at) = mapping.expires_at {
        let now = now_timestamp();

        if now > expires_at {
            return Response::error("Link expired", 410);
        }
    }

    // Log analytics asynchronously (non-blocking)
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let link_id = mapping.link_id.clone();

    // Get the full link to extract org_id (no auth check for public redirects)
    let link = match db::get_link_by_id_no_auth(&db, &link_id).await {
        Ok(Some(link)) => link,
        Ok(None) => {
            // If we can't get the link, we'll skip analytics but still redirect
            return Response::redirect_with_status(Url::parse(&mapping.destination_url)?, 301);
        }
        Err(e) => {
            console_log!("Error getting link: {}", e);
            // If there's an error, we'll skip analytics but still redirect
            return Response::redirect_with_status(Url::parse(&mapping.destination_url)?, 301);
        }
    };

    let referrer = req.headers().get("Referer").ok().flatten();
    let user_agent = req.headers().get("User-Agent").ok().flatten();
    let country = req.headers().get("CF-IPCountry").ok().flatten();
    let city = req.headers().get("CF-IPCity").ok().flatten();

    // Create analytics event
    let now = now_timestamp();
    let event = crate::models::AnalyticsEvent {
        id: None,
        link_id: link_id.clone(),
        org_id: link.org_id, // Use actual org_id from the link
        timestamp: now,
        referrer,
        user_agent,
        country,
        city,
    };

    // Log analytics (awaited to ensure completion before Worker terminates)
    // Note: spawn_local doesn't work reliably in Workers - background tasks can be
    // cancelled when the response is sent. We await these operations instead.
    // Performance impact is minimal (~10-50ms) and ensures analytics are captured.
    if let Err(e) = db::log_analytics_event(&db, &event).await {
        console_log!("Analytics event logging failed: {}", e);
    }
    if let Err(e) = db::increment_click_count(&db, &link_id).await {
        console_log!("Click count increment failed: {}", e);
    }

    // Perform 301 permanent redirect
    // Analytics are now guaranteed to complete
    let destination_url = Url::parse(&mapping.destination_url)?;
    Response::redirect_with_status(destination_url, 301)
}

/// Handle link creation: POST /api/links
pub async fn handle_create_link(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Authenticate request
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let user_id = &user_ctx.user_id;
    let org_id = &user_ctx.org_id;

    // Parse request body with proper error handling
    let raw_body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => {
            return Response::error(format!("Invalid JSON: {}", e), 400);
        }
    };

    // Validate that only expected fields are present
    let expected_fields = ["destination_url", "short_code", "title", "expires_at"];
    if let Some(obj) = raw_body.as_object() {
        for field_name in obj.keys() {
            if !expected_fields.contains(&field_name.as_str()) {
                return Response::error(
                    format!(
                        "Unknown field '{}'. Expected fields: destination_url, short_code (optional), title (optional), expires_at (optional)",
                        field_name
                    ),
                    400,
                );
            }
        }
    } else {
        return Response::error("Request body must be a JSON object", 400);
    }

    // Convert to typed struct
    let body: CreateLinkRequest = match serde_json::from_value(raw_body) {
        Ok(body) => body,
        Err(e) => {
            return Response::error(format!("Invalid request format: {}", e), 400);
        }
    };

    // Validate destination URL
    let destination_url = match validate_url(&body.destination_url) {
        Ok(url) => url,
        Err(e) => {
            return Response::error(format!("Invalid destination URL: {}", e), 400);
        }
    };

    // Generate or validate short code
    let short_code = if let Some(custom_code) = body.short_code {
        match validate_short_code(&custom_code) {
            Ok(code) => code,
            Err(e) => {
                return Response::error(format!("Invalid short code: {}", e), 400);
            }
        };

        // Check if already exists
        let kv = ctx.kv("URL_MAPPINGS")?;
        if kv::links::short_code_exists(&kv, &custom_code).await? {
            return Response::error("Short code already in use", 409);
        }

        custom_code
    } else {
        // Generate random code and check for collisions (very rare)
        let kv = ctx.kv("URL_MAPPINGS")?;
        let mut code = generate_short_code();
        let mut attempts = 0;

        while kv::links::short_code_exists(&kv, &code).await? {
            code = generate_short_code();
            attempts += 1;
            if attempts > 10 {
                return Response::error("Failed to generate unique short code", 500);
            }
        }

        code
    };

    // Create link record
    let link_id = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();

    let link = Link {
        id: link_id.clone(),
        org_id: org_id.to_string(),
        short_code: short_code.clone(),
        destination_url: destination_url.clone(),
        title: body.title,
        created_by: user_id.to_string(),
        created_at: now,
        updated_at: None,
        expires_at: body.expires_at,
        status: LinkStatus::Active,
        click_count: 0,
    };

    // Store in D1
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    db::create_link(&db, &link).await?;

    // Store in KV
    let kv = ctx.kv("URL_MAPPINGS")?;
    let mapping = link.to_mapping();
    kv::store_link_mapping(&kv, org_id, &short_code, &mapping).await?;

    Response::from_json(&link)
}

/// Handle listing links: GET /api/links
pub async fn handle_list_links(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Authenticate request
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    // Parse pagination params
    let url = req.url()?;
    let page: i64 = url
        .query()
        .and_then(|q| {
            q.split('&')
                .find(|s| s.starts_with("page="))
                .and_then(|s| s.split('=').nth(1))
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(1);

    let limit: i64 = url
        .query()
        .and_then(|q| {
            q.split('&')
                .find(|s| s.starts_with("limit="))
                .and_then(|s| s.split('=').nth(1))
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(20);

    let offset = (page - 1) * limit;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let links = db::get_links_by_org(&db, org_id, limit, offset).await?;

    Response::from_json(&links)
}

/// Handle getting a single link: GET /api/links/{id}
pub async fn handle_get_link(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Authenticate request
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing link ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let link = db::get_link_by_id(&db, link_id, org_id).await?;

    match link {
        Some(link) => Response::from_json(&link),
        None => Response::error("Link not found", 404),
    }
}

/// Handle link deletion: DELETE /api/links/{id}
pub async fn handle_delete_link(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Authenticate request
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing link ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get link first to get short_code
    let link = db::get_link_by_id(&db, link_id, org_id).await?;

    let Some(link) = link else {
        return Response::error("Link not found", 404);
    };

    // Hard delete from D1 (frees up short code)
    db::hard_delete_link(&db, link_id, org_id).await?;

    // Hard delete from KV
    let kv = ctx.kv("URL_MAPPINGS")?;
    kv::delete_link_mapping(&kv, org_id, &link.short_code).await?;

    Response::empty()
}

/// Handle link update: PUT /api/links/:id
pub async fn handle_update_link(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Authenticate
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    // Extract link ID from route
    let link_id = match ctx.param("id") {
        Some(id) => id.to_string(),
        None => return Response::error("Missing link ID", 400),
    };

    // Parse request body
    let update_req: UpdateLinkRequest = match req.json().await {
        Ok(req) => req,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    // Validate destination URL if provided
    if let Some(url) = &update_req.destination_url
        && let Err(e) = validate_url(url)
    {
        return Response::error(format!("Invalid URL: {}", e), 400);
    }

    // Validate expiration date if provided
    if let Some(expires_at) = update_req.expires_at {
        let now = now_timestamp();
        if expires_at <= now {
            return Response::error("Expiration date must be in the future", 400);
        }
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Get existing link to verify ownership
    let existing_link = match db::get_link_by_id(&db, &link_id, &user_ctx.org_id).await? {
        Some(link) => link,
        None => return Response::error("Link not found", 404),
    };

    // Update in D1
    let updated_link = db::update_link(
        &db,
        &link_id,
        &user_ctx.org_id,
        update_req.destination_url.as_deref(),
        update_req.title.as_deref(),
        update_req.status.as_ref().map(|s| s.as_str()),
        update_req.expires_at,
    )
    .await?;

    // If destination URL or status changed, update KV mapping
    if update_req.destination_url.is_some() || update_req.status.is_some() {
        let mapping = updated_link.to_mapping();
        kv::update_link_mapping(&kv, &existing_link.short_code, &mapping).await?;
    }

    Response::from_json(&updated_link)
}

/// Handle GitHub OAuth initiation: GET /api/auth/github
pub async fn handle_github_login(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("URL_MAPPINGS")?;
    let client_id = ctx.env.var("GITHUB_CLIENT_ID")?.to_string();
    let domain = ctx.env.var("DOMAIN")?.to_string();

    // Use http for localhost, https for production (consistent with callback handling)
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let redirect_uri = format!("{}://{}/api/auth/callback", scheme, domain);

    auth::oauth::initiate_github_oauth(&kv, &client_id, &redirect_uri, &ctx.env).await
}

/// Handle OAuth callback: GET /api/auth/callback
pub async fn handle_oauth_callback(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Extract code and state from query params
    let url = req.url()?;
    let query = url
        .query()
        .ok_or_else(|| Error::RustError("Missing query parameters".to_string()))?;

    let code = extract_query_param(query, "code")?;
    let state = extract_query_param(query, "state")?;

    let kv = ctx.kv("URL_MAPPINGS")?;
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Handle OAuth callback - returns both access and refresh tokens
    let (user, _org, tokens) =
        auth::oauth::handle_oauth_callback(code, state, &kv, &db, &ctx.env).await?;

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

    // Redirect to frontend with access token in URL (frontend stores in localStorage)
    let redirect_url = format!(
        "{}/auth/callback?token={}",
        frontend_url, tokens.access_token
    );

    // Set refresh token as httpOnly cookie
    let refresh_cookie =
        auth::session::create_refresh_cookie_with_scheme(&tokens.refresh_token, scheme);

    // Build redirect response with refresh cookie
    let headers = Headers::new();
    headers.set("Location", &redirect_url)?;
    headers.set("Set-Cookie", &refresh_cookie)?;

    Ok(Response::empty()?.with_status(302).with_headers(headers))
}

/// Handle get current user: GET /api/auth/me
pub async fn handle_get_current_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let user = db::get_user_by_id(&db, &user_ctx.user_id).await?;

    match user {
        Some(user) => Response::from_json(&user),
        None => Response::error("User not found", 404),
    }
}

/// Handle token refresh: POST /api/auth/refresh
/// Validates refresh token from cookie and returns new access token
pub async fn handle_token_refresh(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Extract refresh token from cookie
    let cookie_header = match req.headers().get("Cookie") {
        Ok(Some(header)) => header,
        Ok(None) => {
            return Response::error("Missing refresh token", 401);
        }
        Err(_) => {
            return Response::error("Failed to read cookies", 500);
        }
    };

    let refresh_token = match auth::session::parse_refresh_cookie_header(&cookie_header) {
        Some(token) => token,
        None => {
            return Response::error("Missing refresh token", 401);
        }
    };

    // Validate refresh token JWT
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let claims = match auth::session::validate_jwt(&refresh_token, &jwt_secret) {
        Ok(claims) => claims,
        Err(_) => {
            return Response::error("Invalid or expired refresh token", 401);
        }
    };

    // Verify it's a refresh token
    if claims.token_type != "refresh" {
        return Response::error("Invalid token type", 401);
    }

    // Verify session still exists in KV
    let kv = ctx.kv("URL_MAPPINGS")?;
    let session = match auth::session::get_session(&kv, &claims.session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            return Response::error("Session expired or invalid", 401);
        }
        Err(_) => {
            return Response::error("Failed to validate session", 500);
        }
    };

    // Verify user_id matches
    if session.user_id != claims.sub {
        return Response::error("Session mismatch", 401);
    }

    // Generate new access token (1 hour)
    let new_access_token = auth::session::create_access_token(
        &claims.sub,
        &claims.org_id,
        &claims.session_id,
        &jwt_secret,
    )?;

    // Return new access token as JSON
    Response::from_json(&serde_json::json!({
        "access_token": new_access_token
    }))
}

/// Handle logout: POST /api/auth/logout
pub async fn handle_logout(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let kv = ctx.kv("URL_MAPPINGS")?;

    auth::session::delete_session(&kv, &user_ctx.session_id).await?;

    // Clear both session cookie and refresh token cookie
    let session_cookie = auth::session::create_logout_cookie();
    let refresh_cookie = auth::session::create_refresh_logout_cookie();

    let mut response = Response::ok("Logged out successfully")?;

    // Set both logout cookies
    // Note: We can only set one Set-Cookie header, so we combine them
    // However, the worker crate may handle multiple Set-Cookie headers differently
    // For now, we'll set the refresh cookie (more important for security)
    response.headers_mut().set("Set-Cookie", &session_cookie)?;
    response
        .headers_mut()
        .append("Set-Cookie", &refresh_cookie)?;

    Ok(response)
}

/// Helper function to extract query parameters
fn extract_query_param(query: &str, name: &str) -> Result<String> {
    query
        .split('&')
        .find_map(|pair| {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 && parts[0] == name {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| Error::RustError(format!("Missing {} parameter", name)))
}
