/// POST /api/reports/links — public abuse report submission
///
/// Accepts both authenticated and anonymous submissions.
/// Duplicate reports for the same link + reason + reporter within 24 h are rejected.
use crate::auth;
use crate::services::ReportService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    post,
    path = "/api/reports/links",
    tag = "Reports",
    summary = "Report a link for abuse",
    description = "Submits an abuse report for a link. Accepts both authenticated and anonymous submissions. Duplicate reports for the same link, reason, and reporter within 24 hours are rejected",
    responses(
        (status = 200, description = "Report submitted successfully"),
        (status = 400, description = "Missing required fields"),
        (status = 404, description = "Link not found or already removed"),
        (status = 422, description = "Link is already disabled"),
        (status = 429, description = "Duplicate report within 24 hours"),
    )
)]
pub async fn handle_report_link(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_report_link(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_report_link(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let link_id = body
        .get("link_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'link_id' field".to_string()))?;

    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'reason' field".to_string()))?;

    let reporter_email = body.get("reporter_email").and_then(|v| v.as_str());

    let (reporter_user_id, reporter_email_opt) = match auth::authenticate_request(&req, &ctx).await
    {
        Ok(user_ctx) => (
            Some(user_ctx.user_id),
            reporter_email.map(|s| s.to_string()),
        ),
        Err(_) => (None, reporter_email.map(|s| s.to_string())),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let report = ReportService::new()
        .submit_link_report(
            &db,
            link_id,
            reason,
            reporter_user_id.as_deref(),
            reporter_email_opt.as_deref(),
        )
        .await?;

    console_log!(
        "{}",
        serde_json::json!({
            "event": "abuse_report_stored",
            "link_id": report.link_id,
            "reason": report.reason,
            "level": "info"
        })
    );

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Report submitted successfully. Thank you for helping keep our platform safe."
    }))
    .map_err(|e| AppError::Internal(e.to_string()))
}
