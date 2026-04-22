/// Org-level analytics handler
///
/// GET /api/analytics/org — aggregate click analytics for the entire organization.
use crate::auth;
use crate::services::analytics_service::{get_org_analytics, parse_time_range_from_query};
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/analytics/org",
    tag = "Analytics",
    summary = "Get org-level analytics",
    description = "Returns aggregate click analytics for the entire organization. Includes total clicks, unique links clicked, clicks over time, top links, referrers, countries, and user agents. The time range is capped by tier retention (7 days Free, 365 days Pro, unlimited Business/Unlimited)",
    params(
        ("days" = Option<i64>, Query, description = "Number of days to look back (default: 7)"),
        ("start" = Option<i64>, Query, description = "Unix timestamp range start (alternative to days)"),
        ("end" = Option<i64>, Query, description = "Unix timestamp range end"),
    ),
    responses(
        (status = 200, description = "Org analytics response with clicks, top links, referrers, countries, and user agents"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Organization not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_org_analytics(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await.unwrap_or_else(|e| e.into_response()))
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = &user_ctx.org_id;

    let url = req.url()?;
    let query = url.query().unwrap_or("");

    // Parse time range: ?days=N, ?start=UNIX&end=UNIX
    let time_range = parse_time_range_from_query(query);

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let analytics_result = get_org_analytics(&db, org_id, time_range).await?;

    let response = crate::models::analytics::OrgAnalyticsResponse {
        total_clicks: analytics_result.total_clicks,
        unique_links_clicked: analytics_result.unique_links,
        clicks_over_time: analytics_result.clicks_over_time,
        top_links: analytics_result.top_links,
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
