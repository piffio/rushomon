/// Per-link analytics handler
///
/// GET /api/links/:id/analytics — click analytics for a single link.
use crate::auth;
use crate::models::{LinkAnalyticsResponse, TimeRange};
use crate::services::analytics_service::get_link_analytics;
use crate::utils::AppError;
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
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = &user_ctx.org_id;

    let link_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing link ID".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Parse time range from query parameters
    // Support both new format (TimeRange enum) and legacy format (start/end timestamps)
    let url = req.url()?;
    let query = url.query().unwrap_or("");

    // Try to parse as new TimeRange format first
    let time_range = if let Ok(time_range_str) = extract_query_param(query, "time_range") {
        // New format: JSON TimeRange object
        serde_json::from_str::<TimeRange>(&time_range_str)
            .map_err(|e| AppError::BadRequest(format!("Invalid time_range parameter: {}", e)))?
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

    let analytics_result = get_link_analytics(&db, link_id, org_id, time_range).await?;

    let response = LinkAnalyticsResponse {
        link: analytics_result.link,
        total_clicks_in_range: analytics_result.total_clicks,
        clicks_over_time: analytics_result.clicks_over_time,
        top_referrers: analytics_result.referrers,
        top_countries: analytics_result.countries,
        top_user_agents: analytics_result.user_agents,
        analytics_gated: if analytics_result.gated {
            Some(true)
        } else {
            None
        },
        gated_reason: analytics_result.gated_reason,
    };

    Ok(Response::from_json(&response)?)
}
