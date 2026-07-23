/// Org domain verification handlers (JIT provisioning)
///
/// POST /api/orgs/{id}/org-domains - Add a domain verification challenge
/// GET /api/orgs/{id}/org-domains - List org domains
/// POST /api/orgs/{id}/verify-org-domain - Verify DNS ownership of a domain
/// DELETE /api/orgs/{id}/org-domains/{domain} - Remove a domain
use crate::auth;
use crate::services::OrgService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

fn org_id_param(ctx: &RouteContext<()>) -> Result<String, AppError> {
    ctx.param("id")
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))
}

async fn parse_domain_field(req: &mut Request) -> Result<String, AppError> {
    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;
    match body["domain"].as_str() {
        Some(d) if !d.trim().is_empty() => Ok(d.trim().to_lowercase()),
        _ => Err(AppError::BadRequest("Domain is required".to_string())),
    }
}

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/org-domains",
    tag = "Organizations",
    summary = "Add an org domain challenge",
    description = "Registers a domain for verification and returns the TXT record to publish. Requires owner or admin role and Business+ tier",
    params(("id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Challenge created; returns domain, token, and DNS instructions"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner/admin and Business+ required"),
        (status = 409, description = "Domain already verified by an organization"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_add_org_domain(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_add_org_domain(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_add_org_domain(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = org_id_param(&ctx)?;
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let domain = parse_domain_field(&mut req).await?;

    let (org_domain, verification_record) = OrgService::new()
        .add_domain_challenge(&db, &org_id, &user_ctx.user_id, &domain)
        .await?;

    let token = org_domain.verification_token.clone();
    let instructions = format!(
        "Add a TXT record to {} with the value: {}",
        org_domain.domain, verification_record
    );

    Response::from_json(&serde_json::json!({
        "domain": org_domain,
        "token": token,
        "verification_record": verification_record,
        "instructions": instructions,
    }))
    .map_err(AppError::from)
}

#[utoipa::path(
    get,
    path = "/api/orgs/{id}/org-domains",
    tag = "Organizations",
    summary = "List org domains",
    description = "Lists all domains for the organization. Any member may view. Pending domains include a best-effort is_cloudflare hint",
    params(("id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "List of org domains"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Organization not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_list_org_domains(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list_org_domains(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list_org_domains(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = org_id_param(&ctx)?;
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let domains = OrgService::new()
        .list_domains(&db, &org_id, &user_ctx.user_id)
        .await?;

    Response::from_json(&serde_json::json!({ "domains": domains })).map_err(AppError::from)
}

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/verify-org-domain",
    tag = "Organizations",
    summary = "Verify an org domain",
    description = "Checks the domain's TXT record and marks it verified on success. Requires owner or admin role and Business+ tier",
    params(("id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Domain verified"),
        (status = 400, description = "DNS record not found or incorrect"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner/admin and Business+ required, or domain belongs to another org"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_verify_org_domain(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_verify_org_domain(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_verify_org_domain(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = org_id_param(&ctx)?;
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let domain = parse_domain_field(&mut req).await?;

    OrgService::new()
        .verify_domain(&db, &org_id, &user_ctx.user_id, &domain)
        .await?;

    Response::ok("Domain verified successfully").map_err(AppError::from)
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/org-domains/{domain}",
    tag = "Organizations",
    summary = "Remove an org domain",
    description = "Removes a domain from the organization. Requires owner or admin role and Business+ tier",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("domain" = String, Path, description = "Domain to remove"),
    ),
    responses(
        (status = 200, description = "Domain removed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner/admin and Business+ required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_delete_org_domain(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_delete_org_domain(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_delete_org_domain(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = org_id_param(&ctx)?;
    let domain = ctx
        .param("domain")
        .ok_or_else(|| AppError::BadRequest("Missing domain".to_string()))?
        .to_string();
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    OrgService::new()
        .delete_domain(&db, &org_id, &user_ctx.user_id, &domain)
        .await?;

    Response::ok("Domain removed").map_err(AppError::from)
}
