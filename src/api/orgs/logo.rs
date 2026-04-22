/// Org logo handlers
///
/// POST   /api/orgs/{id}/logo - Upload org logo
/// GET    /api/orgs/{id}/logo - Get org logo
/// DELETE /api/orgs/{id}/logo - Delete org logo
use crate::auth;
use crate::services::OrgService;
use crate::utils::AppError;
use worker::d1::D1Database;
use worker::*;

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/logo",
    tag = "Organizations",
    summary = "Upload org logo",
    description = "Uploads an organization logo. Accepts multipart/form-data with a field named 'logo'. Max 500 KB. Accepted formats: image/png, image/jpeg, image/webp, image/svg+xml. Requires owner or admin role and Pro+ tier",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Logo uploaded, returns the URL"),
        (status = 400, description = "Missing file, too large, or unsupported format"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required, or Pro+ required"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_upload_org_logo(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_upload_org_logo(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_upload_org_logo(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let service = OrgService::new();

    service
        .check_logo_permission(&db, &org_id, &user_ctx.user_id)
        .await?;

    let form_data = req
        .form_data()
        .await
        .map_err(|_| AppError::BadRequest("Failed to parse multipart form data".to_string()))?;
    let file_entry = form_data
        .get("logo")
        .ok_or_else(|| AppError::BadRequest("Missing 'logo' field in form data".to_string()))?;
    let file = match file_entry {
        worker::FormEntry::File(f) => f,
        worker::FormEntry::Field(_) => {
            return Err(AppError::BadRequest(
                "'logo' field must be a file upload".to_string(),
            ));
        }
    };

    let content_type = file.type_();
    let allowed_types = ["image/png", "image/jpeg", "image/webp", "image/svg+xml"];
    if !allowed_types.contains(&content_type.as_str()) {
        return Err(AppError::BadRequest(
            "Invalid file type. Allowed: PNG, JPEG, WebP, SVG".to_string(),
        ));
    }

    let bytes = file
        .bytes()
        .await
        .map_err(|_| AppError::BadRequest("Failed to read file bytes".to_string()))?;
    const MAX_BYTES: usize = 500 * 1024;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::BadRequest(
            "Logo file must be 500 KB or smaller".to_string(),
        ));
    }

    let bucket = ctx.env.bucket("ASSETS_BUCKET")?;
    let r2_key = format!("logos/{}", org_id);
    bucket
        .put(&r2_key, bytes)
        .custom_metadata([("content-type".to_string(), content_type.clone())])
        .execute()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to store logo: {}", e)))?;

    let logo_url = format!("/api/orgs/{}/logo", org_id);
    service.set_logo_url(&db, &org_id, Some(&logo_url)).await?;

    let origin = req.headers().get("Origin").ok().flatten();
    let response = Response::from_json(&serde_json::json!({ "logo_url": logo_url }))?;
    Ok(crate::add_cors_headers(response, origin, &ctx.env))
}

#[utoipa::path(
    get,
    path = "/api/orgs/{id}/logo",
    tag = "Organizations",
    summary = "Get org logo",
    description = "Serves the organization logo from R2 storage. Public endpoint - no authentication required",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Logo image"),
        (status = 404, description = "No logo found for this org"),
    )
)]
pub async fn handle_get_org_logo(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();
    let bucket = ctx.env.bucket("ASSETS_BUCKET")?;
    let r2_key = format!("logos/{}", org_id);

    let object = bucket
        .get(&r2_key)
        .execute()
        .await
        .map_err(|e| Error::RustError(format!("Failed to read logo: {}", e)))?;

    match object {
        Some(obj) => {
            let metadata = obj.custom_metadata().unwrap_or_default();
            let content_type = metadata
                .get("content-type")
                .cloned()
                .unwrap_or_else(|| "image/png".to_string());
            let body = obj
                .body()
                .ok_or_else(|| Error::RustError("Empty object body".to_string()))?
                .bytes()
                .await
                .map_err(|e| Error::RustError(format!("Failed to read body: {}", e)))?;

            let mut response = Response::from_bytes(body)?;
            let headers = response.headers_mut();
            headers.set("Content-Type", &content_type)?;
            headers.set("Cache-Control", "public, max-age=86400")?;
            headers.set("Access-Control-Allow-Origin", "*")?;
            Ok(response)
        }
        None => Response::error("Logo not found", 404),
    }
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/logo",
    tag = "Organizations",
    summary = "Delete org logo",
    description = "Removes the organization logo from R2 storage. Requires owner or admin role and Pro+ tier",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Logo deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required, or Pro+ required"),
        (status = 404, description = "No logo to delete"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_delete_org_logo(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_delete_org_logo(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_delete_org_logo(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let service = OrgService::new();

    service
        .check_delete_logo_permission(&db, &org_id, &user_ctx.user_id)
        .await?;

    let bucket = ctx.env.bucket("ASSETS_BUCKET")?;
    let r2_key = format!("logos/{}", org_id);
    bucket
        .delete(&r2_key)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to delete logo: {}", e)))?;

    service.set_logo_url(&db, &org_id, None).await?;

    let origin = req.headers().get("Origin").ok().flatten();
    let response = Response::ok("Logo deleted")?;
    Ok(crate::add_cors_headers(response, origin, &ctx.env))
}
