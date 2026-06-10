/// PATCH /api/tags/:name
///
/// Update a tag in the authenticated user's organization.
/// Can rename the tag and/or update its color.
use crate::auth;
use crate::services::TagService;
use crate::utils::AppError;
use serde::Deserialize;
use utoipa::ToSchema;
use worker::d1::D1Database;
use worker::*;

#[derive(Debug, Deserialize, ToSchema)]
struct UpdateTagRequest {
    new_name: Option<String>,
    color_index: Option<i32>,
}

#[utoipa::path(
    patch,
    path = "/api/tags/{name}",
    tag = "Tags",
    summary = "Update a tag",
    description = "Updates a tag's name and/or color in the authenticated organization. Returns the updated tag list",
    params(
        ("name" = String, Path, description = "Current tag name"),
    ),
    request_body = UpdateTagRequest,
    responses(
        (status = 200, description = "Updated tag list"),
        (status = 400, description = "Missing or invalid fields"),
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

    let tag_name = ctx
        .param("name")
        .map(|n| urlencoding::decode(n).unwrap_or_default().into_owned())
        .ok_or_else(|| AppError::BadRequest("Missing tag name".to_string()))?;

    let body: UpdateTagRequest = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let tag_service = TagService::new();

    // Validate that at least one field is being updated
    if body.new_name.is_none() && body.color_index.is_none() {
        return Err(AppError::BadRequest(
            "At least one of new_name or color_index must be provided".to_string(),
        ));
    }

    let tags = tag_service
        .update_tag(
            &db,
            &user_ctx.org_id,
            &tag_name,
            body.new_name,
            body.color_index,
        )
        .await?;

    Ok(Response::from_json(&tags)?)
}
