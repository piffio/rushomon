use crate::auth;
use crate::kv;
use crate::repositories::LinkRepository;
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

    let Some(link) = link else {
        return Response::error("Link not found", 404);
    };

    repo.hard_delete(&db, link_id, org_id).await?;

    let kv_store = ctx.kv("URL_MAPPINGS")?;
    kv::delete_link_mapping(&kv_store, org_id, &link.short_code).await?;

    Response::empty()
}
