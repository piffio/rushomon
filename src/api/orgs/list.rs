/// Org list and switch handlers
///
/// GET  /api/orgs           - List user organizations
/// POST /api/auth/switch-org - Switch active organization
use crate::auth;
use crate::models::Tier;
use crate::repositories::{BillingRepository, OrgRepository};
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

/// Helper to get effective tier for an organization
async fn get_org_tier(db: &D1Database, org: &crate::models::Organization) -> Tier {
    let billing_repo = BillingRepository::new();
    if let Some(ref billing_account_id) = org.billing_account_id
        && let Ok(Some(billing_account)) = billing_repo.get_by_id(db, billing_account_id).await
    {
        return Tier::from_str_value(&billing_account.tier).unwrap_or(Tier::Free);
    }
    Tier::Free
}

#[utoipa::path(
    get,
    path = "/api/orgs",
    tag = "Organizations",
    summary = "List user organizations",
    description = "Returns all organizations the authenticated user belongs to, including their role in each org and the org's current billing tier. Also returns the active org_id from the current session",
    responses(
        (status = 200, description = "List of organizations with role and tier info"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_list_user_orgs(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list_user_orgs(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list_user_orgs(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();
    let orgs = repo.get_user_orgs(&db, &user_ctx.user_id).await?;

    // Add tier information to each org by looking up billing account
    let billing_repo = BillingRepository::new();
    let mut orgs_with_tier = Vec::new();
    for org in orgs {
        let tier = if let Some(org_details) = repo.get_by_id(&db, &org.id).await? {
            if let Some(ref billing_account_id) = org_details.billing_account_id {
                if let Ok(Some(billing_account)) =
                    billing_repo.get_by_id(&db, billing_account_id).await
                {
                    billing_account.tier
                } else {
                    "free".to_string()
                }
            } else {
                "free".to_string()
            }
        } else {
            "free".to_string()
        };

        orgs_with_tier.push(serde_json::json!({
            "id": org.id,
            "name": org.name,
            "tier": tier,
            "role": org.role,
            "joined_at": org.joined_at,
        }));
    }

    Ok(Response::from_json(&serde_json::json!({
        "orgs": orgs_with_tier,
        "current_org_id": user_ctx.org_id,
    }))?)
}

#[utoipa::path(
    post,
    path = "/api/auth/switch-org",
    tag = "Organizations",
    summary = "Switch active organization",
    description = "Switches the authenticated user's active organization context. Re-issues a new access token (and refresh token) scoped to the chosen org. The user must be a member of the target org",
    responses(
        (status = 200, description = "Switched, new access token set in cookies"),
        (status = 400, description = "Missing or invalid org_id"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of the target org"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_switch_org(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_switch_org(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_switch_org(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    let target_org_id = match body["org_id"].as_str() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return Err(AppError::BadRequest("org_id is required".to_string())),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();

    // Verify the user is actually a member of the target org
    let member = repo
        .get_member(&db, &target_org_id, &user_ctx.user_id)
        .await?;
    if member.is_none() {
        return Err(AppError::Forbidden(
            "You are not a member of this organization".to_string(),
        ));
    }

    let org = repo
        .get_by_id(&db, &target_org_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

    let kv = ctx.kv("URL_MAPPINGS")?;
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();

    let new_access_token = auth::session::create_access_token(
        &user_ctx.user_id,
        &target_org_id,
        &user_ctx.session_id,
        &user_ctx.role,
        &jwt_secret,
    )?;

    // Update session KV to reflect the new active org
    auth::session::store_session(&kv, &user_ctx.session_id, &user_ctx.user_id, &target_org_id)
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

    let member_info = member.unwrap();

    // Get tier from billing account for API response
    let tier = get_org_tier(&db, &org).await;

    let mut response = Response::from_json(&serde_json::json!({
        "org": {
            "id": org.id,
            "name": org.name,
            "tier": tier.as_str(),
            "role": member_info.role,
        },
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}
