/// Admin blacklist handlers
///
/// POST   /api/admin/blacklist       — block a destination URL
/// GET    /api/admin/blacklist       — list all blacklist entries
/// DELETE /api/admin/blacklist/:id   — remove a blacklist entry
use crate::auth;
use crate::kv;
use crate::repositories::{BlacklistRepository, LinkRepository, ReportRepository};
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    post,
    path = "/api/admin/blacklist",
    tag = "Admin",
    summary = "Block a destination",
    responses(
        (status = 200, description = "Destination blocked"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_block_destination(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    Ok(inner_block(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_block(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let destination = body
        .get("destination")
        .and_then(|d| d.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'destination' field".to_string()))?
        .to_string();

    let match_type = match body.get("match_type").and_then(|m| m.as_str()) {
        Some(m) if m == "exact" || m == "domain" => m.to_string(),
        Some(_) => {
            return Err(AppError::BadRequest(
                "Invalid match_type. Must be 'exact' or 'domain'".to_string(),
            ));
        }
        None => "exact".to_string(),
    };

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
                destination.clone()
            }
        }
    } else {
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
                let without_protocol = destination
                    .trim_start_matches("http://")
                    .trim_start_matches("https://");
                without_protocol
                    .split('/')
                    .next()
                    .unwrap_or(without_protocol)
                    .to_string()
            }
        }
    };

    let reason = body
        .get("reason")
        .and_then(|r| r.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'reason' field".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = BlacklistRepository::new();

    if repo
        .is_duplicate(&db, &normalized_destination, &match_type)
        .await?
    {
        return Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Destination is already blocked",
            "already_blocked": true
        }))?);
    }

    repo.add(
        &db,
        &normalized_destination,
        &match_type,
        &reason,
        &user_ctx.user_id,
    )
    .await?;

    let candidate_links = repo.get_candidate_links(&db).await?;
    let mut blocked_links = Vec::new();
    for link in candidate_links {
        if repo.is_blacklisted(&db, &link.destination_url).await? {
            blocked_links.push(link);
        }
    }

    let blocked_count = blocked_links.len() as i64;

    let link_repo = LinkRepository::new();
    let report_repo = ReportRepository::new();
    if blocked_count > 0 {
        let kv = ctx.kv("URL_MAPPINGS")?;
        for link in blocked_links {
            link_repo
                .update_status_by_id(&db, &link.id, "blocked")
                .await?;
            if let Err(e) = report_repo
                .resolve_for_link(
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

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Destination blocked successfully",
        "blocked_links": blocked_count
    }))?)
}

#[utoipa::path(
    get,
    path = "/api/admin/blacklist",
    tag = "Admin",
    summary = "List blacklisted destinations",
    responses(
        (status = 200, description = "Array of blacklist entries"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_get_blacklist(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = BlacklistRepository::new();
    let entries = repo.list_all(&db).await?;

    Ok(Response::from_json(&entries)?)
}

#[utoipa::path(
    delete,
    path = "/api/admin/blacklist/{id}",
    tag = "Admin",
    summary = "Remove blacklist entry",
    params(("id" = String, Path, description = "Blacklist entry ID")),
    responses(
        (status = 200, description = "Entry removed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_remove_blacklist(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    Ok(inner_remove(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_remove(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing blacklist entry ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = BlacklistRepository::new();
    repo.remove(&db, &id).await?;

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Blacklist entry removed successfully"
    }))?)
}
