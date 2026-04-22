use crate::auth;
use crate::services::BillingService;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/billing/status",
    tag = "Billing",
    summary = "Get billing status",
    description = "Returns the billing account tier, active subscription details (status, period end, cancel-at-period-end), and whether the caller is the billing owner. Auto-creates a billing account for new users if one does not exist",
    responses(
        (status = 200, description = "Billing status object"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_status(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match BillingService::new()
        .get_billing_status(&db, &user_ctx.user_id)
        .await
    {
        Ok(status) => Response::from_json(&serde_json::json!({
            "tier": status.tier,
            "is_billing_owner": status.is_billing_owner,
            "subscription_status": status.subscription_status,
            "subscription_id": status.subscription_id,
            "current_period_end": status.current_period_end,
            "cancel_at_period_end": status.cancel_at_period_end,
            "provider_customer_id": status.provider_customer_id,
            "billing_account_id": status.billing_account_id,
            "amount_cents": status.amount_cents,
            "currency": status.currency,
            "discount_name": status.discount_name,
            "interval": status.interval,
            "subscription_plan": status.subscription_plan,
        })),
        Err(e) => {
            console_error!("[billing] get_billing_status error: {}", e);
            Response::error("Service temporarily unavailable", 503)
        }
    }
}
