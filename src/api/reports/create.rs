/// POST /api/reports/links — public abuse report submission
///
/// Accepts both authenticated and anonymous submissions.
/// Duplicate reports for the same link + reason + reporter within 24 h are rejected.
use crate::auth;
use crate::models::link::LinkStatus;
use crate::repositories::{LinkRepository, ReportRepository};
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
pub async fn handle_report_link(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: serde_json::Value = match req.json().await {
        Ok(body) => body,
        Err(e) => return Response::error(format!("Invalid JSON: {}", e), 400),
    };

    let link_id = match body.get("link_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Response::error("Missing 'link_id' field", 400),
    };

    let reason = match body.get("reason").and_then(|v| v.as_str()) {
        Some(r) => r.to_string(),
        None => return Response::error("Missing 'reason' field", 400),
    };

    let reporter_email = body.get("reporter_email").and_then(|v| v.as_str());

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let link_repo = LinkRepository::new();

    // Validate link exists and check status
    let link = match link_repo.get_active_by_short_code(&db, &link_id).await {
        Ok(Some(link)) => Some(link),
        Ok(None) => match link_repo.get_by_id_no_auth_all(&db, &link_id).await {
            Ok(Some(link)) => Some(link),
            Ok(None) => {
                return Ok(Response::from_json(&serde_json::json!({
                    "success": false,
                    "message": "This link doesn't exist or has been removed.",
                    "error_type": "link_not_found"
                }))?
                .with_status(404));
            }
            Err(e) => return Response::error(format!("Database error: {}", e), 500),
        },
        Err(e) => return Response::error(format!("Database error: {}", e), 500),
    };

    let link_ref = match link {
        Some(l) => l,
        None => {
            return Ok(Response::from_json(&serde_json::json!({
                "success": false,
                "message": "This link doesn't exist or has been removed.",
                "error_type": "link_not_found"
            }))?
            .with_status(404));
        }
    };

    if matches!(link_ref.status, LinkStatus::Blocked | LinkStatus::Disabled) {
        return Ok(Response::from_json(&serde_json::json!({
            "success": false,
            "message": "This link has already been disabled and cannot be reported.",
            "error_type": "link_already_disabled"
        }))?
        .with_status(422));
    }

    let (reporter_user_id, reporter_email_opt) = match auth::authenticate_request(&req, &ctx).await
    {
        Ok(user_ctx) => (
            Some(user_ctx.user_id),
            reporter_email.map(|s| s.to_string()),
        ),
        Err(_) => (None, reporter_email.map(|s| s.to_string())),
    };

    let actual_link_id = link_ref.id.clone();
    let repo = ReportRepository::new();

    if repo
        .is_duplicate(
            &db,
            &actual_link_id,
            &reason,
            reporter_user_id.as_deref(),
            reporter_email_opt.as_deref(),
        )
        .await
        .unwrap_or(false)
    {
        return Response::error(
            "You have already reported this link for the same reason within the last 24 hours",
            429,
        );
    }

    match repo
        .create(
            &db,
            &actual_link_id,
            &reason,
            reporter_user_id.as_deref(),
            reporter_email_opt.as_deref(),
        )
        .await
    {
        Ok(_) => {
            let reporter_user_id_hash = reporter_user_id.as_ref().map(|id| {
                use hex;
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(id.as_bytes());
                hex::encode(hasher.finalize())
            });
            let reporter_email_hash = reporter_email_opt.as_ref().map(|email| {
                use hex;
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(email.as_bytes());
                hex::encode(hasher.finalize())
            });

            console_log!(
                "{}",
                serde_json::json!({
                    "event": "abuse_report_stored",
                    "link_id": actual_link_id,
                    "reason": reason,
                    "reporter_user_id_hash": reporter_user_id_hash,
                    "reporter_email_hash": reporter_email_hash,
                    "level": "info"
                })
            );

            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Report submitted successfully. Thank you for helping keep our platform safe."
            }))
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "abuse_report_failed",
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to store report. Please try again.", 500)
        }
    }
}
