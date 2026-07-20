/// Account deletion handlers
///
/// POST /api/auth/delete-account  — schedule account deletion (7-day grace period)
/// POST /api/auth/cancel-deletion — cancel pending account deletion
/// GET  /api/auth/deletion-status — check if deletion is pending
use crate::auth;
use crate::services::AccountDeletionService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

/// Request body for account deletion
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct DeleteAccountRequest {
    /// Must be the string "DELETE" to confirm
    pub confirmation: String,
}

/// Response after scheduling account deletion
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct DeleteAccountResponse {
    pub success: bool,
    pub message: String,
    /// Unix timestamp when the account will be permanently deleted
    pub scheduled_deletion_at: i64,
    pub grace_period_seconds: i64,
}

/// Response for deletion status check
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct DeletionStatusResponse {
    pub pending: bool,
    /// Unix timestamp when deletion is scheduled (null if not pending)
    pub scheduled_deletion_at: Option<i64>,
    /// Days remaining until deletion (null if not pending)
    pub days_remaining: Option<i64>,
}

// ── POST /api/auth/delete-account ──────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/auth/delete-account",
    tag = "Authentication",
    summary = "Schedule account deletion",
    description = "Schedules the authenticated user's account for deletion after a 7-day grace period. \
                   Requires confirmation string 'DELETE' in the request body. \
                   Blocks deletion if the user is the last owner of any organization.",
    request_body = DeleteAccountRequest,
    responses(
        (status = 200, description = "Deletion scheduled", body = DeleteAccountResponse),
        (status = 400, description = "Invalid confirmation or blocked by org ownership"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_delete_account(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_delete_account(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_delete_account(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    if body.get("confirmation").and_then(|c| c.as_str()) != Some("DELETE") {
        return Err(AppError::BadRequest(
            "Must provide 'confirmation': 'DELETE' in request body".to_string(),
        ));
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let result = AccountDeletionService::new()
        .request_deletion(&db, &user_ctx.user_id)
        .await?;

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Account deletion scheduled. You have 7 days to cancel.",
        "scheduled_deletion_at": result.scheduled_deletion_at,
        "grace_period_seconds": result.grace_period_seconds
    }))?)
}

// ── POST /api/auth/cancel-deletion ─────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/auth/cancel-deletion",
    tag = "Authentication",
    summary = "Cancel pending account deletion",
    description = "Cancels a previously scheduled account deletion. No-op if no deletion is pending.",
    responses(
        (status = 200, description = "Deletion cancelled"),
        (status = 400, description = "No pending deletion"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_cancel_deletion(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_cancel_deletion(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_cancel_deletion(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    AccountDeletionService::new()
        .cancel_deletion(&db, &user_ctx.user_id)
        .await?;

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Account deletion cancelled."
    }))?)
}

// ── GET /api/auth/deletion-status ──────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/auth/deletion-status",
    tag = "Authentication",
    summary = "Check account deletion status",
    description = "Returns whether the authenticated user has a pending account deletion and the scheduled date.",
    responses(
        (status = 200, description = "Deletion status", body = DeletionStatusResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_deletion_status(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_deletion_status(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_deletion_status(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = crate::repositories::UserRepository::new();
    let user = repo
        .get_user_by_id(&db, &user_ctx.user_id)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to load user: {}", e)))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let pending = user.pending_deletion_at.is_some();
    let scheduled_deletion_at = user.pending_deletion_at;
    let days_remaining = user.pending_deletion_at.map(|ts| {
        let now = crate::utils::now_timestamp();
        let seconds_remaining = (ts - now).max(0);
        (seconds_remaining + 86399) / 86400 // round up to whole days
    });

    Ok(Response::from_json(&serde_json::json!({
        "pending": pending,
        "scheduled_deletion_at": scheduled_deletion_at,
        "days_remaining": days_remaining
    }))?)
}
