use crate::auth;
use crate::repositories::BillingRepository;
use crate::utils::now_timestamp;
use worker::d1::D1Database;
use worker::*;

// ─── POST /api/admin/billing-accounts/:id/reset ──────────────────────────────
/// Resets a billing account to free tier with no active subscriptions.
/// Admin-only.
#[utoipa::path(
    post,
    path = "/api/admin/billing-accounts/{id}/reset",
    tag = "Admin",
    summary = "Reset billing account to free tier",
    params(("id" = String, Path, description = "Billing account ID")),
    responses(
        (status = 200, description = "Billing account reset to free"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "Billing account not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_reset_billing_account(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let billing_account_id = match ctx.param("id") {
        Some(id) => id.to_string(),
        None => return Response::error("Missing billing account ID", 400),
    };

    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[admin-reset] DB binding unavailable: {}", e);
            return Response::error("Service temporarily unavailable", 503);
        }
    };

    // Delete all subscriptions for this billing account
    let del_stmt = db.prepare("DELETE FROM subscriptions WHERE billing_account_id = ?1");
    if let Err(e) = del_stmt
        .bind(&[billing_account_id.clone().into()])?
        .run()
        .await
    {
        console_error!("[admin-reset] Failed to delete subscriptions: {}", e);
        return Response::error("Service temporarily unavailable", 503);
    }

    // Reset billing account tier and clear provider_customer_id
    let upd_stmt = db.prepare(
        "UPDATE billing_accounts SET tier = 'free', provider_customer_id = NULL WHERE id = ?1",
    );
    if let Err(e) = upd_stmt
        .bind(&[billing_account_id.clone().into()])?
        .run()
        .await
    {
        console_error!("[admin-reset] Failed to reset billing account: {}", e);
        return Response::error("Service temporarily unavailable", 503);
    }

    Response::from_json(&serde_json::json!({
        "reset": true,
        "billing_account_id": billing_account_id
    }))
}

// ─── POST /api/admin/cron/trigger-downgrade ──────────────────────────────────
/// Manually triggers the expired-subscription downgrade job.
#[utoipa::path(
    post,
    path = "/api/admin/cron/downgrade",
    tag = "Admin",
    summary = "Trigger subscription downgrade job",
    description = "Manually triggers the expired-subscription downgrade cron job. Downgrades all billing accounts whose subscriptions have expired to the Free tier",
    responses(
        (status = 200, description = "Downgrade job completed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_cron_trigger_downgrade(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[cron-trigger] DB binding unavailable: {}", e);
            return Response::error("Service temporarily unavailable", 503);
        }
    };

    let now = now_timestamp();
    let repo = BillingRepository::new();

    let expired_subscriptions = match repo.get_expired_pending_cancellations(&db, now).await {
        Ok(subs) => subs,
        Err(e) => {
            console_error!(
                "[cron-trigger] Failed to query expired subscriptions: {}",
                e
            );
            return Response::error("Service temporarily unavailable", 503);
        }
    };

    let total = expired_subscriptions.len();
    let mut success_count = 0u32;
    let mut error_count = 0u32;

    for sub in &expired_subscriptions {
        let subscription_id = sub
            .get("provider_subscription_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let billing_account_id = sub
            .get("billing_account_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if let Err(e) = repo.update_tier(&db, billing_account_id, "free").await {
            console_error!(
                "[cron-trigger] Failed to downgrade tier for billing account {}: {}",
                billing_account_id,
                e
            );
            error_count += 1;
            continue;
        }

        if let Err(e) = repo
            .finalize_expired_subscription(&db, subscription_id, now)
            .await
        {
            console_error!(
                "[cron-trigger] Failed to finalize subscription {}: {}",
                subscription_id,
                e
            );
            error_count += 1;
            continue;
        }

        success_count += 1;
    }

    Response::from_json(&serde_json::json!({
        "processed": total,
        "success": success_count,
        "errors": error_count
    }))
}
