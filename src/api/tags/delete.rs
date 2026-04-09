/// DELETE /api/tags/:name
///
/// Delete a tag from the authenticated user's organization.
use crate::auth;
use crate::services::TagService;
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
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let tag_name = match ctx.param("name") {
        Some(name) => urlencoding::decode(name).unwrap_or_default().into_owned(),
        None => return Response::error("Missing tag name", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();

    match tag_service
        .delete_tag(&db, &user_ctx.org_id, &tag_name)
        .await
    {
        Ok(deleted) => {
            if deleted {
                Ok(Response::empty()?.with_status(204))
            } else {
                Response::error("Tag not found", 404)
            }
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "delete_tag_failed",
                    "org_id": user_ctx.org_id,
                    "tag_name": tag_name,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to delete tag", 500)
        }
    }
}
