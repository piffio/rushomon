use crate::auth;
use crate::models::link::UpdateLinkRequest;
use crate::repositories::BlacklistRepository;
use crate::services::LinkService;
use crate::utils::validate_and_normalize_tags;
use crate::utils::{now_timestamp, validate_url};
use serde_json::json;
use worker::d1::D1Database;
use worker::*;

fn json_error(message: &str, status: u16) -> Response {
    Response::from_json(&json!({ "error": message }))
        .unwrap()
        .with_status(status)
}

#[utoipa::path(
    put,
    path = "/api/links/{id}",
    tag = "Links",
    summary = "Update a link",
    description = "Updates a link's destination URL, title, tags, expiry, UTM parameters, redirect type, or forward-query-params setting. Use clear_expiration=true to remove the expiration date. Updates are written to both D1 and KV atomically",
    params(
        ("id" = String, Path, description = "Link ID"),
    ),
    request_body(content = UpdateLinkRequest, description = "Fields to update (all optional)"),
    responses(
        (status = 200, description = "Updated link", body = crate::models::Link),
        (status = 400, description = "Invalid request body"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Link not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_update_link(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let link_id = match ctx.param("id") {
        Some(id) => id.to_string(),
        None => return Ok(json_error("Missing link ID", 400)),
    };

    let update_req: UpdateLinkRequest = match req.json::<UpdateLinkRequest>().await {
        Ok(req) => req,
        Err(_) => return Ok(json_error("Invalid request body", 400)),
    };

    if let Some(url) = &update_req.destination_url {
        if let Err(e) = validate_url(url) {
            return Ok(json_error(&format!("Invalid URL: {}", e), 400));
        }

        let db = ctx.env.get_binding::<D1Database>("rushomon")?;
        let blacklist_repo = BlacklistRepository::new();
        if blacklist_repo.is_blacklisted(&db, url).await? {
            return Ok(json_error("Destination URL is blocked", 403));
        }
    }

    if let Some(ref title) = update_req.title
        && title.len() > 200
    {
        return Ok(json_error("Title must be 200 characters or less", 400));
    }

    let now = now_timestamp();

    // Convert clear_expiration flag to expires_at format for repository
    let expires_at_for_repo = if update_req.clear_expiration == Some(true) {
        Some(None) // Clear expiration
    } else {
        update_req.expires_at.map(Some) // Set to specific timestamp or None
    };

    if let Some(Some(expires_at)) = expires_at_for_repo {
        if expires_at <= now {
            return Ok(json_error("Expiration date must be in the future", 400));
        }
        // Cloudflare KV requires minimum 60 second TTL
        if expires_at - now < 60 {
            return Ok(json_error(
                "Expiration must be at least 60 seconds in the future",
                400,
            ));
        }
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let link_service = LinkService::new();

    match link_service.get_link(&db, &link_id, &user_ctx.org_id).await {
        Ok(None) => return Ok(json_error("Link not found", 404)),
        Ok(Some(_)) => {}
        Err(e) => return Err(worker::Error::RustError(e.to_string())),
    }

    let wants_redirect_type =
        update_req.redirect_type.is_some() && update_req.redirect_type.as_deref() != Some("301");
    let wants_utm_or_forward = update_req
        .utm_params
        .as_ref()
        .map(|u| !u.is_empty())
        .unwrap_or(false)
        || update_req.forward_query_params.is_some();

    let (billing_account_id, tier) = match link_service
        .check_pro_features_for_org(
            &db,
            &user_ctx.org_id,
            wants_redirect_type,
            wants_utm_or_forward,
        )
        .await
    {
        Ok(result) => result,
        Err(crate::utils::AppError::Forbidden(msg)) => return Ok(json_error(&msg, 403)),
        Err(e) => return Err(worker::Error::RustError(e.to_string())),
    };

    let mut normalized_tags = None;
    if let Some(ref tags) = update_req.tags {
        let tags = match validate_and_normalize_tags(tags) {
            Ok(t) => t,
            Err(e) => return Ok(json_error(&e.to_string(), 400)),
        };

        let tier_limits = tier.as_ref().map(|t| t.limits());
        if let Some(ref limits) = tier_limits
            && let Some(max_tags) = limits.max_tags
        {
            match link_service
                .check_tag_limit_for_update(&db, &billing_account_id, &link_id, &tags, max_tags)
                .await
            {
                Ok(()) => {}
                Err(crate::utils::AppError::Forbidden(msg)) => return Ok(json_error(&msg, 403)),
                Err(e) => return Err(worker::Error::RustError(e.to_string())),
            }
        }

        normalized_tags = Some(tags);
    }

    let kv = ctx.kv("URL_MAPPINGS")?;

    let expires_at_value = if update_req.clear_expiration == Some(true) {
        Some(None)
    } else {
        update_req.expires_at.map(Some)
    };

    let status_str = update_req.status.as_ref().map(|s| s.as_str().to_string());

    let updated_link = link_service
        .update_link(
            &db,
            &kv,
            &link_id,
            &user_ctx.org_id,
            update_req.destination_url.clone(),
            update_req.title.clone(),
            expires_at_value,
            normalized_tags,
            update_req
                .utm_params
                .clone()
                .map(|u| if u.is_empty() { None } else { Some(u) }),
            update_req.forward_query_params.map(Some),
            status_str,
            update_req.redirect_type.clone(),
        )
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    Response::from_json(&updated_link)
}
