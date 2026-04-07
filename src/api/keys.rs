use crate::auth::authenticate_request;
use crate::utils::{generate_short_code_with_length, now_timestamp};
use hex; // Add hex crate for formatting
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    let now = now_timestamp();

    // Check if user's tier allows API keys
    let org = match crate::db::get_org_by_id(&db, &user_ctx.org_id).await {
        Ok(Some(org)) => org,
        Ok(None) => return Response::error("Organization not found", 404),
        Err(_) => return Response::error("Failed to validate organization", 500),
    };

    let tier = if let Some(ref billing_account_id) = org.billing_account_id {
        match crate::db::get_billing_account(&db, billing_account_id).await {
            Ok(Some(billing_account)) => crate::models::Tier::from_str_value(&billing_account.tier)
                .unwrap_or(crate::models::Tier::Free),
            Ok(None) => crate::models::Tier::Free,
            Err(_) => return Response::error("Failed to validate billing account", 500),
        }
    } else {
        crate::models::Tier::Free
    };

    if !tier.limits().allow_api_keys {
        return Response::error(
            "API keys are not available on your current plan. Upgrade to Pro or higher to use API keys.",
            403,
        );
    }

    // Calculate expiration if provided
    let expires_at = body.expires_in_days.map(|days| now + (days * 24 * 60 * 60));

    // Generate the raw token (prefix + 32 random chars)
    let raw_token = format!("ro_pat_{}", generate_short_code_with_length(32));

    // Generate the hint (prefix + last 4 chars)
    let hint = format!("ro_pat_...{}", &raw_token[raw_token.len() - 4..]);

    // Hash the token for storage
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    let key_id = uuid::Uuid::new_v4().to_string();

    // Store in database
    let stmt = db.prepare(
        "INSERT INTO api_keys (id, user_id, org_id, name, key_hash, hint, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    );

    let user_id_for_db = user_ctx.user_id.clone();
    let org_id_for_db = user_ctx.org_id.clone();

    stmt.bind(&[
        key_id.clone().into(),
        user_id_for_db.into(),
        org_id_for_db.into(),
        body.name.clone().into(),
        key_hash.clone().into(),
        hint.clone().into(),
        (now as f64).into(),
        expires_at
            .map(|t| (t as f64).into())
            .unwrap_or(worker::wasm_bindgen::JsValue::NULL),
    ])?
    .run()
    .await?;

    // Return the raw token EXACTLY ONCE
    Response::from_json(&serde_json::json!({
        "id": key_id,
        "name": body.name,
        "hint": hint,
        "raw_token": raw_token, // The UI must instruct the user to copy this immediately
        "created_at": now,
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

    let stmt = db.prepare(
        "SELECT id, name, hint, created_at, last_used_at, expires_at
         FROM api_keys
         WHERE user_id = ?1 AND status = 'active'
         ORDER BY created_at DESC",
    );

    let results = stmt.bind(&[user_ctx.user_id.into()])?.all().await?;
    let keys = results.results::<serde_json::Value>()?;

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

    // Soft delete - set status to 'deleted'
    let timestamp = crate::utils::time::now_timestamp();
    let stmt = db.prepare(
        "UPDATE api_keys SET status = 'deleted', updated_at = ?1, updated_by = ?2
         WHERE id = ?3 AND user_id = ?4 AND status = 'active'",
    );
    stmt.bind(&[
        (timestamp as f64).into(),
        user_ctx.user_id.clone().into(),
        key_id.into(),
        user_ctx.user_id.into(),
    ])?
    .run()
    .await?;

    Ok(Response::empty()?.with_status(204))
}
