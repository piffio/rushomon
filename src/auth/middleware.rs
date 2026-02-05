use crate::auth::session::{UserContext, get_session, parse_cookie_header, validate_jwt};
use worker::{Request, Response, RouteContext};

/// Authentication error that can be converted to an HTTP response
pub enum AuthError {
    Unauthorized(String),
    InternalError(String),
}

impl AuthError {
    pub fn into_response(self) -> Response {
        match self {
            AuthError::Unauthorized(msg) => Response::error(msg, 401)
                .unwrap_or_else(|_| Response::error("Unauthorized", 401).unwrap()),
            AuthError::InternalError(msg) => Response::error(msg, 500)
                .unwrap_or_else(|_| Response::error("Internal Server Error", 500).unwrap()),
        }
    }
}

/// Authenticates a request by validating JWT and loading session from KV
/// Returns Ok(UserContext) on success, or Err(AuthError) which can be converted to a proper HTTP response
pub async fn authenticate_request(
    req: &Request,
    ctx: &RouteContext<()>,
) -> Result<UserContext, AuthError> {
    // Extract Cookie header
    let cookie_header = match req.headers().get("Cookie") {
        Ok(Some(header)) => header,
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
    };

    // Parse JWT from cookie
    let jwt = match parse_cookie_header(&cookie_header) {
        Some(token) => token,
        None => {
            return Err(AuthError::Unauthorized(
                "Invalid or missing session token".to_string(),
            ));
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
    // Validate JWT
    let claims = match validate_jwt(&jwt, &jwt_secret) {
        Ok(claims) => claims,
        Err(_e) => {
            return Err(AuthError::Unauthorized(
                "Token expired or invalid".to_string(),
            ));
        }
    };

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

    // Verify user_id matches
    if session.user_id != claims.sub {
        return Err(AuthError::Unauthorized("Session mismatch".to_string()));
    }

    Ok(UserContext {
        user_id: session.user_id,
        org_id: session.org_id,
        session_id: claims.session_id,
    })
}
