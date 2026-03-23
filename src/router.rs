use crate::auth;
use crate::db;
use crate::kv;
use crate::middleware::{RateLimitConfig, RateLimiter};
use crate::models::{
    LinkAnalyticsResponse, PaginatedResponse, PaginationMeta, Tier, TimeRange,
    link::{CreateLinkRequest, Link, LinkStatus, UpdateLinkRequest},
};
use crate::utils::{generate_short_code, now_timestamp, validate_short_code, validate_url};
use chrono::{Datelike, TimeZone};
use std::future::Future;
use std::pin::Pin;
use worker::d1::D1Database;
use worker::*;

/// Get the frontend URL from environment, with localhost fallback for local dev
pub fn get_frontend_url(env: &Env) -> String {
    env.var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
}

/// Extract client IP from Cloudflare headers
fn get_client_ip(req: &Request) -> String {
    // Try CF-Connecting-IP first (most reliable with Cloudflare)
    if let Ok(Some(ip)) = req.headers().get("CF-Connecting-IP") {
        return ip;
    }

    // Fallback to X-Forwarded-For
    if let Ok(Some(forwarded)) = req.headers().get("X-Forwarded-For") {
        // Take first IP in the list
        if let Some(ip) = forwarded.split(',').next() {
            return ip.trim().to_string();
        }
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

async fn resolved_forward_for_link(db: &D1Database, link: &Link) -> bool {
    if let Some(value) = link.forward_query_params {
        value
    } else {
        db::get_org_forward_query_params(db, &link.org_id)
            .await
            .unwrap_or(false)
    }
}

async fn sync_link_mapping_from_link(
    db: &D1Database,
    kv_store: &worker::kv::KvStore,
    link: &Link,
) -> Result<()> {
    match link.status {
        LinkStatus::Blocked => {
            kv::delete_link_mapping(kv_store, &link.org_id, &link.short_code).await
        }
        LinkStatus::Active | LinkStatus::Disabled => {
            let resolved_forward = resolved_forward_for_link(db, link).await;
            let mapping = link.to_mapping(resolved_forward);
            kv::update_link_mapping(kv_store, &link.short_code, &mapping).await
        }
    }
}

/// Result of a redirect operation, containing the response and optional deferred analytics work.
pub struct RedirectResult {
    pub response: Response,
    /// Optional future for analytics logging, to be executed via `ctx.wait_until()`.
    /// This allows the redirect response to be sent immediately without waiting for D1 writes.
    pub analytics_future: Option<Pin<Box<dyn Future<Output = ()> + 'static>>>,
}

/// Handle public short code redirects: GET /{short_code}
///
/// Returns a RedirectResult containing the redirect response and an optional
/// analytics future. The caller should use `Context::wait_until()` to execute
/// the analytics future after sending the response, avoiding blocking the redirect.
pub async fn handle_redirect(
    req: Request,
    ctx: RouteContext<()>,
    short_code: String,
) -> Result<RedirectResult> {
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Rate limiting: 300 requests per minute per IP
    let client_ip = get_client_ip(&req);
    let rate_limit_key = RateLimiter::ip_key("redirect", &client_ip);
    let rate_limit_config = RateLimitConfig::redirect();

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
                "endpoint": "redirect",
                "limit_type": "ip_per_code",
                "ip_hash": ip_hash,
                "short_code": short_code,
                "level": "warn"
            })
        );
        let mut response = Response::error(err.to_error_response(), 429)?;
        if let Some(retry_after) = err.retry_after() {
            response
                .headers_mut()
                .set("Retry-After", &retry_after.to_string())?;
        }
        return Ok(RedirectResult {
            response,
            analytics_future: None,
        });
    }

    // Look up the link mapping in KV
    let mapping = kv::get_link_mapping(&kv, &short_code).await?;

    let Some(mapping) = mapping else {
        let url = Url::parse(&format!("{}/404", get_frontend_url(&ctx.env)))?;
        return Ok(RedirectResult {
            response: Response::redirect_with_status(url, 302)?,
            analytics_future: None,
        });
    };

    // Check if link is active
    if !matches!(mapping.status, LinkStatus::Active) {
        let url = Url::parse(&format!("{}/404", get_frontend_url(&ctx.env)))?;
        return Ok(RedirectResult {
            response: Response::redirect_with_status(url, 302)?,
            analytics_future: None,
        });
    }

    // Check if expired
    if let Some(expires_at) = mapping.expires_at {
        let now = now_timestamp();

        if now > expires_at {
            let url = Url::parse(&format!("{}/404", get_frontend_url(&ctx.env)))?;
            return Ok(RedirectResult {
                response: Response::redirect_with_status(url, 302)?,
                analytics_future: None,
            });
        }
    }

    // Build the redirect URL, applying UTM params and forwarding as needed
    let mut destination_url = Url::parse(&mapping.destination_url)?;

    // Feature A: apply static UTM params (always, if set on the link)
    if let Some(ref utm) = mapping.utm_params {
        let pairs: Vec<(&str, &str)> = [
            ("utm_source", utm.utm_source.as_deref()),
            ("utm_medium", utm.utm_medium.as_deref()),
            ("utm_campaign", utm.utm_campaign.as_deref()),
            ("utm_term", utm.utm_term.as_deref()),
            ("utm_content", utm.utm_content.as_deref()),
            ("utm_ref", utm.utm_ref.as_deref()),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.filter(|s| !s.is_empty()).map(|s| (k, s)))
        .collect();

        if !pairs.is_empty() {
            let mut q = destination_url.query_pairs_mut();
            for (k, v) in pairs {
                q.append_pair(k, v);
            }
        }
    }

    // Feature B: forward incoming visitor query params (visitor wins on conflict)
    if mapping.forward_query_params
        && let Ok(incoming_url) = req.url()
    {
        let visitor_pairs: Vec<(String, String)> = incoming_url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        if !visitor_pairs.is_empty() {
            let mut q = destination_url.query_pairs_mut();
            for (k, v) in &visitor_pairs {
                q.append_pair(k, v);
            }
        }
    }

    let response = Response::redirect_with_status(destination_url, 301)?;

    // Collect analytics data from the request before it's consumed
    let referrer = req.headers().get("Referer").ok().flatten();
    let user_agent = req.headers().get("User-Agent").ok().flatten();
    let country = req.headers().get("CF-IPCountry").ok().flatten();
    let city = req.headers().get("CF-IPCity").ok().flatten();

    // Prepare deferred analytics work (executed via wait_until after response is sent)
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let link_id = mapping.link_id.clone();
    let now = now_timestamp();

    let analytics_future: Pin<Box<dyn Future<Output = ()> + 'static>> = Box::pin(async move {
        // Get the full link to extract org_id and check status (no auth check for public redirects)
        let link = match db::get_link_by_id_no_auth(&db, &link_id).await {
            Ok(Some(link)) => link,
            Ok(None) => {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "analytics_link_not_found",
                        "link_id": link_id,
                        "level": "warn"
                    })
                );
                return;
            }
            Err(_) => {
                return;
            }
        };

        if !matches!(link.status, LinkStatus::Active) {
            return;
        }

        let event = crate::models::AnalyticsEvent {
            id: None,
            link_id: link_id.clone(),
            org_id: link.org_id,
            timestamp: now,
            referrer,
            user_agent,
            country,
            city,
        };

        if let Err(e) = db::log_analytics_event(&db, &event).await {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "analytics_event_failed",
                    "link_id": link_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
        }
        if let Err(e) = db::increment_click_count(&db, &link_id).await {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "click_count_failed",
                    "link_id": link_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
        }
    });

    Ok(RedirectResult {
        response,
        analytics_future: Some(analytics_future),
    })
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

    // Rate limiting: 100 link creations per hour per user
    let kv = ctx.kv("URL_MAPPINGS")?;
    let rate_limit_key = RateLimiter::user_key("create_link", user_id);
    let rate_limit_config = RateLimitConfig::link_creation();

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
        console_log!(
            "{}",
            serde_json::json!({
                "event": "rate_limit_hit",
                "endpoint": "create_link",
                "limit_type": "user",
                "user_id": user_id,
                "level": "warn"
            })
        );
        let mut response = Response::error(err.to_error_response(), 429)?;
        if let Some(retry_after) = err.retry_after() {
            response
                .headers_mut()
                .set("Retry-After", &retry_after.to_string())?;
        }
        response.headers_mut().set(
            "X-RateLimit-Limit",
            &rate_limit_config.max_requests.to_string(),
        )?;
        return Ok(response);
    }

    // Tier-based link creation limit (enforced at billing account level)
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get billing account for this org
    let billing_account = db::get_billing_account_for_org(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("No billing account found for organization".to_string()))?;

    // Get tier limits from billing account (not org)
    let tier = Tier::from_str_value(&billing_account.tier);
    let limits = tier.as_ref().map(|t| t.limits());

    // Check link creation limit for this tier
    // Quota is shared across all orgs in the billing account
    if let Some(ref tier_limits) = limits
        && let Some(max_links) = tier_limits.max_links_per_month
    {
        let now = chrono::Utc::now();
        let year_month = format!("{}-{:02}", now.year(), now.month());

        // Try to increment the counter at billing account level
        let can_create = db::increment_monthly_counter_for_billing_account(
            &db,
            &billing_account.id,
            &year_month,
            max_links,
        )
        .await?;

        if !can_create {
            let current_count =
                db::get_monthly_counter_for_billing_account(&db, &billing_account.id, &year_month)
                    .await?;
            let remaining = max_links.saturating_sub(current_count);
            let message = if remaining > 0 {
                format!(
                    "You can create {} more short links this month across all organizations.",
                    remaining
                )
            } else {
                "You have reached your monthly link limit across all organizations. Upgrade your plan to create more links."
                    .to_string()
            };
            return Response::error(message, 403);
        }
    }

    // Parse request body with proper error handling
    let raw_body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => {
            return Response::error(format!("Invalid JSON: {}", e), 400);
        }
    };

    // Validate that only expected fields are present
    let expected_fields = [
        "destination_url",
        "short_code",
        "title",
        "expires_at",
        "tags",
        "utm_params",
        "forward_query_params",
    ];
    if let Some(obj) = raw_body.as_object() {
        for field_name in obj.keys() {
            if !expected_fields.contains(&field_name.as_str()) {
                return Response::error(
                    format!(
                        "Unknown field '{}'. Expected fields: destination_url, short_code (optional), title (optional), expires_at (optional), tags (optional), utm_params (optional, Pro+), forward_query_params (optional, Pro+)",
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

    // Check if destination is blacklisted
    if db::is_destination_blacklisted(&db, &destination_url).await? {
        return Response::error("Destination URL is blocked", 403);
    }

    // Validate title length (max 200 characters)
    if let Some(ref title) = body.title
        && title.len() > 200
    {
        return Response::error("Title must be 200 characters or less", 400);
    }

    // Check if custom short code is allowed for this tier
    let allow_custom = limits
        .as_ref()
        .map(|l| l.allow_custom_short_code)
        .unwrap_or(false);
    if body.short_code.is_some() && !allow_custom {
        return Response::error(
            "Custom short codes are not available on the free tier. Upgrade to Pro.",
            403,
        );
    }

    // Check if Pro features (UTM params / query forwarding) are allowed
    let is_pro_or_above = matches!(
        tier.as_ref(),
        Some(Tier::Pro) | Some(Tier::Business) | Some(Tier::Unlimited)
    );
    let wants_pro_features = body
        .utm_params
        .as_ref()
        .map(|u| !u.is_empty())
        .unwrap_or(false)
        || body.forward_query_params.is_some();
    if wants_pro_features && !is_pro_or_above {
        return Response::error(
            "UTM parameters and query parameter forwarding require a Pro plan or above.",
            403,
        );
    }

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

    // Validate and normalize tags if provided
    let normalized_tags = if let Some(tags) = body.tags {
        match db::validate_and_normalize_tags(&tags) {
            Ok(t) => t,
            Err(e) => return Response::error(e.to_string(), 400),
        }
    } else {
        Vec::new()
    };

    // Check tag limit for this tier (enforced at billing account level)
    if let Some(ref tier_limits) = limits
        && let Some(max_tags) = tier_limits.max_tags
    {
        // Get current count of distinct tags in the billing account
        let current_tag_count =
            db::count_distinct_tags_for_billing_account(&db, &billing_account.id).await?;

        // Find which tags are new to the BA (not already in use)
        let mut new_tag_count = 0;
        if !normalized_tags.is_empty() {
            // Get all existing tags in the BA
            let existing_tags_query = db.prepare(
                "SELECT DISTINCT tag_name
                 FROM link_tags lt
                 JOIN organizations o ON lt.org_id = o.id
                 WHERE o.billing_account_id = ?1",
            );
            let existing_tags_result = existing_tags_query
                .bind(&[billing_account.id.clone().into()])?
                .all()
                .await?;
            let existing_tags_set: std::collections::HashSet<String> = existing_tags_result
                .results::<serde_json::Value>()?
                .iter()
                .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
                .collect();

            // Count how many of the submitted tags are new
            new_tag_count = normalized_tags
                .iter()
                .filter(|tag| !existing_tags_set.contains(*tag))
                .count() as i64;
        }

        // Check if adding these new tags would exceed the limit
        if current_tag_count + new_tag_count > max_tags {
            let remaining = max_tags.saturating_sub(current_tag_count);
            let message = if remaining > 0 {
                format!(
                    "You can create {} more tag{} across all organizations. Upgrade your plan to add more tags.",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                )
            } else {
                "You have reached your tag limit across all organizations. Upgrade your plan to create more tags."
                    .to_string()
            };
            return Response::error(message, 403);
        }
    }

    // Create link record
    let link_id = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();

    // Normalise UTM params: treat all-empty as None
    let utm_params = body.utm_params.filter(|u| !u.is_empty());

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
        tags: normalized_tags.clone(),
        utm_params,
        forward_query_params: body.forward_query_params,
    };

    // Store in D1
    db::create_link(&db, &link).await?;

    // Store tags
    if !normalized_tags.is_empty() {
        db::set_tags_for_link(&db, &link_id, org_id, &normalized_tags).await?;
    }

    // Resolve forwarding flag: per-link setting overrides org default
    let resolved_forward = link.forward_query_params.unwrap_or(false); // org default fetched below if None
    let resolved_forward = if link.forward_query_params.is_none() {
        db::get_org_forward_query_params(&db, org_id)
            .await
            .unwrap_or(false)
    } else {
        resolved_forward
    };

    // Store in KV
    let kv = ctx.kv("URL_MAPPINGS")?;
    let mapping = link.to_mapping(resolved_forward);
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
    let query = url.query().unwrap_or("");

    let page: i64 = query
        .split('&')
        .find(|s| s.starts_with("page="))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1); // Ensure page is at least 1

    let limit: i64 = query
        .split('&')
        .find(|s| s.starts_with("limit="))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
        .min(100); // Cap at 100 items per page to prevent DoS via unbounded queries

    // Parse search parameter
    let search = query
        .split('&')
        .find(|s| s.starts_with("search="))
        .and_then(|s| s.split('=').nth(1))
        .map(|s| urlencoding::decode(s).unwrap_or_default().into_owned())
        .filter(|s| !s.trim().is_empty() && s.len() <= 100);

    // Parse status filter parameter
    let status_filter = query
        .split('&')
        .find(|s| s.starts_with("status="))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| match s {
            "active" | "disabled" => Some(s),
            _ => None,
        });

    // Parse sort parameter
    let sort = query
        .split('&')
        .find(|s| s.starts_with("sort="))
        .and_then(|s| s.split('=').nth(1))
        .map(|s| match s {
            "clicks" | "updated" | "title" | "code" => s,
            _ => "created", // Default: created
        })
        .unwrap_or("created");

    // Parse tags filter parameter: ?tags=foo,bar (OR semantics)
    let tags_filter: Vec<String> = query
        .split('&')
        .find(|s| s.starts_with("tags="))
        .and_then(|s| s.split('=').nth(1))
        .map(|s| {
            // Replace + with space before decoding (urlencoding crate doesn't handle + correctly)
            let s_plus_fixed = s.replace('+', " ");
            urlencoding::decode(&s_plus_fixed)
                .unwrap_or_default()
                .into_owned()
        })
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let tags_filter_opt: Option<&[String]> = if tags_filter.is_empty() {
        None
    } else {
        Some(&tags_filter)
    };

    let offset = (page - 1) * limit;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get total count, links, and stats using filtered queries
    // D1 doesn't support parallel queries, so we run them sequentially
    let total = db::get_links_count_by_org_filtered(
        &db,
        org_id,
        search.as_deref(),
        status_filter,
        tags_filter_opt,
    )
    .await?;

    let mut links = db::get_links_by_org_filtered(
        &db,
        org_id,
        search.as_deref(),
        status_filter,
        sort,
        limit,
        offset,
        tags_filter_opt,
    )
    .await?;
    let stats = db::get_dashboard_stats(&db, org_id).await?;

    // Batch-fetch tags for all returned links
    let link_ids: Vec<String> = links.iter().map(|l| l.id.clone()).collect();
    let tags_map = db::get_tags_for_links(&db, &link_ids).await?;
    for link in &mut links {
        link.tags = tags_map.get(&link.id).cloned().unwrap_or_default();
    }

    // Build pagination metadata
    let pagination = PaginationMeta::new(page, limit, total);

    // Build paginated response with stats
    let stats_json = serde_json::to_value(&stats)
        .map_err(|e| Error::RustError(format!("Failed to serialize stats: {}", e)))?;
    let response = PaginatedResponse::with_stats(links, pagination, stats_json);

    Response::from_json(&response)
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
        Some(mut link) => {
            link.tags = db::get_tags_for_link(&db, &link.id).await?;
            Response::from_json(&link)
        }
        None => Response::error("Link not found", 404),
    }
}

/// Handle getting a link by short_code: GET /api/links/by-code/{code}
pub async fn handle_get_link_by_code(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Authenticate request
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let short_code = ctx
        .param("code")
        .ok_or_else(|| Error::RustError("Missing short code".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let link = db::get_link_by_short_code(&db, short_code, org_id).await?;

    match link {
        Some(link) => Response::from_json(&link),
        None => Response::error("Link not found", 404),
    }
}

/// Handle getting analytics for a link: GET /api/links/{id}/analytics
pub async fn handle_get_link_analytics(req: Request, ctx: RouteContext<()>) -> Result<Response> {
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

    // Verify link exists and belongs to org
    let link = match db::get_link_by_id(&db, link_id, org_id).await? {
        Some(link) => link,
        None => return Response::error("Link not found", 404),
    };

    // Parse time range from query parameters
    // Support both new format (TimeRange enum) and legacy format (start/end timestamps)
    let url = req.url()?;
    let query = url.query().unwrap_or("");

    // Try to parse as new TimeRange format first
    let time_range = if let Ok(time_range_str) = extract_query_param(query, "time_range") {
        // New format: JSON TimeRange object
        serde_json::from_str::<TimeRange>(&time_range_str)
            .map_err(|e| Error::RustError(format!("Invalid time_range parameter: {}", e)))?
    } else if let Ok(days_str) = extract_query_param(query, "days") {
        // Simple days parameter (e.g., ?days=7)
        let days = days_str.parse::<i64>().unwrap_or(7);
        TimeRange::Days { value: days }
    } else {
        // Legacy format: start/end timestamps for backward compatibility
        let now = crate::models::analytics::now_timestamp();

        let start_legacy = query
            .split('&')
            .find(|s| s.starts_with("start="))
            .and_then(|s| s.split('=').nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| now - 7 * 24 * 60 * 60); // Default: 7 days ago

        let end_legacy = query
            .split('&')
            .find(|s| s.starts_with("end="))
            .and_then(|s| s.split('=').nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(now);

        TimeRange::Custom {
            start: start_legacy,
            end: end_legacy,
        }
    };

    // Calculate timestamps using backend logic (eliminates clock skew)
    let (mut start, end) = time_range.calculate_timestamps();

    // Check tier-based analytics limits from billing account
    let org = db::get_org_by_id(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    // Get tier from billing account (all orgs should have billing accounts after migration)
    let tier = if let Some(ref billing_account_id) = org.billing_account_id {
        db::get_billing_account(&db, billing_account_id)
            .await?
            .and_then(|ba| Tier::from_str_value(&ba.tier))
            .unwrap_or(Tier::Free)
    } else {
        Tier::Free
    };

    let mut analytics_gated = false;
    let mut gated_reason = None;

    let limits = tier.limits();

    // Enforce analytics retention window — clamp start date
    if let Some(retention_days) = limits.analytics_retention_days {
        let now = crate::models::analytics::now_timestamp();
        let retention_start = now - retention_days * 24 * 60 * 60;
        if start < retention_start {
            start = retention_start;
            if !analytics_gated {
                analytics_gated = true;
                gated_reason = Some("retention_limited".to_string());
            }
        }
    }

    // If analytics are gated, return empty data with gating info
    if analytics_gated {
        let response = LinkAnalyticsResponse {
            link,
            total_clicks_in_range: 0,
            clicks_over_time: vec![],
            top_referrers: vec![],
            top_countries: vec![],
            top_user_agents: vec![],
            analytics_gated: Some(true),
            gated_reason,
        };
        return Response::from_json(&response);
    }

    // Run analytics queries sequentially (D1 limitation)
    let total_clicks_in_range =
        db::get_link_total_clicks_in_range(&db, link_id, org_id, start, end).await?;
    let clicks_over_time = db::get_link_clicks_over_time(&db, link_id, org_id, start, end).await?;
    let top_referrers = db::get_link_top_referrers(&db, link_id, org_id, start, end, 10).await?;
    let top_countries = db::get_link_top_countries(&db, link_id, org_id, start, end, 10).await?;
    let top_user_agents =
        db::get_link_top_user_agents(&db, link_id, org_id, start, end, 20).await?;

    let response = LinkAnalyticsResponse {
        link,
        total_clicks_in_range,
        clicks_over_time,
        top_referrers,
        top_countries,
        top_user_agents,
        analytics_gated: None,
        gated_reason: None,
    };

    Response::from_json(&response)
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

    // Delete tags first (FK constraint)
    db::delete_tags_for_link(&db, link_id).await?;

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
    if let Some(url) = &update_req.destination_url {
        if let Err(e) = validate_url(url) {
            return Response::error(format!("Invalid URL: {}", e), 400);
        }

        // Check if destination is blacklisted
        let db = ctx.env.get_binding::<D1Database>("rushomon")?;
        if db::is_destination_blacklisted(&db, url).await? {
            return Response::error("Destination URL is blocked", 403);
        }
    }

    // Validate title length if provided (max 200 characters)
    if let Some(ref title) = update_req.title
        && title.len() > 200
    {
        return Response::error("Title must be 200 characters or less", 400);
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
    let _existing_link = match db::get_link_by_id(&db, &link_id, &user_ctx.org_id).await? {
        Some(link) => link,
        None => return Response::error("Link not found", 404),
    };

    // Tier check for Pro-only features in update
    let billing_account_update = db::get_billing_account_for_org(&db, &user_ctx.org_id)
        .await?
        .ok_or_else(|| Error::RustError("No billing account found for organization".to_string()))?;
    let tier_update = Tier::from_str_value(&billing_account_update.tier);
    let is_pro_or_above_update = matches!(
        tier_update.as_ref(),
        Some(Tier::Pro) | Some(Tier::Business) | Some(Tier::Unlimited)
    );
    let wants_pro_features_update = update_req
        .utm_params
        .as_ref()
        .map(|u| !u.is_empty())
        .unwrap_or(false)
        || update_req.forward_query_params.is_some();
    if wants_pro_features_update && !is_pro_or_above_update {
        return Response::error(
            "UTM parameters and query parameter forwarding require a Pro plan or above.",
            403,
        );
    }

    // Normalise UTM: None means "not provided" (no change); Some(empty) means clear it
    let utm_json_for_db: Option<Option<String>> = update_req.utm_params.as_ref().map(|u| {
        if u.is_empty() {
            None
        } else {
            u.to_json_string()
        }
    });

    // Update in D1
    let mut updated_link = db::update_link(
        &db,
        &link_id,
        &user_ctx.org_id,
        update_req.destination_url.as_deref(),
        update_req.title.as_deref(),
        update_req.status.as_ref().map(|s| s.as_str()),
        update_req.expires_at,
        utm_json_for_db.as_ref().map(|o| o.as_deref()),
        update_req.forward_query_params.map(Some),
    )
    .await?;

    // Update tags if provided
    if let Some(tags) = update_req.tags {
        let normalized_tags = match db::validate_and_normalize_tags(&tags) {
            Ok(t) => t,
            Err(e) => return Response::error(e.to_string(), 400),
        };

        // Check tag limit for this tier (enforced at billing account level)
        let tier_limits = tier_update.as_ref().map(|t| t.limits());
        if let Some(ref limits) = tier_limits
            && let Some(max_tags) = limits.max_tags
        {
            // Get current count of distinct tags in the billing account
            let current_tag_count =
                db::count_distinct_tags_for_billing_account(&db, &billing_account_update.id)
                    .await?;

            // Get existing tags for this link to see which ones are being removed
            let existing_link_tags = db::get_tags_for_link(&db, &link_id).await?;
            let existing_tags_set: std::collections::HashSet<String> =
                existing_link_tags.into_iter().collect();
            let new_tags_set: std::collections::HashSet<String> =
                normalized_tags.iter().cloned().collect();

            // Get all existing tags in the BA
            let existing_ba_tags_query = db.prepare(
                "SELECT DISTINCT tag_name
                 FROM link_tags lt
                 JOIN organizations o ON lt.org_id = o.id
                 WHERE o.billing_account_id = ?1",
            );
            let existing_ba_tags_result = existing_ba_tags_query
                .bind(&[billing_account_update.id.clone().into()])?
                .all()
                .await?;
            let existing_ba_tags_set: std::collections::HashSet<String> = existing_ba_tags_result
                .results::<serde_json::Value>()?
                .iter()
                .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
                .collect();

            // Calculate net change in distinct tags
            // Tags being removed from this link that aren't used elsewhere in the BA
            let tags_being_removed: std::collections::HashSet<String> = existing_tags_set
                .difference(&new_tags_set)
                .cloned()
                .collect();

            // Tags being added that don't exist elsewhere in the BA
            let tags_being_added: std::collections::HashSet<String> = new_tags_set
                .difference(&existing_tags_set)
                .cloned()
                .collect();

            // Count how many removed tags are actually disappearing from the BA
            let mut disappearing_count = 0;
            for tag in &tags_being_removed {
                // Check if this tag is used anywhere else in the BA (excluding this link)
                let usage_query = db.prepare(
                    "SELECT COUNT(*) as count
                     FROM link_tags lt
                     JOIN organizations o ON lt.org_id = o.id
                     WHERE o.billing_account_id = ?1 AND lt.tag_name = ?2 AND lt.link_id != ?3",
                );
                let usage_result = usage_query
                    .bind(&[
                        billing_account_update.id.clone().into(),
                        tag.as_str().into(),
                        link_id.as_str().into(),
                    ])?
                    .first::<serde_json::Value>(None)
                    .await?;
                let usage_count = if let Some(result) = usage_result {
                    result["count"].as_f64().unwrap_or(0.0) as i64
                } else {
                    0
                };

                if usage_count == 0 {
                    disappearing_count += 1;
                }
            }

            // Count how many added tags are truly new to the BA
            let new_to_ba_count = tags_being_added
                .iter()
                .filter(|tag| !existing_ba_tags_set.contains(*tag))
                .count() as i64;

            // Net change in tag count
            let net_change = new_to_ba_count - disappearing_count;

            // Check if this would exceed the limit
            if current_tag_count + net_change > max_tags {
                let remaining = max_tags.saturating_sub(current_tag_count);
                let message = if remaining > 0 {
                    format!(
                        "You can create {} more tag{} across all organizations. Upgrade your plan to add more tags.",
                        remaining,
                        if remaining == 1 { "" } else { "s" }
                    )
                } else {
                    "You have reached your tag limit across all organizations. Upgrade your plan to create more tags."
                        .to_string()
                };
                return Response::error(message, 403);
            }
        }

        db::set_tags_for_link(&db, &link_id, &user_ctx.org_id, &normalized_tags).await?;
        updated_link.tags = normalized_tags;
    } else {
        updated_link.tags = db::get_tags_for_link(&db, &link_id).await?;
    }

    // Update KV if any redirect-affecting field changed
    let kv_needs_update = update_req.destination_url.is_some()
        || update_req.status.is_some()
        || update_req.utm_params.is_some()
        || update_req.forward_query_params.is_some();
    if kv_needs_update {
        sync_link_mapping_from_link(&db, &kv, &updated_link).await?;
    }

    Response::from_json(&updated_link)
}

// ─── CSV helpers ─────────────────────────────────────────────────────────────

/// Escape a single CSV field: wraps in double-quotes if the value contains
/// a comma, double-quote, or newline; doubles any embedded double-quotes.
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

// ─── Import / Export structs ─────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct ImportLinkRow {
    destination_url: String,
    short_code: Option<String>,
    title: Option<String>,
    tags: Option<Vec<String>>,
    expires_at: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct ImportRequest {
    links: Vec<ImportLinkRow>,
}

#[derive(Debug, serde::Serialize)]
struct ImportError {
    row: usize,
    destination_url: String,
    reason: String,
}

#[derive(Debug, serde::Serialize)]
struct ImportWarning {
    row: usize,
    destination_url: String,
    reason: String,
}

#[derive(Debug, serde::Serialize)]
struct ImportResponse {
    created: usize,
    skipped: usize,
    failed: usize,
    errors: Vec<ImportError>,
    warnings: Vec<ImportWarning>,
}

// ─── Export handler ───────────────────────────────────────────────────────────

/// Handle CSV export of all org links: GET /api/links/export
pub async fn handle_export_links(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let mut links = db::get_all_links_for_org_export(&db, org_id).await?;

    let link_ids: Vec<String> = links.iter().map(|l| l.id.clone()).collect();
    let tags_map = db::get_tags_for_links(&db, &link_ids).await?;
    for link in &mut links {
        link.tags = tags_map.get(&link.id).cloned().unwrap_or_default();
    }

    let mut csv = String::from(
        "short_code,destination_url,title,tags,status,click_count,created_at,expires_at,utm_source,utm_medium,utm_campaign,utm_term,utm_content,utm_ref,forward_query_params\n",
    );

    for link in &links {
        let title = link.title.as_deref().unwrap_or("");
        let tags_str = link.tags.join("|");
        let created_at = chrono::DateTime::from_timestamp(link.created_at, 0)
            .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();
        let expires_at = link
            .expires_at
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();
        let utm_source = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_source.as_deref())
            .unwrap_or("");
        let utm_medium = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_medium.as_deref())
            .unwrap_or("");
        let utm_campaign = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_campaign.as_deref())
            .unwrap_or("");
        let utm_term = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_term.as_deref())
            .unwrap_or("");
        let utm_content = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_content.as_deref())
            .unwrap_or("");
        let utm_ref = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_ref.as_deref())
            .unwrap_or("");
        let forward_query = link
            .forward_query_params
            .map(|v| if v { "true" } else { "false" })
            .unwrap_or("");

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&link.short_code),
            csv_escape(&link.destination_url),
            csv_escape(title),
            csv_escape(&tags_str),
            csv_escape(link.status.as_str()),
            link.click_count,
            created_at,
            expires_at,
            csv_escape(utm_source),
            csv_escape(utm_medium),
            csv_escape(utm_campaign),
            csv_escape(utm_term),
            csv_escape(utm_content),
            csv_escape(utm_ref),
            forward_query,
        ));
    }

    let date_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let filename = format!("rushomon-links-{}.csv", date_str);
    let mut response = Response::ok(csv)?;
    response
        .headers_mut()
        .set("Content-Type", "text/csv; charset=utf-8")?;
    response.headers_mut().set(
        "Content-Disposition",
        &format!("attachment; filename=\"{}\"", filename),
    )?;
    Ok(response)
}

// ─── Import handler ───────────────────────────────────────────────────────────

/// Handle batch import of links from CSV (normalised JSON): POST /api/links/import
pub async fn handle_import_links(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let user_id = &user_ctx.user_id;
    let org_id = &user_ctx.org_id;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let billing_account = db::get_billing_account_for_org(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("No billing account found for organization".to_string()))?;
    let tier = Tier::from_str_value(&billing_account.tier);
    let limits = tier.as_ref().map(|t| t.limits());
    let is_pro_or_above = matches!(
        tier.as_ref(),
        Some(Tier::Pro) | Some(Tier::Business) | Some(Tier::Unlimited)
    );

    let body: ImportRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid JSON body", 400),
    };

    if body.links.is_empty() {
        return Response::from_json(&ImportResponse {
            created: 0,
            skipped: 0,
            failed: 0,
            errors: vec![],
            warnings: vec![],
        });
    }

    if body.links.len() > 50 {
        return Response::error("Maximum 50 links per import batch", 400);
    }

    let kv = ctx.kv("URL_MAPPINGS")?;
    let now = now_timestamp();
    let year_month = {
        let dt = chrono::Utc::now();
        format!("{}-{:02}", dt.year(), dt.month())
    };

    let mut created: usize = 0;
    let mut skipped: usize = 0;
    let mut failed: usize = 0;
    let mut errors: Vec<ImportError> = Vec::new();
    let mut warnings: Vec<ImportWarning> = Vec::new();

    for (idx, row) in body.links.iter().enumerate() {
        let row_num = idx + 1;

        // Validate destination URL
        let destination_url = match validate_url(&row.destination_url) {
            Ok(url) => url,
            Err(e) => {
                failed += 1;
                errors.push(ImportError {
                    row: row_num,
                    destination_url: row.destination_url.clone(),
                    reason: format!("Invalid URL: {}", e),
                });
                continue;
            }
        };

        // Check blacklist
        if db::is_destination_blacklisted(&db, &destination_url).await? {
            failed += 1;
            errors.push(ImportError {
                row: row_num,
                destination_url: destination_url.clone(),
                reason: "Destination URL is blocked".to_string(),
            });
            continue;
        }

        // Monthly quota check (increment counter; rolls back nothing on partial failure)
        if let Some(ref tier_limits) = limits
            && let Some(max_links) = tier_limits.max_links_per_month
        {
            let can_create = db::increment_monthly_counter_for_billing_account(
                &db,
                &billing_account.id,
                &year_month,
                max_links,
            )
            .await?;
            if !can_create {
                failed += 1;
                errors.push(ImportError {
                    row: row_num,
                    destination_url: destination_url.clone(),
                    reason: "Monthly link limit reached".to_string(),
                });
                continue;
            }
        }

        // Determine short code
        let short_code: String;
        if is_pro_or_above && let Some(provided_code) = row.short_code.as_ref() {
            if let Err(e) = validate_short_code(provided_code) {
                skipped += 1;
                errors.push(ImportError {
                    row: row_num,
                    destination_url: destination_url.clone(),
                    reason: format!("Invalid short code: {}", e),
                });
                continue;
            }

            // Try provided code, then auto-suffix on conflict (promo → promo-1 → … → promo-10)
            let mut resolved: Option<String> = None;
            for attempt in 0u32..=10 {
                let candidate = if attempt == 0 {
                    provided_code.clone()
                } else {
                    format!("{}-{}", provided_code, attempt)
                };
                if !kv::links::short_code_exists(&kv, &candidate).await? {
                    resolved = Some(candidate);
                    break;
                }
            }

            match resolved {
                Some(c) => short_code = c,
                None => {
                    // All suffix attempts exhausted — fall back to a random code
                    let mut fallback: Option<String> = None;
                    for _ in 0..10u32 {
                        let candidate = generate_short_code();
                        if !kv::links::short_code_exists(&kv, &candidate).await? {
                            fallback = Some(candidate);
                            break;
                        }
                    }
                    match fallback {
                        Some(c) => {
                            warnings.push(ImportWarning {
                                row: row_num,
                                destination_url: destination_url.clone(),
                                reason: format!(
                                    "Short code '{}' conflicted with an existing link; a random code was assigned",
                                    provided_code
                                ),
                            });
                            short_code = c;
                        }
                        None => {
                            failed += 1;
                            errors.push(ImportError {
                                row: row_num,
                                destination_url: destination_url.clone(),
                                reason: "Failed to generate a unique short code after conflict"
                                    .to_string(),
                            });
                            continue;
                        }
                    }
                }
            }
        } else {
            // Free tier OR Pro without provided code: auto-generate
            let mut resolved: Option<String> = None;
            for _ in 0..10u32 {
                let candidate = generate_short_code();
                if !kv::links::short_code_exists(&kv, &candidate).await? {
                    resolved = Some(candidate);
                    break;
                }
            }
            match resolved {
                Some(c) => short_code = c,
                None => {
                    failed += 1;
                    errors.push(ImportError {
                        row: row_num,
                        destination_url: destination_url.clone(),
                        reason: "Failed to generate unique short code".to_string(),
                    });
                    continue;
                }
            }
        }

        // Validate and normalize tags (silently drop bad tags)
        let mut normalized_tags = if let Some(ref tags) = row.tags {
            db::validate_and_normalize_tags(tags).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Check tag limit for this tier (enforced at billing account level)
        if let Some(ref tier_limits) = limits
            && let Some(max_tags) = tier_limits.max_tags
        {
            // Get current count of distinct tags in the billing account
            let current_tag_count =
                db::count_distinct_tags_for_billing_account(&db, &billing_account.id).await?;

            // Find which tags are new to the BA (not already in use)
            let mut new_tag_count = 0;
            if !normalized_tags.is_empty() {
                // Get all existing tags in the BA
                let existing_tags_query = db.prepare(
                    "SELECT DISTINCT tag_name
                     FROM link_tags lt
                     JOIN organizations o ON lt.org_id = o.id
                     WHERE o.billing_account_id = ?1",
                );
                let existing_tags_result = existing_tags_query
                    .bind(&[billing_account.id.clone().into()])?
                    .all()
                    .await?;
                let existing_tags_set: std::collections::HashSet<String> = existing_tags_result
                    .results::<serde_json::Value>()?
                    .iter()
                    .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
                    .collect();

                // Count how many of the submitted tags are new
                new_tag_count = normalized_tags
                    .iter()
                    .filter(|tag| !existing_tags_set.contains(*tag))
                    .count() as i64;
            }

            // Check if adding these new tags would exceed the limit
            if current_tag_count + new_tag_count > max_tags {
                // For imports, we'll skip the row but allow the import to continue
                skipped += 1;
                warnings.push(ImportWarning {
                    row: row_num,
                    destination_url: destination_url.clone(),
                    reason: format!(
                        "Tags skipped: would exceed tag limit ({} max). Consider upgrading your plan.",
                        max_tags
                    ),
                });
                // Clear tags to avoid exceeding limit
                normalized_tags.clear();
            }
        }

        // Clamp title to 200 chars
        let title = row.title.as_ref().and_then(|t| {
            let trimmed = t.trim().to_string();
            if trimmed.is_empty() || trimmed.len() > 200 {
                None
            } else {
                Some(trimmed)
            }
        });

        let link_id = uuid::Uuid::new_v4().to_string();
        let link = Link {
            id: link_id.clone(),
            org_id: org_id.to_string(),
            short_code: short_code.clone(),
            destination_url: destination_url.clone(),
            title,
            created_by: user_id.to_string(),
            created_at: now,
            updated_at: None,
            expires_at: row.expires_at,
            status: crate::models::link::LinkStatus::Active,
            click_count: 0,
            tags: normalized_tags.clone(),
            utm_params: None,
            forward_query_params: None,
        };

        db::create_link(&db, &link).await?;

        if !normalized_tags.is_empty() {
            db::set_tags_for_link(&db, &link_id, org_id, &normalized_tags).await?;
        }

        let mapping = link.to_mapping(false);
        kv::store_link_mapping(&kv, org_id, &short_code, &mapping).await?;

        created += 1;
    }

    Response::from_json(&ImportResponse {
        created,
        skipped,
        failed,
        errors,
        warnings,
    })
}

/// Returns the list of enabled OAuth providers: GET /api/auth/providers
pub async fn handle_list_auth_providers(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use serde_json::json;

    let env = &ctx.env;
    let mut providers = Vec::new();

    if auth::providers::GITHUB.is_enabled(env) {
        providers.push(json!({ "name": "github", "label": "GitHub" }));
    }
    if auth::providers::GOOGLE.is_enabled(env) {
        providers.push(json!({ "name": "google", "label": "Google" }));
    }

    let origin = req.headers().get("Origin").ok().flatten();
    let response = Response::from_json(&json!({ "providers": providers }))?;
    Ok(crate::add_cors_headers(response, origin, env))
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

/// Handle GitHub OAuth initiation: GET /api/auth/github
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

/// Handle Google OAuth initiation: GET /api/auth/google
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

/// Handle OAuth callback: GET /api/auth/callback
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

/// Handle get current user: GET /api/auth/me
pub async fn handle_get_current_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    // Rate limiting: 100 requests per minute per session
    let kv = ctx.kv("URL_MAPPINGS")?;
    let rate_limit_key = RateLimiter::session_key("auth_check", &user_ctx.session_id);
    let rate_limit_config = RateLimitConfig::auth_check();

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
    let user = db::get_user_by_id(&db, &user_ctx.user_id).await?;

    match user {
        Some(user) => Response::from_json(&user),
        None => Response::error("User not found", 404),
    }
}

/// Handle getting usage info: GET /api/usage
/// Returns tier, limits, and current usage for the authenticated org
pub async fn handle_get_usage(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let org = db::get_org_by_id(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    // Get billing account for usage tracking
    let billing_account_id = org
        .billing_account_id
        .as_ref()
        .ok_or_else(|| Error::RustError("Organization has no billing account".to_string()))?;
    let billing_account = db::get_billing_account(&db, billing_account_id)
        .await?
        .ok_or_else(|| Error::RustError("Billing account not found".to_string()))?;

    let tier = Tier::from_str_value(&billing_account.tier).unwrap_or(Tier::Free);
    let limits = tier.limits();

    // Use billing account monthly counter for efficiency
    let now = chrono::Utc::now();
    let year_month = format!("{}-{:02}", now.year(), now.month());
    let links_created_this_month =
        db::get_monthly_counter_for_billing_account(&db, &billing_account.id, &year_month).await?;

    // Get tag count for the billing account
    let tags_count = db::count_distinct_tags_for_billing_account(&db, &billing_account.id).await?;

    // Calculate next reset time (first day of next month at midnight UTC)
    let now = chrono::Utc::now();
    let next_reset = chrono::Utc
        .with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
        .single()
        .unwrap_or_else(chrono::Utc::now);
    let next_reset_timestamp = next_reset.timestamp();

    let usage = serde_json::json!({
        "tier": tier.as_str(),
        "limits": {
            "max_links_per_month": limits.max_links_per_month,
            "analytics_retention_days": limits.analytics_retention_days,
            "allow_custom_short_code": limits.allow_custom_short_code,
            "allow_utm_parameters": limits.allow_utm_parameters,
            "allow_query_forwarding": limits.allow_query_forwarding,
            "max_tags": limits.max_tags,
        },
        "usage": {
            "links_created_this_month": links_created_this_month,
            "tags_count": tags_count,
        },
        "next_reset": {
            "utc": next_reset.to_rfc3339(),
            "timestamp": next_reset_timestamp,
        }
    });

    Response::from_json(&usage)
}

/// Handle getting org tags: GET /api/tags
/// Returns all tags for the authenticated org with usage counts
pub async fn handle_get_org_tags(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tags = db::get_org_tags(&db, org_id).await?;

    Response::from_json(&tags)
}

/// Handle token refresh: POST /api/auth/refresh
/// Validates refresh token from cookie and returns new access token
pub async fn handle_token_refresh(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("URL_MAPPINGS")?;

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

    // Rate limiting: 30 requests per hour per session
    let rate_limit_key = RateLimiter::session_key("token_refresh", &claims.session_id);
    let rate_limit_config = RateLimitConfig::token_refresh();

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
        Ok(None) => {
            return Response::error("Session expired or invalid", 401);
        }
        Err(_) => {
            return Response::error("Failed to validate session", 500);
        }
    };

    // Verify user_id matches (constant-time comparison to prevent timing attacks)
    use subtle::ConstantTimeEq;
    let session_user_id_bytes = session.user_id.as_bytes();
    let claims_user_id_bytes = claims.sub.as_bytes();

    // Pad to same length to prevent length-based timing leaks
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
    let user = match db::get_user_by_id(&db, &claims.sub).await? {
        Some(user) => user,
        None => return Response::error("User not found", 404),
    };

    // Generate new access token (1 hour) with fresh role from database
    let new_access_token = auth::session::create_access_token(
        &claims.sub,
        &claims.org_id,
        &claims.session_id,
        &user.role, // Use fresh role from database
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

    // Set new access token as httpOnly cookie
    let access_cookie = auth::session::create_access_cookie_with_scheme(&new_access_token, scheme);

    let mut response = Response::ok("Token refreshed successfully")?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;

    Ok(response)
}

/// Handle logout: POST /api/auth/logout
pub async fn handle_logout(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let kv = ctx.kv("URL_MAPPINGS")?;

    auth::session::delete_session(&kv, &user_ctx.session_id).await?;

    // Clear all three cookies: access token, refresh token, and legacy session
    let access_cookie = auth::session::create_access_logout_cookie();
    let refresh_cookie = auth::session::create_refresh_logout_cookie();
    let session_cookie = auth::session::create_logout_cookie();

    let mut response = Response::ok("Logged out successfully")?;

    // Set all three logout cookies
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    response
        .headers_mut()
        .append("Set-Cookie", &refresh_cookie)?;
    response
        .headers_mut()
        .append("Set-Cookie", &session_cookie)?;

    Ok(response)
}

/// Handle listing all users: GET /api/admin/users (admin only)
pub async fn handle_admin_list_users(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

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
        .unwrap_or(50)
        .min(100); // Cap at 100 items per page to prevent DoS via unbounded queries

    let offset = (page - 1) * limit;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let users = db::get_all_users_with_billing_info(&db, limit, offset).await?;
    let total = db::get_user_count(&db).await?;

    Response::from_json(&serde_json::json!({
        "users": users,
        "total": total,
        "page": page,
        "limit": limit,
    }))
}

/// Handle getting a single user: GET /api/admin/users/:id (admin only)
pub async fn handle_admin_get_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let user_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing user ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let user = db::get_user_by_id(&db, user_id).await?;

    match user {
        Some(user) => Response::from_json(&user),
        None => Response::error("User not found", 404),
    }
}

/// Handle listing Polar discounts: GET /api/admin/discounts (admin only)
pub async fn handle_admin_list_discounts(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let polar = match crate::billing::polar::polar_client_from_env(&ctx.env) {
        Ok(p) => p,
        Err(_) => return Response::error("Billing not configured", 503),
    };

    match polar.list_discounts().await {
        Ok(discounts) => Response::from_json(&discounts),
        Err(e) => {
            worker::console_error!("[admin/discounts] Polar API error: {}", e);
            Response::error("Failed to fetch discounts from Polar", 502)
        }
    }
}

/// Handle listing Polar products: GET /api/admin/products (admin only)
pub async fn handle_admin_list_products(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let polar = match crate::billing::polar::polar_client_from_env(&ctx.env) {
        Ok(p) => p,
        Err(_) => return Response::error("Billing not configured", 503),
    };
    let products = polar.list_products().await?;

    Response::from_json(&products)
}

/// Handle getting pricing information: GET /api/billing/pricing (public)
pub async fn handle_billing_pricing(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Fetch pricing from local database based on configured product IDs
    let db = &ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

    // Get configured product IDs from settings
    let settings_map = match crate::db::queries::get_all_settings(db).await {
        Ok(s) => s,
        Err(e) => {
            worker::console_error!("Failed to fetch settings for pricing: {}", e);
            return Response::error("Failed to fetch pricing configuration", 500);
        }
    };

    // Helper to get setting value
    let get_setting = |key: &str| -> String { settings_map.get(key).cloned().unwrap_or_default() };

    let mut products = Vec::new();

    // Helper function to fetch product from cached_products table
    async fn get_cached_product(
        db: &worker::d1::D1Database,
        product_id: &str,
    ) -> Option<serde_json::Value> {
        if product_id.is_empty() {
            return None;
        }

        // Query the cached_products table
        let stmt = db.prepare(
            "SELECT id, name, description, price_amount, price_currency,
                    recurring_interval, recurring_interval_count, is_archived,
                    polar_product_id, polar_price_id
             FROM cached_products
             WHERE polar_product_id = ?1 AND is_archived = FALSE",
        );

        if let Ok(Some(result)) = stmt
            .bind(&[product_id.into()])
            .unwrap()
            .first::<serde_json::Value>(None)
            .await
        {
            Some(serde_json::json!({
                "id": result.get("polar_product_id").unwrap_or(&serde_json::Value::String(product_id.to_string())),
                "polar_product_id": result.get("polar_product_id").unwrap_or(&serde_json::Value::String(product_id.to_string())),
                "polar_price_id": result.get("polar_price_id").unwrap_or(&serde_json::Value::String(product_id.to_string())),
                "name": result.get("name").unwrap_or(&serde_json::Value::String("Unknown".to_string())),
                "price_amount": result.get("price_amount").unwrap_or(&serde_json::Value::Number(serde_json::Number::from(0))),
                "price_currency": result.get("price_currency").unwrap_or(&serde_json::Value::String("EUR".to_string())),
                "recurring_interval": result.get("recurring_interval"),
                "recurring_interval_count": result.get("recurring_interval_count")
            }))
        } else {
            None
        }
    }

    // Add Pro Monthly
    if let Some(mut entry) = get_cached_product(db, &get_setting("product_pro_monthly_id")).await {
        if let Some(obj) = entry.as_object_mut() {
            obj.insert(
                "id".to_string(),
                serde_json::Value::String("product_pro_monthly_id".to_string()),
            );
        }
        products.push(entry);
    }

    // Add Pro Annual
    if let Some(mut entry) = get_cached_product(db, &get_setting("product_pro_annual_id")).await {
        if let Some(obj) = entry.as_object_mut() {
            obj.insert(
                "id".to_string(),
                serde_json::Value::String("product_pro_annual_id".to_string()),
            );
        }
        products.push(entry);
    }

    // Add Business Monthly
    if let Some(mut entry) =
        get_cached_product(db, &get_setting("product_business_monthly_id")).await
    {
        if let Some(obj) = entry.as_object_mut() {
            obj.insert(
                "id".to_string(),
                serde_json::Value::String("product_business_monthly_id".to_string()),
            );
        }
        products.push(entry);
    }

    // Add Business Annual
    if let Some(mut entry) =
        get_cached_product(db, &get_setting("product_business_annual_id")).await
    {
        if let Some(obj) = entry.as_object_mut() {
            obj.insert(
                "id".to_string(),
                serde_json::Value::String("product_business_annual_id".to_string()),
            );
        }
        products.push(entry);
    }

    Response::from_json(&serde_json::json!({
        "products": products
    }))
}

/// Handle syncing products from Polar: POST /api/admin/products/sync (admin only)
pub async fn handle_admin_sync_products(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let polar = match crate::billing::polar::polar_client_from_env(&ctx.env) {
        Ok(p) => p,
        Err(_) => return Response::error("Billing not configured", 503),
    };

    match polar.list_products().await {
        Ok(products) => {
            let response = serde_json::json!({
                "success": true,
                "message": "Products fetched successfully",
                "products_count": products["items"].as_array().map(|arr| arr.len()).unwrap_or(0)
            });

            Response::from_json(&response)
        }
        Err(_) => Response::error("Failed to fetch products from Polar", 502),
    }
}

/// Handle saving complete product configuration: POST /api/admin/products/save (admin only)
pub async fn handle_admin_save_products(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let polar = match crate::billing::polar::polar_client_from_env(&ctx.env) {
        Ok(p) => p,
        Err(_) => return Response::error("Billing not configured", 503),
    };

    // Fetch products from Polar
    let products = match polar.list_products().await {
        Ok(p) => p,
        Err(e) => {
            worker::console_error!("Failed to fetch products from Polar: {}", e);
            return Response::error("Failed to fetch products from Polar", 502);
        }
    };

    // Cache products in database
    let db = &ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

    if let Err(e) = cache_products_from_polar(db, &products).await {
        worker::console_error!("Failed to cache products: {}", e);
        return Response::error("Failed to cache products", 500);
    }

    let response = serde_json::json!({
        "success": true,
        "message": "Products configuration saved and cached successfully",
        "products_count": products["items"].as_array().map(|arr| arr.len()).unwrap_or(0)
    });

    Response::from_json(&response)
}

/// Cache products from Polar API into local database
async fn cache_products_from_polar(
    db: &worker::d1::D1Database,
    products: &serde_json::Value,
) -> Result<()> {
    if let Some(items) = products.get("items").and_then(|i| i.as_array()) {
        // Clear existing cached products
        let delete_stmt = db.prepare("DELETE FROM cached_products");
        delete_stmt.run().await?;

        // Insert each product with its prices
        for product in items {
            let product_id = product.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let product_name = product.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let product_description = product
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            // Extract prices array from the product
            if let Some(prices) = product.get("prices").and_then(|p| p.as_array()) {
                for price in prices {
                    let price_amount: i32 = price
                        .get("price_amount")
                        .and_then(|p| p.as_i64())
                        .unwrap_or(0) as i32;
                    let price_currency: String = price
                        .get("price_currency")
                        .and_then(|c| c.as_str())
                        .unwrap_or("EUR")
                        .to_string();
                    let price_id: String = price
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Handle nullable values properly
                    let recurring_interval_val =
                        price.get("recurring_interval").and_then(|i| i.as_str());
                    let recurring_interval_count_val = price
                        .get("recurring_interval_count")
                        .and_then(|c| c.as_i64());

                    // Insert into cached_products table
                    let insert_stmt = db.prepare(
                        "INSERT INTO cached_products (
                            id, name, description, price_amount, price_currency,
                            recurring_interval, recurring_interval_count, is_archived,
                            polar_product_id, polar_price_id, created_at, updated_at
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    );

                    let now = worker::Date::now().to_string();
                    insert_stmt
                        .bind(&[
                            price_id.clone().into(),
                            product_name.into(),
                            product_description.into(),
                            (price_amount as f64).into(),
                            price_currency.into(),
                            recurring_interval_val.unwrap_or("").into(),
                            (recurring_interval_count_val.unwrap_or(1) as f64).into(), // default to 1
                            false.into(), // is_archived as boolean
                            product_id.into(),
                            price_id.into(),
                            now.clone().into(), // created_at
                            now.into(),         // updated_at
                        ])?
                        .run()
                        .await?;
                }
            }
        }
    }

    Ok(())
}

/// Handle getting all settings: GET /api/admin/settings (admin only)
pub async fn handle_admin_get_settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let settings = db::get_all_settings(&db).await?;

    let settings_map: serde_json::Map<String, serde_json::Value> = settings
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();

    Response::from_json(&serde_json::Value::Object(settings_map))
}

/// Handle updating a setting: PUT /api/admin/settings (admin only)
pub async fn handle_admin_update_setting(
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

    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let key = match body.get("key").and_then(|k| k.as_str()) {
        Some(k) => k.to_string(),
        None => return Response::error("Missing 'key' field", 400),
    };

    let value = match body.get("value").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return Response::error("Missing 'value' field", 400),
    };

    // Validate known settings
    match key.as_str() {
        "signups_enabled" => {
            if value != "true" && value != "false" {
                return Response::error(
                    "Invalid value for 'signups_enabled'. Must be 'true' or 'false'",
                    400,
                );
            }
        }
        "default_user_tier" => {
            if Tier::from_str_value(&value).is_none() {
                return Response::error(
                    "Invalid value for 'default_user_tier'. Must be 'free' or 'unlimited'",
                    400,
                );
            }
        }
        "founder_pricing_active" => {
            if value != "true" && value != "false" {
                return Response::error(
                    "Invalid value for 'founder_pricing_active'. Must be 'true' or 'false'",
                    400,
                );
            }
        }
        "active_discount_pro_monthly"
        | "active_discount_pro_annual"
        | "active_discount_business_monthly"
        | "active_discount_business_annual"
        | "active_discount_amount_pro_monthly"
        | "active_discount_amount_pro_annual"
        | "active_discount_amount_business_monthly"
        | "active_discount_amount_business_annual"
        | "product_pro_monthly_id"
        | "product_pro_annual_id"
        | "product_business_monthly_id"
        | "product_business_annual_id" => {
            // Any string is valid (discount UUID / product UUID / amount in cents, or empty string to clear)
        }
        _ => return Response::error(format!("Unknown setting: {}", key), 400),
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    db::set_setting(&db, &key, &value).await?;

    // Return updated settings
    let settings = db::get_all_settings(&db).await?;
    let settings_map: serde_json::Map<String, serde_json::Value> = settings
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();

    Response::from_json(&serde_json::Value::Object(settings_map))
}

/// Handle resetting monthly counter: POST /api/admin/orgs/:id/reset-counter (admin only)
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

/// Handle listing all links for admin moderation: GET /api/admin/links (admin only)
pub async fn handle_admin_list_links(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    // Parse pagination and filter params
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
        .unwrap_or(50)
        .min(100);

    let offset = (page - 1) * limit;

    // Parse filters
    let org_filter = url.query().and_then(|q| {
        q.split('&')
            .find(|s| s.starts_with("org="))
            .and_then(|s| s.split('=').nth(1))
    });

    let email_filter = url.query().and_then(|q| {
        q.split('&')
            .find(|s| s.starts_with("email="))
            .and_then(|s| s.split('=').nth(1))
    });

    let domain_filter = url.query().and_then(|q| {
        q.split('&')
            .find(|s| s.starts_with("domain="))
            .and_then(|s| s.split('=').nth(1))
    });

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv = ctx.kv("URL_MAPPINGS")?;
    let links = db::get_all_links_admin(
        &db,
        &kv,
        limit,
        offset,
        org_filter,
        email_filter,
        domain_filter,
    )
    .await?;
    let total = db::get_all_links_admin_count(&db, org_filter, email_filter, domain_filter).await?;

    Response::from_json(&serde_json::json!({
        "links": links,
        "total": total,
        "page": page,
        "limit": limit,
    }))
}

/// Handle updating a link's status: PUT /api/admin/links/:id (admin only)
pub async fn handle_admin_update_link_status(
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

    let link_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing link ID".to_string()))?
        .to_string();

    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let status = match body.get("status").and_then(|s| s.as_str()) {
        Some(s) if s == "active" || s == "disabled" || s == "blocked" => s.to_string(),
        Some(_) => {
            return Response::error(
                "Invalid status. Must be 'active', 'disabled', or 'blocked'",
                400,
            );
        }
        None => return Response::error("Missing 'status' field", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Update link status using the same function as regular endpoint
    db::update_link_status_by_id(&db, &link_id, &status).await?;

    // Sync KV using the shared function (now that get_link_by_id_no_auth_all is fixed)
    let kv = ctx.kv("URL_MAPPINGS")?;
    match db::get_link_by_id_no_auth_all(&db, &link_id).await? {
        Some(updated_link) => {
            sync_link_mapping_from_link(&db, &kv, &updated_link).await?;
        }
        None => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "admin_link_sync_not_found",
                    "link_id": link_id,
                    "level": "critical"
                })
            );
        }
    }

    // Auto-resolve all pending reports for this link if status is being changed to disabled/blocked
    if (status == "disabled" || status == "blocked")
        && let Err(e) = db::resolve_reports_for_link(
            &db,
            &link_id,
            "reviewed",
            &format!("Action taken: Link {}", status),
            &user_ctx.user_id,
        )
        .await
    {
        console_log!(
            "{}",
            serde_json::json!({
                "event": "reports_resolve_failed",
                "link_id": link_id,
                "error": e.to_string(),
                "level": "error"
            })
        );
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": format!("Link status updated to {}", status)
    }))
}

/// Handle deleting a link: DELETE /api/admin/links/:id (admin only)
pub async fn handle_admin_delete_link(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let link_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing link ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get link first to get short_code for KV deletion
    let link = match db::get_link_by_id_no_auth_all(&db, &link_id).await? {
        Some(link) => link,
        None => return Response::error("Link not found", 404),
    };

    // Hard delete from D1
    db::hard_delete_link(&db, &link_id, &link.org_id).await?;

    // Delete from KV
    let kv = ctx.kv("URL_MAPPINGS")?;
    kv::delete_link_mapping(&kv, &link.org_id, &link.short_code).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Link deleted successfully"
    }))
}

/// Handle re-syncing a link's KV entry: POST /api/admin/links/:id/sync-kv (admin only)
pub async fn handle_admin_sync_link_kv(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let link_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing link ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Get link from D1
    let link = match db::get_link_by_id_no_auth_all(&db, &link_id).await? {
        Some(link) => link,
        None => return Response::error("Link not found", 404),
    };

    // Re-sync based on D1 status (treat D1 as source of truth)
    match link.status.as_str() {
        "active" => {
            // Link should be active in KV - recreate/update the KV entry
            let link_model = crate::models::Link {
                id: link.id.clone(),
                org_id: link.org_id.clone(),
                short_code: link.short_code.clone(),
                destination_url: link.destination_url.clone(),
                title: link.title.clone(),
                created_by: link.created_by.clone(),
                created_at: link.created_at,
                updated_at: link.updated_at,
                expires_at: link.expires_at,
                status: crate::models::link::LinkStatus::Active,
                click_count: link.click_count,
                tags: link.tags, // Handle missing tags field
                utm_params: link.utm_params,
                forward_query_params: link.forward_query_params,
            };

            // Resolve forward query params (same logic as in other places)
            let resolved_forward = if let Some(forward) = link.forward_query_params {
                forward
            } else {
                db::get_org_forward_query_params(&db, &link.org_id)
                    .await
                    .unwrap_or(false)
            };

            let mapping = link_model.to_mapping(resolved_forward);
            kv::store_link_mapping(&kv, &link.org_id, &link.short_code, &mapping).await?;
        }
        "blocked" | "disabled" => {
            // Link should NOT be active in KV - delete the KV entry
            kv::delete_link_mapping(&kv, &link.org_id, &link.short_code).await?;
        }
        _ => {
            return Response::error("Cannot sync link with unknown status", 400);
        }
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Link KV entry re-synced successfully"
    }))
}

/// Handle blocking a destination: POST /api/admin/blacklist (admin only)
pub async fn handle_admin_block_destination(
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

    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let destination = match body.get("destination").and_then(|d| d.as_str()) {
        Some(d) => d.to_string(),
        None => return Response::error("Missing 'destination' field", 400),
    };

    let match_type = match body.get("match_type").and_then(|m| m.as_str()) {
        Some(m) if m == "exact" || m == "domain" => m.to_string(),
        Some(_) => return Response::error("Invalid match_type. Must be 'exact' or 'domain'", 400),
        None => "exact".to_string(), // Default to exact
    };

    // Normalize the destination URL for exact matches to ensure consistency
    let normalized_destination = if match_type == "exact" {
        match crate::utils::normalize_url_for_blacklist(&destination) {
            Ok(url) => url,
            Err(e) => {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "url_normalize_failed",
                        "url": destination,
                        "error": e.to_string(),
                        "level": "warn"
                    })
                );
                // Fall back to original URL if normalization fails
                destination.clone()
            }
        }
    } else {
        // For domain matches, extract just the domain/hostname
        match url::Url::parse(&destination) {
            Ok(url) => url.host_str().unwrap_or(&destination).to_string(),
            Err(_) => {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "domain_extract_failed",
                        "url": destination,
                        "level": "warn"
                    })
                );
                // If URL parsing fails, try simple extraction
                let without_protocol = destination
                    .trim_start_matches("http://")
                    .trim_start_matches("https://");
                let without_path = without_protocol
                    .split('/')
                    .next()
                    .unwrap_or(without_protocol);
                without_path.to_string()
            }
        }
    };

    let reason = match body.get("reason").and_then(|r| r.as_str()) {
        Some(r) => r.to_string(),
        None => return Response::error("Missing 'reason' field", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Check if destination is already blacklisted
    if db::is_destination_already_blacklisted(&db, &normalized_destination, &match_type).await? {
        return Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Destination is already blocked",
            "already_blocked": true
        }));
    }

    // Add to blacklist
    db::add_to_blacklist(
        &db,
        &normalized_destination,
        &match_type,
        &reason,
        &user_ctx.user_id,
    )
    .await?;

    let candidate_links = db::get_links_for_blacklist_scan(&db).await?;

    let mut blocked_links = Vec::new();
    for link in candidate_links {
        if db::is_destination_blacklisted(&db, &link.destination_url).await? {
            blocked_links.push(link);
        }
    }

    let blocked_count = blocked_links.len() as i64;

    // Auto-resolve all pending reports for the blocked links
    if blocked_count > 0 {
        let kv = ctx.kv("URL_MAPPINGS")?;
        for link in blocked_links {
            db::update_link_status_by_id(&db, &link.id, "blocked").await?;
            if let Err(e) = db::resolve_reports_for_link(
                &db,
                &link.id,
                "reviewed",
                &format!("Action taken: Blocked {} ({})", match_type, destination),
                &user_ctx.user_id,
            )
            .await
            {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "reports_resolve_failed",
                        "link_id": link.id,
                        "error": e.to_string(),
                        "level": "error"
                    })
                );
            }

            match kv::delete_link_mapping(&kv, &link.org_id, &link.short_code).await {
                Ok(()) => {}
                Err(e) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "admin_block_destination_kv_delete_failed",
                            "short_code": link.short_code,
                            "link_id": link.id,
                            "error": e.to_string(),
                            "level": "error"
                        })
                    );
                }
            }
        }
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Destination blocked successfully",
        "blocked_links": blocked_count
    }))
}

/// Handle getting blacklist entries: GET /api/admin/blacklist (admin only)
pub async fn handle_admin_get_blacklist(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let entries = db::get_all_blacklist(&db).await?;

    Response::from_json(&entries)
}

/// Handle removing blacklist entry: DELETE /api/admin/blacklist/:id (admin only)
pub async fn handle_admin_remove_blacklist(
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

    let id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing blacklist entry ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    db::remove_from_blacklist(&db, &id).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Blacklist entry removed successfully"
    }))
}

/// Handle suspending a user: PUT /api/admin/users/:id/suspend (admin only)
pub async fn handle_admin_suspend_user(
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

    let target_user_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing user ID".to_string()))?
        .to_string();

    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let reason = match body.get("reason").and_then(|r| r.as_str()) {
        Some(r) => r.to_string(),
        None => return Response::error("Missing 'reason' field", 400),
    };

    // Safety guard: Cannot suspend self
    if target_user_id == user_ctx.user_id {
        return Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Cannot suspend yourself"
        }));
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Safety guard: Cannot suspend the last admin
    let admin_count = db::get_admin_count(&db).await?;
    if admin_count <= 1 {
        // Check if target user is admin
        if let Some(target_user) = db::get_user_by_id(&db, &target_user_id).await?
            && target_user.role == "admin"
        {
            return Response::error("Cannot suspend the last admin", 400);
        }
    }

    let target_user = match db::get_user_by_id(&db, &target_user_id).await? {
        Some(user) => user,
        None => return Response::error("User not found", 404),
    };
    let org_links = db::get_links_by_org(&db, &target_user.org_id).await?;

    // Suspend user
    db::suspend_user(&db, &target_user_id, &reason, &user_ctx.user_id).await?;

    // Disable all links for the user's organization
    let disabled_count = db::disable_all_links_for_org(&db, &target_user.org_id).await?;

    let kv = ctx.kv("URL_MAPPINGS")?;
    for link in org_links {
        if matches!(link.status, LinkStatus::Blocked) {
            continue;
        }
        let mut disabled_link = link;
        disabled_link.status = LinkStatus::Disabled;
        sync_link_mapping_from_link(&db, &kv, &disabled_link).await?;
    }

    // Invalidate all sessions for the user
    // Note: In a production system, we'd need to track user sessions and delete them
    // For now, we'll rely on the suspended_at check in auth middleware

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "User suspended successfully",
        "disabled_links": disabled_count
    }))
}

/// Handle unsuspending a user: PUT /api/admin/users/:id/unsuspend (admin only)
pub async fn handle_admin_unsuspend_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let target_user_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing user ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    db::unsuspend_user(&db, &target_user_id).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "User unsuspended successfully"
    }))
}

/// Handle deleting a user: DELETE /api/admin/users/:id (admin only)
pub async fn handle_admin_delete_user(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let target_user_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing user ID".to_string()))?
        .to_string();

    // Safety guard: Cannot delete self
    if target_user_id == user_ctx.user_id {
        return Response::error("Cannot delete your own account", 400);
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get target user info for safety checks
    let target_user = match db::get_user_by_id(&db, &target_user_id).await? {
        Some(user) => user,
        None => return Response::error("User not found", 404),
    };

    // Safety guard: Cannot delete the last admin in an organization
    if target_user.role == "admin"
        && db::is_last_admin_in_org(&db, &target_user_id, &target_user.org_id).await?
    {
        return Response::error("Cannot delete the last admin in an organization", 400);
    }

    // Parse request body for confirmation
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let confirmation = body.get("confirmation").and_then(|c| c.as_str());
    if confirmation != Some("DELETE") {
        return Response::error("Must provide 'confirmation': 'DELETE' in request body", 400);
    }

    let user_links = db::get_links_by_creator(&db, &target_user_id).await?;
    let kv = ctx.kv("URL_MAPPINGS")?;
    for link in &user_links {
        kv::delete_link_mapping(&kv, &link.org_id, &link.short_code).await?;
    }

    // Delete user and all their data
    let (user_count, links_count, analytics_count) =
        db::delete_user(&db, &target_user_id, &user_ctx.user_id).await?;

    if user_count == 0 {
        return Response::error("Failed to delete user", 500);
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "User and all associated data deleted successfully",
        "deleted_user_count": user_count,
        "deleted_links_count": links_count,
        "deleted_analytics_count": analytics_count
    }))
}

/// Handle updating user role: PUT /api/admin/users/:id (admin only)
pub async fn handle_admin_update_user_role(
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

    let target_user_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing user ID".to_string()))?
        .to_string();

    // Safety guard: Cannot modify own role
    if target_user_id == user_ctx.user_id {
        return Response::error("Cannot modify your own role", 400);
    }

    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let new_role = match body.get("role").and_then(|v| v.as_str()) {
        Some(role) => {
            if role != "admin" && role != "member" {
                return Response::error("Role must be 'admin' or 'member'", 400);
            }
            role.to_string()
        }
        None => return Response::error("Missing 'role' field", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get target user info for safety checks
    let target_user = match db::get_user_by_id(&db, &target_user_id).await? {
        Some(user) => user,
        None => return Response::error("User not found", 404),
    };

    // Safety guard: Cannot demote the last admin
    if new_role == "member" && target_user.role == "admin" {
        let admin_count = match db::get_admin_count(&db).await {
            Ok(count) => count,
            Err(e) => {
                console_log!("Error checking admin count: {}", e);
                return Response::error("Failed to verify admin count", 500);
            }
        };

        if admin_count <= 1 {
            return Response::error("Cannot demote the last admin user", 400);
        }
    }

    // Update user role
    match db::update_user_role(&db, &target_user_id, &new_role).await {
        Ok(_) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "user_role_updated",
                    "target_user_id": target_user_id,
                    "old_role": target_user.role,
                    "new_role": new_role,
                    "admin_user_id": user_ctx.user_id,
                    "level": "info"
                })
            );

            // Get updated user data
            let updated_user = match db::get_user_by_id(&db, &target_user_id).await {
                Ok(Some(user)) => Ok(user),
                Ok(None) => Err(Error::RustError(
                    "Failed to retrieve updated user".to_string(),
                )),
                Err(e) => Err(e),
            }?;

            Ok(Response::from_json(&serde_json::json!({
                "id": updated_user.id,
                "email": updated_user.email,
                "name": updated_user.name,
                "avatar_url": updated_user.avatar_url,
                "oauth_provider": updated_user.oauth_provider,
                "oauth_id": updated_user.oauth_id,
                "org_id": updated_user.org_id,
                "role": updated_user.role,
                "created_at": updated_user.created_at,
                "suspended_at": updated_user.suspended_at,
                "suspension_reason": updated_user.suspension_reason,
                "suspended_by": updated_user.suspended_by,
            }))?)
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "user_role_update_failed",
                    "target_user_id": target_user_id,
                    "new_role": new_role,
                    "error": e.to_string(),
                    "admin_user_id": user_ctx.user_id,
                    "level": "error"
                })
            );
            Response::error("Failed to update user role", 500)
        }
    }
}

/// Handle abuse report submission: POST /api/reports/links
pub async fn handle_report_link(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let link_id = match body.get("link_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Response::error("Missing 'link_id' field", 400),
    };

    let reason = match body.get("reason").and_then(|v| v.as_str()) {
        Some(r) => r.to_string(),
        None => return Response::error("Missing 'reason' field", 400),
    };

    let reporter_email = body.get("reporter_email").and_then(|v| v.as_str());

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Validate that the link exists and check its status
    let link = match db::get_active_link_by_short_code(&db, &link_id).await {
        Ok(Some(link)) => Some(link),
        Ok(None) => {
            // Try by ID if short code not found
            match db::get_link_by_id_no_auth_all(&db, &link_id).await {
                Ok(Some(link)) => Some(link),
                Ok(None) => {
                    return Ok(Response::from_json(&serde_json::json!({
                        "success": false,
                        "message": "This link doesn't exist or has been removed.",
                        "error_type": "link_not_found"
                    }))?
                    .with_status(404));
                }
                Err(e) => {
                    return Response::error(format!("Database error: {}", e), 500);
                }
            }
        }
        Err(e) => {
            return Response::error(format!("Database error: {}", e), 500);
        }
    };

    if link.is_none() {
        return Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "This link doesn't exist or has been removed.",
            "error_type": "link_not_found"
        }))?
        .with_status(404));
    }

    let link_ref = link.unwrap();

    // Check if link is already blocked or disabled
    if matches!(link_ref.status, LinkStatus::Blocked | LinkStatus::Disabled) {
        return Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "This link has already been disabled and cannot be reported.",
            "error_type": "link_already_disabled"
        }))?
        .with_status(422));
    }

    // Get reporter info (authenticated user or email)
    let (reporter_user_id, reporter_email_opt) = match auth::authenticate_request(&req, &ctx).await
    {
        Ok(user_ctx) => {
            // Authenticated user
            (
                Some(user_ctx.user_id),
                reporter_email.map(|s| s.to_string()),
            )
        }
        Err(_) => {
            // Anonymous user - use provided email
            (None, reporter_email.map(|s| s.to_string()))
        }
    };

    let actual_link_id = link_ref.id.clone();

    // Check for duplicate reports (same link, reason, reporter within 24h)
    let is_duplicate = db::is_duplicate_report(
        &db,
        &actual_link_id,
        &reason,
        reporter_user_id.as_deref(),
        reporter_email_opt.as_deref(),
    )
    .await;

    if is_duplicate.unwrap_or(false) {
        return Response::error(
            "You have already reported this link for the same reason within the last 24 hours",
            429,
        );
    }

    // Rate limiting check
    let _client_ip = if let Ok(Some(ip)) = req.headers().get("CF-Connecting-IP") {
        ip.to_string()
    } else if let Ok(Some(ip)) = req.headers().get("X-Forwarded-For") {
        ip.to_string()
    } else {
        "unknown".to_string()
    };

    // For now, we'll implement basic rate limiting in the application layer
    // In production, you might want to use Redis or a dedicated rate limiting service

    // Store the report in database
    match db::create_link_report(
        &db,
        &actual_link_id,
        &reason,
        reporter_user_id.as_deref(),
        reporter_email_opt.as_deref(),
    )
    .await
    {
        Ok(_) => {
            // Hash sensitive fields for logging (privacy protection while maintaining correlation capability)
            let reporter_user_id_hash = reporter_user_id.as_ref().map(|id| {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(id.as_bytes());
                format!("{:x}", hasher.finalize())
            });
            let reporter_email_hash = reporter_email_opt.as_ref().map(|email| {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(email.as_bytes());
                format!("{:x}", hasher.finalize())
            });

            console_log!(
                "{}",
                serde_json::json!({
                    "event": "abuse_report_stored",
                    "link_id": actual_link_id,
                    "reason": reason,
                    "reporter_user_id_hash": reporter_user_id_hash,
                    "reporter_email_hash": reporter_email_hash,
                    "level": "info"
                })
            );

            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Report submitted successfully. Thank you for helping keep our platform safe."
            }))
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "abuse_report_failed",
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to store report. Please try again.", 500)
        }
    }
}

/// Handle getting reports for admin: GET /api/admin/reports
pub async fn handle_admin_get_reports(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Parse query parameters
    let url = req.url()?;
    let query_pairs = url.query_pairs();

    let mut page: u32 = 1;
    let mut limit: u32 = 50;
    let mut status_filter: Option<String> = None;

    for (key, value) in query_pairs {
        match key.as_ref() {
            "page" => page = value.parse().unwrap_or(1),
            "limit" => limit = value.parse().unwrap_or(50),
            "status" => status_filter = Some(value.to_string()),
            _ => {}
        }
    }

    // Validate limits
    if page < 1 {
        page = 1;
    }
    limit = limit.clamp(10, 100);

    match db::get_link_reports(&db, page, limit, status_filter.as_deref()).await {
        Ok((reports, total)) => Response::from_json(&serde_json::json!({
            "reports": reports,
            "pagination": {
                "page": page,
                "limit": limit,
                "total": total,
                "pages": (total as f64 / limit as f64).ceil() as u32
            }
        })),
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "reports_fetch_failed",
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to retrieve reports", 500)
        }
    }
}

/// Handle getting a single report: GET /api/admin/reports/:id
pub async fn handle_admin_get_report(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let report_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing report ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match db::get_link_report_by_id(&db, &report_id).await {
        Ok(report) => Response::from_json(&report),
        Err(_) => Response::error("Report not found", 404),
    }
}

/// Handle updating report status: PUT /api/admin/reports/:id
pub async fn handle_admin_update_report(
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

    let report_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing report ID".to_string()))?
        .to_string();

    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let status = match body.get("status").and_then(|v| v.as_str()) {
        Some(s) if s == "reviewed" || s == "dismissed" => s.to_string(),
        Some(_) => {
            return Response::error("Invalid status. Must be 'reviewed' or 'dismissed'", 400);
        }
        None => return Response::error("Missing 'status' field", 400),
    };

    let admin_notes = body.get("admin_notes").and_then(|v| v.as_str());

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match db::update_link_report_status(&db, &report_id, &status, &user_ctx.user_id, admin_notes)
        .await
    {
        Ok(_) => Response::from_json(&serde_json::json!({
            "success": true,
            "message": "Report status updated successfully"
        })),
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "report_update_failed",
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to update report status", 500)
        }
    }
}

/// Handle getting pending reports count: GET /api/admin/reports/pending/count
pub async fn handle_admin_get_pending_reports_count(
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

    match db::get_pending_reports_count(&db).await {
        Ok(count) => Response::from_json(&serde_json::json!({
            "count": count
        })),
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "reports_count_failed",
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to get pending reports count", 500)
        }
    }
}

/// Handle listing all billing accounts: GET /api/admin/billing-accounts (admin only)
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
        Ok((accounts, total)) => Response::from_json(&serde_json::json!({
            "accounts": accounts,
            "total": total,
            "page": page,
            "limit": limit,
        })),
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

/// Handle getting billing account details: GET /api/admin/billing-accounts/:id (admin only)
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

/// Handle updating billing account tier: PUT /api/admin/billing-accounts/:id/tier (admin only)
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

/// Handle resetting billing account counter: POST /api/admin/billing-accounts/:id/reset-counter (admin only)
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

/// Handle updating subscription status: PUT /api/admin/billing-accounts/:id/subscription (admin only)
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

// ─── Tag Management ─────────────────────────────────────────────────────────────

/// Handle deleting a tag: DELETE /api/tags/:name
pub async fn handle_delete_org_tag(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let tag_name = match ctx.param("name") {
        Some(name) => name.to_string(),
        None => return Response::error("Missing tag name", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Delete the tag from the organization
    match db::delete_tag_for_org(&db, &user_ctx.org_id, &tag_name).await {
        Ok(()) => Ok(Response::empty()?.with_status(204)),
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "delete_tag_failed",
                    "org_id": user_ctx.org_id,
                    "tag_name": tag_name,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to delete tag", 500)
        }
    }
}

/// Handle renaming a tag: PATCH /api/tags/:name
pub async fn handle_rename_org_tag(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let old_name = match ctx.param("name") {
        Some(name) => name.to_string(),
        None => return Response::error("Missing tag name", 400),
    };

    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    let new_name = match body.get("new_name").and_then(|v| v.as_str()) {
        Some(name) => name.to_string(),
        None => return Response::error("Missing new_name field", 400),
    };

    // Validate and normalize the new tag name
    let normalized_new_name = match db::normalize_tag(&new_name) {
        Some(name) => name,
        None => return Response::error("Invalid tag name", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Rename the tag
    match db::rename_tag_for_org(&db, &user_ctx.org_id, &old_name, &normalized_new_name).await {
        Ok(()) => {
            // Return updated tag list
            match db::get_org_tags(&db, &user_ctx.org_id).await {
                Ok(tags) => Response::from_json(&tags),
                Err(e) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "get_tags_after_rename_failed",
                            "org_id": user_ctx.org_id,
                            "error": e.to_string(),
                            "level": "error"
                        })
                    );
                    Response::error("Tag renamed but failed to fetch updated list", 500)
                }
            }
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "rename_tag_failed",
                    "org_id": user_ctx.org_id,
                    "old_name": old_name,
                    "new_name": normalized_new_name,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to rename tag", 500)
        }
    }
}
