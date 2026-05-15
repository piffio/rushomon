/// DELETE /api/orgs/:id/domains/:hostname
/// Remove a custom domain from an organization
use crate::auth;
use crate::repositories::CustomDomainRepository;
use crate::services::OrgService;
use crate::utils::{AppError, cf_saas};
use worker::d1::D1Database;
use worker::*;

pub async fn handle_delete_domain(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();

    let hostname = ctx
        .param("hostname")
        .ok_or_else(|| AppError::BadRequest("Missing hostname".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    OrgService::new()
        .require_owner_or_admin(
            &db,
            &org_id,
            &user_ctx.user_id,
            "Only org owners and admins can manage custom domains",
        )
        .await?;

    let domain_repo = CustomDomainRepository::new();
    let domain = domain_repo
        .get_by_hostname_and_org(&db, &hostname, &org_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound("Custom domain not found".to_string()))?;

    // Delete from Cloudflare for SaaS (if registered)
    if let Some(ref cf_id) = domain.cf_hostname_id {
        cf_saas::delete_custom_hostname(&ctx.env, cf_id)
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to remove domain from Cloudflare: {}", e))
            })?;
    }

    // Clean up all KV entries for this hostname
    let kv = ctx.kv("URL_MAPPINGS")?;
    let short_codes_result = db
        .prepare("SELECT short_code FROM links WHERE org_id = ?1 AND status = 'active'")
        .bind(&[org_id.as_str().into()])
        .map_err(|e| AppError::Internal(e.to_string()))?
        .all()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .results::<serde_json::Value>()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    for row in &short_codes_result {
        if let Some(sc) = row["short_code"].as_str() {
            let key = format!("{}:{}", hostname, sc);
            let _ = kv.delete(&key).await;
        }
    }

    // Delete from D1
    domain_repo
        .delete(&db, &domain.id, &org_id)
        .await
        .map_err(AppError::from)?;

    Ok(Response::from_json(
        &serde_json::json!({ "deleted": true }),
    )?)
}
