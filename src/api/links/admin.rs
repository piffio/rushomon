use crate::auth;
use crate::models::link::LinkStatus;
use crate::services::LinkService;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/admin/links",
    tag = "Admin",
    summary = "List all links (admin)",
    params(
        ("page" = Option<i64>, Query, description = "Page number"),
        ("limit" = Option<i64>, Query, description = "Items per page (max 100)"),
        ("org" = Option<String>, Query, description = "Filter by org ID"),
        ("email" = Option<String>, Query, description = "Filter by creator email"),
        ("domain" = Option<String>, Query, description = "Filter by destination domain"),
    ),
    responses(
        (status = 200, description = "Paginated list of all links with KV sync status"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_list_links(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_admin_list_links(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_admin_list_links(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, crate::utils::AppError> {
    let user_ctx = crate::auth::authenticate_request(&req, &ctx).await?;
    crate::auth::require_admin(&user_ctx)?;

    let url = req
        .url()
        .map_err(|e| crate::utils::AppError::Internal(format!("Invalid URL: {}", e)))?;
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

    let _offset = (page - 1) * limit;

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
    let service = LinkService::new();

    let (links, total) = service
        .admin_list_links(
            &db,
            &kv,
            page,
            limit,
            org_filter,
            email_filter,
            domain_filter,
        )
        .await?;

    Response::from_json(&serde_json::json!({
        "links": links,
        "total": total,
        "page": page,
        "limit": limit,
    }))
    .map_err(|e| crate::utils::AppError::Internal(format!("JSON error: {}", e)))
}

#[utoipa::path(
    put,
    path = "/api/admin/links/{id}",
    tag = "Admin",
    summary = "Update link status",
    params(("id" = String, Path, description = "Link ID")),
    responses(
        (status = 200, description = "Status updated"),
        (status = 400, description = "Invalid status value"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_update_link_status(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    Ok(inner_admin_update_link_status(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_admin_update_link_status(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, crate::utils::AppError> {
    let user_ctx = crate::auth::authenticate_request(&req, &ctx).await?;
    crate::auth::require_admin(&user_ctx)?;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| crate::utils::AppError::BadRequest("Missing link ID".to_string()))?
        .to_string();

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| crate::utils::AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let status = match body.get("status").and_then(|s| s.as_str()) {
        Some(s) if s == "active" || s == "disabled" || s == "blocked" => s.to_string(),
        Some(_) => {
            return Err(crate::utils::AppError::BadRequest(
                "Invalid status. Must be 'active', 'disabled', or 'blocked'".to_string(),
            ));
        }
        None => {
            return Err(crate::utils::AppError::BadRequest(
                "Missing 'status' field".to_string(),
            ));
        }
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv = ctx.kv("URL_MAPPINGS")?;
    let service = LinkService::new();

    // Parse status string to LinkStatus enum
    let status_enum = match status.as_str() {
        "active" => LinkStatus::Active,
        "disabled" => LinkStatus::Disabled,
        "blocked" => LinkStatus::Blocked,
        _ => {
            return Err(crate::utils::AppError::BadRequest(
                "Invalid status".to_string(),
            ));
        }
    };

    service
        .admin_update_link_status(&db, &kv, &link_id, status_enum)
        .await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": format!("Link status updated to {}", status)
    }))
    .map_err(|e| crate::utils::AppError::Internal(format!("JSON error: {}", e)))
}

#[utoipa::path(
    delete,
    path = "/api/admin/links/{id}",
    tag = "Admin",
    summary = "Hard delete a link",
    params(("id" = String, Path, description = "Link ID")),
    responses(
        (status = 200, description = "Link deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "Link not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_delete_link(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_admin_delete_link(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_admin_delete_link(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, crate::utils::AppError> {
    let user_ctx = crate::auth::authenticate_request(&req, &ctx).await?;
    crate::auth::require_admin(&user_ctx)?;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| crate::utils::AppError::BadRequest("Missing link ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv = ctx.kv("URL_MAPPINGS")?;
    let service = LinkService::new();

    service.admin_delete_link(&db, &kv, &link_id).await?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Link deleted successfully"
    }))
    .map_err(|e| crate::utils::AppError::Internal(format!("JSON error: {}", e)))
}

#[utoipa::path(
    post,
    path = "/api/admin/links/{id}/sync-kv",
    tag = "Admin",
    summary = "Re-sync link KV entry",
    params(("id" = String, Path, description = "Link ID")),
    responses(
        (status = 200, description = "KV entry synced"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "Link not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
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

    LinkService::new()
        .admin_sync_link_kv(&db, &kv, &link_id)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?;

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Link KV entry re-synced successfully"
    }))
}
