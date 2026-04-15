use crate::auth;
use crate::db;
use crate::models::{Tier, link::UpdateLinkRequest};
use crate::repositories::tag_repository::validate_and_normalize_tags;
use crate::repositories::{LinkRepository, TagRepository};
use crate::utils::{now_timestamp, validate_url};
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    put,
    path = "/api/links/{id}",
    tag = "Links",
    summary = "Update a link",
    description = "Updates a link's destination URL, title, tags, expiry, UTM parameters, redirect type, or forward-query-params setting. Updates are written to both D1 and KV atomically",
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
        None => return Response::error("Missing link ID", 400),
    };

    let update_req: UpdateLinkRequest = match req.json().await {
        Ok(req) => req,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    if let Some(url) = &update_req.destination_url {
        if let Err(e) = validate_url(url) {
            return Response::error(format!("Invalid URL: {}", e), 400);
        }

        let db = ctx.env.get_binding::<D1Database>("rushomon")?;
        if db::is_destination_blacklisted(&db, url).await? {
            return Response::error("Destination URL is blocked", 403);
        }
    }

    if let Some(ref title) = update_req.title
        && title.len() > 200
    {
        return Response::error("Title must be 200 characters or less", 400);
    }

    if let Some(expires_at) = update_req.expires_at {
        let now = now_timestamp();
        if expires_at <= now {
            return Response::error("Expiration date must be in the future", 400);
        }
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv = ctx.kv("URL_MAPPINGS")?;
    let repo = LinkRepository::new();

    let _existing_link = match repo.get_by_id(&db, &link_id, &user_ctx.org_id).await? {
        Some(link) => link,
        None => return Response::error("Link not found", 404),
    };

    let billing_account = db::get_billing_account_for_org(&db, &user_ctx.org_id)
        .await?
        .ok_or_else(|| Error::RustError("No billing account found for organization".to_string()))?;
    let tier = Tier::from_str_value(&billing_account.tier);
    let is_pro_or_above = matches!(
        tier.as_ref(),
        Some(Tier::Pro) | Some(Tier::Business) | Some(Tier::Unlimited)
    );

    let wants_pro_features = update_req
        .utm_params
        .as_ref()
        .map(|u| !u.is_empty())
        .unwrap_or(false)
        || update_req.forward_query_params.is_some()
        || (update_req.redirect_type.is_some()
            && update_req.redirect_type.as_deref() != Some("301"));
    if wants_pro_features && !is_pro_or_above {
        let error_msg = if update_req.redirect_type.is_some()
            && update_req.redirect_type.as_deref() != Some("301")
        {
            "Custom redirect types (307) require a Pro plan or above."
        } else {
            "UTM parameters and query parameter forwarding require a Pro plan or above."
        };
        return Response::error(error_msg, 403);
    }

    let utm_json_for_db: Option<Option<String>> = update_req.utm_params.as_ref().map(|u| {
        if u.is_empty() {
            None
        } else {
            u.to_json_string()
        }
    });

    let mut updated_link = repo
        .update(
            &db,
            &link_id,
            &user_ctx.org_id,
            update_req.destination_url.as_deref(),
            update_req.title.as_deref(),
            update_req.status.as_ref().map(|s| s.as_str()),
            update_req.expires_at,
            utm_json_for_db.as_ref().map(|o| o.as_deref()),
            update_req.forward_query_params.map(Some),
            update_req.redirect_type.as_deref(),
        )
        .await?;

    if let Some(tags) = update_req.tags {
        let normalized_tags = match validate_and_normalize_tags(&tags) {
            Ok(t) => t,
            Err(e) => return Response::error(e.to_string(), 400),
        };

        let tier_limits = tier.as_ref().map(|t| t.limits());
        if let Some(ref limits) = tier_limits
            && let Some(max_tags) = limits.max_tags
        {
            let current_tag_count = TagRepository::new()
                .count_distinct_tags_for_billing_account(&db, &billing_account.id)
                .await?;

            let existing_link_tags = repo.get_tags(&db, &link_id).await?;
            let existing_tags_set: std::collections::HashSet<String> =
                existing_link_tags.into_iter().collect();
            let new_tags_set: std::collections::HashSet<String> =
                normalized_tags.iter().cloned().collect();

            let existing_ba_tags_query = db.prepare(
                "SELECT DISTINCT tag_name
                 FROM link_tags lt
                 JOIN organizations o ON lt.org_id = o.id
                 WHERE o.billing_account_id = ?1",
            );
            let existing_ba_tags_result = existing_ba_tags_query
                .bind(&[billing_account.id.clone().into()])?
                .all()
                .await?;
            let existing_ba_tags_set: std::collections::HashSet<String> = existing_ba_tags_result
                .results::<serde_json::Value>()?
                .iter()
                .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
                .collect();

            let tags_being_removed: std::collections::HashSet<String> = existing_tags_set
                .difference(&new_tags_set)
                .cloned()
                .collect();

            let tags_being_added: std::collections::HashSet<String> = new_tags_set
                .difference(&existing_tags_set)
                .cloned()
                .collect();

            let mut disappearing_count = 0;
            for tag in &tags_being_removed {
                let usage_query = db.prepare(
                    "SELECT COUNT(*) as count
                     FROM link_tags lt
                     JOIN organizations o ON lt.org_id = o.id
                     WHERE o.billing_account_id = ?1 AND lt.tag_name = ?2 AND lt.link_id != ?3",
                );
                let usage_result = usage_query
                    .bind(&[
                        billing_account.id.clone().into(),
                        tag.as_str().into(),
                        link_id.as_str().into(),
                    ])?
                    .first::<serde_json::Value>(None)
                    .await?;
                let usage_count = usage_result
                    .and_then(|r| r["count"].as_f64())
                    .unwrap_or(0.0) as i64;

                if usage_count == 0 {
                    disappearing_count += 1;
                }
            }

            let new_to_ba_count = tags_being_added
                .iter()
                .filter(|tag| !existing_ba_tags_set.contains(*tag))
                .count() as i64;

            let net_change = new_to_ba_count - disappearing_count;

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

        repo.set_tags(&db, &link_id, &user_ctx.org_id, &normalized_tags)
            .await?;
        updated_link.tags = normalized_tags;
    } else {
        updated_link.tags = repo.get_tags(&db, &link_id).await?;
    }

    let kv_needs_update = update_req.destination_url.is_some()
        || update_req.status.is_some()
        || update_req.utm_params.is_some()
        || update_req.forward_query_params.is_some();
    if kv_needs_update {
        repo.sync_kv_from_link(&db, &kv, &updated_link).await?;
    }

    Response::from_json(&updated_link)
}
