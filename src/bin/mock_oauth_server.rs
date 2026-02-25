//! Mock OAuth Server for Integration Testing
//!
//! This server simulates both GitHub and Google OAuth endpoints for local testing.
//! It allows the integration tests to perform the full OAuth flow without
//! requiring actual provider credentials.
//!
//! GitHub endpoints (prefix: /github):
//! - GET  /github/login/oauth/authorize      - Redirects back with a mock code
//! - POST /github/login/oauth/access_token   - Returns a mock access token
//! - GET  /github/api/user                   - Returns a mock GitHub user profile
//!
//! Google endpoints (prefix: /google):
//! - GET  /google/o/oauth2/v2/auth           - Redirects back with a mock code
//! - POST /google/token                      - Returns a mock access token
//! - GET  /google/openidconnect/v1/userinfo  - Returns a mock Google user profile
//!
//! Health check:
//! - GET  /health

use axum::{
    Json, Router,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

// Atomic counter for generating unique user IDs (shared across providers)
static USER_COUNTER: AtomicU64 = AtomicU64::new(1000);

// ─── Shared types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AuthorizeParams {
    client_id: String,
    redirect_uri: String,
    state: String,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    response_type: String,
    #[serde(default)]
    access_type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokenRequest {
    client_id: String,
    client_secret: String,
    code: String,
    redirect_uri: String,
    #[serde(default)]
    grant_type: String,
}

// ─── GitHub handlers ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct GitHubTokenResponse {
    access_token: String,
    token_type: String,
    scope: String,
}

#[derive(Debug, Serialize)]
struct GitHubUser {
    id: u64,
    login: String,
    email: Option<String>,
    name: Option<String>,
    avatar_url: Option<String>,
}

/// GET /github/login/oauth/authorize
async fn github_authorize(Query(params): Query<AuthorizeParams>) -> impl IntoResponse {
    println!(
        "[Mock OAuth/GitHub] Authorize: client_id={}, state={}",
        params.client_id, params.state
    );

    let code = format!("mock-gh-code-{}", params.state);
    let redirect_url = format!(
        "{}?code={}&state={}",
        params.redirect_uri, code, params.state
    );

    println!("[Mock OAuth/GitHub] Redirecting to: {}", redirect_url);
    Redirect::temporary(&redirect_url)
}

/// POST /github/login/oauth/access_token
async fn github_token(Json(request): Json<TokenRequest>) -> impl IntoResponse {
    println!("[Mock OAuth/GitHub] Token exchange: code={}", request.code);

    if !request.code.starts_with("mock-gh-code-") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_grant", "error_description": "Invalid authorization code"})),
        )
            .into_response();
    }

    let response = GitHubTokenResponse {
        access_token: format!("mock-gh-token-{}", request.code),
        token_type: "bearer".to_string(),
        scope: "user:email".to_string(),
    };

    println!("[Mock OAuth/GitHub] Returning access token");
    Json(response).into_response()
}

/// GET /github/api/user
async fn github_user() -> impl IntoResponse {
    let user_id = USER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let user = GitHubUser {
        id: user_id,
        login: format!("gh-test-user-{}", timestamp),
        email: Some(format!("gh-test-{}@test.local", timestamp)),
        name: Some(format!("GitHub Test User {}", user_id)),
        avatar_url: Some("https://avatars.githubusercontent.com/u/0".to_string()),
    };

    // Hash sensitive information for logging
    let user_id_hash = {
        let mut hasher = Sha256::new();
        hasher.update(user.id.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    };
    let user_login_hash = {
        let mut hasher = Sha256::new();
        hasher.update(user.login.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    println!(
        "[Mock OAuth/GitHub] Returning user: id_hash={}, login_hash={}",
        user_id_hash, user_login_hash
    );

    Json(user)
}

// ─── Google handlers ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct GoogleTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Debug, Serialize)]
struct GoogleUser {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
}

/// GET /google/o/oauth2/v2/auth
async fn google_authorize(Query(params): Query<AuthorizeParams>) -> impl IntoResponse {
    println!(
        "[Mock OAuth/Google] Authorize: client_id={}, state={}",
        params.client_id, params.state
    );

    let code = format!("mock-google-code-{}", params.state);
    let redirect_url = format!(
        "{}?code={}&state={}",
        params.redirect_uri, code, params.state
    );

    println!("[Mock OAuth/Google] Redirecting to: {}", redirect_url);
    Redirect::temporary(&redirect_url)
}

/// POST /google/token
async fn google_token(body: String) -> impl IntoResponse {
    println!("[Mock OAuth/Google] Token exchange request body: {}", body);

    // Parse form-encoded data (like real Google OAuth)
    let code = if body.contains("application/json") {
        // Handle JSON format (for compatibility)
        "mock-google-code-json".to_string()
    } else {
        // Parse form-encoded data: client_id=...&code=...&redirect_uri=...
        body.split('&')
            .find(|part| part.starts_with("code="))
            .and_then(|part| part.strip_prefix("code="))
            .unwrap_or("empty-code")
            .to_string()
    };

    println!("[Mock OAuth/Google] Extracted code: {}", code);

    // Accept any auth code format for testing
    if code.is_empty() || code == "empty-code" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_grant", "error_description": "Authorization code is required"})),
        )
            .into_response();
    }

    let response = GoogleTokenResponse {
        access_token: format!("mock-google-token-{}", code),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
    };

    println!("[Mock OAuth/Google] Returning access token");
    Json(response).into_response()
}

/// GET /google/openidconnect/v1/userinfo
async fn google_user() -> impl IntoResponse {
    let user_id = USER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let user = GoogleUser {
        sub: format!("{}", user_id),
        email: Some(format!("google-test-{}@gmail.com", timestamp)),
        name: Some(format!("Google Test User {}", user_id)),
        picture: Some("https://lh3.googleusercontent.com/photo.jpg".to_string()),
    };

    // Hash sensitive information for logging
    let user_sub_hash = {
        let mut hasher = Sha256::new();
        hasher.update(user.sub.as_bytes());
        format!("{:x}", hasher.finalize())
    };
    let user_email_hash = user.email.as_ref().map(|email| {
        let mut hasher = Sha256::new();
        hasher.update(email.as_bytes());
        format!("{:x}", hasher.finalize())
    });

    println!(
        "[Mock OAuth/Google] Returning user: sub_hash={}, email_hash={:?}",
        user_sub_hash, user_email_hash
    );

    Json(user)
}

// ─── Health check ─────────────────────────────────────────────────────────────

async fn health() -> &'static str {
    "Mock OAuth Server OK"
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("MOCK_OAUTH_PORT")
        .unwrap_or_else(|_| "9999".to_string())
        .parse()
        .expect("Invalid port number");

    let app = Router::new()
        // GitHub OAuth endpoints
        .route("/github/login/oauth/authorize", get(github_authorize))
        .route("/github/login/oauth/access_token", post(github_token))
        .route("/github/api/user", get(github_user))
        // Google OAuth endpoints
        .route("/google/o/oauth2/v2/auth", get(google_authorize))
        .route("/google/token", post(google_token))
        .route("/google/openidconnect/v1/userinfo", get(google_user))
        // Health check
        .route("/health", get(health));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("[Mock OAuth] Starting server on http://{}", addr);
    println!("[Mock OAuth] GitHub endpoints:");
    println!("  GET  /github/login/oauth/authorize");
    println!("  POST /github/login/oauth/access_token");
    println!("  GET  /github/api/user");
    println!("[Mock OAuth] Google endpoints:");
    println!("  GET  /google/o/oauth2/v2/auth");
    println!("  POST /google/token");
    println!("  GET  /google/openidconnect/v1/userinfo");
    println!("[Mock OAuth] Health: GET /health");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
