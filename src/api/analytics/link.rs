/// Per-link analytics handler
///
/// GET /api/links/:id/analytics — click analytics for a single link.
use crate::auth;
use crate::db;
use crate::models::{LinkAnalyticsResponse, Tier, TimeRange};
use crate::repositories::AnalyticsRepository;
use crate::services::analytics_service::apply_analytics_gating;
use worker::d1::D1Database;
use worker::*;

/// Helper function to extract query parameters
fn extract_query_param(query: &str, name: &str) -> Result<String> {
    query
        .split('&')
        .find_map(|pair| {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 && parts[0] == name {
                let decoded = urlencoding::decode(parts[1]).ok()?;
                Some(decoded.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| Error::RustError(format!("Missing {} parameter", name)))
}

#[utoipa::path(
    get,
    path = "/api/links/{id}/analytics",
    tag = "Links",
    summary = "Get link analytics",
    description = "Returns click analytics for a single link. The time range is capped according to the organization's tier retention window (7 days for Free, 365 days for Pro, unlimited for Business/Unlimited)",
    params(
        ("id" = String, Path, description = "Link ID"),
        ("days" = Option<i64>, Query, description = "Number of days to look back (default: 7)"),
        ("start" = Option<i64>, Query, description = "Unix timestamp range start (alternative to days)"),
        ("end" = Option<i64>, Query, description = "Unix timestamp range end"),
    ),
    responses(
        (status = 200, description = "Analytics data for the link"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Link not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_link_analytics(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Authenticate request
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing link ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify link exists and belongs to org
    let link = match db::get_link_by_id(&db, link_id, org_id).await? {
        Some(link) => link,
        None => return Response::error("Link not found", 404),
    };

    // Parse time range from query parameters
    // Support both new format (TimeRange enum) and legacy format (start/end timestamps)
    let url = req.url()?;
    let query = url.query().unwrap_or("");

    // Try to parse as new TimeRange format first
    let time_range = if let Ok(time_range_str) = extract_query_param(query, "time_range") {
        // New format: JSON TimeRange object
        serde_json::from_str::<TimeRange>(&time_range_str)
            .map_err(|e| Error::RustError(format!("Invalid time_range parameter: {}", e)))?
    } else if let Ok(days_str) = extract_query_param(query, "days") {
        // Simple days parameter (e.g., ?days=7)
        let days = days_str.parse::<i64>().unwrap_or(7);
        TimeRange::Days { value: days }
    } else {
        // Legacy format: start/end timestamps for backward compatibility
        let now = crate::models::analytics::now_timestamp();

        let start_legacy = query
            .split('&')
            .find(|s| s.starts_with("start="))
            .and_then(|s| s.split('=').nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| now - 7 * 24 * 60 * 60); // Default: 7 days ago

        let end_legacy = query
            .split('&')
            .find(|s| s.starts_with("end="))
            .and_then(|s| s.split('=').nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(now);

        TimeRange::Custom {
            start: start_legacy,
            end: end_legacy,
        }
    };

    // Calculate timestamps using backend logic (eliminates clock skew)
    let (mut start, end) = time_range.calculate_timestamps();

    // Check tier-based analytics limits from billing account
    let org = db::get_org_by_id(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    // Get tier from billing account (all orgs should have billing accounts after migration)
    let tier = if let Some(ref billing_account_id) = org.billing_account_id {
        db::get_billing_account(&db, billing_account_id)
            .await?
            .and_then(|ba| Tier::from_str_value(&ba.tier))
            .unwrap_or(Tier::Free)
    } else {
        Tier::Free
    };

    let now = crate::models::analytics::now_timestamp();
    let gating_result = apply_analytics_gating(tier, start, end, now);
    start = gating_result.adjusted_start;

    // If analytics are gated, return empty data with gating info
    if gating_result.gated {
        let response = LinkAnalyticsResponse {
            link,
            total_clicks_in_range: 0,
            clicks_over_time: vec![],
            top_referrers: vec![],
            top_countries: vec![],
            top_user_agents: vec![],
            analytics_gated: Some(true),
            gated_reason: gating_result.reason,
        };
        return Response::from_json(&response);
    }

    let repo = AnalyticsRepository::new();

    // Run analytics queries sequentially (D1 limitation)
    let total_clicks_in_range = repo
        .get_link_total_clicks_in_range(&db, link_id, org_id, start, end)
        .await?;
    let clicks_over_time = repo
        .get_link_clicks_over_time(&db, link_id, org_id, start, end)
        .await?;
    let top_referrers = repo
        .get_link_top_referrers(&db, link_id, org_id, start, end, 10)
        .await?;
    let top_countries = repo
        .get_link_top_countries(&db, link_id, org_id, start, end, 10)
        .await?;
    let top_user_agents = repo
        .get_link_top_user_agents(&db, link_id, org_id, start, end, 20)
        .await?;

    let response = LinkAnalyticsResponse {
        link,
        total_clicks_in_range,
        clicks_over_time,
        top_referrers,
        top_countries,
        top_user_agents,
        analytics_gated: None,
        gated_reason: None,
    };

    Response::from_json(&response)
}
