use crate::auth;
use crate::billing::polar::polar_client_from_env;
use crate::billing::provider::BillingProvider;
use crate::repositories::{BillingRepository, SettingsRepository};
use worker::d1::D1Database;
use worker::*;

fn get_frontend_url(env: &Env) -> String {
    env.var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
}

#[utoipa::path(
    post,
    path = "/api/billing/checkout",
    tag = "Billing",
    summary = "Create checkout session",
    description = "Creates a Polar Checkout session for the given product/price ID and returns the hosted checkout URL. Optionally accepts a discount code. The caller must be the billing account owner",
    responses(
        (status = 200, description = "Checkout URL"),
        (status = 400, description = "Missing product_id or invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 502, description = "Polar API error"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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
    let settings = SettingsRepository::new().get_all_settings(&db).await?;

    let polar_product_id = match settings.get(product_id_key) {
        Some(id) => id.clone(),
        None => {
            console_error!("[checkout] Product ID not found for plan: {}", plan);
            return Response::error("Plan not configured", 503);
        }
    };

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

    let billing_repo = BillingRepository::new();

    let billing_account = match billing_repo.get_for_user(&db, &user_ctx.user_id).await? {
        Some(ba) => ba,
        None => {
            let org = crate::db::create_default_org(&db, &user_ctx.user_id, "Personal").await?;
            match billing_repo
                .get_by_id(&db, org.billing_account_id.as_deref().unwrap_or(""))
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

    let polar_customer_id = if let Some(existing_id) = &billing_account.provider_customer_id {
        Some(existing_id.clone())
    } else {
        match polar
            .find_customer_by_external_id(&billing_account.id)
            .await
        {
            Ok(Some(cid)) => {
                if let Err(e) = billing_repo
                    .update_provider_customer_id(&db, &billing_account.id, &cid)
                    .await
                {
                    console_error!("[checkout] Failed to store found customer_id: {}", e);
                }
                Some(cid)
            }
            Ok(None) => None,
            Err(e) => {
                console_error!(
                    "[checkout] Failed to query Polar for existing customer: {}",
                    e
                );
                None
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
        customer_id: polar_customer_id,
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
