/// Usage handler
///
/// GET /api/usage — returns tier, limits, current monthly usage, and next reset.
use crate::auth;
use crate::db;
use crate::models::Tier;
use crate::repositories::{AnalyticsRepository, TagRepository};
use crate::utils::AppError;
use chrono::{Datelike, TimeZone};
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/usage",
    tag = "Usage",
    summary = "Get current usage",
    description = "Returns the authenticated organization's tier, feature limits, current monthly link usage, tag count, and the date/time of the next monthly counter reset",
    responses(
        (status = 200, description = "Usage and limits for the current org"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_usage(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = &user_ctx.org_id;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let org = db::get_org_by_id(&db, org_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

    // Get billing account for usage tracking
    let billing_account_id = org
        .billing_account_id
        .as_ref()
        .ok_or_else(|| AppError::Internal("Organization has no billing account".to_string()))?;
    let billing_account = db::get_billing_account(&db, billing_account_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Billing account not found".to_string()))?;

    let tier = Tier::from_str_value(&billing_account.tier).unwrap_or(Tier::Free);
    let limits = tier.limits();

    // Use billing account monthly counter for efficiency
    let now = chrono::Utc::now();
    let year_month = format!("{}-{:02}", now.year(), now.month());
    let analytics_repo = AnalyticsRepository::new();
    let links_created_this_month = analytics_repo
        .get_monthly_counter_for_billing_account(&db, &billing_account.id, &year_month)
        .await?;

    // Get tag count for the billing account
    let tags_count = TagRepository::new()
        .count_distinct_tags_for_billing_account(&db, &billing_account.id)
        .await?;

    // Calculate next reset time (first day of next month at midnight UTC)
    let now = chrono::Utc::now();
    let next_reset = chrono::Utc
        .with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
        .single()
        .unwrap_or_else(chrono::Utc::now);
    let next_reset_timestamp = next_reset.timestamp();

    let usage = serde_json::json!({
        "tier": tier.as_str(),
        "limits": {
            "max_links_per_month": limits.max_links_per_month,
            "analytics_retention_days": limits.analytics_retention_days,
            "allow_custom_short_code": limits.allow_custom_short_code,
            "allow_utm_parameters": limits.allow_utm_parameters,
            "allow_query_forwarding": limits.allow_query_forwarding,
            "max_tags": limits.max_tags,
        },
        "usage": {
            "links_created_this_month": links_created_this_month,
            "tags_count": tags_count,
        },
        "next_reset": {
            "utc": next_reset.to_rfc3339(),
            "timestamp": next_reset_timestamp,
        }
    });

    Ok(Response::from_json(&usage)?)
}
