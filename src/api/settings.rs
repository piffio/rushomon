use crate::db;
use worker::*;

/// Public endpoint to expose non-sensitive settings
/// Returns founder pricing status and other public configuration
pub async fn handle_get_public_settings(
    _req: Request,
    ctx: worker::RouteContext<()>,
) -> Result<Response> {
    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

    // Get founder pricing status from settings table
    let founder_pricing_active = match db::get_setting(&db, "founder_pricing_active").await {
        Ok(Some(value)) => value == "true",
        _ => false, // Default to false if setting not found
    };

    // Get Polar org slug if available
    let polar_org_slug = ctx
        .env
        .var("POLAR_ORG_SLUG")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "".to_string());

    Response::from_json(&serde_json::json!({
        "founder_pricing_active": founder_pricing_active,
        "polar_org_slug": polar_org_slug,
    }))
}
