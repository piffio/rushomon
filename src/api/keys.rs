use crate::auth::authenticate_request;
use crate::utils::{generate_short_code_with_length, now_timestamp};
use sha2::{Digest, Sha256};
use worker::*;

#[derive(serde::Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_in_days: Option<i64>,
}

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

    // Calculate expiration if provided
    let expires_at = body.expires_in_days.map(|days| now + (days * 24 * 60 * 60));

    // Generate the raw token (prefix + 32 random chars)
    let raw_token = format!("ro_pat_{}", generate_short_code_with_length(32));

    // Generate the hint (prefix + last 4 chars)
    let hint = format!("ro_pat_...{}", &raw_token[raw_token.len() - 4..]);

    // Hash the token for storage
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    let key_hash = format!("{:x}", hasher.finalize());

    let key_id = uuid::Uuid::new_v4().to_string();

    // Store in database
    let stmt = db.prepare(
        "INSERT INTO api_keys (id, user_id, org_id, name, key_hash, hint, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    );

    stmt.bind(&[
        key_id.clone().into(),
        user_ctx.user_id.into(),
        user_ctx.org_id.into(),
        body.name.clone().into(),
        key_hash.into(),
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

pub async fn handle_list_api_keys(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

    let stmt = db.prepare(
        "SELECT id, name, hint, created_at, last_used_at, expires_at 
         FROM api_keys 
         WHERE user_id = ?1 
         ORDER BY created_at DESC",
    );

    let results = stmt.bind(&[user_ctx.user_id.into()])?.all().await?;
    let keys = results.results::<serde_json::Value>()?;

    Response::from_json(&keys)
}

pub async fn handle_revoke_api_key(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let key_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing ID".to_string()))?;
    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

    // Ensure they only delete their own key
    let stmt = db.prepare("DELETE FROM api_keys WHERE id = ?1 AND user_id = ?2");
    stmt.bind(&[key_id.into(), user_ctx.user_id.into()])?
        .run()
        .await?;

    Ok(Response::empty()?.with_status(204))
}
