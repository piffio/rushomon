use crate::auth;
use crate::repositories::BillingRepository;
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
    let repo = BillingRepository::new();

    let billing_account = match repo.get_for_user(&db, &user_ctx.user_id).await? {
        Some(ba) => ba,
        None => {
            let org = crate::db::create_default_org(&db, &user_ctx.user_id, "Personal").await?;
            match repo
                .get_by_id(&db, org.billing_account_id.as_deref().unwrap_or(""))
                .await?
            {
                Some(ba) => ba,
                None => {
                    return Response::from_json(&serde_json::json!({
                        "tier": "free",
                        "subscription_status": null,
                        "subscription_id": null,
                        "current_period_end": null,
                        "cancel_at_period_end": false,
                        "provider_customer_id": null,
                        "billing_account_id": null,
                        "amount_cents": null,
                        "currency": null,
                        "discount_name": null,
                        "interval": null,
                        "subscription_plan": null,
                    }));
                }
            }
        }
    };

    let subscription = repo.get_subscription(&db, &billing_account.id).await?;
    let is_billing_owner = billing_account.owner_user_id == user_ctx.user_id;

    match subscription {
        Some(sub) => Response::from_json(&serde_json::json!({
            "tier": billing_account.tier,
            "is_billing_owner": is_billing_owner,
            "subscription_status": sub["status"],
            "subscription_id": sub["id"],
            "current_period_end": sub["current_period_end"],
            "cancel_at_period_end": sub["cancel_at_period_end"].as_i64().unwrap_or(0) != 0,
            "provider_customer_id": billing_account.provider_customer_id,
            "billing_account_id": billing_account.id,
            "amount_cents": sub["amount_cents"],
            "currency": sub["currency"],
            "discount_name": sub["discount_name"],
            "interval": sub["interval"],
            "subscription_plan": sub["plan"],
        })),
        None => Response::from_json(&serde_json::json!({
            "tier": billing_account.tier,
            "is_billing_owner": is_billing_owner,
            "subscription_status": null,
            "subscription_id": null,
            "current_period_end": null,
            "cancel_at_period_end": false,
            "provider_customer_id": billing_account.provider_customer_id,
            "billing_account_id": billing_account.id,
            "amount_cents": null,
            "currency": null,
            "discount_name": null,
            "interval": null,
            "subscription_plan": null,
        })),
    }
}
