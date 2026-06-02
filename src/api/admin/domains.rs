/// Admin custom domain handlers
///
/// GET /api/admin/domains — List all custom domains
/// POST /api/admin/domains/poll — Manually poll all pending custom domains
use crate::auth;
use crate::models::CustomDomain;
use crate::repositories::CustomDomainRepository;
use crate::scheduled::poll_domain_status::poll_pending_domains;
use crate::utils::AppError;
use utoipa::ToSchema;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/admin/domains",
    tag = "Admin",
    summary = "List all custom domains",
    responses(
        (status = 200, description = "List of custom domains", body = Vec<CustomDomain>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_list_domains(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_handle_admin_list_domains(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_handle_admin_list_domains(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx).map_err(AppError::from)?;

    let db = ctx
        .env
        .get_binding::<D1Database>("rushomon")
        .map_err(|_| AppError::Internal("Database not available".to_string()))?;

    let domains = CustomDomainRepository::new().get_all(&db).await.map_err(|e| {
        console_log!("{}", serde_json::json!({"event": "admin_list_domains_failed", "error": e.to_string(), "level": "error"}));
        AppError::Internal("Failed to list custom domains".to_string())
    })?;

    Ok(Response::from_json(&domains)?)
}

#[utoipa::path(
    post,
    path = "/api/admin/domains/poll",
    tag = "Admin",
    summary = "Poll pending custom domains",
    responses(
        (status = 200, description = "Polling complete", body = PollDomainsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_poll_domains(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_handle_admin_poll_domains(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

#[derive(serde::Serialize, ToSchema)]
struct PollDomainsResponse {
    success: bool,
    domains_processed: u64,
    status_changes: u64,
    message: String,
}

async fn inner_handle_admin_poll_domains(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx).map_err(AppError::from)?;

    let db = ctx
        .env
        .get_binding::<D1Database>("rushomon")
        .map_err(|_| AppError::Internal("Database not available".to_string()))?;
    let kv = ctx
        .kv("URL_MAPPINGS")
        .map_err(|e| AppError::Internal(format!("KV store not available: {}", e)))?;

    let (processed, changes) = poll_pending_domains(&db, &kv, &ctx.env).await;

    let message = if processed == 0 {
        "No pending domains found".to_string()
    } else if changes == 0 {
        format!("Polled {} domain(s), no status changes", processed)
    } else {
        format!(
            "Polled {} domain(s), {} status change(s)",
            processed, changes
        )
    };

    Ok(Response::from_json(&PollDomainsResponse {
        success: true,
        domains_processed: processed as u64,
        status_changes: changes as u64,
        message,
    })?)
}
