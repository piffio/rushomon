use crate::auth;
use crate::db;
use crate::kv;
use crate::models::{Link, link::CreateLinkRequest};
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
    if !mapping.is_active {
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
        is_active: true,
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

    // Soft delete in D1
    db::soft_delete_link(&db, link_id, org_id).await?;

    // Hard delete from KV
    let kv = ctx.kv("URL_MAPPINGS")?;
    kv::delete_link_mapping(&kv, org_id, &link.short_code).await?;

    Response::empty()
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

    // Handle OAuth callback
    let (user, _org, jwt) =
        auth::oauth::handle_oauth_callback(code, state, &kv, &db, &ctx.env).await?;

    // Extract session ID from JWT claims
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let claims = auth::session::validate_jwt(&jwt, &jwt_secret)?;

    // Store session in KV
    auth::session::store_session(&kv, &claims.session_id, &user.id, &user.org_id).await?;

    // Redirect to frontend with token in URL
    // Frontend will set the cookie on its own domain
    let frontend_url = ctx
        .env
        .var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string());

    let redirect_url = format!("{}/auth/callback?token={}", frontend_url, jwt);

    // Build redirect response (no cookie - frontend will set it)
    let headers = Headers::new();
    headers.set("Location", &redirect_url)?;

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

/// Handle logout: POST /api/auth/logout
pub async fn handle_logout(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let kv = ctx.kv("URL_MAPPINGS")?;

    auth::session::delete_session(&kv, &user_ctx.session_id).await?;

    let cookie = auth::session::create_logout_cookie();
    let mut response = Response::ok("Logged out")?;
    response.headers_mut().set("Set-Cookie", &cookie)?;

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
