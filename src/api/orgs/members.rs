/// Org member management handlers
///
/// DELETE /api/orgs/{id}/members/{user_id}      - Remove a member
/// PUT    /api/orgs/{id}/members/{user_id}/role  - Update a member's role
use crate::auth;
use crate::services::{ApiKeyService, OrgService};
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/members/{user_id}",
    tag = "Organizations",
    summary = "Remove a member",
    description = "Removes a user from the organization. Owners can remove anyone except the last owner. Admins can remove members but not owners. Any member can remove themselves",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("user_id" = String, Path, description = "User ID to remove"),
    ),
    responses(
        (status = 200, description = "Member removed"),
        (status = 400, description = "Cannot remove last owner"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Insufficient role"),
        (status = 404, description = "Member not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_remove_member(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_remove_member(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_remove_member(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();
    let target_user_id = ctx
        .param("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user_id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    OrgService::new()
        .remove_member(&db, &org_id, &user_ctx.user_id, &target_user_id)
        .await?;

    // Auto-revoke API keys scoped exclusively to this org for the removed user.
    if let Err(e) = ApiKeyService::new()
        .handle_user_removed_from_org(&db, &target_user_id, &org_id)
        .await
    {
        worker::console_log!(
            "{}",
            serde_json::json!({
                "event": "api_key_auto_revoke_failed",
                "user_id": target_user_id,
                "org_id": org_id,
                "error": e.to_string(),
                "level": "warn"
            })
        );
    }

    Ok(Response::ok("Member removed")?)
}

#[utoipa::path(
    put,
    path = "/api/orgs/{id}/members/{user_id}/role",
    tag = "Organizations",
    summary = "Update a member's role",
    description = "Changes a member's role within the organization. Owners can promote/demote any non-owner member. Admins can promote members to admin but cannot change the role of owners or other admins. Cannot change your own role. Role must be 'member' or 'admin' — ownership transfer is not supported via this endpoint.",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("user_id" = String, Path, description = "User ID whose role to update"),
    ),
    responses(
        (status = 200, description = "Role updated"),
        (status = 400, description = "Invalid role value"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Member not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_update_member_role(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_update_member_role(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_update_member_role(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();
    let target_user_id = ctx
        .param("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user_id".to_string()))?
        .to_string();

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    let new_role = body["role"]
        .as_str()
        .ok_or_else(|| AppError::BadRequest("'role' field is required".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    OrgService::new()
        .update_member_role(&db, &org_id, &user_ctx.user_id, &target_user_id, &new_role)
        .await?;

    Ok(Response::from_json(
        &serde_json::json!({ "role": new_role }),
    )?)
}
