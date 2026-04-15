use crate::auth;
use crate::repositories::LinkRepository;
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
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing link ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = LinkRepository::new();
    let link = repo.get_by_id(&db, link_id, org_id).await?;

    match link {
        Some(mut link) => {
            link.tags = repo.get_tags(&db, &link.id).await?;
            Response::from_json(&link)
        }
        None => Response::error("Link not found", 404),
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
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let short_code = ctx
        .param("code")
        .ok_or_else(|| Error::RustError("Missing short code".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = LinkRepository::new();
    let link = repo.get_by_short_code(&db, short_code, org_id).await?;

    match link {
        Some(link) => Response::from_json(&link),
        None => Response::error("Link not found", 404),
    }
}
