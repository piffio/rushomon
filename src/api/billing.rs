use crate::auth;
use crate::billing::polar::{
    build_price_map, plan_from_price_id, polar_client_from_env, product_catalog_from_env,
};
use crate::billing::provider::BillingProvider;
use crate::db;
use crate::utils::now_timestamp;
use subtle::ConstantTimeEq;
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
            console_log!("Creating billing account for user: {}", user_ctx.user_id);
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
        Some(p) => {
            console_log!("[checkout] Received plan: {}", p);
            p.to_string()
        }
        None => {
            console_error!("[checkout] plan is required");
            return Response::error("plan is required", 400);
        }
    };

    // Validate plan name and map to product ID setting key
    let product_id_key = match plan.as_str() {
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
        Some(id) => {
            console_log!("[checkout] Found product_id for {}: {}", plan, id);
            id.clone()
        }
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
        let discount_key = match plan.as_str() {
            "pro_monthly" => "active_discount_pro_monthly",
            "pro_annual" => "active_discount_pro_annual",
            "business_monthly" => "active_discount_business_monthly",
            "business_annual" => "active_discount_business_annual",
            _ => "",
        };

        let discount_id = settings.get(discount_key).cloned().unwrap_or_default();
        if !discount_id.is_empty() {
            console_log!("[checkout] Using discount_id for {}: {}", plan, discount_id);
            Some(discount_id)
        } else {
            console_log!("[checkout] No discount configured for {}", plan);
            None
        }
    } else {
        console_log!("[checkout] Founder pricing not active, no discount applied");
        None
    };

    console_log!("[checkout] Initializing Polar client...");
    let polar = match polar_client_from_env(&ctx.env) {
        Ok(s) => {
            console_log!("[checkout] Polar client initialized successfully");
            s
        }
        Err(e) => {
            console_error!("[checkout] Failed to initialize Polar client: {}", e);
            return Response::error("Billing not configured", 503);
        }
    };

    // Get or create billing account for user
    console_log!(
        "[checkout] Getting billing account for user: {}",
        user_ctx.user_id
    );
    let billing_account = match db::get_user_billing_account(&db, &user_ctx.user_id).await? {
        Some(ba) => {
            console_log!("[checkout] Found existing billing account: {}", ba.id);
            ba
        }
        None => {
            // Auto-create billing account and org for new users
            console_log!(
                "[checkout] Creating billing account for user: {}",
                user_ctx.user_id
            );
            let org = db::create_default_org(&db, &user_ctx.user_id, "Personal").await?;
            match db::get_billing_account(&db, org.billing_account_id.as_deref().unwrap_or(""))
                .await?
            {
                Some(ba) => {
                    console_log!("[checkout] Created billing account: {}", ba.id);
                    ba
                }
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

    console_log!(
        "[checkout] Creating checkout session with params: billing_account_id={}, product_id={}, coupon_id={:?}",
        params.billing_account_id,
        params.price_id,
        params.coupon_id
    );

    match polar.create_checkout_session(params).await {
        Ok(session) => {
            console_log!(
                "[checkout] Checkout session created successfully: {}",
                session.url
            );
            Response::from_json(&serde_json::json!({ "url": session.url }))
        }
        Err(e) => {
            console_error!("[checkout] Polar API error: {}", e);
            Response::error("Failed to create checkout session", 500)
        }
    }
}

// ─── POST /api/billing/subscription-update ──────────────────────────────────
/// Internal endpoint called by the SvelteKit webhook handler after verifying the
/// Polar webhook signature. Authenticated with a shared INTERNAL_WEBHOOK_SECRET.
pub async fn handle_subscription_update(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let expected_secret = match ctx.env.secret("INTERNAL_WEBHOOK_SECRET") {
        Ok(s) => s.to_string(),
        Err(_) => return Response::error("Internal endpoint not configured", 503),
    };

    let provided = req.headers().get("X-Internal-Secret")?.unwrap_or_default();

    let secrets_match: bool = provided.as_bytes().ct_eq(expected_secret.as_bytes()).into();
    if !secrets_match {
        return Response::error("Unauthorized", 401);
    }

    let body: serde_json::Value = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let event_type = body["event_type"].as_str().unwrap_or("");
    let now = now_timestamp();

    match event_type {
        "subscription_activated" | "subscription_created" => {
            let subscription_id = body["subscription_id"].as_str().unwrap_or("").to_string();
            let customer_id = body["customer_id"].as_str().unwrap_or("").to_string();
            let billing_account_id = body["billing_account_id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let price_id = body["price_id"].as_str().unwrap_or("").to_string();
            let interval = body["interval"].as_str().unwrap_or("month").to_string();

            // Extract pricing data from webhook payload
            let amount_cents = body["amount"].as_i64(); // Actual amount charged (post-discount)
            let currency = body["currency"].as_str().unwrap_or("usd");
            let discount_name = body["discount"]["name"].as_str();

            let (plan, resolved_interval) =
                match db::get_cached_product_by_price_id(&db, &price_id).await {
                    Ok(Some(product)) => {
                        // Extract plan from product name or recurring_interval
                        let product_name = product["name"].as_str().unwrap_or("");
                        let plan_name = if product_name.contains("Pro") {
                            "pro"
                        } else if product_name.contains("Business") {
                            "business"
                        } else {
                            "free"
                        };
                        let interval = product["recurring_interval"].as_str().unwrap_or("month");
                        (plan_name.to_string(), interval.to_string())
                    }
                    Ok(None) => {
                        console_error!(
                            "[subscription_update] Product not found for price_id: {}",
                            price_id
                        );
                        ("free".to_string(), interval)
                    }
                    Err(e) => {
                        console_error!("[subscription_update] Failed to lookup product: {}", e);
                        ("free".to_string(), interval)
                    }
                };

            let actual_billing_account_id = if !billing_account_id.is_empty() {
                billing_account_id
            } else {
                console_error!(
                    "No billing account ID provided in webhook. Customer ID: {}",
                    customer_id
                );
                return Response::error("No billing account ID provided", 400);
            };

            db::upsert_subscription(
                &db,
                &actual_billing_account_id,
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
            .await?;
            db::update_billing_account_tier(&db, &actual_billing_account_id, &plan).await?;

            // Store the Polar customer ID on the billing account for portal access
            db::update_billing_account_provider_customer_id(
                &db,
                &actual_billing_account_id,
                &customer_id,
            )
            .await?;
        }
        "subscription_updated" => {
            let subscription_id = body["subscription_id"].as_str().unwrap_or("").to_string();
            let customer_id = body["customer_id"].as_str().unwrap_or("").to_string();
            let billing_account_id = body["billing_account_id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let status = body["status"].as_str().unwrap_or("active");
            let price_id = body["price_id"].as_str().unwrap_or("").to_string();
            let interval = body["interval"].as_str().unwrap_or("month");
            let current_period_start = body["current_period_start"].as_i64().unwrap_or(0);
            let current_period_end = body["current_period_end"].as_i64().unwrap_or(0);
            let cancel_at_period_end = body["cancel_at_period_end"].as_bool().unwrap_or(false);

            // Extract pricing data from webhook payload
            let amount_cents = body["amount"].as_i64();
            let currency = body["currency"].as_str().unwrap_or("usd");
            let discount_name = body["discount"]["name"].as_str();

            let polar = match polar_client_from_env(&ctx.env) {
                Ok(p) => p,
                Err(_) => return Response::error("Billing not configured", 503),
            };

            let (plan, resolved_interval) = if let Ok(catalog) = product_catalog_from_env(&ctx.env)
            {
                let price_map = build_price_map(&polar, &catalog).await;
                plan_from_price_id(&price_id, &price_map)
            } else {
                ("free".to_string(), interval.to_string())
            };

            let actual_billing_account_id = if !billing_account_id.is_empty() {
                billing_account_id
            } else {
                console_error!(
                    "No billing account ID provided in subscription_updated webhook. Customer ID: {}",
                    customer_id
                );
                return Response::error("No billing account ID provided", 400);
            };

            db::upsert_subscription(
                &db,
                &actual_billing_account_id,
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
            .await?;
            db::update_billing_account_tier(&db, &actual_billing_account_id, &plan).await?;
        }
        "subscription_canceled" | "subscription_revoked" => {
            let subscription_id = body["subscription_id"].as_str().unwrap_or("").to_string();
            let customer_id = body["customer_id"].as_str().unwrap_or("").to_string();
            let billing_account_id = body["billing_account_id"]
                .as_str()
                .unwrap_or("")
                .to_string();

            console_log!(
                "Processing subscription cancellation: subscription_id={}, billing_account_id={}, customer_id={}",
                subscription_id,
                billing_account_id,
                customer_id
            );

            if !billing_account_id.is_empty() {
                db::mark_subscription_canceled(&db, &subscription_id, now).await?;
                db::update_billing_account_tier(&db, &billing_account_id, "free").await?;
                console_log!(
                    "Successfully marked subscription as canceled and updated billing account to free"
                );
            } else {
                console_error!(
                    "No billing account ID provided in webhook for subscription cancellation. Customer ID: {}",
                    customer_id
                );
            }
        }
        _ => {}
    }

    Response::from_json(&serde_json::json!({ "received": true }))
}
