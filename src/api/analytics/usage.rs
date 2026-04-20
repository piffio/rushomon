/// Usage handler
///
/// GET /api/usage — returns tier, limits, current monthly usage, and next reset.
use crate::auth;
use crate::services::analytics_service::get_usage;
use crate::utils::AppError;
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

    let usage_info = get_usage(&db, org_id).await?;

    let usage = serde_json::json!({
        "tier": usage_info.tier,
        "limits": {
            "max_links_per_month": usage_info.limits.max_links_per_month,
            "analytics_retention_days": usage_info.limits.analytics_retention_days,
            "allow_custom_short_code": usage_info.limits.allow_custom_short_code,
            "allow_utm_parameters": usage_info.limits.allow_utm_parameters,
            "allow_query_forwarding": usage_info.limits.allow_query_forwarding,
            "max_tags": usage_info.limits.max_tags,
        },
        "usage": {
            "links_created_this_month": usage_info.links_created_this_month,
            "tags_count": usage_info.tags_count,
        },
        "next_reset": {
            "utc": usage_info.next_reset_utc,
            "timestamp": usage_info.next_reset_timestamp,
        }
    });

    Ok(Response::from_json(&usage)?)
}
