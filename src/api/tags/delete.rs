/// DELETE /api/tags/:name
///
/// Delete a tag from the authenticated user's organization.
use crate::auth;
use crate::services::TagService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    delete,
    path = "/api/tags/{name}",
    tag = "Tags",
    summary = "Delete a tag",
    description = "Deletes a tag from all links in the authenticated organization. Returns 204 No Content on success",
    params(
        ("name" = String, Path, description = "Tag name to delete"),
    ),
    responses(
        (status = 204, description = "Tag deleted successfully"),
        (status = 400, description = "Missing tag name"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tag not found"),
        (status = 500, description = "Failed to delete tag"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_delete_org_tag(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let tag_name = ctx
        .param("name")
        .map(|n| urlencoding::decode(n).unwrap_or_default().into_owned())
        .ok_or_else(|| AppError::BadRequest("Missing tag name".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();

    let deleted = tag_service
        .delete_tag(&db, &user_ctx.org_id, &tag_name)
        .await?;

    if deleted {
        Ok(Response::empty()?.with_status(204))
    } else {
        Err(AppError::NotFound("Tag not found".to_string()))
    }
}
