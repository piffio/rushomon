/// Admin product & discount handlers
///
/// GET  /api/admin/discounts        — list Polar discounts
/// GET  /api/admin/products         — list Polar products
/// POST /api/admin/products/sync    — fetch from Polar (dry-run, no DB write)
/// POST /api/admin/products/save    — fetch from Polar and cache in DB
use crate::auth;
use crate::billing::polar::polar_client_from_env;
use crate::services::ProductService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/admin/discounts",
    tag = "Admin",
    summary = "List Polar discounts",
    responses(
        (status = 200, description = "Discounts from Polar"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 503, description = "Billing not configured"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_list_discounts(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list_discounts(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list_discounts(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let polar = polar_client_from_env(&ctx.env)
        .map_err(|_| AppError::Internal("Billing not configured".to_string()))?;

    polar
        .list_discounts()
        .await
        .map(|d| Response::from_json(&d))?
        .map_err(AppError::from)
}

#[utoipa::path(
    get,
    path = "/api/admin/products",
    tag = "Admin",
    summary = "List Polar products",
    responses(
        (status = 200, description = "Products from Polar"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 503, description = "Billing not configured"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_list_products(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_list_products(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_list_products(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let polar = polar_client_from_env(&ctx.env)
        .map_err(|_| AppError::Internal("Billing not configured".to_string()))?;

    let products = polar.list_products().await?;
    Ok(Response::from_json(&products)?)
}

#[utoipa::path(
    post,
    path = "/api/admin/products/sync",
    tag = "Admin",
    summary = "Sync products from Polar",
    description = "Fetches the product list from the Polar API (dry-run: no database write)",
    responses(
        (status = 200, description = "Products fetched from Polar"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 503, description = "Billing not configured"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_sync_products(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_sync_products(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_sync_products(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let polar = polar_client_from_env(&ctx.env)
        .map_err(|_| AppError::Internal("Billing not configured".to_string()))?;

    let products = polar
        .list_products()
        .await
        .map_err(|_| AppError::Internal("Failed to fetch products from Polar".to_string()))?;

    let count = products["items"].as_array().map(|a| a.len()).unwrap_or(0);
    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Products fetched successfully",
        "products_count": count,
    }))?)
}

#[utoipa::path(
    post,
    path = "/api/admin/products/save",
    tag = "Admin",
    summary = "Sync and cache products from Polar",
    description = "Fetches products from Polar and replaces the local cached_products table",
    responses(
        (status = 200, description = "Products cached in database"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Admin required"),
        (status = 503, description = "Billing not configured"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_admin_save_products(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_save_products(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_save_products(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    auth::require_admin(&user_ctx)?;

    let polar = polar_client_from_env(&ctx.env)
        .map_err(|_| AppError::Internal("Billing not configured".to_string()))?;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let products = ProductService::new()
        .sync_from_polar(&db, &polar)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to sync products: {}", e)))?;

    let count = products["items"].as_array().map(|a| a.len()).unwrap_or(0);
    Ok(Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Products configuration saved and cached successfully",
        "products_count": count,
    }))?)
}
