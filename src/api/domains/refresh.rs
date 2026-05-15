/// POST /api/orgs/:id/domains/:hostname/refresh
/// Poll CF for SaaS to update the status of a pending custom domain.
/// If the domain is now active, also syncs KV entries for all active links in the org.
use crate::auth;
use crate::models::custom_domain::{DnsInstructions, STATUS_ACTIVE, TxtRecord, TxtRecordPurpose};
use crate::repositories::CustomDomainRepository;
use crate::services::OrgService;
use crate::utils::env::get_fallback_domain;
use crate::utils::{AppError, cf_saas};
use worker::d1::D1Database;
use worker::*;

pub async fn handle_refresh_domain(req: Request, ctx: RouteContext<()>) -> Result<Response> {
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

    let cf_hostname_id = domain.cf_hostname_id.as_deref().ok_or_else(|| {
        AppError::BadRequest(
            "Domain has no Cloudflare record — it may have been registered without CF for SaaS."
                .to_string(),
        )
    })?;

    let cf_result = cf_saas::get_custom_hostname(&ctx.env, cf_hostname_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to poll Cloudflare: {}", e)))?;

    let new_status = if let Some(ref cf) = cf_result {
        match cf.status.as_str() {
            "active" => STATUS_ACTIVE,
            "pending" | "pending_validation" => "pending",
            _ => "failed",
        }
    } else {
        "pending"
    };

    // Persist updated status
    domain_repo
        .update_status(
            &db,
            &domain.id,
            new_status,
            cf_result.as_ref().map(|cf| cf.id.as_str()),
            if new_status == STATUS_ACTIVE {
                Some(crate::utils::now_timestamp())
            } else {
                None
            },
        )
        .await
        .map_err(AppError::from)?;

    // If domain just became active, sync KV for all active org links
    if new_status == STATUS_ACTIVE {
        sync_kv_for_domain(&ctx, &db, &org_id, &hostname).await?;
    }

    let updated = domain_repo
        .get_by_hostname_and_org(&db, &hostname, &org_id)
        .await
        .map_err(AppError::from)?;

    // Build dns_instructions with current SSL validation records (if still pending)
    let dns_instructions = if new_status != STATUS_ACTIVE {
        let mut txt_records: Vec<TxtRecord> = Vec::new();

        if let Some(ref cf) = cf_result {
            // Add ownership TXT (always present)
            if let Some(ref ownership) = cf.ownership_verification {
                txt_records.push(TxtRecord {
                    name: ownership.name.clone(),
                    value: ownership.value.clone(),
                    purpose: TxtRecordPurpose::Ownership,
                });
            }
            // Add SSL validation TXT records if certificate is pending
            if let Some(ref records) = cf.ssl.validation_records {
                for record in records {
                    if let (Some(name), Some(value)) =
                        (record.txt_name.clone(), record.txt_value.clone())
                    {
                        txt_records.push(TxtRecord {
                            name,
                            value,
                            purpose: TxtRecordPurpose::SslValidation,
                        });
                    }
                }
            }
        }

        let needs_txt = !txt_records.is_empty();
        Some(DnsInstructions {
            cname_target: get_fallback_domain(&ctx.env),
            txt_records,
            needs_cname: true,
            needs_txt,
        })
    } else {
        None
    };

    Ok(Response::from_json(&serde_json::json!({
        "domain": updated,
        "dns_instructions": dns_instructions,
    }))?)
}

/// Write {hostname}:{short_code} KV entries for all active links in the org.
async fn sync_kv_for_domain(
    ctx: &RouteContext<()>,
    db: &D1Database,
    org_id: &str,
    hostname: &str,
) -> Result<(), AppError> {
    use crate::repositories::OrgRepository;

    let org_repo = OrgRepository::new();
    let links = org_repo
        .get_links(db, org_id)
        .await
        .map_err(AppError::from)?;

    let kv = ctx.kv("URL_MAPPINGS")?;

    for link in links {
        if link.status == crate::models::link::LinkStatus::Active {
            let resolved_forward = link.forward_query_params.unwrap_or(false);
            let mapping = link.to_mapping(resolved_forward);
            let key = format!("{}:{}", hostname, link.short_code);
            kv.put(&key, &mapping)
                .map_err(|e| AppError::Internal(e.to_string()))?
                .execute()
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }

    Ok(())
}
