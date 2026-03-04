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

    // Get Polar org slug
    let polar_org_slug = ctx
        .env
        .var("POLAR_ORG_SLUG")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "".to_string());

    // Get active discount IDs from DB (admin-configurable at runtime)
    let active_discount_pro_monthly = db::get_setting(&db, "active_discount_pro_monthly")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let active_discount_pro_annual = db::get_setting(&db, "active_discount_pro_annual")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let active_discount_business_monthly = db::get_setting(&db, "active_discount_business_monthly")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let active_discount_business_annual = db::get_setting(&db, "active_discount_business_annual")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    // Get cached discount amounts in cents (saved by admin when assigning a discount)
    let amount_pro_monthly = db::get_setting(&db, "active_discount_amount_pro_monthly")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);

    let amount_pro_annual = db::get_setting(&db, "active_discount_amount_pro_annual")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);

    let amount_business_monthly = db::get_setting(&db, "active_discount_amount_business_monthly")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);

    let amount_business_annual = db::get_setting(&db, "active_discount_amount_business_annual")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);

    // Get product IDs
    let product_pro_monthly = db::get_setting(&db, "product_pro_monthly_id")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let product_pro_annual = db::get_setting(&db, "product_pro_annual_id")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let product_business_monthly = db::get_setting(&db, "product_business_monthly_id")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let product_business_annual = db::get_setting(&db, "product_business_annual_id")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    Response::from_json(&serde_json::json!({
        "founder_pricing_active": founder_pricing_active,
        "polar_org_slug": polar_org_slug,
        "active_discount_pro_monthly": active_discount_pro_monthly,
        "active_discount_pro_annual": active_discount_pro_annual,
        "active_discount_business_monthly": active_discount_business_monthly,
        "active_discount_business_annual": active_discount_business_annual,
        "active_discount_amount_pro_monthly": amount_pro_monthly,
        "active_discount_amount_pro_annual": amount_pro_annual,
        "active_discount_amount_business_monthly": amount_business_monthly,
        "active_discount_amount_business_annual": amount_business_annual,
        "product_pro_monthly_id": product_pro_monthly,
        "product_pro_annual_id": product_pro_annual,
        "product_business_monthly_id": product_business_monthly,
        "product_business_annual_id": product_business_annual,
    }))
}
