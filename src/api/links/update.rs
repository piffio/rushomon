use crate::auth;
use crate::models::link::UpdateLinkRequest;
use crate::repositories::BlacklistRepository;
use crate::services::LinkService;
use crate::utils::validate_and_normalize_tags;
use crate::utils::{now_timestamp, validate_destination};
use serde_json::json;
use worker::d1::D1Database;
use worker::*;

fn json_error(message: &str, status: u16) -> Response {
    Response::from_json(&json!({ "error": message }))
        .unwrap()
        .with_status(status)
}

fn final_device_url<'a>(
    update: Option<&'a str>,
    clear: Option<bool>,
    current: Option<&'a str>,
) -> Option<&'a str> {
    if clear == Some(true) {
        None
    } else {
        update.or(current)
    }
}

#[utoipa::path(
    put,
    path = "/api/links/{id}",
    tag = "Links",
    summary = "Update a link",
    description = "Updates a link's normal or inline MiniJinja template destination, title, tags, expiry, UTM parameters, redirect type, or forward-query-params setting. Template links require Pro+, a 307 redirect. Use clear_expiration=true to remove the expiration date. Updates are written to both D1 and KV",
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

    let mut update_req: UpdateLinkRequest = match req.json::<UpdateLinkRequest>().await {
        Ok(req) => req,
        Err(_) => return Ok(json_error("Invalid request body", 400)),
    };

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

    let existing_link = match link_service.get_link(&db, &link_id, &user_ctx.org_id).await {
        Ok(None) => return Ok(json_error("Link not found", 404)),
        Ok(Some(link)) => link,
        Err(e) => return Err(worker::Error::RustError(e.to_string())),
    };

    let final_is_template = update_req.is_template.unwrap_or(existing_link.is_template);
    let final_redirect_type = update_req
        .redirect_type
        .as_deref()
        .unwrap_or(&existing_link.redirect_type);
    if final_is_template && final_redirect_type != "307" {
        return Ok(json_error(
            "Template links require a 307 temporary redirect",
            400,
        ));
    }

    let final_destination = update_req
        .destination_url
        .as_deref()
        .unwrap_or(&existing_link.destination_url);
    let normalized_destination = match validate_destination(final_destination, final_is_template) {
        Ok(value) => value,
        Err(error) => {
            return Ok(json_error(
                &format!("Invalid destination URL: {}", error),
                400,
            ));
        }
    };
    if update_req.destination_url.is_some() {
        update_req.destination_url = Some(normalized_destination.clone());
    }

    let final_ios_url = final_device_url(
        update_req.ios_url.as_deref(),
        update_req.clear_ios_url,
        existing_link.ios_url.as_deref(),
    );
    let final_android_url = final_device_url(
        update_req.android_url.as_deref(),
        update_req.clear_android_url,
        existing_link.android_url.as_deref(),
    );
    let final_desktop_url = final_device_url(
        update_req.desktop_url.as_deref(),
        update_req.clear_desktop_url,
        existing_link.desktop_url.as_deref(),
    );
    for (label, destination) in [
        ("iOS", final_ios_url),
        ("Android", final_android_url),
        ("desktop", final_desktop_url),
    ] {
        if let Some(destination) = destination
            && let Err(error) = validate_destination(destination, final_is_template)
        {
            return Ok(json_error(
                &format!("Invalid {} URL: {}", label, error),
                400,
            ));
        }
    }

    for (label, destination) in [
        ("iOS", &mut update_req.ios_url),
        ("Android", &mut update_req.android_url),
        ("desktop", &mut update_req.desktop_url),
    ] {
        if let Some(value) = destination {
            *value = match validate_destination(value, final_is_template) {
                Ok(value) => value,
                Err(error) => {
                    return Ok(json_error(
                        &format!("Invalid {} URL: {}", label, error),
                        400,
                    ));
                }
            };
        }
    }

    if !final_is_template
        && (update_req.destination_url.is_some() || update_req.is_template.is_some())
    {
        let blacklist_repo = BlacklistRepository::new();
        if blacklist_repo
            .is_blacklisted(&db, &normalized_destination)
            .await?
        {
            return Ok(json_error("Destination URL is blocked", 403));
        }
    }

    let wants_redirect_type = final_is_template
        || (update_req.redirect_type.is_some()
            && update_req.redirect_type.as_deref() != Some("301"));
    let wants_utm_or_forward = update_req
        .utm_params
        .as_ref()
        .map(|u| !u.is_empty())
        .unwrap_or(false)
        || update_req.forward_query_params.is_some();
    let wants_device_routing = update_req.ios_url.is_some()
        || update_req.android_url.is_some()
        || update_req.desktop_url.is_some()
        || update_req.clear_ios_url == Some(true)
        || update_req.clear_android_url == Some(true)
        || update_req.clear_desktop_url == Some(true);

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

    // Check device routing tier requirement
    if wants_device_routing {
        let allow_device_routing = tier
            .as_ref()
            .map(|t| t.limits().allow_device_routing)
            .unwrap_or(false);
        if !allow_device_routing {
            return Ok(json_error(
                "Device-based routing is available on Business tier and above.",
                403,
            ));
        }
    }

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

    // Handle device URL updates with clear flags
    let ios_url_value = if update_req.clear_ios_url == Some(true) {
        Some(None) // Clear the URL
    } else {
        update_req.ios_url.map(Some)
    };
    let android_url_value = if update_req.clear_android_url == Some(true) {
        Some(None) // Clear the URL
    } else {
        update_req.android_url.map(Some)
    };
    let desktop_url_value = if update_req.clear_desktop_url == Some(true) {
        Some(None) // Clear the URL
    } else {
        update_req.desktop_url.map(Some)
    };

    let status_str = update_req.status.as_ref().map(|s| s.as_str().to_string());

    let updated_link = link_service
        .update_link(
            &db,
            &kv,
            &link_id,
            &user_ctx.org_id,
            update_req.destination_url.clone(),
            update_req.is_template,
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
            ios_url_value,
            android_url_value,
            desktop_url_value,
        )
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    Response::from_json(&updated_link)
}
