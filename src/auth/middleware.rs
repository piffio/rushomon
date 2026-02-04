use crate::auth::session::{UserContext, get_session, parse_cookie_header, validate_jwt};
use worker::{Request, Response, RouteContext, console_log};

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
    console_log!("[AUTH DEBUG] Starting authentication...");

    // Extract Cookie header
    let cookie_header = match req.headers().get("Cookie") {
        Ok(Some(header)) => {
            console_log!(
                "[AUTH DEBUG] Cookie header found: {}",
                &header[..std::cmp::min(100, header.len())]
            );
            header
        }
        Ok(None) => {
            console_log!("[AUTH DEBUG] No Cookie header found");
            return Err(AuthError::Unauthorized(
                "Authentication required".to_string(),
            ));
        }
        Err(e) => {
            console_log!("[AUTH DEBUG] Error reading headers: {:?}", e);
            return Err(AuthError::InternalError(
                "Failed to read headers".to_string(),
            ));
        }
    };

    // Parse JWT from cookie
    let jwt = match parse_cookie_header(&cookie_header) {
        Some(token) => {
            console_log!(
                "[AUTH DEBUG] JWT extracted from cookie, length: {}",
                token.len()
            );
            console_log!(
                "[AUTH DEBUG] JWT first 50 chars: {}",
                &token[..std::cmp::min(50, token.len())]
            );
            token
        }
        None => {
            console_log!("[AUTH DEBUG] Failed to parse JWT from cookie header");
            return Err(AuthError::Unauthorized(
                "Invalid or missing session token".to_string(),
            ));
        }
    };

    // Get JWT secret from environment
    let jwt_secret = match ctx.env.secret("JWT_SECRET") {
        Ok(secret) => {
            let secret_str = secret.to_string();
            console_log!(
                "[AUTH DEBUG] JWT_SECRET loaded, length: {}",
                secret_str.len()
            );
            console_log!(
                "[AUTH DEBUG] JWT_SECRET first 10 chars: {}",
                &secret_str[..std::cmp::min(10, secret_str.len())]
            );
            secret_str
        }
        Err(e) => {
            console_log!("[AUTH DEBUG] Failed to load JWT_SECRET: {:?}", e);
            return Err(AuthError::InternalError(
                "Server configuration error".to_string(),
            ));
        }
    };

    // Validate JWT
    console_log!("[AUTH DEBUG] Validating JWT...");
    let claims = match validate_jwt(&jwt, &jwt_secret) {
        Ok(claims) => {
            console_log!(
                "[AUTH DEBUG] JWT valid! sub={}, session_id={}",
                claims.sub,
                claims.session_id
            );
            claims
        }
        Err(e) => {
            console_log!("[AUTH DEBUG] JWT validation FAILED: {:?}", e);
            return Err(AuthError::Unauthorized(
                "Token expired or invalid".to_string(),
            ));
        }
    };

    // Load session from KV
    console_log!(
        "[AUTH DEBUG] Loading session from KV for session_id: {}",
        claims.session_id
    );
    let kv = match ctx.kv("URL_MAPPINGS") {
        Ok(kv) => {
            console_log!("[AUTH DEBUG] KV binding obtained successfully");
            kv
        }
        Err(e) => {
            console_log!("[AUTH DEBUG] Failed to get KV binding: {:?}", e);
            return Err(AuthError::InternalError(
                "Server configuration error".to_string(),
            ));
        }
    };

    let session = match get_session(&kv, &claims.session_id).await {
        Ok(Some(session)) => {
            console_log!(
                "[AUTH DEBUG] Session found! user_id={}, org_id={}",
                session.user_id,
                session.org_id
            );
            session
        }
        Ok(None) => {
            console_log!(
                "[AUTH DEBUG] Session NOT found in KV for session_id: {}",
                claims.session_id
            );
            return Err(AuthError::Unauthorized(
                "Session expired or invalid".to_string(),
            ));
        }
        Err(e) => {
            console_log!("[AUTH DEBUG] Error retrieving session: {:?}", e);
            return Err(AuthError::InternalError(
                "Failed to validate session".to_string(),
            ));
        }
    };

    // Verify user_id matches
    console_log!(
        "[AUTH DEBUG] Comparing user_ids: session.user_id={} vs claims.sub={}",
        session.user_id,
        claims.sub
    );
    if session.user_id != claims.sub {
        console_log!("[AUTH DEBUG] User ID mismatch!");
        return Err(AuthError::Unauthorized("Session mismatch".to_string()));
    }

    console_log!(
        "[AUTH DEBUG] Authentication successful for user: {}",
        session.user_id
    );
    Ok(UserContext {
        user_id: session.user_id,
        org_id: session.org_id,
        session_id: claims.session_id,
    })
}
