/// PATCH /api/tags/:name
///
/// Rename a tag in the authenticated user's organization.
use crate::auth;
use crate::services::TagService;
use crate::utils::AppError;
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
pub async fn handle_rename_org_tag(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let old_name = ctx
        .param("name")
        .map(|n| urlencoding::decode(n).unwrap_or_default().into_owned())
        .ok_or_else(|| AppError::BadRequest("Missing tag name".to_string()))?;

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let new_name = body
        .get("new_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing new_name field".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();

    let tags = tag_service
        .rename_tag(&db, &user_ctx.org_id, &old_name, &new_name)
        .await?;

    Ok(Response::from_json(&tags)?)
}
