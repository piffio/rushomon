/// GET /api/tags/analytics
///
/// Get comprehensive analytics for tags in the organization.
/// Returns statistics, top tags, unused tags, and similar tag suggestions.
use crate::auth;
use crate::services::TagService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/tags/analytics",
    tag = "Tags",
    summary = "Get tag analytics",
    description = "Returns comprehensive analytics for tags including total counts, top tags, unused tags, and similar tag suggestions for potential merges.",
    responses(
        (status = 200, description = "Tag analytics data"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_tag_analytics(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();

    let analytics = tag_service.get_tag_analytics(&db, &user_ctx.org_id).await?;

    Ok(Response::from_json(&analytics)?)
}
