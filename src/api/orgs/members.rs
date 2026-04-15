/// Org member management handlers
///
/// DELETE /api/orgs/{id}/members/{user_id} - Remove a member
use crate::auth;
use crate::repositories::OrgRepository;
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
    let repo = OrgRepository::new();

    let requester_member = repo.get_member(&db, &org_id, &user_ctx.user_id).await?;
    let requester = match requester_member {
        Some(m) => m,
        None => return Err(AppError::NotFound("Organization not found".to_string())),
    };

    let is_self_removal = target_user_id == user_ctx.user_id;

    // Check the target is actually in this org
    let target_member = repo.get_member(&db, &org_id, &target_user_id).await?;
    if target_member.is_none() {
        return Err(AppError::NotFound(
            "Member not found in this organization".to_string(),
        ));
    }
    let target_role = target_member.unwrap().role;

    // Permission checks for removing others
    if !is_self_removal {
        match requester.role.as_str() {
            "owner" => {
                // Owners can remove anyone
            }
            "admin" => {
                // Admins can remove members but not owners or other admins
                if target_role == "owner" {
                    return Err(AppError::Forbidden(
                        "Admins cannot remove owners".to_string(),
                    ));
                }
                if target_role == "admin" {
                    return Err(AppError::Forbidden(
                        "Admins cannot remove other admins".to_string(),
                    ));
                }
            }
            _ => {
                // Regular members can only remove themselves
                return Err(AppError::Forbidden(
                    "Only org owners and admins can remove members".to_string(),
                ));
            }
        }
    }

    // Prevent removing the last owner
    if target_role == "owner" {
        let owner_count = repo.count_owners(&db, &org_id).await?;
        if owner_count <= 1 {
            return Err(AppError::BadRequest(
                "Cannot remove the last owner. Transfer ownership first.".to_string(),
            ));
        }
    }

    repo.remove_member(&db, &org_id, &target_user_id).await?;

    Ok(Response::ok("Member removed")?)
}
