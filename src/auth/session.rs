use ::chrono::TimeDelta;
use jwt_compact::{
    TimeOptions,
    alg::{Hs256, Hs256Key},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use worker::{Error, Result, kv::KvStore};

const SESSION_TTL_SECONDS: u64 = 604800; // 7 days
const ACCESS_TOKEN_TTL_SECONDS: u64 = 3600; // 1 hour
const REFRESH_TOKEN_TTL_SECONDS: u64 = 604800; // 7 days

/// Token type for distinguishing between access and refresh tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Access,
    Refresh,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenType::Access => "access",
            TokenType::Refresh => "refresh",
        }
    }

    pub fn ttl_seconds(&self) -> u64 {
        match self {
            TokenType::Access => ACCESS_TOKEN_TTL_SECONDS,
            TokenType::Refresh => REFRESH_TOKEN_TTL_SECONDS,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JwtClaims {
    pub sub: String, // user_id
    pub org_id: String,
    pub session_id: String,
    pub token_type: String, // "access" or "refresh"
    #[serde(default)]
    pub role: String, // "admin" or "member" (instance-level)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: String,
    pub org_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub org_id: String,
    pub session_id: String,
    pub role: String, // "admin" or "member" (instance-level)
}

/// Creates a JWT token with user and organization information
/// This is the legacy function that creates a token with default (refresh) TTL
/// For new code, use create_jwt_with_type() to explicitly specify the token type
pub fn create_jwt(
    user_id: &str,
    org_id: &str,
    session_id: &str,
    role: &str,
    secret: &str,
) -> Result<String> {
    create_jwt_with_type(
        user_id,
        org_id,
        session_id,
        role,
        secret,
        TokenType::Refresh,
    )
}

/// Creates a JWT token with a specific token type (access or refresh)
pub fn create_jwt_with_type(
    user_id: &str,
    org_id: &str,
    session_id: &str,
    role: &str,
    secret: &str,
    token_type: TokenType,
) -> Result<String> {
    let key = Hs256Key::new(secret.as_bytes());
    let header = Header::empty();
    let time_options = TimeOptions::default();

    let claims = Claims::new(JwtClaims {
        sub: user_id.to_string(),
        org_id: org_id.to_string(),
        session_id: session_id.to_string(),
        token_type: token_type.as_str().to_string(),
        role: role.to_string(),
    })
    .set_duration_and_issuance(
        &time_options,
        TimeDelta::seconds(token_type.ttl_seconds() as i64),
    );

    Hs256
        .token(&header, &claims, &key)
        .map_err(|e| Error::RustError(format!("Failed to create JWT: {}", e)))
}

/// Creates an access token (1 hour expiry)
pub fn create_access_token(
    user_id: &str,
    org_id: &str,
    session_id: &str,
    role: &str,
    secret: &str,
) -> Result<String> {
    create_jwt_with_type(user_id, org_id, session_id, role, secret, TokenType::Access)
}

/// Creates a refresh token (7 days expiry)
pub fn create_refresh_token(
    user_id: &str,
    org_id: &str,
    session_id: &str,
    role: &str,
    secret: &str,
) -> Result<String> {
    create_jwt_with_type(
        user_id,
        org_id,
        session_id,
        role,
        secret,
        TokenType::Refresh,
    )
}

/// Validates a JWT token and returns the claims
pub fn validate_jwt(token: &str, secret: &str) -> Result<JwtClaims> {
    let key = Hs256Key::new(secret.as_bytes());

    let untrusted_token = match UntrustedToken::new(token) {
        Ok(t) => t,
        Err(e) => {
            return Err(Error::RustError(format!("Invalid JWT format: {}", e)));
        }
    };

    let token: Token<JwtClaims> = match Hs256.validator(&key).validate(&untrusted_token) {
        Ok(t) => t,
        Err(e) => {
            return Err(Error::RustError(format!("Invalid JWT: {}", e)));
        }
    };

    // Check expiration using jwt_compact's built-in validation
    let time_options = TimeOptions::default();
    let claims = token.claims();

    if let Err(_e) = claims.validate_expiration(&time_options) {
        return Err(Error::RustError("Token expired".to_string()));
    }

    Ok(claims.custom.clone())
}

/// Creates a session cookie with the JWT token
pub fn create_session_cookie(jwt: &str) -> String {
    create_session_cookie_with_scheme(jwt, "https")
}

/// Creates a session cookie with the JWT token and specified scheme
pub fn create_session_cookie_with_scheme(jwt: &str, scheme: &str) -> String {
    let secure_part = if scheme == "https" { " Secure;" } else { "" };
    format!(
        "rushomon_session={}; HttpOnly;{} SameSite=Lax; Path=/; Max-Age={}",
        jwt, secure_part, SESSION_TTL_SECONDS
    )
}

/// Parses the Cookie header and extracts the session token
pub fn parse_cookie_header(cookie_header: &str) -> Option<String> {
    cookie_header.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        cookie
            .strip_prefix("rushomon_session=")
            .map(|value| value.to_string())
    })
}

/// Creates a logout cookie that expires immediately
pub fn create_logout_cookie() -> String {
    "rushomon_session=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0".to_string()
}

/// Creates a refresh token cookie (httpOnly for security)
pub fn create_refresh_cookie(jwt: &str) -> String {
    create_refresh_cookie_with_scheme(jwt, "https")
}

/// Creates a refresh token cookie with specified scheme
pub fn create_refresh_cookie_with_scheme(jwt: &str, scheme: &str) -> String {
    let secure_part = if scheme == "https" { " Secure;" } else { "" };
    format!(
        "rushomon_refresh={}; HttpOnly;{} SameSite=Lax; Path=/; Max-Age={}",
        jwt, secure_part, REFRESH_TOKEN_TTL_SECONDS
    )
}

/// Creates a logout cookie for refresh token that expires immediately
pub fn create_refresh_logout_cookie() -> String {
    "rushomon_refresh=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0".to_string()
}

/// Parses the Cookie header and extracts the refresh token
pub fn parse_refresh_cookie_header(cookie_header: &str) -> Option<String> {
    cookie_header.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        cookie
            .strip_prefix("rushomon_refresh=")
            .map(|value| value.to_string())
    })
}

/// Stores session data in KV with TTL
pub async fn store_session(
    kv: &KvStore,
    session_id: &str,
    user_id: &str,
    org_id: &str,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let session_data = SessionData {
        user_id: user_id.to_string(),
        org_id: org_id.to_string(),
        created_at: now,
    };

    let key = format!("session:{}", session_id);
    let value = serde_json::to_string(&session_data)
        .map_err(|e| Error::RustError(format!("Failed to serialize session: {}", e)))?;

    kv.put(&key, value)?
        .expiration_ttl(SESSION_TTL_SECONDS)
        .execute()
        .await?;

    Ok(())
}

/// Retrieves session data from KV
pub async fn get_session(kv: &KvStore, session_id: &str) -> Result<Option<SessionData>> {
    let key = format!("session:{}", session_id);

    match kv.get(&key).text().await? {
        Some(value) => {
            let session_data = serde_json::from_str(&value)
                .map_err(|e| Error::RustError(format!("Failed to parse session: {}", e)))?;
            Ok(Some(session_data))
        }
        None => Ok(None),
    }
}

/// Deletes a session from KV
pub async fn delete_session(kv: &KvStore, session_id: &str) -> Result<()> {
    let key = format!("session:{}", session_id);
    kv.delete(&key).await?;
    Ok(())
}

// Temporary chrono replacement for Wasm compatibility
mod chrono {
    pub struct Utc;

    impl Utc {
        pub fn now() -> DateTime {
            DateTime
        }
    }

    pub struct DateTime;

    impl DateTime {
        pub fn timestamp(&self) -> i64 {
            // Get current time in milliseconds and convert to seconds
            (js_sys::Date::now() / 1000.0) as i64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Skip JWT tests in non-wasm targets since they use js-sys::Date
    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_jwt_roundtrip() {
        let secret = "test-secret-32-chars-minimum!!";
        let jwt = create_jwt("user1", "org1", "sess1", "admin", secret).unwrap();
        let claims = validate_jwt(&jwt, secret).unwrap();

        assert_eq!(claims.sub, "user1");
        assert_eq!(claims.org_id, "org1");
        assert_eq!(claims.session_id, "sess1");
        assert_eq!(claims.role, "admin");
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_jwt_invalid_secret() {
        let secret1 = "test-secret-32-chars-minimum!!";
        let secret2 = "different-secret-32-chars-min!";
        let jwt = create_jwt("user1", "org1", "sess1", "member", secret1).unwrap();

        let result = validate_jwt(&jwt, secret2);
        assert!(result.is_err());
    }

    #[test]
    fn test_cookie_parsing() {
        let header = "rushomon_session=abc123; other=xyz";
        assert_eq!(parse_cookie_header(header), Some("abc123".to_string()));

        let header2 = "other=xyz; rushomon_session=def456";
        assert_eq!(parse_cookie_header(header2), Some("def456".to_string()));

        let header3 = "other=xyz";
        assert_eq!(parse_cookie_header(header3), None);
    }

    #[test]
    fn test_session_cookie_format() {
        let jwt = "test.jwt.token";
        let cookie = create_session_cookie(jwt);

        assert!(cookie.contains("rushomon_session=test.jwt.token"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains(&format!("Max-Age={}", SESSION_TTL_SECONDS)));
    }

    #[test]
    fn test_logout_cookie() {
        let cookie = create_logout_cookie();
        assert!(cookie.contains("Max-Age=0"));
    }
}
