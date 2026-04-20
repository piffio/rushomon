/// Admin API key handlers
///
/// GET    /api/admin/api-keys             — paginated list
/// DELETE /api/admin/api-keys/:id         — revoke (active → revoked)
/// POST   /api/admin/api-keys/:id/reactivate — reactivate (revoked → active)
/// POST   /api/admin/api-keys/:id/delete  — soft-delete
/// POST   /api/admin/api-keys/:id/restore — restore (deleted → active)
use crate::auth;
use crate::services::AdminService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/admin/api-keys",
    tag = "Admin",
    summary = "List all API keys",
    params(
        ("page" = Option<i64>, Query, description = "Page number"),
        ("limit" = Option<i64>, Query, description = "Items per page (max 100)"),
        ("search" = Option<String>, Query, description = "Search by name or user"),
        ("status" = Option<String>, Query, description = "Filter by status"),
    ),
    responses(
        (status = 200, description = "Paginated list of API keys"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_list_api_keys(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let url = req.url()?;
    let query: std::collections::HashMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let page: i64 = query
        .get("page")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .max(1);
    let limit: i64 = query
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20)
        .clamp(1, 100);
    let search = query.get("search").map(|s| s.as_str());
    let status_filter = query.get("status").map(|s| s.as_str());

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let (keys, total) = AdminService::new()
        .list_api_keys(&db, page, limit, search, status_filter)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to list API keys: {}", e)))?;

    Ok(Response::from_json(&serde_json::json!({
        "keys": keys,
        "total": total,
        "page": page,
        "limit": limit,
    }))?)
}

#[utoipa::path(
    delete,
    path = "/api/admin/api-keys/{id}",
    tag = "Admin",
    summary = "Revoke an API key",
    params(("id" = String, Path, description = "API key ID")),
    responses(
        (status = 200, description = "Key revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_revoke_api_key(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_action(req, ctx, "revoke")
        .await
        .unwrap_or_else(|e| e.into_response()))
}

#[utoipa::path(
    post,
    path = "/api/admin/api-keys/{id}/reactivate",
    tag = "Admin",
    summary = "Reactivate an API key",
    params(("id" = String, Path, description = "API key ID")),
    responses(
        (status = 200, description = "Key reactivated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_reactivate_api_key(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    Ok(inner_action(req, ctx, "reactivate")
        .await
        .unwrap_or_else(|e| e.into_response()))
}

#[utoipa::path(
    post,
    path = "/api/admin/api-keys/{id}/delete",
    tag = "Admin",
    summary = "Soft-delete an API key",
    params(("id" = String, Path, description = "API key ID")),
    responses(
        (status = 200, description = "Key deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_delete_api_key(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_action(req, ctx, "delete")
        .await
        .unwrap_or_else(|e| e.into_response()))
}

#[utoipa::path(
    post,
    path = "/api/admin/api-keys/{id}/restore",
    tag = "Admin",
    summary = "Restore a deleted API key",
    params(("id" = String, Path, description = "API key ID")),
    responses(
        (status = 200, description = "Key restored"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_restore_api_key(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_action(req, ctx, "restore")
        .await
        .unwrap_or_else(|e| e.into_response()))
}

/// Shared implementation for the four single-key mutation handlers.
async fn inner_action(
    req: Request,
    ctx: RouteContext<()>,
    action: &str,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let key_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing key ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let result = match action {
        "revoke" => {
            AdminService::new()
                .revoke_api_key(&db, &key_id, &user_ctx.user_id)
                .await
        }
        "reactivate" => {
            AdminService::new()
                .reactivate_api_key(&db, &key_id, &user_ctx.user_id)
                .await
        }
        "delete" => {
            AdminService::new()
                .delete_api_key(&db, &key_id, &user_ctx.user_id)
                .await
        }
        "restore" => {
            AdminService::new()
                .restore_api_key(&db, &key_id, &user_ctx.user_id)
                .await
        }
        _ => unreachable!(),
    };

    match result {
        Ok(_) => {
            let event = format!(
                "admin_api_key_{}",
                action
                    .replace("revoke", "revoked")
                    .replace("reactivate", "reactivated")
                    .replace("delete", "deleted")
                    .replace("restore", "restored")
            );
            console_log!(
                "{}",
                serde_json::json!({
                    "event": event,
                    "key_id": key_id,
                    "admin_user_id": user_ctx.user_id,
                    "level": "info"
                })
            );
            Ok(Response::from_json(&serde_json::json!({
                "success": true,
                "message": format!("API key {}d successfully", action)
            }))?)
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": format!("admin_{}_api_key_failed", action),
                    "key_id": key_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Err(AppError::Internal(format!("Failed to {} API key", action)))
        }
    }
}
