/// Admin user-management handlers
///
/// GET    /api/admin/users           — list all users (paginated)
/// GET    /api/admin/users/:id       — get one user
/// PUT    /api/admin/users/:id       — update user role
/// PUT    /api/admin/users/:id/suspend   — suspend user
/// PUT    /api/admin/users/:id/unsuspend — unsuspend user
/// DELETE /api/admin/users/:id       — delete user + data
use crate::api::links::sync_link_mapping_from_link;
use crate::auth;
use crate::kv;
use crate::models::link::LinkStatus;
use crate::repositories::UserRepository;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/admin/users",
    tag = "Admin",
    summary = "List all users",
    params(
        ("page" = Option<i64>, Query, description = "Page number"),
        ("limit" = Option<i64>, Query, description = "Items per page (max 100)"),
    ),
    responses(
        (status = 200, description = "Paginated list of users"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_list_users(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list_users(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list_users(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

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
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = UserRepository::new();

    let users = repo.list_with_billing_info(&db, limit, offset).await?;
    let total = repo.count(&db).await?;

    Ok(Response::from_json(&serde_json::json!({
        "users": users,
        "total": total,
        "page": page,
        "limit": limit,
    }))?)
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{id}",
    tag = "Admin",
    summary = "Get a user",
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = 200, description = "User details"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "User not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_get_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_get_user(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_get_user(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let user_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = UserRepository::new();

    match repo.get_user_by_id(&db, &user_id).await? {
        Some(user) => Ok(Response::from_json(&user)?),
        None => Err(AppError::NotFound("User not found".to_string())),
    }
}

#[utoipa::path(
    put,
    path = "/api/admin/users/{id}/suspend",
    tag = "Admin",
    summary = "Suspend a user",
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = 200, description = "User suspended"),
        (status = 400, description = "Cannot suspend self or last admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "User not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_suspend_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_suspend_user(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_suspend_user(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let target_user_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?
        .to_string();

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let reason = body
        .get("reason")
        .and_then(|r| r.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'reason' field".to_string()))?
        .to_string();

    if target_user_id == user_ctx.user_id {
        return Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "Cannot suspend yourself"
        }))?);
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = UserRepository::new();

    let admin_count = repo.admin_count(&db).await?;
    if admin_count <= 1
        && let Some(target_user) = repo.get_user_by_id(&db, &target_user_id).await?
        && target_user.role == "admin"
    {
        return Err(AppError::BadRequest(
            "Cannot suspend the last admin".to_string(),
        ));
    }

    let target_user = repo
        .get_user_by_id(&db, &target_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let org_links = repo.get_links_by_org(&db, &target_user.org_id).await?;
    repo.suspend(&db, &target_user_id, &reason, &user_ctx.user_id)
        .await?;
    let disabled_count = repo
        .disable_all_links_for_org(&db, &target_user.org_id)
        .await?;

    let kv = ctx.kv("URL_MAPPINGS")?;
    for link in org_links {
        if matches!(link.status, LinkStatus::Blocked) {
            continue;
        }
        let mut disabled_link = link;
        disabled_link.status = LinkStatus::Disabled;
        sync_link_mapping_from_link(&db, &kv, &disabled_link).await?;
    }

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "User suspended successfully",
        "disabled_links": disabled_count
    }))?)
}

#[utoipa::path(
    put,
    path = "/api/admin/users/{id}/unsuspend",
    tag = "Admin",
    summary = "Unsuspend a user",
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = 200, description = "User unsuspended"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_unsuspend_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_unsuspend_user(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_unsuspend_user(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let target_user_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = UserRepository::new();
    repo.unsuspend(&db, &target_user_id).await?;

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "User unsuspended successfully"
    }))?)
}

#[utoipa::path(
    delete,
    path = "/api/admin/users/{id}",
    tag = "Admin",
    summary = "Delete a user",
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = 200, description = "User deleted"),
        (status = 400, description = "Cannot delete self or last org admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "User not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_delete_user(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_delete_user(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_delete_user(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let target_user_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?
        .to_string();

    if target_user_id == user_ctx.user_id {
        return Err(AppError::BadRequest(
            "Cannot delete your own account".to_string(),
        ));
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = UserRepository::new();

    let target_user = repo
        .get_user_by_id(&db, &target_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if target_user.role == "admin"
        && repo
            .is_last_admin_in_org(&db, &target_user_id, &target_user.org_id)
            .await?
    {
        return Err(AppError::BadRequest(
            "Cannot delete the last admin in an organization".to_string(),
        ));
    }

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    if body.get("confirmation").and_then(|c| c.as_str()) != Some("DELETE") {
        return Err(AppError::BadRequest(
            "Must provide 'confirmation': 'DELETE' in request body".to_string(),
        ));
    }

    let user_links = repo.get_links_by_creator(&db, &target_user_id).await?;
    let kv = ctx.kv("URL_MAPPINGS")?;
    for link in &user_links {
        kv::delete_link_mapping(&kv, &link.org_id, &link.short_code).await?;
    }

    let (user_count, links_count, analytics_count) = repo.delete(&db, &target_user_id).await?;

    if user_count == 0 {
        return Err(AppError::Internal("Failed to delete user".to_string()));
    }

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "User and all associated data deleted successfully",
        "deleted_user_count": user_count,
        "deleted_links_count": links_count,
        "deleted_analytics_count": analytics_count
    }))?)
}

#[utoipa::path(
    put,
    path = "/api/admin/users/{id}",
    tag = "Admin",
    summary = "Update user role",
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = 200, description = "Updated user"),
        (status = 400, description = "Invalid role or cannot demote last admin"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "User not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_update_user_role(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    Ok(inner_update_user_role(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_update_user_role(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let target_user_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing user ID".to_string()))?
        .to_string();

    if target_user_id == user_ctx.user_id {
        return Err(AppError::BadRequest(
            "Cannot modify your own role".to_string(),
        ));
    }

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let new_role = match body.get("role").and_then(|v| v.as_str()) {
        Some(role) if role == "admin" || role == "member" => role.to_string(),
        Some(_) => {
            return Err(AppError::BadRequest(
                "Role must be 'admin' or 'member'".to_string(),
            ));
        }
        None => return Err(AppError::BadRequest("Missing 'role' field".to_string())),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = UserRepository::new();

    let target_user = repo
        .get_user_by_id(&db, &target_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if new_role == "member" && target_user.role == "admin" {
        let admin_count = repo
            .admin_count(&db)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to verify admin count: {}", e)))?;
        if admin_count <= 1 {
            return Err(AppError::BadRequest(
                "Cannot demote the last admin user".to_string(),
            ));
        }
    }

    match repo.update_role(&db, &target_user_id, &new_role).await {
        Ok(()) => {
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
            let updated_user = repo
                .get_user_by_id(&db, &target_user_id)
                .await?
                .ok_or_else(|| AppError::Internal("Failed to retrieve updated user".to_string()))?;

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
            Err(AppError::Internal("Failed to update user role".to_string()))
        }
    }
}
