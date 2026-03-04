use crate::db;
use worker::*;

/// Public endpoint to expose non-sensitive settings
/// Returns founder pricing status and other public configuration
pub async fn handle_get_public_settings(
    _req: Request,
    ctx: worker::RouteContext<()>,
) -> Result<Response> {
    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

    // Get all settings in a single query (much faster than running individual queries)
    let settings = db::get_all_settings(&db).await?;

    // Get founder pricing status from settings
    let founder_pricing_active = settings
        .get("founder_pricing_active")
        .map(|v| v == "true")
        .unwrap_or(false);

    // Get Polar org slug from env
    let polar_org_slug = ctx
        .env
        .var("POLAR_ORG_SLUG")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "".to_string());

    // Helper to get setting or default
    let get_setting = |key: &str| -> String { settings.get(key).cloned().unwrap_or_default() };

    // Helper to parse setting as i64
    let get_setting_i64 = |key: &str| -> i64 {
        settings
            .get(key)
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0)
    };

    Response::from_json(&serde_json::json!({
        "founder_pricing_active": founder_pricing_active,
        "polar_org_slug": polar_org_slug,
        "active_discount_pro_monthly": get_setting("active_discount_pro_monthly"),
        "active_discount_pro_annual": get_setting("active_discount_pro_annual"),
        "active_discount_business_monthly": get_setting("active_discount_business_monthly"),
        "active_discount_business_annual": get_setting("active_discount_business_annual"),
        "active_discount_amount_pro_monthly": get_setting_i64("active_discount_amount_pro_monthly"),
        "active_discount_amount_pro_annual": get_setting_i64("active_discount_amount_pro_annual"),
        "active_discount_amount_business_monthly": get_setting_i64("active_discount_amount_business_monthly"),
        "active_discount_amount_business_annual": get_setting_i64("active_discount_amount_business_annual"),
        "product_pro_monthly_id": get_setting("product_pro_monthly_id"),
        "product_pro_annual_id": get_setting("product_pro_annual_id"),
        "product_business_monthly_id": get_setting("product_business_monthly_id"),
        "product_business_annual_id": get_setting("product_business_annual_id"),
    }))
}
