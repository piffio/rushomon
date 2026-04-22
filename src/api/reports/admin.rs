/// Admin abuse report handlers
///
/// GET  /api/admin/reports               — list with pagination and status filter
/// GET  /api/admin/reports/:id           — single report
/// PUT  /api/admin/reports/:id           — update status / notes
/// GET  /api/admin/reports/pending/count — badge count
use crate::auth;
use crate::services::ReportService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/admin/reports",
    tag = "Admin",
    summary = "List abuse reports",
    params(
        ("page" = Option<u32>, Query, description = "Page number"),
        ("limit" = Option<u32>, Query, description = "Items per page (10-100)"),
        ("status" = Option<String>, Query, description = "Filter by status: pending, reviewed, dismissed"),
    ),
    responses(
        (status = 200, description = "Paginated list of reports"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_get_reports(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let url = req.url()?;
    let mut page: u32 = 1;
    let mut limit: u32 = 50;
    let mut status_filter: Option<String> = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "page" => page = value.parse().unwrap_or(1),
            "limit" => limit = value.parse().unwrap_or(50),
            "status" => status_filter = Some(value.to_string()),
            _ => {}
        }
    }
    if page < 1 {
        page = 1;
    }
    limit = limit.clamp(10, 100);

    let (reports, total) = ReportService::new()
        .list_reports(&db, page, limit, status_filter.as_deref())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to retrieve reports: {}", e)))?;

    Ok(Response::from_json(&serde_json::json!({
        "reports": reports,
        "pagination": {
            "page": page,
            "limit": limit,
            "total": total,
            "pages": (total as f64 / limit as f64).ceil() as u32
        }
    }))?)
}

#[utoipa::path(
    get,
    path = "/api/admin/reports/{id}",
    tag = "Admin",
    summary = "Get a single abuse report",
    params(("id" = String, Path, description = "Report ID")),
    responses(
        (status = 200, description = "Report details"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 404, description = "Report not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_get_report(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_get(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_get(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let report_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing report ID".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let report = ReportService::new().get_report(&db, &report_id).await?;
    Ok(Response::from_json(&report)?)
}

#[utoipa::path(
    put,
    path = "/api/admin/reports/{id}",
    tag = "Admin",
    summary = "Update report status",
    params(("id" = String, Path, description = "Report ID")),
    responses(
        (status = 200, description = "Report updated"),
        (status = 400, description = "Invalid status"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_update_report(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_update(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_update(mut req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let report_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing report ID".to_string()))?
        .to_string();

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    let status = body
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'status' field".to_string()))?;

    let admin_notes = body.get("admin_notes").and_then(|v| v.as_str());
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    ReportService::new()
        .update_report_status(&db, &report_id, status, &user_ctx.user_id, admin_notes)
        .await?;

    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Report status updated successfully"
    }))?)
}

#[utoipa::path(
    get,
    path = "/api/admin/reports/pending/count",
    tag = "Admin",
    summary = "Get pending reports count",
    responses(
        (status = 200, description = "Count of pending reports"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_get_pending_reports_count(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    Ok(inner_count(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_count(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let count = ReportService::new()
        .count_pending_reports(&db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get pending reports count: {}", e)))?;

    Ok(Response::from_json(&serde_json::json!({ "count": count }))?)
}
