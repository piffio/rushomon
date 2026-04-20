/// Org CRUD handlers
///
/// POST   /api/orgs      - Create organization
/// GET    /api/orgs/{id} - Get organization
/// PATCH  /api/orgs/{id} - Update organization
/// DELETE /api/orgs/{id} - Delete organization
use crate::auth;
use crate::models::Tier;
use crate::repositories::{BillingRepository, LinkRepository, OrgRepository};
use crate::services::OrgService;
use crate::utils::AppError;
use chrono::Datelike;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    post,
    path = "/api/orgs",
    tag = "Organizations",
    summary = "Create an organization",
    description = "Creates a new organization for the authenticated user. The user becomes the owner. Respects tier organization limits",
    responses(
        (status = 200, description = "New organization"),
        (status = 400, description = "Missing name"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Org limit reached for current tier"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_create_org(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_create_org(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_create_org(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();
    OrgService::new()
        .check_org_limit(&db, &user_ctx.user_id)
        .await?;

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    let name = match body["name"].as_str() {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => {
            return Err(AppError::BadRequest(
                "Organization name is required".to_string(),
            ));
        }
    };
    if name.len() > 100 {
        return Err(AppError::BadRequest(
            "Organization name must be 100 characters or less".to_string(),
        ));
    }

    // Create the org linked to the user's billing account
    let billing_account = crate::repositories::BillingRepository::new()
        .get_for_user(&db, &user_ctx.user_id)
        .await?
        .ok_or_else(|| AppError::Internal("No billing account found".to_string()))?;
    let org = repo
        .create_with_billing_account(&db, &name, &user_ctx.user_id, &billing_account.id)
        .await?;
    repo.add_member(&db, &org.id, &user_ctx.user_id, "owner")
        .await?;

    // Issue a new access token scoped to the new org
    let kv_store = ctx.kv("URL_MAPPINGS")?;
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let new_access_token = auth::session::create_access_token(
        &user_ctx.user_id,
        &org.id,
        &user_ctx.session_id,
        &user_ctx.role,
        &jwt_secret,
    )?;

    // Update session KV to the new org
    auth::session::store_session(&kv_store, &user_ctx.session_id, &user_ctx.user_id, &org.id)
        .await?;

    let domain = ctx
        .env
        .var("DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "localhost:8787".to_string());
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let access_cookie = auth::session::create_access_cookie_with_scheme(&new_access_token, scheme);

    let mut response = Response::from_json(&serde_json::json!({
        "org": org,
        "role": "owner",
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/orgs/{id}",
    tag = "Organizations",
    summary = "Get organization",
    description = "Returns org details including the member list with roles and pending invitations. The caller must be a member of the org",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Organization with members and invitations"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this org"),
        (status = 404, description = "Organization not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_get_org(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_get_org(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_get_org(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();

    // Verify user is a member of this org
    let member = repo.get_member(&db, &org_id, &user_ctx.user_id).await?;
    let member = match member {
        Some(m) => m,
        None => return Err(AppError::NotFound("Organization not found".to_string())),
    };

    let org = repo
        .get_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

    let members = repo.get_members(&db, &org_id).await?;

    // Get tier from billing account for API response
    let tier = OrgService::new().get_org_tier(&db, &org).await;

    // Owners and admins see pending invitations
    let pending_invitations = if member.role == "owner" || member.role == "admin" {
        repo.list_pending_invitations(&db, &org_id).await?
    } else {
        vec![]
    };

    let logo_url = repo.get_logo_url(&db, &org_id).await.unwrap_or(None);

    Ok(Response::from_json(&serde_json::json!({
        "org": {
            "id": org.id,
            "name": org.name,
            "tier": tier.as_str(),
            "created_at": org.created_at,
            "role": member.role,
            "logo_url": logo_url,
        },
        "members": members,
        "pending_invitations": pending_invitations,
    }))?)
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{id}",
    tag = "Organizations",
    summary = "Update organization",
    description = "Renames an organization. Requires owner or admin role within the org",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Updated organization"),
        (status = 400, description = "Missing or empty name"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required"),
        (status = 404, description = "Organization not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_update_org(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_update_org(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_update_org(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();

    // Verify user is an owner or admin of this org
    let member = repo.get_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => {
            return Err(AppError::Forbidden(
                "Only org owners and admins can rename the organization".to_string(),
            ));
        }
        None => return Err(AppError::NotFound("Organization not found".to_string())),
    }

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    let name = match body["name"].as_str() {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => {
            return Err(AppError::BadRequest(
                "Organization name is required".to_string(),
            ));
        }
    };
    if name.len() > 100 {
        return Err(AppError::BadRequest(
            "Organization name must be 100 characters or less".to_string(),
        ));
    }

    repo.update_name(&db, &org_id, &name).await?;

    let updated_org = repo
        .get_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| AppError::Internal("Organization not found after update".to_string()))?;

    // Get tier from billing account for API response
    let tier = OrgService::new().get_org_tier(&db, &updated_org).await;

    Ok(Response::from_json(&serde_json::json!({
        "org": {
            "id": updated_org.id,
            "name": updated_org.name,
            "tier": tier.as_str(),
            "created_at": updated_org.created_at,
            "billing_account_id": updated_org.billing_account_id,
        }
    }))?)
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}",
    tag = "Organizations",
    summary = "Delete an organization",
    description = "Permanently deletes an organization. Requires owner role and the user must belong to at least one other org. Links can either be migrated to another org or deleted",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Organization deleted"),
        (status = 400, description = "Cannot delete last org or invalid migration target"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner required"),
        (status = 404, description = "Organization not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_delete_org(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_delete_org(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_delete_org(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv = ctx.kv("URL_MAPPINGS")?;
    let repo = OrgRepository::new();

    // Verify user is an owner of this org
    let member = repo.get_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" => {}
        Some(_) => {
            return Err(AppError::Forbidden(
                "Only org owners can delete the organization".to_string(),
            ));
        }
        None => return Err(AppError::NotFound("Organization not found".to_string())),
    }

    // Verify user has multiple orgs
    let owned_orgs = repo.count_user_owned_orgs(&db, &user_ctx.user_id).await?;
    if owned_orgs <= 1 {
        return Err(AppError::BadRequest(
            "Cannot delete your only organization".to_string(),
        ));
    }

    // Parse request body for deletion options
    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    let action = body["action"].as_str().ok_or_else(|| {
        AppError::BadRequest("action is required (delete or migrate)".to_string())
    })?;

    let link_repo = LinkRepository::new();
    let billing_repo = BillingRepository::new();

    match action {
        "delete" => {
            // Get all link IDs and their short codes for KV cleanup
            let link_ids = repo.get_link_ids(&db, &org_id).await?;

            // Delete from KV first
            for link_id in &link_ids {
                if let Some(link) = link_repo.get_by_id(&db, link_id, &org_id).await? {
                    let _ = kv.delete(&link.short_code).await;
                }
            }

            // Hard delete links and analytics from D1
            repo.delete_all_links(&db, &org_id).await?;
        }
        "migrate" => {
            let target_org_id = body["target_org_id"].as_str().ok_or_else(|| {
                AppError::BadRequest("target_org_id is required when action is migrate".to_string())
            })?;

            // Verify user is owner of target org
            let target_member = repo
                .get_member(&db, target_org_id, &user_ctx.user_id)
                .await?;
            match &target_member {
                Some(m) if m.role == "owner" => {}
                Some(_) => {
                    return Err(AppError::Forbidden(
                        "You must be an owner of the target organization".to_string(),
                    ));
                }
                None => {
                    return Err(AppError::NotFound(
                        "Target organization not found".to_string(),
                    ));
                }
            }

            // Get target org and check capacity at billing account level
            let target_org = repo
                .get_by_id(&db, target_org_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Target organization not found".to_string()))?;

            let target_billing_account_id =
                target_org.billing_account_id.as_deref().ok_or_else(|| {
                    AppError::Internal("Target organization has no billing account".to_string())
                })?;
            let target_billing_account = billing_repo
                .get_by_id(&db, target_billing_account_id)
                .await?
                .ok_or_else(|| AppError::NotFound("Billing account not found".to_string()))?;

            let target_tier =
                Tier::from_str_value(&target_billing_account.tier).unwrap_or(Tier::Free);
            let target_limits = target_tier.limits();

            // Count links in source org
            let source_link_count = repo.count_links(&db, &org_id).await?;

            // Check if target billing account has enough capacity
            if let Some(max_links) = target_limits.max_links_per_month {
                let now = chrono::Utc::now();
                let year_month = format!("{}-{:02}", now.year(), now.month());
                let target_current_usage = billing_repo
                    .get_monthly_counter(&db, target_billing_account_id, &year_month)
                    .await?;

                let target_available = max_links - target_current_usage;

                if source_link_count > target_available {
                    return Err(AppError::BadRequest(format!(
                        "Target billing account has insufficient capacity. Available slots: {}, Required: {}",
                        target_available, source_link_count
                    )));
                }

                // Update target billing account's monthly counter
                billing_repo
                    .increment_monthly_counter(
                        &db,
                        target_billing_account_id,
                        &year_month,
                        source_link_count,
                    )
                    .await?;
            }

            // Migrate links in D1
            repo.migrate_links(&db, &org_id, target_org_id).await?;

            // Update KV mappings
            let migrated_link_ids = repo.get_link_ids(&db, target_org_id).await?;
            for link_id in &migrated_link_ids {
                if let Some(link) = link_repo.get_by_id(&db, link_id, target_org_id).await? {
                    let resolved_forward = link.forward_query_params.unwrap_or(false);
                    let mapping = link.to_mapping(resolved_forward);
                    if let Ok(put_builder) = kv.put(&link.short_code, mapping) {
                        let _ = put_builder.execute().await;
                    }
                }
            }
        }
        _ => {
            return Err(AppError::BadRequest(
                "Invalid action. Must be 'delete' or 'migrate'".to_string(),
            ));
        }
    }

    // Delete the organization
    repo.delete(&db, &org_id).await?;

    // Find another owned org to switch to
    let user_orgs = repo.get_user_orgs(&db, &user_ctx.user_id).await?;
    let fallback_org = user_orgs
        .into_iter()
        .find(|o| o.role == "owner" && o.id != org_id)
        .ok_or_else(|| AppError::Internal("No other owned organization found".to_string()))?;

    // Issue new access token scoped to the fallback org
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let new_access_token = auth::session::create_access_token(
        &user_ctx.user_id,
        &fallback_org.id,
        &user_ctx.session_id,
        &user_ctx.role,
        &jwt_secret,
    )?;

    auth::session::store_session(
        &kv,
        &user_ctx.session_id,
        &user_ctx.user_id,
        &fallback_org.id,
    )
    .await?;

    let domain = ctx
        .env
        .var("DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "localhost:8787".to_string());
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let access_cookie = auth::session::create_access_cookie_with_scheme(&new_access_token, scheme);

    let mut response = Response::from_json(&serde_json::json!({
        "success": true,
        "switched_to_org": {
            "id": fallback_org.id,
            "name": fallback_org.name,
        },
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}
