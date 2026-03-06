use crate::auth;
use crate::billing::polar::polar_client_from_env;
use crate::billing::provider::BillingProvider;
use crate::db;
use crate::utils::{now_timestamp, verify_polar_webhook_signature};
use worker::d1::D1Database;
use worker::*;

fn get_frontend_url(env: &Env) -> String {
    env.var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
}

// ─── GET /api/billing/status ─────────────────────────────────────────────────
/// Returns the billing/subscription status for the authenticated user's billing account.
pub async fn handle_get_status(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get or create billing account for user
    let billing_account = match db::get_user_billing_account(&db, &user_ctx.user_id).await? {
        Some(ba) => ba,
        None => {
            // Auto-create billing account and org for new users
            let org = db::create_default_org(&db, &user_ctx.user_id, "Personal").await?;
            match db::get_billing_account(&db, org.billing_account_id.as_deref().unwrap_or(""))
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

    let subscription = db::get_subscription_for_billing_account(&db, &billing_account.id).await?;

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

// ─── POST /api/billing/checkout ──────────────────────────────────────────────
/// Creates a Polar Checkout session and returns the hosted checkout URL.
pub async fn handle_create_checkout(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let body: serde_json::Value = match req.json().await {
        Ok(b) => b,
        Err(_) => {
            console_error!("[checkout] Invalid request body");
            return Response::error("Invalid request body", 400);
        }
    };

    let plan = match body["plan"].as_str() {
        Some(p) => p,
        None => {
            console_error!("[checkout] plan is required");
            return Response::error("plan is required", 400);
        }
    };

    // Validate plan name and map to product ID setting key
    let product_id_key = match plan {
        "pro_monthly" => "product_pro_monthly_id",
        "pro_annual" => "product_pro_annual_id",
        "business_monthly" => "product_business_monthly_id",
        "business_annual" => "product_business_annual_id",
        _ => {
            console_error!("[checkout] Invalid plan: {}", plan);
            return Response::error("Invalid plan", 400);
        }
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get all settings in a single query
    let settings = db::get_all_settings(&db).await?;

    // Look up product ID securely from settings
    let polar_product_id = match settings.get(product_id_key) {
        Some(id) => id.clone(),
        None => {
            console_error!("[checkout] Product ID not found for plan: {}", plan);
            return Response::error("Plan not configured", 503);
        }
    };

    // Look up discount ID from settings if founder pricing is active
    let coupon_id = if settings
        .get("founder_pricing_active")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        let discount_key = match plan {
            "pro_monthly" => "active_discount_pro_monthly",
            "pro_annual" => "active_discount_pro_annual",
            "business_monthly" => "active_discount_business_monthly",
            "business_annual" => "active_discount_business_annual",
            _ => "",
        };

        let discount_id = settings.get(discount_key).cloned().unwrap_or_default();
        if !discount_id.is_empty() {
            Some(discount_id)
        } else {
            None
        }
    } else {
        None
    };

    let polar = match polar_client_from_env(&ctx.env) {
        Ok(s) => s,
        Err(e) => {
            console_error!("[checkout] Failed to initialize Polar client: {}", e);
            return Response::error("Billing not configured", 503);
        }
    };

    // Get or create billing account for user
    let billing_account = match db::get_user_billing_account(&db, &user_ctx.user_id).await? {
        Some(ba) => ba,
        None => {
            // Auto-create billing account and org for new users
            let org = db::create_default_org(&db, &user_ctx.user_id, "Personal").await?;
            match db::get_billing_account(&db, org.billing_account_id.as_deref().unwrap_or(""))
                .await?
            {
                Some(ba) => ba,
                None => {
                    console_error!("[checkout] Failed to create billing account");
                    return Response::error("Failed to create billing account", 500);
                }
            }
        }
    };

    let frontend_url = get_frontend_url(&ctx.env);
    let success_url = format!(
        "{}/billing/success?session_id={{CHECKOUT_SESSION_ID}}",
        frontend_url
    );
    let cancel_url = format!("{}/billing/cancelled", frontend_url);

    let params = crate::billing::types::CreateCheckoutSessionParams {
        billing_account_id: billing_account.id.clone(),
        customer_id: billing_account.provider_customer_id.clone(),
        price_id: polar_product_id,
        success_url,
        cancel_url,
        coupon_id,
        client_reference_id: billing_account.id.clone(),
    };

    match polar.create_checkout_session(params).await {
        Ok(session) => Response::from_json(&serde_json::json!({ "url": session.url })),
        Err(e) => {
            console_error!("[checkout] Polar API error: {}", e);
            Response::error("Failed to create checkout session", 500)
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

    // Read the raw body bytes for signature verification before parsing as JSON
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

    // Polar wraps events: { "type": "subscription.active", "data": { ... } }
    let event_type = match body["type"].as_str() {
        Some(t) => t.to_string(),
        None => {
            console_error!("[webhook] Missing event type in payload");
            return Response::error("Missing event type", 400);
        }
    };

    let data = &body["data"];

    // ── 3. Get DB – failure is transient, let Polar retry ────────────────────
    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[webhook] DB binding unavailable: {}", e);
            return Response::error("Service temporarily unavailable", 503);
        }
    };

    let now = now_timestamp();

    // ── 4. Dispatch on event type ────────────────────────────────────────────
    match event_type.as_str() {
        "subscription.active" | "subscription.created" => {
            let subscription_id = data["id"].as_str().unwrap_or("").to_string();
            let customer_id = data["customer_id"].as_str().unwrap_or("").to_string();
            // external_id is our internal billing_account_id
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

            if billing_account_id.is_empty() {
                console_error!(
                    "[webhook] {} missing billing_account_id (externalId). customer_id={}",
                    event_type,
                    customer_id
                );
                // Permanent data issue – don't retry
                return Response::error("Missing billing account ID", 400);
            }

            let (plan, resolved_interval) =
                match db::get_cached_product_by_price_id(&db, &price_id).await {
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
                        // Transient DB failure – let Polar retry
                        return Response::error("Service temporarily unavailable", 503);
                    }
                };

            if let Err(e) = db::upsert_subscription(
                &db,
                &billing_account_id,
                &subscription_id,
                &customer_id,
                "active",
                &plan,
                &resolved_interval,
                &price_id,
                0,
                0,
                false,
                amount_cents,
                currency,
                discount_name,
                now,
            )
            .await
            {
                console_error!("[webhook] DB error upserting subscription: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }

            if let Err(e) = db::update_billing_account_tier(&db, &billing_account_id, &plan).await {
                console_error!("[webhook] DB error updating billing tier: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }

            if let Err(e) = db::update_billing_account_provider_customer_id(
                &db,
                &billing_account_id,
                &customer_id,
            )
            .await
            {
                console_error!("[webhook] DB error storing customer_id: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }
        }

        "subscription.updated" | "subscription.uncanceled" => {
            let subscription_id = data["id"].as_str().unwrap_or("").to_string();
            let customer_id = data["customer_id"].as_str().unwrap_or("").to_string();
            let billing_account_id = data["customer"]["external_id"]
                .as_str()
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
            let current_period_start = data["currentPeriodStart"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let current_period_end = data["currentPeriodEnd"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp())
                .unwrap_or(0);
            let cancel_at_period_end = data["cancelAtPeriodEnd"].as_bool().unwrap_or(false);
            let amount_cents = data["amount"].as_i64();
            let currency = data["currency"].as_str().unwrap_or("usd");
            let discount_name = data["discount"]["name"].as_str();

            if billing_account_id.is_empty() {
                console_error!(
                    "[webhook] subscription.updated missing billing_account_id. customer_id={}",
                    customer_id
                );
                return Response::error("Missing billing account ID", 400);
            }

            let (plan, resolved_interval) =
                match db::get_cached_product_by_price_id(&db, &price_id).await {
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
                        // Transient DB failure – let Polar retry
                        return Response::error("Service temporarily unavailable", 503);
                    }
                };

            if let Err(e) = db::upsert_subscription(
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
                now,
            )
            .await
            {
                console_error!("[webhook] DB error upserting subscription: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }

            if let Err(e) = db::update_billing_account_tier(&db, &billing_account_id, &plan).await {
                console_error!("[webhook] DB error updating billing tier: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }
        }

        "subscription.canceled" | "subscription.revoked" => {
            let subscription_id = data["id"].as_str().unwrap_or("").to_string();
            let customer_id = data["customerId"].as_str().unwrap_or("").to_string();
            let billing_account_id = data["customer"]["externalId"]
                .as_str()
                .unwrap_or("")
                .to_string();

            if billing_account_id.is_empty() {
                console_error!(
                    "[webhook] {} missing billing_account_id. customer_id={}",
                    event_type,
                    customer_id
                );
                return Response::error("Missing billing account ID", 400);
            }

            if let Err(e) = db::mark_subscription_canceled(&db, &subscription_id, now).await {
                console_error!("[webhook] DB error canceling subscription: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }

            if let Err(e) = db::update_billing_account_tier(&db, &billing_account_id, "free").await
            {
                console_error!("[webhook] DB error downgrading tier: {}", e);
                return Response::error("Service temporarily unavailable", 503);
            }
        }

        other => {
            console_warn!("[webhook] Unhandled event type: {} – acknowledging", other);
        }
    }

    Response::from_json(&serde_json::json!({ "received": true }))
}

// ─── POST /api/billing/portal ─────────────────────────────────────────────────
/// Generates a Polar Customer Portal URL for the authenticated user.
/// The frontend receives the URL and redirects the user.
pub async fn handle_portal(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(c) => c,
        Err(e) => return Ok(e.into_response()),
    };

    let polar = match polar_client_from_env(&ctx.env) {
        Ok(p) => p,
        Err(_) => return Response::error("Billing not configured", 503),
    };

    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[portal] DB binding unavailable: {}", e);
            return Response::error("Service temporarily unavailable", 503);
        }
    };

    let billing_account = match db::get_user_billing_account(&db, &user_ctx.user_id).await? {
        Some(ba) => ba,
        None => {
            return Response::error("No billing account found", 400);
        }
    };

    let customer_id = match billing_account.provider_customer_id {
        Some(ref id) if !id.is_empty() => id.clone(),
        _ => {
            console_error!(
                "[portal] No Polar customer_id for billing_account: {}",
                billing_account.id
            );
            return Response::error(
                "No billing account found. Please create a subscription first.",
                400,
            );
        }
    };

    let frontend_url = get_frontend_url(&ctx.env);
    let return_url = format!("{}/billing", frontend_url);

    match polar
        .create_customer_portal_session(&customer_id, &return_url)
        .await
    {
        Ok(portal_url) => Response::from_json(&serde_json::json!({ "url": portal_url })),
        Err(e) => {
            console_error!("[portal] Polar API error: {}", e);
            Response::error("Failed to create portal session", 500)
        }
    }
}
