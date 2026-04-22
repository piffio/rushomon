use crate::services::LinkService;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/links/{id}",
    tag = "Links",
    summary = "Get a link",
    description = "Returns details for a single link belonging to the authenticated organization",
    params(
        ("id" = String, Path, description = "Link ID"),
    ),
    responses(
        (status = 200, description = "Link details", body = crate::models::Link),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Link not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_link(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_get_link(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_get_link(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, crate::utils::AppError> {
    let user_ctx = crate::auth::authenticate_request(&req, &ctx).await?;
    let org_id = &user_ctx.org_id;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| crate::utils::AppError::BadRequest("Missing link ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let service = LinkService::new();
    let link = service.get_link(&db, link_id, org_id).await?;

    match link {
        Some(link) => Ok(Response::from_json(&link)?),
        None => Err(crate::utils::AppError::NotFound(
            "Link not found".to_string(),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/api/links/by-code/{code}",
    tag = "Links",
    summary = "Get a link by short code",
    description = "Looks up a link by its short code instead of its internal ID",
    params(
        ("code" = String, Path, description = "Short code of the link"),
    ),
    responses(
        (status = 200, description = "Link details", body = crate::models::Link),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Link not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_link_by_code(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_get_link_by_code(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_get_link_by_code(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, crate::utils::AppError> {
    let user_ctx = crate::auth::authenticate_request(&req, &ctx).await?;
    let org_id = &user_ctx.org_id;

    let short_code = ctx
        .param("code")
        .ok_or_else(|| crate::utils::AppError::BadRequest("Missing short code".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let service = LinkService::new();
    let link = service.get_link_by_code(&db, short_code, org_id).await?;

    match link {
        Some(link) => Ok(Response::from_json(&link)?),
        None => Err(crate::utils::AppError::NotFound(
            "Link not found".to_string(),
        )),
    }
}
