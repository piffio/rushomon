/// Admin counter reset handlers
///
/// POST /api/admin/billing-accounts/{id}/reset-counter — Reset monthly counter for a billing account
use crate::auth;
use crate::db;
use crate::utils::AppError;
use chrono::Datelike;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    post,
    path = "/api/admin/billing-accounts/{id}/reset-counter",
    tag = "Admin",
    summary = "Reset billing account monthly counter",
    params(("id" = String, Path, description = "Billing Account ID")),
    responses(
        (status = 200, description = "Counter reset"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_reset_monthly_counter(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    Ok(inner_handle_admin_reset_monthly_counter(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_handle_admin_reset_monthly_counter(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    console_log!(
        "{}",
        serde_json::json!({
            "event": "admin_reset_counter_called",
            "level": "info"
        })
    );

    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx).map_err(AppError::from)?;

    let billing_account_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing billing account ID".to_string()))?;

    let db = ctx
        .env
        .get_binding::<D1Database>("rushomon")
        .map_err(|_| AppError::Internal("Database not available".to_string()))?;

    let now = chrono::Utc::now();
    let year_month = format!("{}-{:02}", now.year(), now.month());

    db::reset_monthly_counter_for_billing_account(&db, billing_account_id, &year_month)
        .await
        .map_err(|e| {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "admin_reset_counter_failed",
                    "billing_account_id": billing_account_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            AppError::Internal("Failed to reset monthly counter".to_string())
        })?;

    console_log!(
        "{}",
        serde_json::json!({
            "event": "admin_reset_counter_success",
            "billing_account_id": billing_account_id,
            "level": "info"
        })
    );
    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Monthly counter reset for billing account"
    }))?)
}
