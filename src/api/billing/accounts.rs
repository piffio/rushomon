/// GET /api/billing/accounts
///
/// Returns all billing accounts owned by the authenticated user, ordered
/// highest-tier first (business → pro → free), each enriched with its
/// subscription status and the list of organizations it covers.
///
/// This is the data source for the multi-BA billing page (Option A).
use crate::auth;
use crate::repositories::BillingRepository;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/billing/accounts",
    tag = "Billing",
    summary = "List all billing accounts owned by the current user",
    description = "Returns every billing account for which the caller is the owner, \
                   ordered highest-tier first. Each entry includes tier, subscription \
                   status, and the organizations covered.",
    responses(
        (status = 200, description = "Array of billing account summaries"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_list_accounts(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let billing_repo = BillingRepository::new();

    let accounts = billing_repo
        .get_all_owned_by_user(&db, &user_ctx.user_id)
        .await?;

    let mut result = Vec::with_capacity(accounts.len());

    for ba in &accounts {
        // Subscription details for this BA.
        let subscription = billing_repo.get_subscription(&db, &ba.id).await?;

        // Organizations linked to this BA.
        let org_rows = db
            .prepare(
                "SELECT o.id, o.name, o.slug,
                        (SELECT COUNT(*) FROM org_members om WHERE om.org_id = o.id) AS member_count,
                        (SELECT COUNT(*) FROM links l WHERE l.org_id = o.id AND l.status = 'active') AS link_count
                 FROM organizations o
                 WHERE o.billing_account_id = ?1
                 ORDER BY o.created_at ASC",
            )
            .bind(&[ba.id.clone().into()])?
            .all()
            .await?
            .results::<serde_json::Value>()?;

        let subscription_status = subscription
            .as_ref()
            .and_then(|s| s["status"].as_str())
            .map(|s| s.to_string());
        let current_period_end = subscription
            .as_ref()
            .and_then(|s| s["current_period_end"].as_i64());
        let cancel_at_period_end = subscription
            .as_ref()
            .and_then(|s| s["cancel_at_period_end"].as_i64())
            .unwrap_or(0)
            != 0;
        let amount_cents = subscription
            .as_ref()
            .and_then(|s| s["amount_cents"].as_i64());
        let currency = subscription
            .as_ref()
            .and_then(|s| s["currency"].as_str())
            .map(|s| s.to_string());
        let interval = subscription
            .as_ref()
            .and_then(|s| s["interval"].as_str())
            .map(|s| s.to_string());

        result.push(serde_json::json!({
            "id": ba.id,
            "tier": ba.tier,
            "is_billing_owner": true,
            "subscription_status": subscription_status,
            "current_period_end": current_period_end,
            "cancel_at_period_end": cancel_at_period_end,
            "amount_cents": amount_cents,
            "currency": currency,
            "interval": interval,
            "organizations": org_rows,
        }));
    }

    Response::from_json(&serde_json::json!({ "accounts": result }))
}
