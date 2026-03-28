use crate::db;
use crate::utils::short_code::{
    DEFAULT_MIN_CUSTOM_CODE_LENGTH, DEFAULT_MIN_RANDOM_CODE_LENGTH, DEFAULT_SYSTEM_MIN_CODE_LENGTH,
};
use worker::*;

/// Public endpoint to expose non-sensitive settings
/// Returns founder pricing status, discount amounts needed for pricing page, and minimum short code lengths
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

    // Helper to parse setting as i64 (with default)
    let get_setting_i64 = |key: &str, default: i64| -> i64 {
        settings
            .get(key)
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(default)
    };

    let raw_min_random = get_setting_i64(
        "min_random_code_length",
        DEFAULT_MIN_RANDOM_CODE_LENGTH as i64,
    );
    let raw_min_custom = get_setting_i64(
        "min_custom_code_length",
        DEFAULT_MIN_CUSTOM_CODE_LENGTH as i64,
    );
    let system_min = get_setting_i64(
        "system_min_code_length",
        DEFAULT_SYSTEM_MIN_CODE_LENGTH as i64,
    );

    let effective_min_random = raw_min_random.max(system_min);
    let effective_min_custom = raw_min_custom.max(system_min);

    Response::from_json(&serde_json::json!({
        "founder_pricing_active": founder_pricing_active,
        "min_random_code_length": effective_min_random,
        "min_custom_code_length": effective_min_custom,
        "system_min_code_length": system_min,
        "active_discount_amount_pro_monthly": get_setting_i64("active_discount_amount_pro_monthly", 0),
        "active_discount_amount_pro_annual": get_setting_i64("active_discount_amount_pro_annual", 0),
        "active_discount_amount_business_monthly": get_setting_i64("active_discount_amount_business_monthly", 0),
        "active_discount_amount_business_annual": get_setting_i64("active_discount_amount_business_annual", 0),
    }))
}
