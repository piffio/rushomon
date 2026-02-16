use crate::auth::session::{UserContext, get_session, parse_cookie_header, validate_jwt};
use worker::D1Database;
use worker::{Request, Response, RouteContext};

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

    // Get JWT secret from environment
    let jwt_secret = match ctx.env.secret("JWT_SECRET") {
        Ok(secret) => secret.to_string(),
        Err(_e) => {
            return Err(AuthError::InternalError(
                "Server configuration error".to_string(),
            ));
        }
    };

    // Validate JWT
    let claims = match validate_jwt(&jwt, &jwt_secret) {
        Ok(claims) => claims,
        Err(_e) => {
            return Err(AuthError::Unauthorized(
                "Token expired or invalid".to_string(),
            ));
        }
    };

    // STRICT: Only access tokens allowed for general API access
    // Refresh tokens are long-lived (7 days) and should ONLY be used for token refresh endpoint
    if claims.token_type != "access" {
        return Err(AuthError::Unauthorized(
            "Invalid token type - use access token".to_string(),
        ));
    }

    // Load session from KV
    // Load session from KV
    let kv = match ctx.kv("URL_MAPPINGS") {
        Ok(kv) => kv,
        Err(_e) => {
            return Err(AuthError::InternalError(
                "Server configuration error".to_string(),
            ));
        }
    };

    let session = match get_session(&kv, &claims.session_id).await {
        Ok(Some(session)) => session,
        Ok(None) => {
            return Err(AuthError::Unauthorized(
                "Session expired or invalid".to_string(),
            ));
        }
        Err(_e) => {
            return Err(AuthError::InternalError(
                "Failed to validate session".to_string(),
            ));
        }
    };

    // Verify user_id matches (constant-time comparison to prevent timing attacks)
    if !crate::utils::secure_compare(&session.user_id, &claims.sub) {
        return Err(AuthError::Unauthorized("Session mismatch".to_string()));
    }

    // Check if user is suspended
    let db = match ctx.env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(_e) => {
            return Err(AuthError::InternalError(
                "Server configuration error".to_string(),
            ));
        }
    };

    let user = match crate::db::get_user_by_id(&db, &session.user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Err(AuthError::Unauthorized("User not found".to_string()));
        }
        Err(_e) => {
            return Err(AuthError::InternalError(
                "Failed to validate user".to_string(),
            ));
        }
    };

    // Check if user is suspended
    if user.suspended_at.is_some() {
        return Err(AuthError::Forbidden("Account suspended".to_string()));
    }

    Ok(UserContext {
        user_id: session.user_id,
        org_id: session.org_id,
        session_id: claims.session_id,
        role: claims.role,
    })
}
