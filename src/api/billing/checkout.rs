use crate::auth;
use crate::services::BillingService;
use worker::d1::D1Database;
use worker::*;

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
        Some(p) => p.to_string(),
        None => {
            console_error!("[checkout] plan is required");
            return Response::error("plan is required", 400);
        }
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match BillingService::new()
        .create_checkout(&db, &ctx.env, &user_ctx.user_id, &plan)
        .await
    {
        Ok(session) => Response::from_json(&serde_json::json!({ "url": session.url })),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Invalid plan") {
                return Response::error("Invalid plan", 400);
            }
            if msg.contains("Plan not configured") {
                return Response::error("Plan not configured", 503);
            }
            if msg.contains("Billing not configured") {
                return Response::error("Billing not configured", 503);
            }
            console_error!("[checkout] error: {}", e);
            Response::error("Failed to create checkout session", 500)
        }
    }
}
