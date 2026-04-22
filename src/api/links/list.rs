use crate::models::{PaginatedResponse, PaginationMeta};
use crate::services::LinkService;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/links",
    tag = "Links",
    summary = "List links",
    description = "Returns a paginated list of links for the authenticated organization",
    params(
        ("page" = Option<i64>, Query, description = "Page number (default: 1)"),
        ("limit" = Option<i64>, Query, description = "Items per page (default: 50, max: 100)"),
        ("tag" = Option<String>, Query, description = "Filter by tag"),
        ("search" = Option<String>, Query, description = "Search by title or URL"),
        ("sort" = Option<String>, Query, description = "Sort field: created_at, click_count"),
        ("order" = Option<String>, Query, description = "Sort order: asc, desc"),
    ),
    responses(
        (status = 200, description = "Paginated list of links"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_list_links(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list_links(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list_links(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, crate::utils::AppError> {
    let user_ctx = crate::auth::authenticate_request(&req, &ctx).await?;
    let org_id = &user_ctx.org_id;

    let url = req
        .url()
        .map_err(|e| crate::utils::AppError::Internal(format!("Invalid URL: {}", e)))?;
    let query = url.query().unwrap_or("");

    let page: i64 = query
        .split('&')
        .find(|s| s.starts_with("page="))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);

    let limit: i64 = query
        .split('&')
        .find(|s| s.starts_with("limit="))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(20)
        .min(100);

    let search = query
        .split('&')
        .find(|s| s.starts_with("search="))
        .and_then(|s| s.split('=').nth(1))
        .map(|s| urlencoding::decode(s).unwrap_or_default().into_owned())
        .filter(|s| !s.trim().is_empty() && s.len() <= 100);

    let status_filter = query
        .split('&')
        .find(|s| s.starts_with("status="))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| match s {
            "active" | "disabled" => Some(s),
            _ => None,
        });

    let sort = query
        .split('&')
        .find(|s| s.starts_with("sort="))
        .and_then(|s| s.split('=').nth(1))
        .map(|s| match s {
            "clicks" | "updated" | "title" | "code" => s,
            _ => "created",
        })
        .unwrap_or("created");

    let tags_filter: Vec<String> = query
        .split('&')
        .find(|s| s.starts_with("tags="))
        .and_then(|s| s.split('=').nth(1))
        .map(|s| {
            let s_plus_fixed = s.replace('+', " ");
            urlencoding::decode(&s_plus_fixed)
                .unwrap_or_default()
                .into_owned()
        })
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let tags_filter_opt: Option<&[String]> = if tags_filter.is_empty() {
        None
    } else {
        Some(&tags_filter)
    };

    let offset = (page - 1) * limit;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let service = LinkService::new();

    let (links, total, stats_json) = service
        .list_links(
            &db,
            org_id,
            search.as_deref(),
            status_filter,
            sort,
            limit,
            offset,
            tags_filter_opt,
        )
        .await?;

    let pagination = PaginationMeta::new(page, limit, total);
    let response = PaginatedResponse::with_stats(links, pagination, stats_json);

    Ok(Response::from_json(&response)?)
}
