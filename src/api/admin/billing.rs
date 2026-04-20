use crate::auth;
use crate::repositories::BillingRepository;
use crate::services::BillingService;
use crate::utils::now_timestamp;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/admin/billing-accounts",
    tag = "Admin",
    summary = "List billing accounts",
    params(
        ("page" = Option<i64>, Query, description = "Page number"),
        ("limit" = Option<i64>, Query, description = "Results per page"),
        ("search" = Option<String>, Query, description = "Filter by owner email"),
        ("tier" = Option<String>, Query, description = "Filter by tier"),
    ),
    responses(
        (status = 200, description = "Paginated list of billing accounts"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_list_billing_accounts(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let url = req.url()?;

    let page = url
        .query_pairs()
        .find(|(k, _)| k == "page")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(1i64);
    let limit = url
        .query_pairs()
        .find(|(k, _)| k == "limit")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(50i64);
    let search = url
        .query_pairs()
        .find(|(k, _)| k == "search")
        .map(|(_, v)| v.to_string());
    let tier_filter = url
        .query_pairs()
        .find(|(k, _)| k == "tier")
        .map(|(_, v)| v.to_string());

    match BillingService::new()
        .admin_list_billing_accounts(&db, page, limit, search.as_deref(), tier_filter.as_deref())
        .await
    {
        Ok((accounts, total)) => {
            use chrono::{Datelike, TimeZone};
            let now = chrono::Utc::now();
            let next_reset = chrono::Utc
                .with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
                .single()
                .unwrap_or_else(chrono::Utc::now);
            Response::from_json(&serde_json::json!({
                "accounts": accounts,
                "total": total,
                "page": page,
                "limit": limit,
                "next_reset": {
                    "utc": next_reset.to_rfc3339(),
                    "timestamp": next_reset.timestamp(),
                }
            }))
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "list_billing_accounts_failed",
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to list billing accounts", 500)
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/billing-accounts/{id}",
    tag = "Admin",
    summary = "Get billing account details",
    params(("id" = String, Path, description = "Billing account ID")),
    responses(
        (status = 200, description = "Billing account details"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "Not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_get_billing_account(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let billing_account_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing billing account ID".to_string()))?;
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match BillingService::new()
        .admin_get_billing_account(&db, billing_account_id)
        .await
    {
        Ok(Some(details)) => Response::from_json(&details),
        Ok(None) => Response::error("Billing account not found", 404),
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "get_billing_account_failed",
                    "billing_account_id": billing_account_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to get billing account details", 500)
        }
    }
}

#[utoipa::path(
    put,
    path = "/api/admin/billing-accounts/{id}/tier",
    tag = "Admin",
    summary = "Update billing account tier",
    params(("id" = String, Path, description = "Billing account ID")),
    responses(
        (status = 200, description = "Tier updated"),
        (status = 400, description = "Invalid tier"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_update_billing_account_tier(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let billing_account_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing billing account ID".to_string()))?;

    #[derive(serde::Deserialize)]
    struct UpdateTierRequest {
        tier: String,
    }

    let body: UpdateTierRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    if !matches!(
        body.tier.as_str(),
        "free" | "pro" | "business" | "unlimited"
    ) {
        return Response::error(
            "Invalid tier. Must be: free, pro, business, or unlimited",
            400,
        );
    }

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match BillingRepository::new()
        .update_tier(&db, billing_account_id, &body.tier)
        .await
    {
        Ok(_) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "billing_account_tier_updated",
                    "billing_account_id": billing_account_id,
                    "new_tier": body.tier,
                    "admin_user_id": user_ctx.user_id,
                    "level": "info"
                })
            );
            Response::from_json(&serde_json::json!({
                "success": true,
                "message": "Billing account tier updated successfully",
                "tier": body.tier
            }))
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "update_billing_account_tier_failed",
                    "billing_account_id": billing_account_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
            Response::error("Failed to update billing account tier", 500)
        }
    }
}

#[utoipa::path(
    put,
    path = "/api/admin/billing-accounts/{id}/subscription",
    tag = "Admin",
    summary = "Update subscription status",
    params(("id" = String, Path, description = "Billing account ID")),
    responses(
        (status = 200, description = "Subscription status updated"),
        (status = 400, description = "Missing status"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "No subscription found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_update_subscription_status(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    if let Err(e) = auth::require_admin(&user_ctx) {
        return Ok(e.into_response());
    }

    let billing_account_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing billing account ID".to_string()))?;

    let body: serde_json::Value = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid request body", 400),
    };
    let status = match body["status"].as_str() {
        Some(s) => s.to_string(),
        None => return Response::error("status is required", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = BillingRepository::new();

    match repo.get_subscription(&db, billing_account_id).await? {
        Some(subscription) => {
            let subscription_id = subscription["id"].as_str().unwrap_or("");
            let now = now_timestamp();

            match repo
                .update_subscription_status(&db, subscription_id, &status, now)
                .await
            {
                Ok(_) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "subscription_status_updated",
                            "billing_account_id": billing_account_id,
                            "subscription_id": subscription_id,
                            "new_status": status,
                            "admin_user_id": user_ctx.user_id,
                            "level": "info"
                        })
                    );
                    if status == "canceled" {
                        repo.update_tier(&db, billing_account_id, "free").await?;
                    }
                    Response::from_json(&serde_json::json!({
                        "success": true,
                        "message": "Subscription status updated successfully",
                        "subscription_id": subscription_id,
                        "new_status": status
                    }))
                }
                Err(e) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "update_subscription_status_failed",
                            "billing_account_id": billing_account_id,
                            "subscription_id": subscription_id,
                            "error": e.to_string(),
                            "level": "error"
                        })
                    );
                    Response::error("Failed to update subscription status", 500)
                }
            }
        }
        None => Response::error("No subscription found for this billing account", 404),
    }
}
