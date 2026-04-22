use crate::services::LinkService;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    delete,
    path = "/api/links/{id}",
    tag = "Links",
    summary = "Delete a link",
    description = "Soft-deletes a link belonging to the authenticated organization and removes its KV mapping so it stops redirecting",
    params(
        ("id" = String, Path, description = "Link ID"),
    ),
    responses(
        (status = 200, description = "Link deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Link not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_delete_link(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_delete_link(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_delete_link(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, crate::utils::AppError> {
    let user_ctx = crate::auth::authenticate_request(&req, &ctx).await?;
    let org_id = &user_ctx.org_id;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| crate::utils::AppError::BadRequest("Missing link ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv_store = ctx.kv("URL_MAPPINGS")?;
    let service = LinkService::new();

    service.delete_link(&db, &kv_store, link_id, org_id).await?;

    Ok(Response::empty()?)
}
