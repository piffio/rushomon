use crate::auth;
use crate::billing::polar::polar_client_from_env;
use crate::repositories::BillingRepository;
use worker::d1::D1Database;
use worker::*;

fn get_frontend_url(env: &Env) -> String {
    env.var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
}

#[utoipa::path(
    post,
    path = "/api/billing/portal",
    tag = "Billing",
    summary = "Get customer portal URL",
    description = "Generates a Polar Customer Portal URL for the authenticated user. The frontend should redirect the user to the returned URL to manage their subscription, payment methods, and invoices",
    responses(
        (status = 200, description = "Portal URL"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "No billing account or customer ID found"),
        (status = 502, description = "Polar API error"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

    let billing_account = match BillingRepository::new()
        .get_for_user(&db, &user_ctx.user_id)
        .await?
    {
        Some(ba) => ba,
        None => return Response::error("No billing account found", 400),
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
