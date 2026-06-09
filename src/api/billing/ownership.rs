/// Billing Account Ownership Transfer API Handlers
///
/// Endpoints:
///   POST   /api/billing/transfer                 — initiate (BA owner only)
///   DELETE /api/billing/transfer                 — cancel   (BA owner only)
///   GET    /api/billing-transfer/:token          — public info page
///   POST   /api/billing-transfer/:token/accept   — accept   (authenticated, email must match)
///   POST   /api/admin/billing-accounts/:id/transfer — admin force-transfer
use crate::auth;
use crate::services::ownership_transfer_service::{OwnershipTransferService, TransferError};
use worker::d1::D1Database;
use worker::*;

// ─── Helper ───────────────────────────────────────────────────────────────────

fn transfer_error_response(e: TransferError) -> worker::Result<Response> {
    Response::error(e.message().to_string(), e.status_code())
}

// ─── Initiate transfer ────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/billing/transfer",
    tag = "Billing",
    summary = "Initiate billing account ownership transfer",
    description = "Sends a confirmation email to the target user. The target must be an existing \
                   member of one of the organizations in the billing account. Any previous pending \
                   transfer for this billing account is cancelled first.",
    request_body(
        content_type = "application/json",
        description = "Email of the intended new owner",
        content = serde_json::Value
    ),
    responses(
        (status = 200, description = "Transfer initiated, email sent"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Billing account not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_initiate_transfer(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    #[derive(serde::Deserialize)]
    struct Body {
        to_email: String,
        /// The billing account to transfer. Optional: if omitted, the user's own BA is looked up.
        billing_account_id: Option<String>,
    }

    let body: Body = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    let to_email = body.to_email.trim().to_lowercase();
    if to_email.is_empty() {
        return Response::error("to_email is required", 400);
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Resolve billing account ID: use explicit ID if provided, otherwise default
    // to the highest-tier BA the user owns (not the users.org_id pivot).
    let ba_id = match body.billing_account_id {
        Some(id) => id,
        None => {
            match crate::repositories::BillingRepository::new()
                .get_owned_by_user(&db, &user_ctx.user_id)
                .await?
            {
                Some(ba) => ba.id,
                None => return Response::error("No billing account found for your account", 404),
            }
        }
    };

    match OwnershipTransferService::new()
        .initiate_transfer(&db, &ctx.env, &ba_id, &user_ctx.user_id, &to_email)
        .await
    {
        Ok(action) => Response::from_json(&serde_json::json!({
            "success": true,
            "message": "Transfer initiated. An email has been sent to the recipient.",
            "token": action.id,
            "expires_at": action.expires_at,
            "to_email": action.to_email,
        })),
        Err(e) => transfer_error_response(e),
    }
}

// ─── Cancel transfer ──────────────────────────────────────────────────────────

#[utoipa::path(
    delete,
    path = "/api/billing/transfer",
    tag = "Billing",
    summary = "Cancel a pending billing account ownership transfer",
    description = "Cancels any outstanding transfer for the caller's billing account.",
    responses(
        (status = 200, description = "Transfer cancelled"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Billing account not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_cancel_transfer(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Resolve billing account — use highest-tier owned BA.
    let ba_id = match crate::repositories::BillingRepository::new()
        .get_owned_by_user(&db, &user_ctx.user_id)
        .await?
    {
        Some(ba) => ba.id,
        None => return Response::error("No billing account found for your account", 404),
    };

    match OwnershipTransferService::new()
        .cancel_transfer(&db, &ba_id, &user_ctx.user_id)
        .await
    {
        Ok(()) => Response::from_json(&serde_json::json!({
            "success": true,
            "message": "Pending transfer cancelled.",
        })),
        Err(e) => transfer_error_response(e),
    }
}

// ─── Get transfer info (public) ───────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/billing-transfer/{token}",
    tag = "Billing",
    summary = "Get info about a pending ownership transfer",
    description = "Public endpoint — no authentication required. Returns basic details about \
                   the pending transfer so the acceptance UI can display context.",
    params(("token" = String, Path, description = "Transfer token (from email link)")),
    responses(
        (status = 200, description = "Transfer info"),
        (status = 404, description = "Token not found"),
        (status = 410, description = "Transfer expired, accepted, or cancelled"),
    )
)]
pub async fn handle_get_transfer_info(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let token = ctx
        .param("token")
        .ok_or_else(|| Error::RustError("Missing token".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Best-effort: include authenticated user's email if logged in, so the UI can
    // pre-validate without a second round-trip.
    let caller_email = auth::authenticate_request(&req, &ctx)
        .await
        .ok()
        .and_then(|c| {
            // We need the user email — fetch it synchronously isn't possible here;
            // the frontend can cross-check on its own.
            let _ = c;
            None::<String>
        });
    let _ = caller_email;

    match OwnershipTransferService::new()
        .get_transfer_info(&db, token)
        .await
    {
        Ok(info) => Response::from_json(&serde_json::json!({
            "token": info.token,
            "billing_account_id": info.billing_account_id,
            "billing_account_tier": info.billing_account_tier,
            "from_user_name": info.from_user_name,
            "from_user_email": info.from_user_email,
            "to_email": info.to_email,
            "expires_at": info.expires_at,
        })),
        Err(e) => transfer_error_response(e),
    }
}

// ─── Accept transfer ──────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/billing-transfer/{token}/accept",
    tag = "Billing",
    summary = "Accept a pending billing account ownership transfer",
    description = "The caller must be authenticated and their account email must match the \
                   to_email stored in the transfer record.",
    params(("token" = String, Path, description = "Transfer token (from email link)")),
    responses(
        (status = 200, description = "Transfer accepted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Email mismatch or forbidden"),
        (status = 404, description = "Token not found"),
        (status = 410, description = "Transfer expired, accepted, or cancelled"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_accept_transfer(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let token = ctx
        .param("token")
        .ok_or_else(|| Error::RustError("Missing token".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // We need the acceptor's email to validate it against the transfer record.
    let acceptor = match crate::repositories::UserRepository::new()
        .get_user_by_id(&db, &user_ctx.user_id)
        .await?
    {
        Some(u) => u,
        None => return Response::error("User not found", 401),
    };

    match OwnershipTransferService::new()
        .accept_transfer(&db, &ctx.env, token, &user_ctx.user_id, &acceptor.email)
        .await
    {
        Ok(()) => Response::from_json(&serde_json::json!({
            "success": true,
            "message": "Billing account ownership successfully transferred to your account.",
        })),
        Err(e) => transfer_error_response(e),
    }
}

// ─── Admin force-transfer ─────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/admin/billing-accounts/{id}/transfer",
    tag = "Admin",
    summary = "Force-transfer billing account ownership (admin only)",
    description = "Immediately reassigns the billing account to a new owner with no email \
                   confirmation. Any pending transfer is cancelled. The target user must be a \
                   member of one of the billing account's organizations.",
    params(("id" = String, Path, description = "Billing account ID")),
    request_body(
        content_type = "application/json",
        description = "Target user ID",
        content = serde_json::Value
    ),
    responses(
        (status = 200, description = "Ownership transferred"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "Billing account or user not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_force_transfer(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };
    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let ba_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing billing account ID".to_string()))?;

    #[derive(serde::Deserialize)]
    struct Body {
        to_user_id: String,
    }

    let body: Body = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid request body. Expected {to_user_id}", 400),
    };

    if body.to_user_id.is_empty() {
        return Response::error("to_user_id is required", 400);
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match OwnershipTransferService::new()
        .admin_force_transfer(&db, ba_id, &body.to_user_id, &user_ctx.user_id)
        .await
    {
        Ok(()) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "admin_force_transfer",
                    "billing_account_id": ba_id,
                    "to_user_id": body.to_user_id,
                    "admin_id": user_ctx.user_id,
                    "level": "info"
                })
            );
            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Billing account ownership has been transferred.",
            }))
        }
        Err(e) => transfer_error_response(e),
    }
}
