/// POST /api/tags
///
/// Create a new tag manually without associating it with a link.
/// This creates an orphaned tag that can be used later.
use crate::auth;
use crate::services::TagService;
use crate::utils::AppError;
use serde::Deserialize;
use utoipa::ToSchema;
use worker::d1::D1Database;
use worker::*;

#[derive(Debug, Deserialize, ToSchema)]
struct CreateTagRequest {
    name: String,
    color_index: Option<i32>,
}

#[utoipa::path(
    post,
    path = "/api/tags",
    tag = "Tags",
    summary = "Create a new tag",
    description = "Creates a new tag manually without associating it with a link. Returns the updated tag list.",
    request_body = CreateTagRequest,
    responses(
        (status = 201, description = "Tag created successfully, returns updated tag list"),
        (status = 200, description = "Tag already exists, returns current tag list"),
        (status = 400, description = "Invalid tag name"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_create_tag(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let body: CreateTagRequest = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();

    let (tags, created) = tag_service
        .create_tag(&db, &user_ctx.org_id, &body.name, body.color_index)
        .await?;

    let status = if created { 201 } else { 200 };
    Ok(Response::from_json(&tags)?.with_status(status))
}
