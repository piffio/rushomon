/// POST /api/tags/merge
///
/// Merge multiple source tags into a destination tag.
/// All links using any of the source tags will be updated to use the destination tag.
/// Source tags are automatically deleted after merging.
use crate::auth;
use crate::services::TagService;
use crate::services::tag_service::{MergeResult, MergeTagsRequest};
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    post,
    path = "/api/tags/merge",
    tag = "Tags",
    summary = "Merge tags",
    description = "Merges multiple source tags into a destination tag. All links using source tags are updated to use the destination tag. Returns the number of affected links.",
    request_body = MergeTagsRequest,
    responses(
        (status = 200, description = "Tags merged successfully, returns merge result with affected link count"),
        (status = 400, description = "Invalid request - missing source tags or invalid tag names"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_merge_tags(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let request: MergeTagsRequest = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    // Validate request
    if request.source_tags.is_empty() {
        return Err(AppError::BadRequest(
            "At least one source tag is required".to_string(),
        ));
    }

    if request.destination_tag.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Destination tag cannot be empty".to_string(),
        ));
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();

    let result: MergeResult = tag_service
        .merge_tags(&db, &user_ctx.org_id, request)
        .await?;

    Ok(Response::from_json(&result)?)
}
