/// Org settings handlers
///
/// GET  /api/orgs/{id}/settings - Get org settings
/// PATCH /api/orgs/{id}/settings - Update org settings
use crate::auth;
use crate::models::Tier;
use crate::repositories::OrgRepository;
use crate::services::OrgService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/orgs/{id}/settings",
    tag = "Organizations",
    summary = "Get org settings",
    description = "Returns organization-level settings. The forward_query_params setting is only available on Pro+ tiers",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Org settings"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this org"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_get_org_settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_get_org_settings(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_get_org_settings(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();

    // Only members of this org can read settings
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();
    if repo
        .get_member(&db, &org_id, &user_ctx.user_id)
        .await?
        .is_none()
    {
        return Err(AppError::NotFound("Organization not found".to_string()));
    }

    let forward_query_params = repo.get_forward_query_params(&db, &org_id).await?;

    Ok(Response::from_json(&serde_json::json!({
        "forward_query_params": forward_query_params
    }))?)
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{id}/settings",
    tag = "Organizations",
    summary = "Update org settings",
    description = "Updates organization-level settings. forward_query_params requires Pro+ tier. Caller must be owner or admin",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Updated settings"),
        (status = 400, description = "Invalid setting value"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required, or Pro+ required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_update_org_settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_update_org_settings(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_update_org_settings(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();

    // Verify owner or admin
    let member = repo.get_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => {
            return Err(AppError::Forbidden(
                "Only org owners and admins can change organization settings".to_string(),
            ));
        }
        None => return Err(AppError::NotFound("Organization not found".to_string())),
    }

    // Tier check: forward_query_params is Pro+ only
    let org = repo
        .get_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| AppError::Internal("Organization not found".to_string()))?;
    let tier = OrgService::new().get_org_tier(&db, &org).await;
    let is_pro_or_above = matches!(tier, Tier::Pro | Tier::Business | Tier::Unlimited);

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    if let Some(forward) = body["forward_query_params"].as_bool() {
        if forward && !is_pro_or_above {
            return Err(AppError::Forbidden(
                "Query parameter forwarding requires a Pro plan or above.".to_string(),
            ));
        }
        repo.set_forward_query_params(&db, &org_id, forward).await?;
    } else {
        return Err(AppError::BadRequest(
            "forward_query_params (boolean) is required".to_string(),
        ));
    }

    let updated_forward = repo.get_forward_query_params(&db, &org_id).await?;

    Ok(Response::from_json(&serde_json::json!({
        "forward_query_params": updated_forward
    }))?)
}
