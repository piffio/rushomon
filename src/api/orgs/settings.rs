/// Org settings handlers
///
/// GET  /api/orgs/{id}/settings - Get org settings
/// PATCH /api/orgs/{id}/settings - Update org settings
use crate::auth;
use crate::services::OrgService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/orgs/{id}/settings",
    tag = "Organizations",
    summary = "Get org settings",
    description = "Returns organization-level settings (forward_query_params, exclude_ambiguous_chars). The forward_query_params setting is only available on Pro+ tiers",
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

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let settings = OrgService::new()
        .get_org_settings(&db, &org_id, &user_ctx.user_id)
        .await?;

    Ok(Response::from_json(&settings)?)
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{id}/settings",
    tag = "Organizations",
    summary = "Update org settings",
    description = "Updates organization-level settings. Omitted fields are unchanged. Enabling forward_query_params requires Pro+ tier. Caller must be owner or admin",
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

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON body".to_string()))?;

    let forward = match body.get("forward_query_params") {
        None => None,
        Some(v) => Some(v.as_bool().ok_or_else(|| {
            AppError::BadRequest("forward_query_params must be a boolean".to_string())
        })?),
    };
    let exclude_ambiguous = match body.get("exclude_ambiguous_chars") {
        None => None,
        Some(v) => Some(v.as_bool().ok_or_else(|| {
            AppError::BadRequest("exclude_ambiguous_chars must be a boolean".to_string())
        })?),
    };

    if forward.is_none() && exclude_ambiguous.is_none() {
        return Err(AppError::BadRequest(
            "At least one setting (forward_query_params, exclude_ambiguous_chars) is required"
                .to_string(),
        ));
    }

    let updated = OrgService::new()
        .update_org_settings(&db, &org_id, &user_ctx.user_id, forward, exclude_ambiguous)
        .await?;

    Ok(Response::from_json(&updated)?)
}
