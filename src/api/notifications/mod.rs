/// Notification preference API handlers
///
/// GET  /api/notifications/preferences  — return the current user's preferences
/// PATCH /api/notifications/preferences — update the current user's preferences
/// POST /api/admin/cron/trigger-monthly-stats — manually trigger the monthly stats email job
use crate::auth::{authenticate_request, require_admin};
use crate::repositories::notification_preferences_repository::{
    NotificationPreferences, NotificationPreferencesRepository,
};
use serde::Deserialize;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/notifications/preferences",
    tag = "Notifications",
    summary = "Get notification preferences",
    description = "Returns the current user's email notification preferences. Missing preferences default to all-enabled.",
    responses(
        (status = 200, description = "Notification preferences"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_notification_preferences(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;
    let prefs = NotificationPreferencesRepository::new()
        .get_by_user_id(&db, &user_ctx.user_id)
        .await?;

    Response::from_json(&prefs)
}

#[derive(Deserialize)]
pub struct UpdateNotificationPreferencesRequest {
    pub email_monthly_stats: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/notifications/preferences",
    tag = "Notifications",
    summary = "Update notification preferences",
    description = "Updates one or more email notification preference flags for the authenticated user.",
    request_body(
        content = inline(serde_json::Value),
        description = r#"Fields to update, e.g. `{"email_monthly_stats": false}`"#
    ),
    responses(
        (status = 200, description = "Updated preferences"),
        (status = 400, description = "Invalid request body"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_update_notification_preferences(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let body: UpdateNotificationPreferencesRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;
    let repo = NotificationPreferencesRepository::new();

    // Load existing (or default) preferences, then apply the patch
    let current = repo.get_by_user_id(&db, &user_ctx.user_id).await?;

    let updated = NotificationPreferences {
        email_monthly_stats: body
            .email_monthly_stats
            .unwrap_or(current.email_monthly_stats),
    };

    let saved = repo.upsert(&db, &user_ctx.user_id, &updated).await?;
    Response::from_json(&saved)
}

#[utoipa::path(
    post,
    path = "/api/admin/cron/trigger-monthly-stats",
    tag = "Admin",
    summary = "Trigger monthly stats email job",
    description = "Manually triggers the monthly statistics summary email job. Sends emails to all opted-in users.",
    responses(
        (status = 200, description = "Email job completed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_cron_trigger_monthly_stats(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[cron-trigger] DB binding unavailable: {}", e);
            return Response::error("Service temporarily unavailable", 503);
        }
    };

    console_log!("[cron-trigger] Manually triggering monthly stats email job");
    let (sent, errors) =
        crate::services::email_notification_service::send_monthly_stats_to_all_users(&db, &ctx.env)
            .await;

    Response::from_json(&serde_json::json!({
        "sent": sent,
        "errors": errors
    }))
}
