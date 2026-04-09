/// GET /api/tags
///
/// Get all tags for the authenticated user's organization with usage counts.
use crate::auth;
use crate::services::TagService;
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
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();
    let tags = tag_service.get_org_tags(&db, org_id).await?;

    Response::from_json(&tags)
}
