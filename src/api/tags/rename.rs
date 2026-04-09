/// PATCH /api/tags/:name
///
/// Rename a tag in the authenticated user's organization.
use crate::auth;
use crate::services::TagService;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    patch,
    path = "/api/tags/{name}",
    tag = "Tags",
    summary = "Rename a tag",
    description = "Renames a tag across all links in the authenticated organization. Returns the updated tag list",
    params(
        ("name" = String, Path, description = "Current tag name"),
    ),
    responses(
        (status = 200, description = "Updated tag list"),
        (status = 400, description = "Missing or invalid new tag name"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_rename_org_tag(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let old_name = match ctx.param("name") {
        Some(name) => urlencoding::decode(name).unwrap_or_default().into_owned(),
        None => return Response::error("Missing tag name", 400),
    };

    // Parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    let new_name = match body.get("new_name").and_then(|v| v.as_str()) {
        Some(name) => name.to_string(),
        None => return Response::error("Missing new_name field", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();

    match tag_service
        .rename_tag(&db, &user_ctx.org_id, &old_name, &new_name)
        .await
    {
        Ok(tags) => Response::from_json(&tags),
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "rename_tag_failed",
                    "org_id": user_ctx.org_id,
                    "old_name": old_name,
                    "new_name": new_name,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to rename tag", 500)
        }
    }
}
