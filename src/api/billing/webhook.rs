use crate::repositories::{BillingRepository, ProductRepository};
use crate::utils::{now_timestamp, verify_polar_webhook_signature};
use worker::d1::D1Database;
use worker::*;

/// Resolves a billing account ID from webhook data.
///
/// Prefers `external_id` from the webhook payload, but falls back to looking up
/// by `customer_id` if `external_id` is missing.
async fn resolve_billing_account_id(
    db: &D1Database,
    event_type: &str,
    external_id: &str,
    customer_id: &str,
) -> Result<String, Response> {
    if !external_id.is_empty() {
        return Ok(external_id.to_string());
    }

    console_warn!(
        "[webhook] {} missing external_id, falling back to customer_id lookup. customer_id={}",
        event_type,
        customer_id
    );

    match BillingRepository::new()
        .get_id_by_provider_customer(db, customer_id)
        .await
    {
        Ok(Some(id)) => Ok(id),
        Ok(None) => {
            console_error!(
                "[webhook] {} billing account not found by customer_id. customer_id={}",
                event_type,
                customer_id
            );
            Err(Response::error("Missing billing account ID", 400).unwrap())
        }
        Err(e) => {
            console_error!(
                "[webhook] {} DB error looking up billing account by customer_id: {}",
                event_type,
                e
            );
            Err(Response::error("Service temporarily unavailable", 503).unwrap())
        }
    }
}

// ─── POST /api/billing/webhook ───────────────────────────────────────────────
/// Receives Polar webhook events, verifies the HMAC-SHA256 signature, and
/// processes subscription state changes directly in the database.
///
/// Error handling strategy:
///   - 401 (no retry): invalid or missing signature
///   - 400 (no retry): malformed / unprocessable payload
///   - 503 (retry):   transient infrastructure failures (DB, Polar API)
///   - 200:           event processed successfully or intentionally skipped
#[utoipa::path(
    post,
    path = "/api/billing/webhook",
    tag = "Billing",
    summary = "Polar webhook receiver",
    description = "Receives and processes Polar webhook events (subscription created/updated/cancelled, order created). Verifies the webhook signature before processing. Returns 200 for success or known-skip, 400 for malformed payloads, 503 for transient failures",
    responses(
        (status = 200, description = "Event processed or intentionally skipped"),
        (status = 400, description = "Invalid or malformed webhook payload"),
        (status = 503, description = "Transient infrastructure failure"),
    )
)]
pub async fn handle_webhook(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // ── 1. Verify signature ──────────────────────────────────────────────────
    let webhook_secret = match ctx.env.secret("POLAR_WEBHOOK_SECRET") {
        Ok(s) => s.to_string(),
        Err(_) => {
            console_error!("[webhook] POLAR_WEBHOOK_SECRET not configured");
            return Response::error("Webhook not configured", 503);
        }
    };

    let signature = match req.headers().get("webhook-signature")? {
        Some(s) => s,
        None => {
            console_error!("[webhook] Missing webhook-signature header");
            return Response::error("Missing signature", 401);
        }
    };
    let webhook_id = match req.headers().get("webhook-id")? {
        Some(s) => s,
        None => {
            console_error!("[webhook] Missing webhook-id header");
            return Response::error("Missing webhook-id header", 401);
        }
    };
    let webhook_timestamp = match req.headers().get("webhook-timestamp")? {
        Some(s) => s,
        None => {
            console_error!("[webhook] Missing webhook-timestamp header");
            return Response::error("Missing webhook-timestamp header", 401);
        }
    };

    let body_bytes = match req.bytes().await {
        Ok(b) => b,
        Err(e) => {
            console_error!("[webhook] Failed to read request body: {}", e);
            return Response::error("Failed to read request body", 503);
        }
    };

    match verify_polar_webhook_signature(
        &body_bytes,
        &webhook_id,
        &webhook_timestamp,
        &signature,
        &webhook_secret,
    ) {
        Ok(true) => {}
        Ok(false) => {
            console_error!("[webhook] Signature verification failed");
            return Response::error("Invalid signature", 401);
        }
        Err(e) => {
            console_error!("[webhook] Signature verification error: {}", e);
            return Response::error("Invalid signature", 401);
        }
    }

    // ── 2. Parse JSON payload ────────────────────────────────────────────────
    let body: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(b) => b,
        Err(e) => {
            console_error!("[webhook] Invalid JSON payload: {}", e);
            return Response::error("Invalid payload", 400);
        }
    };

    let event_type = match body["type"].as_str() {
        Some(t) => t.to_string(),
        None => {
            console_error!("[webhook] Missing event type in payload");
            return Response::error("Missing event type", 400);
        }
    };
    let data = &body["data"];

    // ── 3. Get DB ────────────────────────────────────────────────────────────
    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[webhook] DB binding unavailable: {}", e);
            return Response::error("Service temporarily unavailable", 503);
        }
    };

    let now = now_timestamp();
    let billing_repo = BillingRepository::new();

    // ── 3b. Check idempotency ────────────────────────────────────────────────
    if billing_repo
        .webhook_already_processed(&db, "polar", &webhook_id)
        .await?
    {
        console_log!("[webhook] Duplicate webhook ignored: {}", webhook_id);
        return Response::from_json(&serde_json::json!({
            "received": true,
            "duplicate": true
        }));
    }

    // ── 4. Dispatch on event type ────────────────────────────────────────────
    match event_type.as_str() {
        "subscription.active" | "subscription.created" => {
            let subscription_id = data["id"].as_str().unwrap_or("").to_string();
            let customer_id = data["customer_id"].as_str().unwrap_or("").to_string();
            let billing_account_id = data["customer"]["external_id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let price_id = data["prices"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|p| p["id"].as_str())
                .unwrap_or("")
                .to_string();
            let interval = data["recurringInterval"]
                .as_str()
                .unwrap_or("month")
                .to_string();
            let amount_cents = data["amount"].as_i64();
            let currency = data["currency"].as_str().unwrap_or("usd");
            let discount_name = data["discount"]["name"].as_str();
            let current_period_start = data["current_period_start"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let current_period_end = data["current_period_end"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let cancel_at_period_end = data["cancel_at_period_end"].as_bool().unwrap_or(false);
            let ends_at = data["ends_at"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp());

            let billing_account_id = match resolve_billing_account_id(
                &db,
                &event_type,
                &billing_account_id,
                &customer_id,
            )
            .await
            {
                Ok(id) => id,
                Err(response) => return Ok(response),
            };

            let (plan, resolved_interval) = match ProductRepository::new()
                .get_by_price_id(&db, &price_id)
                .await
            {
                Ok(Some(product)) => {
                    let product_name = product["name"].as_str().unwrap_or("");
                    let plan_name = if product_name.contains("Pro") {
                        "pro"
                    } else if product_name.contains("Business") {
                        "business"
                    } else {
                        "free"
                    };
                    let ri = product["recurring_interval"].as_str().unwrap_or("month");
                    (plan_name.to_string(), ri.to_string())
                }
                Ok(None) => {
                    console_error!("[webhook] Product not found for price_id: {}", price_id);
                    ("free".to_string(), interval)
                }
                Err(e) => {
                    console_error!("[webhook] DB error looking up product: {}", e);
                    return Response::error("Service temporarily unavailable", 503);
                }
            };

            if let Err(e) = billing_repo
                .upsert_subscription(
                    &db,
                    &billing_account_id,
                    &subscription_id,
                    &customer_id,
                    "active",
                    &plan,
                    &resolved_interval,
                    &price_id,
                    current_period_start,
                    current_period_end,
                    cancel_at_period_end,
                    amount_cents,
                    currency,
                    discount_name,
                    ends_at,
                    now,
                )
                .await
            {
                console_error!("[webhook] DB error upserting subscription: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }

            if let Err(e) = billing_repo
                .update_tier(&db, &billing_account_id, &plan)
                .await
            {
                console_error!("[webhook] DB error updating billing tier: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }

            if let Err(e) = billing_repo
                .update_provider_customer_id(&db, &billing_account_id, &customer_id)
                .await
            {
                console_error!("[webhook] DB error storing customer_id: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }
        }

        "subscription.updated" | "subscription.uncanceled" => {
            let subscription_id = data["id"].as_str().unwrap_or("").to_string();
            let customer_id = data["customer_id"]
                .as_str()
                .or_else(|| data["customerId"].as_str())
                .unwrap_or("")
                .to_string();
            let billing_account_id = data["customer"]["external_id"]
                .as_str()
                .or_else(|| data["customer"]["externalId"].as_str())
                .unwrap_or("")
                .to_string();
            let status = data["status"].as_str().unwrap_or("active");
            let price_id = data["prices"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|p| p["id"].as_str())
                .unwrap_or("")
                .to_string();
            let interval = data["recurringInterval"].as_str().unwrap_or("month");
            let current_period_start = data["current_period_start"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let current_period_end = data["current_period_end"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let cancel_at_period_end = data["cancel_at_period_end"].as_bool().unwrap_or(false);
            let ends_at = data["ends_at"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp());
            let amount_cents = data["amount"].as_i64();
            let currency = data["currency"].as_str().unwrap_or("usd");
            let discount_name = data["discount"]["name"].as_str();

            let billing_account_id = match resolve_billing_account_id(
                &db,
                &event_type,
                &billing_account_id,
                &customer_id,
            )
            .await
            {
                Ok(id) => id,
                Err(response) => return Ok(response),
            };

            let (plan, resolved_interval) = match ProductRepository::new()
                .get_by_price_id(&db, &price_id)
                .await
            {
                Ok(Some(product)) => {
                    let product_name = product["name"].as_str().unwrap_or("");
                    let plan_name = if product_name.contains("Pro") {
                        "pro"
                    } else if product_name.contains("Business") {
                        "business"
                    } else {
                        "free"
                    };
                    let ri = product["recurring_interval"].as_str().unwrap_or("month");
                    (plan_name.to_string(), ri.to_string())
                }
                Ok(None) => {
                    console_error!("[webhook] Product not found for price_id: {}", price_id);
                    ("free".to_string(), interval.to_string())
                }
                Err(e) => {
                    console_error!("[webhook] DB error looking up product: {}", e);
                    return Response::error("Service temporarily unavailable", 503);
                }
            };

            if let Err(e) = billing_repo
                .upsert_subscription(
                    &db,
                    &billing_account_id,
                    &subscription_id,
                    &customer_id,
                    status,
                    &plan,
                    &resolved_interval,
                    &price_id,
                    current_period_start,
                    current_period_end,
                    cancel_at_period_end,
                    amount_cents,
                    currency,
                    discount_name,
                    ends_at,
                    now,
                )
                .await
            {
                console_error!("[webhook] DB error upserting subscription: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }

            if cancel_at_period_end {
                if let Err(e) = billing_repo
                    .set_subscription_pending_cancellation(
                        &db,
                        &subscription_id,
                        current_period_end,
                    )
                    .await
                {
                    console_error!(
                        "[webhook] {} DB error setting pending_cancellation: {}",
                        event_type,
                        e
                    );
                }
            } else if event_type == "subscription.uncanceled"
                && let Err(e) = billing_repo
                    .clear_subscription_pending_cancellation(&db, &subscription_id)
                    .await
            {
                console_error!(
                    "[webhook] {} DB error clearing pending_cancellation: {}",
                    event_type,
                    e
                );
            }

            if status == "active"
                && let Err(e) = billing_repo
                    .update_tier(&db, &billing_account_id, &plan)
                    .await
            {
                console_error!("[webhook] DB error updating billing tier: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }
        }

        "subscription.canceled" | "subscription.revoked" => {
            let subscription_id = data["id"].as_str().unwrap_or("").to_string();
            let customer_id = data["customer_id"]
                .as_str()
                .or_else(|| data["customerId"].as_str())
                .unwrap_or("")
                .to_string();
            let billing_account_id = data["customer"]["external_id"]
                .as_str()
                .or_else(|| data["customer"]["externalId"].as_str())
                .unwrap_or("")
                .to_string();

            let billing_account_id = match resolve_billing_account_id(
                &db,
                &event_type,
                &billing_account_id,
                &customer_id,
            )
            .await
            {
                Ok(id) => id,
                Err(response) => return Ok(response),
            };

            let cancel_at_period_end = data["cancel_at_period_end"].as_bool().unwrap_or(false);
            let current_period_end = data["current_period_end"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let status = data["status"].as_str().unwrap_or("canceled");
            let is_immediate_cancellation = status == "canceled" || !cancel_at_period_end;

            if event_type == "subscription.canceled" && !is_immediate_cancellation {
                if let Err(e) = billing_repo
                    .set_subscription_pending_cancellation(
                        &db,
                        &subscription_id,
                        current_period_end,
                    )
                    .await
                {
                    console_error!(
                        "[webhook] {} DB error setting pending_cancellation: {}",
                        event_type,
                        e
                    );
                    return Response::error("Service temporarily unavailable", 503);
                }
            } else {
                if let Err(e) = billing_repo
                    .mark_subscription_canceled(&db, &subscription_id, now)
                    .await
                {
                    console_error!("[webhook] DB error canceling subscription: {}", e);
                    return Response::error("Service temporarily unavailable", 503);
                }

                if let Err(e) = billing_repo
                    .update_tier(&db, &billing_account_id, "free")
                    .await
                {
                    console_error!("[webhook] DB error downgrading tier: {}", e);
                    return Response::error("Service temporarily unavailable", 503);
                }
            }
        }

        other => {
            console_warn!("[webhook] Unhandled event type: {} – acknowledging", other);
        }
    }

    // ── 5. Mark as processed (idempotency) ───────────────────────────────────
    if let Err(e) = billing_repo
        .mark_webhook_processed(&db, "polar", &webhook_id, &event_type, 2592000)
        .await
    {
        console_error!("[webhook] Failed to mark webhook as processed: {}", e);
    }

    Response::from_json(&serde_json::json!({ "received": true }))
}
