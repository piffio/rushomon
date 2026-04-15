use crate::auth::authenticate_request;
use crate::services::ApiKeyService;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use worker::*;

#[derive(Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    #[schema(example = "My Production API Key")]
    pub name: String,
    #[schema(example = 30)]
    pub expires_in_days: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    #[schema(example = "key-123456")]
    pub id: String,
    #[schema(example = "My Production API Key")]
    pub name: String,
    #[schema(example = "ro_pat_...abcd")]
    pub hint: String,
    /// The raw token - show this ONLY ONCE to the user
    #[schema(example = "ro_pat_abc123def456ghi789jkl012mno345pq")]
    pub raw_token: String,
    #[schema(example = 1609459200)]
    pub created_at: i64,
    #[schema(example = 1612137600)]
    pub expires_at: Option<i64>,
}

#[utoipa::path(
    post,
    path = "/api/keys",
    tag = "API Keys",
    summary = "Create an API key",
    description = "Generates a new personal access token (PAT) for programmatic API access. The raw token is returned only once — copy it immediately. Requires Pro tier or higher",
    request_body(content = CreateApiKeyRequest, description = "API key creation payload"),
    responses(
        (status = 200, description = "API key created with raw token", body = CreateApiKeyResponse),
        (status = 400, description = "Empty name"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Plan does not support API keys"),
        (status = 404, description = "Organization not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_create_api_key(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let body: CreateApiKeyRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid request body", 400),
    };

    if body.name.trim().is_empty() {
        return Response::error("API Key name cannot be empty", 400);
    }

    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

    let (key_id, raw_token, hint, created_at, expires_at) = match ApiKeyService::new()
        .create(
            &db,
            &user_ctx.user_id,
            &user_ctx.org_id,
            &body.name,
            body.expires_in_days,
        )
        .await
    {
        Ok(result) => result,
        Err(worker::Error::RustError(msg)) if msg.contains("Upgrade to Pro") => {
            return Response::error(msg, 403);
        }
        Err(e) => return Err(e),
    };

    // Return the raw token EXACTLY ONCE
    Response::from_json(&serde_json::json!({
        "id": key_id,
        "name": body.name,
        "hint": hint,
        "raw_token": raw_token,
        "created_at": created_at,
        "expires_at": expires_at
    }))
}

#[utoipa::path(
    get,
    path = "/api/keys",
    tag = "API Keys",
    summary = "List API keys",
    description = "Returns all active API keys for the authenticated user. The raw token is never returned here — only the hint (last 4 chars)",
    responses(
        (status = 200, description = "Array of active API keys"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_list_api_keys(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;
    let keys = ApiKeyService::new().list(&db, &user_ctx.user_id).await?;
    Response::from_json(&keys)
}

#[utoipa::path(
    delete,
    path = "/api/keys/{id}",
    tag = "API Keys",
    summary = "Revoke an API key",
    description = "Soft-deletes an API key owned by the authenticated user. Returns 204 on success",
    params(
        ("id" = String, Path, description = "API key ID"),
    ),
    responses(
        (status = 204, description = "Key revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Key not found or not owned by user"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_revoke_api_key(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let key_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing ID".to_string()))?;
    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

    ApiKeyService::new()
        .revoke(&db, key_id, &user_ctx.user_id)
        .await?;

    Ok(Response::empty()?.with_status(204))
}
