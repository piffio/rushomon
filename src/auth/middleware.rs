use crate::auth::session::{UserContext, get_session, parse_cookie_header, validate_jwt};
use crate::utils::time::now_timestamp;
use sha2::{Digest, Sha256};
use worker::{D1Database, console_log};
use worker::{Request, Response, RouteContext}; // Assuming you have a time util, or use chrono

/// Authentication error that can be converted to an HTTP response
pub enum AuthError {
    Unauthorized(String),
    Forbidden(String),
    InternalError(String),
}

impl AuthError {
    pub fn into_response(self) -> Response {
        match self {
            AuthError::Unauthorized(msg) => Response::error(msg, 401)
                .unwrap_or_else(|_| Response::error("Unauthorized", 401).unwrap()),
            AuthError::Forbidden(msg) => Response::error(msg, 403)
                .unwrap_or_else(|_| Response::error("Forbidden", 403).unwrap()),
            AuthError::InternalError(msg) => Response::error(msg, 500)
                .unwrap_or_else(|_| Response::error("Internal Server Error", 500).unwrap()),
        }
    }
}

/// Checks that the authenticated user has instance-level admin role.
/// Returns Err(AuthError::Forbidden) if the user is not an admin.
pub fn require_admin(user_ctx: &UserContext) -> Result<(), AuthError> {
    if user_ctx.role == "admin" {
        Ok(())
    } else {
        Err(AuthError::Forbidden("Admin access required".to_string()))
    }
}

/// Authenticates a request by validating JWT and loading session from KV
/// Returns Ok(UserContext) on success, or Err(AuthError) which can be converted to a proper HTTP response
///
/// Supports three authentication methods (in priority order):
/// 1. Cookie: rushomon_access=<token> (NEW - httpOnly access token cookie)
/// 2. Authorization: Bearer <token> header (backward compatible for cross-domain)
/// 3. Cookie: rushomon_session=<token> (legacy - backward compatible)
pub async fn authenticate_request(
    req: &Request,
    ctx: &RouteContext<()>,
) -> Result<UserContext, AuthError> {
    // Try access token cookie first (NEW secure method)
    let jwt = if let Ok(Some(cookie_header)) = req.headers().get("Cookie") {
        use crate::auth::session::parse_access_cookie_header;
        parse_access_cookie_header(&cookie_header)
    } else {
        None
    };

    // Fallback to Authorization header (backward compatible)
    let jwt = if let Some(token) = jwt {
        Some(token)
    } else if let Ok(Some(auth_header)) = req.headers().get("Authorization") {
        auth_header
            .strip_prefix("Bearer ")
            .map(|token| token.to_string())
    } else {
        None
    };

    // Fallback to legacy session cookie (backward compatible)
    let jwt = if let Some(token) = jwt {
        token
    } else {
        match req.headers().get("Cookie") {
            Ok(Some(header)) => match parse_cookie_header(&header) {
                Some(token) => token,
                None => {
                    return Err(AuthError::Unauthorized(
                        "Authentication required".to_string(),
                    ));
                }
            },
            Ok(None) => {
                return Err(AuthError::Unauthorized(
                    "Authentication required".to_string(),
                ));
            }
            Err(_e) => {
                return Err(AuthError::InternalError(
                    "Failed to read headers".to_string(),
                ));
            }
        }
    };

    // Intercept Personal Access Tokens (PAT)
    if jwt.starts_with("ro_pat_") {
        let db = match ctx.env.get_binding::<D1Database>("rushomon") {
            Ok(db) => db,
            Err(_e) => {
                console_log!("DB binding failed for PAT auth");
                return Err(AuthError::InternalError(
                    "Server configuration error".to_string(),
                ));
            }
        };

        // 1. Hash the incoming token
        let mut hasher = Sha256::new();
        hasher.update(jwt.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());

        // 2. Query database directly for API key with current tier information
        let api_key_with_tier =
            match crate::db::queries::get_api_key_by_hash_with_tier(&db, &key_hash).await {
                Ok(Some(key)) => key,
                Ok(None) => {
                    console_log!("Invalid PAT attempt");
                    return Err(AuthError::Unauthorized("Invalid API Key".to_string()));
                }
                Err(e) => {
                    console_log!("Database error during API key validation: {:?}", e);
                    return Err(AuthError::Unauthorized("Invalid API Key".to_string()));
                }
            };

        // 4. Check expiration
        if let Some(expires_at) = api_key_with_tier.expires_at
            && now_timestamp() > expires_at
        {
            return Err(AuthError::Unauthorized("API Key has expired".to_string()));
        }

        // 4b. Check if key is active (not revoked or deleted)
        if api_key_with_tier.status != "active" {
            return Err(AuthError::Unauthorized(format!(
                "API Key has been {}",
                api_key_with_tier.status
            )));
        }

        // 5. Check if tier allows API keys
        let tier = match api_key_with_tier.tier {
            Some(tier_str) => match crate::models::Tier::from_str_value(&tier_str) {
                Some(tier) => tier,
                None => {
                    return Err(AuthError::InternalError(
                        "Invalid tier configuration".to_string(),
                    ));
                }
            },
            None => {
                // No billing account = Free tier
                crate::models::Tier::Free
            }
        };

        if !tier.limits().allow_api_keys {
            return Err(AuthError::Forbidden(
                "API keys are not available on your current plan. Upgrade to Pro or higher to use API keys.".to_string()
            ));
        }

        // 6. Verify the user exists and isn't suspended
        let user = match crate::db::get_user_by_id(&db, &api_key_with_tier.user_id).await {
            Ok(Some(u)) => u,
            Ok(None) => return Err(AuthError::Unauthorized("User not found".to_string())),
            Err(_) => {
                return Err(AuthError::InternalError(
                    "Failed to validate user".to_string(),
                ));
            }
        };

        if user.suspended_at.is_some() {
            return Err(AuthError::Forbidden("Account suspended".to_string()));
        }

        // 7. Update the 'last_used_at' timestamp
        if let Err(e) = crate::db::queries::update_api_key_last_used(
            &db,
            &api_key_with_tier.id,
            now_timestamp(),
        )
        .await
        {
            console_log!("Failed to update API key last_used_at: {:?}", e);
        }

        // 6. Successfully authenticate!
        return Ok(UserContext {
            user_id: user.id,
            org_id: api_key_with_tier.org_id,
            session_id: format!("pat_{}", api_key_with_tier.user_id),
            role: user.role,
        });
    }

    // Get JWT secret from environment
    let jwt_secret = match ctx.env.secret("JWT_SECRET") {
        Ok(secret) => secret.to_string(),
        Err(_e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "auth_jwt_secret_missing",
                    "level": "error"
                })
            );
            return Err(AuthError::InternalError(
                "Server configuration error".to_string(),
            ));
        }
    };

    // Validate JWT
    let claims = match validate_jwt(&jwt, &jwt_secret) {
        Ok(claims) => claims,
        Err(_e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "auth_jwt_invalid",
                    "level": "warn"
                })
            );
            return Err(AuthError::Unauthorized(
                "Token expired or invalid".to_string(),
            ));
        }
    };

    // STRICT: Only access tokens allowed for general API access
    // Refresh tokens are long-lived (7 days) and should ONLY be used for token refresh endpoint
    if claims.token_type != "access" {
        console_log!(
            "{}",
            serde_json::json!({
                "event": "auth_wrong_token_type",
                "token_type": claims.token_type,
                "level": "warn"
            })
        );
        return Err(AuthError::Unauthorized(
            "Invalid token type - use access token".to_string(),
        ));
    }

    // Load session from KV
    // Load session from KV
    let kv = match ctx.kv("URL_MAPPINGS") {
        Ok(kv) => kv,
        Err(_e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "auth_kv_error",
                    "level": "error"
                })
            );
            return Err(AuthError::InternalError(
                "Server configuration error".to_string(),
            ));
        }
    };

    let session = match get_session(&kv, &claims.session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "auth_session_not_found",
                    "session_id": claims.session_id,
                    "level": "warn"
                })
            );
            return Err(AuthError::Unauthorized(
                "Session expired or invalid".to_string(),
            ));
        }
        Err(_e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "auth_session_lookup_failed",
                    "level": "error"
                })
            );
            return Err(AuthError::InternalError(
                "Failed to validate session".to_string(),
            ));
        }
    };

    // Verify user_id matches (constant-time comparison to prevent timing attacks)
    if !crate::utils::secure_compare(&session.user_id, &claims.sub) {
        console_log!(
            "{}",
            serde_json::json!({
                "event": "auth_session_mismatch",
                "level": "warn"
            })
        );
        return Err(AuthError::Unauthorized("Session mismatch".to_string()));
    }

    // Check if user is suspended
    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(_e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "auth_db_binding_failed",
                    "level": "error"
                })
            );
            return Err(AuthError::InternalError(
                "Server configuration error".to_string(),
            ));
        }
    };

    let user = match crate::db::get_user_by_id(&db, &session.user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "auth_user_not_found",
                    "user_id": session.user_id,
                    "level": "warn"
                })
            );
            return Err(AuthError::Unauthorized("User not found".to_string()));
        }
        Err(_e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "auth_user_lookup_failed",
                    "level": "error"
                })
            );
            return Err(AuthError::InternalError(
                "Failed to validate user".to_string(),
            ));
        }
    };

    // Check if user is suspended
    if user.suspended_at.is_some() {
        console_log!(
            "{}",
            serde_json::json!({
                "event": "auth_user_suspended",
                "user_id": user.id,
                "level": "warn"
            })
        );
        return Err(AuthError::Forbidden("Account suspended".to_string()));
    }

    Ok(UserContext {
        user_id: session.user_id,
        org_id: session.org_id,
        session_id: claims.session_id,
        role: claims.role,
    })
}
