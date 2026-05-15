/// POST /api/orgs/:id/domains
/// Add a custom domain to an organization (Pro+ only)
use crate::auth;
use crate::models::custom_domain::{CustomDomain, DnsInstructions, STATUS_PENDING};
use crate::repositories::{BillingRepository, CustomDomainRepository, OrgRepository};
use crate::services::OrgService;
use crate::utils::env::get_fallback_domain;
use crate::utils::{AppError, cf_saas};
use worker::d1::D1Database;
use worker::*;

pub async fn handle_create_domain(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
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

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    let hostname = body["hostname"]
        .as_str()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::BadRequest("hostname is required".to_string()))?;

    validate_hostname(&hostname)?;

    let org = OrgRepository::new()
        .get_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

    // Check tier allows custom domains
    let billing_repo = BillingRepository::new();
    let billing_account = billing_repo
        .get_for_org(&db, &org_id)
        .await?
        .ok_or_else(|| {
            AppError::Internal("No billing account found for organization".to_string())
        })?;

    let tier = crate::models::Tier::from_str_value(&billing_account.tier)
        .unwrap_or(crate::models::Tier::Free);
    let limits = tier.limits();

    match limits.max_custom_domains {
        Some(0) => {
            return Err(AppError::Forbidden(
                "Custom domains are not available on the Free plan. Upgrade to Pro or Business."
                    .to_string(),
            ));
        }
        Some(max) => {
            let current = CustomDomainRepository::new()
                .count_non_failed_for_org(&db, &org_id)
                .await
                .map_err(AppError::from)?;
            if current >= max {
                return Err(AppError::Forbidden(format!(
                    "Custom domain limit reached ({}/{}). Upgrade your plan to add more domains.",
                    current, max
                )));
            }
        }
        None => {}
    }

    // Check hostname is not already taken (globally)
    let existing = CustomDomainRepository::new()
        .get_by_hostname(&db, &hostname)
        .await
        .map_err(AppError::from)?;
    if existing.is_some() {
        return Err(AppError::BadRequest(format!(
            "'{}' is already registered as a custom domain.",
            hostname
        )));
    }

    // Register with Cloudflare for SaaS
    let cf_result = match cf_saas::create_custom_hostname(&ctx.env, &hostname).await {
        Ok(result) => result,
        Err(e) => {
            // Check if it's a quota error (Enterprise-only feature)
            let error_msg = e.to_string();
            if error_msg.contains("quota") || error_msg.contains("1404") {
                // CF for SaaS not available - fall back to dev/test mode
                None
            } else {
                return Err(AppError::Internal(format!(
                    "Failed to register domain with Cloudflare: {}",
                    e
                )));
            }
        }
    };

    let (cf_hostname_id, dns_instructions) = if let Some(cf) = cf_result {
        // Cloudflare for SaaS may return TXT records in two places:
        // 1. ownership_verification - for domain ownership (older method)
        // 2. ssl.validation_records - for ACME SSL challenges (newer method)
        let (txt_name, txt_value) = if let Some(records) = cf.ssl.validation_records {
            // Use ACME validation records if available
            if let Some(record) = records.first() {
                (record.txt_name.clone(), record.txt_value.clone())
            } else {
                (None, None)
            }
        } else {
            // Fall back to ownership_verification (older method)
            (
                cf.ownership_verification.as_ref().map(|v| v.name.clone()),
                cf.ownership_verification.as_ref().map(|v| v.value.clone()),
            )
        };
        let needs_txt = txt_name.is_some();
        let cname_target = get_fallback_domain(&ctx.env);
        let instructions = DnsInstructions {
            cname_target,
            txt_name,
            txt_value,
            needs_cname: true,
            needs_txt,
        };
        (Some(cf.id), instructions)
    } else {
        // CF for SaaS not configured (dev/test mode) — return stub instructions
        let cname_target = get_fallback_domain(&ctx.env);
        let instructions = DnsInstructions {
            cname_target,
            txt_name: None,
            txt_value: None,
            needs_cname: true,
            needs_txt: false,
        };
        (None, instructions)
    };

    let domain = CustomDomain {
        id: CustomDomain::generate_id(),
        org_id: org.id,
        hostname,
        status: STATUS_PENDING.to_string(),
        cf_hostname_id,
        created_at: crate::utils::now_timestamp(),
        verified_at: None,
    };

    CustomDomainRepository::new()
        .create(&db, &domain)
        .await
        .map_err(AppError::from)?;

    Ok(Response::from_json(&serde_json::json!({
        "domain": domain,
        "dns_instructions": dns_instructions,
    }))?)
}

/// Basic hostname validation: must look like a valid subdomain or domain
fn validate_hostname(hostname: &str) -> Result<(), AppError> {
    if hostname.is_empty() || hostname.len() > 253 {
        return Err(AppError::BadRequest("Invalid hostname length".to_string()));
    }
    // Must not start/end with hyphen or dot
    if hostname.starts_with('-')
        || hostname.ends_with('-')
        || hostname.starts_with('.')
        || hostname.ends_with('.')
    {
        return Err(AppError::BadRequest(
            "Hostname must not start or end with a hyphen or dot".to_string(),
        ));
    }
    // Must contain at least one dot (e.g. go.example.com)
    if !hostname.contains('.') {
        return Err(AppError::BadRequest(
            "Hostname must contain at least one dot (e.g. go.example.com)".to_string(),
        ));
    }
    // Only allow valid hostname characters
    if !hostname
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return Err(AppError::BadRequest(
            "Hostname contains invalid characters. Use only a-z, 0-9, hyphens, and dots."
                .to_string(),
        ));
    }
    Ok(())
}
