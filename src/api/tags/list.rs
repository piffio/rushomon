/// GET /api/tags
///
/// Get all tags for the authenticated user's organization with usage counts.
use crate::auth;
use crate::services::TagService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/tags",
    tag = "Tags",
    summary = "Get organization tags",
    description = "Returns all tags for the authenticated user's organization with usage counts, sorted by count desc then name asc",
    responses(
        (status = 200, description = "List of tags with usage counts"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_org_tags(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();
    let tags = tag_service.get_org_tags(&db, &user_ctx.org_id).await?;
    Ok(Response::from_json(&tags)?)
}
