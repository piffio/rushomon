use crate::auth;
use crate::kv;
use crate::middleware::{RateLimitConfig, RateLimiter};
use crate::models::{
    Tier,
    link::{CreateLinkRequest, Link, LinkStatus},
};
use crate::repositories::tag_repository::validate_and_normalize_tags;
use crate::repositories::{
    BillingRepository, BlacklistRepository, LinkRepository, OrgRepository, TagRepository,
};
use crate::utils::{generate_short_code, now_timestamp, validate_short_code, validate_url};
use chrono::Datelike;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    post,
    path = "/api/links",
    tag = "Links",
    summary = "Create a link",
    description = "Creates a new short link for the authenticated organization. Respects monthly tier limits. Optionally accepts a custom short code (Pro+), UTM parameters (Pro+), tags, expiry, and redirect type",
    request_body(content = CreateLinkRequest, description = "Link creation payload"),
    responses(
        (status = 201, description = "Link created", body = Link),
        (status = 400, description = "Invalid request body or URL"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Monthly link limit reached for current tier"),
        (status = 409, description = "Short code already in use"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_create_link(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let user_id = &user_ctx.user_id;
    let org_id = &user_ctx.org_id;

    let kv = ctx.kv("URL_MAPPINGS")?;
    let rate_limit_key = RateLimiter::user_key("create_link", user_id);
    let rate_limit_config = RateLimitConfig::link_creation();

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

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let billing_repo = BillingRepository::new();
    let billing_account = billing_repo
        .get_for_org(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("No billing account found for organization".to_string()))?;

    let tier = Tier::from_str_value(&billing_account.tier);
    let limits = tier.as_ref().map(|t| t.limits());

    if let Some(ref tier_limits) = limits
        && let Some(max_links) = tier_limits.max_links_per_month
    {
        let now = chrono::Utc::now();
        let year_month = format!("{}-{:02}", now.year(), now.month());

        let can_create = billing_repo
            .increment_monthly_counter(&db, &billing_account.id, &year_month, max_links)
            .await?;

        if !can_create {
            let current_count = billing_repo
                .get_monthly_counter(&db, &billing_account.id, &year_month)
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

    let raw_body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => {
            return Response::error(format!("Invalid JSON: {}", e), 400);
        }
    };

    let expected_fields = [
        "destination_url",
        "short_code",
        "title",
        "expires_at",
        "tags",
        "utm_params",
        "forward_query_params",
        "redirect_type",
    ];
    if let Some(obj) = raw_body.as_object() {
        for field_name in obj.keys() {
            if !expected_fields.contains(&field_name.as_str()) {
                return Response::error(
                    format!(
                        "Unknown field '{}'. Expected fields: destination_url, short_code (optional), title (optional), expires_at (optional), tags (optional), utm_params (optional, Pro+), forward_query_params (optional, Pro+), redirect_type (optional, defaults to 301)",
                        field_name
                    ),
                    400,
                );
            }
        }
    } else {
        return Response::error("Request body must be a JSON object", 400);
    }

    let body: CreateLinkRequest = match serde_json::from_value(raw_body) {
        Ok(body) => body,
        Err(e) => {
            return Response::error(format!("Invalid request format: {}", e), 400);
        }
    };

    let destination_url = match validate_url(&body.destination_url) {
        Ok(url) => url,
        Err(e) => {
            return Response::error(format!("Invalid destination URL: {}", e), 400);
        }
    };

    let blacklist_repo = BlacklistRepository::new();
    if blacklist_repo.is_blacklisted(&db, &destination_url).await? {
        return Response::error("Destination URL is blocked", 403);
    }

    if let Some(ref title) = body.title
        && title.len() > 200
    {
        return Response::error("Title must be 200 characters or less", 400);
    }

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

    let is_pro_or_above = matches!(
        tier.as_ref(),
        Some(Tier::Pro) | Some(Tier::Business) | Some(Tier::Unlimited)
    );

    let wants_pro_features = body
        .utm_params
        .as_ref()
        .map(|u| !u.is_empty())
        .unwrap_or(false)
        || body.forward_query_params.is_some()
        || body.redirect_type != "301";
    if wants_pro_features && !is_pro_or_above {
        let error_msg = if body.redirect_type != "301" {
            "Custom redirect types (307) require a Pro plan or above."
        } else {
            "UTM parameters and query parameter forwarding require a Pro plan or above."
        };
        return Response::error(error_msg, 403);
    }

    let kv = ctx.kv("URL_MAPPINGS")?;

    let short_code = if let Some(custom_code) = body.short_code {
        match validate_short_code(&custom_code) {
            Ok(code) => code,
            Err(e) => {
                return Response::error(format!("Invalid short code: {}", e), 400);
            }
        };

        if kv::links::short_code_exists(&kv, &custom_code).await? {
            return Response::error("Short code already in use", 409);
        }

        custom_code
    } else {
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

    let normalized_tags = if let Some(tags) = body.tags {
        match validate_and_normalize_tags(&tags) {
            Ok(t) => t,
            Err(e) => return Response::error(e.to_string(), 400),
        }
    } else {
        Vec::new()
    };

    if let Some(ref tier_limits) = limits
        && let Some(max_tags) = tier_limits.max_tags
    {
        let current_tag_count = TagRepository::new()
            .count_distinct_tags_for_billing_account(&db, &billing_account.id)
            .await?;

        let mut new_tag_count = 0;
        if !normalized_tags.is_empty() {
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

            new_tag_count = normalized_tags
                .iter()
                .filter(|tag| !existing_tags_set.contains(*tag))
                .count() as i64;
        }

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

    let link_id = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();
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
        redirect_type: body.redirect_type.clone(),
    };

    let repo = LinkRepository::new();
    let org_repo = OrgRepository::new();
    repo.create(&db, &link).await?;

    if !normalized_tags.is_empty() {
        repo.set_tags(&db, &link_id, org_id, &normalized_tags)
            .await?;
    }

    let resolved_forward = if link.forward_query_params.is_none() {
        org_repo
            .get_forward_query_params(&db, org_id)
            .await
            .unwrap_or(false)
    } else {
        link.forward_query_params.unwrap_or(false)
    };

    let mapping = link.to_mapping(resolved_forward);
    kv::store_link_mapping(&kv, org_id, &short_code, &mapping).await?;

    Response::from_json(&link)
}
