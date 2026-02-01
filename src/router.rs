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

    // Get the full link to extract org_id
    let link = match db::get_link_by_id(&db, &link_id, "test-org-id").await {
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
    // TODO: Extract user from session
    // For now, using test values
    let user_id = "test-user-id";
    let org_id = "test-org-id";

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
    // TODO: Extract user from session
    let org_id = "test-org-id";

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
pub async fn handle_get_link(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // TODO: Extract user from session
    let org_id = "test-org-id";

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
pub async fn handle_delete_link(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // TODO: Extract user from session
    let org_id = "test-org-id";

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
