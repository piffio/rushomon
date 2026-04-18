/// Org-level analytics handler
///
/// GET /api/analytics/org — aggregate click analytics for the entire organization.
use crate::auth;
use crate::models::Tier;
use crate::repositories::{AnalyticsRepository, BillingRepository, OrgRepository};
use crate::services::analytics_service::{apply_analytics_gating, parse_time_range_from_query};
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
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let url = req.url()?;
    let query = url.query().unwrap_or("");

    // Parse time range: ?days=N, ?start=UNIX&end=UNIX
    let time_range = parse_time_range_from_query(query);

    let (mut start, end) = time_range.calculate_timestamps();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let org_repo = OrgRepository::new();
    let billing_repo = BillingRepository::new();

    // Resolve tier from billing account
    let org = org_repo
        .get_by_id(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    let tier = if let Some(ref billing_account_id) = org.billing_account_id {
        billing_repo
            .get_by_id(&db, billing_account_id)
            .await?
            .and_then(|ba| Tier::from_str_value(&ba.tier))
            .unwrap_or(Tier::Free)
    } else {
        Tier::Free
    };

    let now = crate::models::analytics::now_timestamp();
    let gating_result = apply_analytics_gating(tier, start, end, now);

    // Use the adjusted start date for queries
    start = gating_result.adjusted_start;

    let repo = AnalyticsRepository::new();

    // Run queries sequentially (D1 limitation)
    let total_clicks = repo
        .get_org_total_clicks_in_range(&db, org_id, start, end)
        .await?;
    let unique_links_clicked = repo
        .get_org_unique_links_clicked(&db, org_id, start, end)
        .await?;
    let clicks_over_time = repo
        .get_org_clicks_over_time(&db, org_id, start, end)
        .await?;
    let top_links = repo.get_org_top_links(&db, org_id, start, end, 10).await?;
    let top_referrers = repo
        .get_org_top_referrers(&db, org_id, start, end, 10)
        .await?;
    let top_countries = repo
        .get_org_top_countries(&db, org_id, start, end, 10)
        .await?;
    let top_user_agents = repo
        .get_org_top_user_agents(&db, org_id, start, end, 20)
        .await?;

    let response = crate::models::analytics::OrgAnalyticsResponse {
        total_clicks,
        unique_links_clicked,
        clicks_over_time,
        top_links,
        top_referrers,
        top_countries,
        top_user_agents,
        analytics_gated: if gating_result.gated {
            Some(true)
        } else {
            None
        },
        gated_reason: gating_result.reason,
    };

    Response::from_json(&response)
}
