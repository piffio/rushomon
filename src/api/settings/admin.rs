/// Admin settings API handlers
///
/// GET /api/admin/settings and PUT /api/admin/settings
use crate::auth;
use crate::services::SettingsService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/admin/settings",
    tag = "Admin",
    summary = "Get system settings",
    responses(
        (status = 200, description = "Key-value map of all settings"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_get_settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_get(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_get(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let settings_service = SettingsService::new();
    let settings = settings_service.get_all_settings(&db).await?;

    let settings_map: serde_json::Map<String, serde_json::Value> = settings
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();

    Ok(Response::from_json(&serde_json::Value::Object(
        settings_map,
    ))?)
}

#[utoipa::path(
    put,
    path = "/api/admin/settings",
    tag = "Admin",
    summary = "Update a system setting",
    responses(
        (status = 200, description = "Updated settings map"),
        (status = 400, description = "Unknown or invalid setting value"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_update_setting(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_update(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_update(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    // Parse request body
    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let key = body
        .get("key")
        .and_then(|k| k.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'key' field".to_string()))?
        .to_string();

    let value = body
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'value' field".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let settings_service = SettingsService::new();

    let settings = settings_service.update_setting(&db, &key, &value).await?;

    let settings_map: serde_json::Map<String, serde_json::Value> = settings
        .into_iter()
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();

    Ok(Response::from_json(&serde_json::Value::Object(
        settings_map,
    ))?)
}
