/// Public pricing handler
///
/// GET /api/billing/pricing — returns cached product data for the pricing page.
/// This is a public endpoint (no authentication required).
use crate::services::ProductService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

pub async fn handle_billing_pricing(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(ctx: RouteContext<()>) -> Result<Response, AppError> {
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let products = ProductService::new().list_for_pricing(&db).await?;
    Ok(Response::from_json(
        &serde_json::json!({ "products": products }),
    )?)
}
