//! Mock OAuth Server for Integration Testing
//!
//! This server simulates GitHub's OAuth endpoints for local testing.
//! It allows the integration tests to perform the full OAuth flow without
//! requiring actual GitHub credentials.
//!
//! Endpoints:
//! - GET  /login/oauth/authorize - Redirects back with a mock code
//! - POST /login/oauth/access_token - Returns a mock access token
//! - GET  /api/user - Returns a mock GitHub user profile

use axum::{
    Json, Router,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

// Atomic counter for generating unique user IDs
static USER_COUNTER: AtomicU64 = AtomicU64::new(1000);

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Mock OAuth server doesn't use all GitHub OAuth fields
struct AuthorizeParams {
    client_id: String,
    redirect_uri: String,
    state: String,
    #[serde(default)]
    scope: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Mock OAuth server doesn't use all GitHub OAuth fields
struct TokenRequest {
    client_id: String,
    client_secret: String,
    code: String,
    redirect_uri: String,
}

#[derive(Debug, Serialize)]
struct TokenResponse {
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

/// GET /login/oauth/authorize
/// Simulates GitHub's authorization page by immediately redirecting back
/// with a mock authorization code.
async fn authorize(Query(params): Query<AuthorizeParams>) -> impl IntoResponse {
    println!(
        "[Mock OAuth] Authorize request: client_id={}, state={}",
        params.client_id, params.state
    );

    // Generate a mock authorization code
    let code = format!("mock-auth-code-{}", params.state);

    // Build redirect URL with code and state
    let redirect_url = format!(
        "{}?code={}&state={}",
        params.redirect_uri, code, params.state
    );

    println!("[Mock OAuth] Redirecting to: {}", redirect_url);

    Redirect::temporary(&redirect_url)
}

/// POST /login/oauth/access_token
/// Exchanges the authorization code for an access token.
async fn token(Json(request): Json<TokenRequest>) -> impl IntoResponse {
    println!(
        "[Mock OAuth] Token exchange: client_id={}, code={}",
        request.client_id, request.code
    );

    // Validate that the code looks like our mock code
    if !request.code.starts_with("mock-auth-code-") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid_grant", "error_description": "Invalid authorization code"})),
        )
            .into_response();
    }

    // Return a mock access token
    let response = TokenResponse {
        access_token: format!("mock-access-token-{}", request.code),
        token_type: "bearer".to_string(),
        scope: "user:email".to_string(),
    };

    println!("[Mock OAuth] Returning access token");

    Json(response).into_response()
}

/// GET /api/user
/// Returns a mock GitHub user profile.
async fn user() -> impl IntoResponse {
    // Generate a unique user ID for each test run
    let user_id = USER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let user = GitHubUser {
        id: user_id,
        login: format!("test-user-{}", timestamp),
        email: Some(format!("test-user-{}@test.local", timestamp)),
        name: Some(format!("Test User {}", user_id)),
        avatar_url: Some("https://avatars.githubusercontent.com/u/0".to_string()),
    };

    println!(
        "[Mock OAuth] Returning user profile: id={}, login={}",
        user.id, user.login
    );

    Json(user)
}

/// Health check endpoint
async fn health() -> &'static str {
    "Mock OAuth Server OK"
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("MOCK_OAUTH_PORT")
        .unwrap_or_else(|_| "9999".to_string())
        .parse()
        .expect("Invalid port number");

    let app = Router::new()
        // GitHub OAuth endpoints
        .route("/login/oauth/authorize", get(authorize))
        .route("/login/oauth/access_token", post(token))
        .route("/api/user", get(user))
        // Health check
        .route("/health", get(health));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("[Mock OAuth] Starting server on http://{}", addr);
    println!("[Mock OAuth] Endpoints:");
    println!("  GET  /login/oauth/authorize");
    println!("  POST /login/oauth/access_token");
    println!("  GET  /api/user");
    println!("  GET  /health");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
