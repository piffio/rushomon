use crate::auth;
use crate::services::BillingService;
use worker::d1::D1Database;
use worker::*;

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

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    match BillingService::new()
        .create_portal_session(&db, &ctx.env, &user_ctx.user_id)
        .await
    {
        Ok(session) => Response::from_json(&serde_json::json!({ "url": session.url })),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("No billing account found") {
                return Response::error(&msg, 400);
            }
            if msg.contains("Billing not configured") {
                return Response::error("Billing not configured", 503);
            }
            console_error!("[portal] error: {}", e);
            Response::error("Failed to create portal session", 500)
        }
    }
}
