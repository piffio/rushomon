use crate::db::queries;
use crate::models::{Organization, User, user::CreateUserData};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use worker::{
    D1Database, Env, Error, Fetch, Headers, Method, Request, RequestInit, Response, Result,
    kv::KvStore,
};

const OAUTH_STATE_TTL_SECONDS: u64 = 600; // 10 minutes

// Default GitHub OAuth URLs (can be overridden via environment for testing)
const DEFAULT_GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const DEFAULT_GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const DEFAULT_GITHUB_USER_URL: &str = "https://api.github.com/user";

/// Get GitHub authorize URL from environment or use default
fn get_github_authorize_url(env: &Env) -> String {
    env.var("GITHUB_AUTHORIZE_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_GITHUB_AUTHORIZE_URL.to_string())
}

/// Get GitHub token URL from environment or use default
fn get_github_token_url(env: &Env) -> String {
    env.var("GITHUB_TOKEN_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_GITHUB_TOKEN_URL.to_string())
}

/// Get GitHub user URL from environment or use default
fn get_github_user_url(env: &Env) -> String {
    env.var("GITHUB_USER_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_GITHUB_USER_URL.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthState {
    pub state: String,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubTokenResponse {
    pub access_token: String,
}

/// Initiates the GitHub OAuth flow
pub async fn initiate_github_oauth(
    kv: &KvStore,
    client_id: &str,
    redirect_uri: &str,
    env: &Env,
) -> Result<Response> {
    // Generate random state for CSRF protection
    let state = uuid::Uuid::new_v4().to_string();
    let now = (js_sys::Date::now() / 1000.0) as i64;

    // Store state in KV with TTL
    let state_data = OAuthState {
        state: state.clone(),
        created_at: now,
    };

    let key = format!("oauth_state:{}", state);
    let value = serde_json::to_string(&state_data)
        .map_err(|e| Error::RustError(format!("Failed to serialize OAuth state: {}", e)))?;

    kv.put(&key, value)?
        .expiration_ttl(OAUTH_STATE_TTL_SECONDS)
        .execute()
        .await?;

    // Build GitHub authorization URL
    let github_authorize_url = get_github_authorize_url(env);
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&state={}&scope=user:email",
        github_authorize_url,
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&state)
    );

    // Redirect to GitHub
    Response::redirect(
        auth_url
            .parse()
            .map_err(|e| Error::RustError(format!("Invalid GitHub URL: {}", e)))?,
    )
}

/// Handles the OAuth callback from GitHub
/// OAuth callback result with both access and refresh tokens
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn handle_oauth_callback(
    code: String,
    state: String,
    kv: &KvStore,
    db: &D1Database,
    env: &Env,
) -> Result<(User, Organization, OAuthTokens)> {
    // Validate state from KV
    let key = format!("oauth_state:{}", state);
    let stored_state = kv.get(&key).text().await?;

    if stored_state.is_none() {
        return Err(Error::RustError(
            "Invalid or expired OAuth state".to_string(),
        ));
    }

    // Delete state after validation (one-time use)
    kv.delete(&key).await?;

    // Get OAuth credentials from environment
    let client_id = env.var("GITHUB_CLIENT_ID")?.to_string();
    let client_secret = env.secret("GITHUB_CLIENT_SECRET")?.to_string();
    let domain = env.var("DOMAIN")?.to_string();

    // Use http for localhost, https for production (consistent with login handling)
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let redirect_uri = format!("{}://{}/api/auth/callback", scheme, domain);

    // Exchange code for access token
    let github_token_url = get_github_token_url(env);
    let access_token = exchange_code_for_token(
        &code,
        &client_id,
        &client_secret,
        &redirect_uri,
        &github_token_url,
    )
    .await?;

    // Fetch user profile from GitHub
    let github_user_url = get_github_user_url(env);
    let github_user = fetch_github_user(&access_token, &github_user_url).await?;

    // Create or get user
    let (user, org) = create_or_get_user(db, github_user).await?;

    // Generate access token (1 hour) and refresh token (7 days)
    let session_id = uuid::Uuid::new_v4().to_string();
    let jwt_secret = env.secret("JWT_SECRET")?.to_string();

    let access_token = crate::auth::session::create_access_token(
        &user.id,
        &user.org_id,
        &session_id,
        &user.role,
        &jwt_secret,
    )?;
    let refresh_token = crate::auth::session::create_refresh_token(
        &user.id,
        &user.org_id,
        &session_id,
        &user.role,
        &jwt_secret,
    )?;

    Ok((
        user,
        org,
        OAuthTokens {
            access_token,
            refresh_token,
        },
    ))
}

/// Exchanges authorization code for access token
async fn exchange_code_for_token(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    token_url: &str,
) -> Result<String> {
    let body = serde_json::json!({
        "client_id": client_id,
        "client_secret": client_secret,
        "code": code,
        "redirect_uri": redirect_uri,
    });

    let headers = Headers::new();
    headers.set("Accept", "application/json")?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body.to_string())));

    let request = Request::new_with_init(token_url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::RustError(format!(
            "GitHub token exchange failed: {}",
            error_text
        )));
    }

    let token_response: GitHubTokenResponse = response.json().await?;
    Ok(token_response.access_token)
}

/// Fetches user profile from GitHub API
async fn fetch_github_user(access_token: &str, user_url: &str) -> Result<GitHubUser> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", access_token))?;
    headers.set("User-Agent", "Rushomon")?;
    headers.set("Accept", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);

    let request = Request::new_with_init(user_url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::RustError(format!(
            "GitHub user fetch failed: {}",
            error_text
        )));
    }

    let github_user: GitHubUser = response.json().await?;
    Ok(github_user)
}

/// Creates or retrieves a user based on GitHub profile
async fn create_or_get_user(
    db: &D1Database,
    github_user: GitHubUser,
) -> Result<(User, Organization)> {
    let oauth_id = github_user.id.to_string();

    // Check if user exists by OAuth ID
    let stmt = db.prepare(
        "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at
         FROM users
         WHERE oauth_provider = ?1 AND oauth_id = ?2",
    );

    let existing_user = stmt
        .bind(&["github".into(), oauth_id.clone().into()])?
        .first::<User>(None)
        .await?;

    if let Some(user) = existing_user {
        // User exists - update profile and return with org
        let create_data = CreateUserData {
            email: github_user
                .email
                .unwrap_or_else(|| format!("{}@users.noreply.github.com", github_user.login)),
            name: Some(
                github_user
                    .name
                    .unwrap_or_else(|| github_user.login.clone()),
            ),
            avatar_url: github_user.avatar_url,
            oauth_provider: "github".to_string(),
            oauth_id: oauth_id.clone(),
        };

        let updated_user = queries::create_or_update_user(db, create_data, &user.org_id).await?;

        // Fetch organization
        let org = queries::get_org_by_id(db, &user.org_id)
            .await?
            .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

        Ok((updated_user, org))
    } else {
        // New user - check if signups are enabled (first user is always allowed for bootstrapping)
        let user_count = queries::get_user_count(db).await?;
        if user_count > 0 {
            let signups_enabled = queries::get_setting(db, "signups_enabled")
                .await?
                .unwrap_or_else(|| "true".to_string());
            if signups_enabled != "true" {
                return Err(Error::RustError("SIGNUPS_DISABLED".to_string()));
            }
        }

        // Create organization first, then user
        let org_name = format!("{}'s Organization", github_user.login);
        let temp_user_id = uuid::Uuid::new_v4().to_string();
        let org = queries::create_default_org(db, &temp_user_id, &org_name).await?;

        let create_data = CreateUserData {
            email: github_user
                .email
                .clone()
                .unwrap_or_else(|| format!("{}@users.noreply.github.com", github_user.login)),
            name: Some(
                github_user
                    .name
                    .unwrap_or_else(|| github_user.login.clone()),
            ),
            avatar_url: github_user.avatar_url,
            oauth_provider: "github".to_string(),
            oauth_id,
        };

        let user = queries::create_or_update_user(db, create_data, &org.id).await?;

        Ok((user, org))
    }
}

// Helper module for URL encoding
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
