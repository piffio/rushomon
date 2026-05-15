/// GET /api/orgs/:id/domains
/// List all custom domains for an organization
use crate::auth;
use crate::repositories::CustomDomainRepository;
use crate::services::OrgService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

pub async fn handle_list_domains(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    OrgService::new()
        .require_owner_or_admin(&db, &org_id, &user_ctx.user_id, "Access denied")
        .await?;

    let domains = CustomDomainRepository::new()
        .get_by_org(&db, &org_id)
        .await
        .map_err(AppError::from)?;

    Ok(Response::from_json(&domains)?)
}
