/// Org member management handlers
///
/// DELETE /api/orgs/{id}/members/{user_id} - Remove a member
use crate::auth;
use crate::services::OrgService;
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

    Ok(Response::ok("Member removed")?)
}
