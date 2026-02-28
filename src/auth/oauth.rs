use crate::auth::providers::{NormalizedUser, OAuthProviderConfig};
use crate::db::queries;
use crate::models::{Organization, User, user::CreateUserData};
use serde::{Deserialize, Serialize};
use worker::{D1Database, Env, Error, Request, Response, Result, kv::KvStore};

const OAUTH_STATE_TTL_SECONDS: u64 = 600; // 10 minutes

/// Generate a request fingerprint for CSRF protection
///
/// Creates a hash of request metadata to bind the OAuth state to the original request.
/// This prevents an attacker from using a state parameter in a different context.
///
/// Components:
/// - User-Agent: Browser/client identifier
/// - CF-Connecting-IP: Client IP address (Cloudflare header)
/// - Accept-Language: Browser language preferences
///
/// The fingerprint is hashed with SHA-256 to:
/// 1. Normalize varying lengths
/// 2. Prevent information disclosure in logs
/// 3. Protect against timing attacks
fn generate_request_fingerprint(req: &Request) -> String {
    use sha2::{Digest, Sha256};

    let headers = req.headers();

    let user_agent = headers.get("User-Agent").ok().flatten().unwrap_or_default();
    let ip = headers
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .unwrap_or_default();
    let accept_language = headers
        .get("Accept-Language")
        .ok()
        .flatten()
        .unwrap_or_default();

    let combined = format!("{}|{}|{}", user_agent, ip, accept_language);
    let mut hasher = Sha256::new();
    hasher.update(combined.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthState {
    pub state: String,
    pub created_at: i64,
    /// Request fingerprint for CSRF protection - binds state to original request context
    pub fingerprint: String,
    /// Provider name that initiated this OAuth flow ("github" | "google")
    pub provider: String,
    /// Optional redirect URL after successful authentication
    pub redirect: Option<String>,
}

/// OAuth callback result with both access and refresh tokens
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
}

/// Initiates an OAuth flow for the given provider
pub async fn initiate_oauth(
    req: &Request,
    kv: &KvStore,
    provider: &OAuthProviderConfig,
    redirect_uri: &str,
    env: &Env,
) -> Result<Response> {
    let client_id = provider.client_id(env)?;

    // Generate random state for CSRF protection
    let state = uuid::Uuid::new_v4().to_string();
    let now = (js_sys::Date::now() / 1000.0) as i64;

    // Generate request fingerprint to bind state to original request context
    let fingerprint = generate_request_fingerprint(req);

    // Extract redirect parameter from request URL if present
    let redirect = req.url().ok().and_then(|url| {
        url.query_pairs()
            .find(|(k, _)| k == "redirect")
            .map(|(_, v)| v.to_string())
    });

    // Store state with fingerprint, provider, and redirect in KV with TTL
    let state_data = OAuthState {
        state: state.clone(),
        created_at: now,
        fingerprint,
        provider: provider.name.to_string(),
        redirect,
    };

    let key = format!("oauth_state:{}", state);
    let value = serde_json::to_string(&state_data)
        .map_err(|e| Error::RustError(format!("Failed to serialize OAuth state: {}", e)))?;

    kv.put(&key, value)?
        .expiration_ttl(OAUTH_STATE_TTL_SECONDS)
        .execute()
        .await?;

    // Build authorization URL
    let authorize_url = provider.authorize_url(env);
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&state={}&scope={}",
        authorize_url,
        urlencoding::encode(&client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&state),
        urlencoding::encode(provider.scopes),
    );

    // Add response_type=code for providers that require it (Google)
    let auth_url = if provider.name == "google" {
        format!("{}&response_type=code&access_type=offline", auth_url)
    } else {
        auth_url
    };

    Response::redirect(
        auth_url
            .parse()
            .map_err(|e| Error::RustError(format!("Invalid OAuth URL: {}", e)))?,
    )
}

/// Handles the OAuth callback — provider-agnostic entry point.
/// Reads the provider from stored KV state, dispatches to the correct token/user fetcher.
pub async fn handle_oauth_callback(
    req: &Request,
    code: String,
    state: String,
    kv: &KvStore,
    db: &D1Database,
    env: &Env,
) -> Result<(User, Organization, OAuthTokens, Option<String>)> {
    // Validate state from KV
    let key = format!("oauth_state:{}", state);
    let stored_state = kv.get(&key).text().await?;

    let stored_state_value = match stored_state {
        Some(value) => value,
        None => {
            return Err(Error::RustError(
                "Invalid or expired OAuth state".to_string(),
            ));
        }
    };

    // Parse stored state data
    let state_data: OAuthState = serde_json::from_str(&stored_state_value)
        .map_err(|e| Error::RustError(format!("Failed to parse OAuth state: {}", e)))?;

    // Constant-time comparison to prevent timing attacks on OAuth state
    if !crate::utils::secure_compare(&state_data.state, &state) {
        return Err(Error::RustError(
            "Invalid or expired OAuth state".to_string(),
        ));
    }

    // Validate request fingerprint matches the original request
    let current_fingerprint = generate_request_fingerprint(req);
    if !crate::utils::secure_compare(&state_data.fingerprint, &current_fingerprint) {
        return Err(Error::RustError(
            "OAuth state validation failed: request context mismatch".to_string(),
        ));
    }

    // Extract redirect URL before deleting state
    let redirect = state_data.redirect.clone();

    // Delete state after validation (one-time use)
    kv.delete(&key).await?;

    let domain = env.var("DOMAIN")?.to_string();
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let redirect_uri = format!("{}://{}/api/auth/callback", scheme, domain);

    // Dispatch to provider-specific token exchange and user fetch
    let normalized_user = match state_data.provider.as_str() {
        "github" => {
            let provider = &crate::auth::providers::GITHUB;
            let client_id = provider.client_id(env)?;
            let client_secret = provider.client_secret(env)?;
            let token_url = provider.token_url(env);
            let user_url = provider.user_url(env);

            let access_token = crate::auth::github::exchange_code_for_token(
                &code,
                &client_id,
                &client_secret,
                &redirect_uri,
                &token_url,
            )
            .await?;

            crate::auth::github::fetch_user(&access_token, &user_url).await?
        }
        "google" => {
            let provider = &crate::auth::providers::GOOGLE;
            let client_id = provider.client_id(env)?;
            let client_secret = provider.client_secret(env)?;
            let token_url = provider.token_url(env);
            let user_url = provider.user_url(env);

            let access_token = crate::auth::google::exchange_code_for_token(
                &code,
                &client_id,
                &client_secret,
                &redirect_uri,
                &token_url,
            )
            .await?;

            crate::auth::google::fetch_user(&access_token, &user_url).await?
        }
        unknown => {
            return Err(Error::RustError(format!(
                "Unknown OAuth provider: {}",
                unknown
            )));
        }
    };

    // Create or get user (with account linking by email)
    let (user, org) = create_or_get_user(db, normalized_user).await?;

    // SECURITY: Generate session ID AFTER all validation completes successfully
    // This prevents session fixation attacks where an attacker could predict
    // or influence the session ID before authentication is confirmed.
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
        redirect,
    ))
}

/// Creates or retrieves a user based on normalized provider profile.
///
/// Account linking strategy (v1):
/// 1. Look up by (provider, provider_id) → exact match → update profile, return
/// 2. Look up by email → match found → update provider info + profile (link accounts), return same user/org
/// 3. No match → new signup flow
async fn create_or_get_user(
    db: &D1Database,
    normalized_user: NormalizedUser,
) -> Result<(User, Organization)> {
    // Step 1: look up by (provider, provider_id)
    let stmt = db.prepare(
        "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at
         FROM users
         WHERE oauth_provider = ?1 AND oauth_id = ?2",
    );

    let existing_user = stmt
        .bind(&[
            normalized_user.provider.clone().into(),
            normalized_user.provider_id.clone().into(),
        ])?
        .first::<User>(None)
        .await?;

    if let Some(user) = existing_user {
        let create_data = CreateUserData {
            email: normalized_user.email,
            name: normalized_user.name,
            avatar_url: normalized_user.avatar_url,
            oauth_provider: normalized_user.provider,
            oauth_id: normalized_user.provider_id,
        };

        let updated_user = queries::create_or_update_user(db, create_data, &user.org_id).await?;

        let org = queries::get_org_by_id(db, &user.org_id)
            .await?
            .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

        return Ok((updated_user, org));
    }

    // Step 2: look up by email (account linking across providers)
    let stmt = db.prepare(
        "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at
         FROM users
         WHERE email = ?1",
    );

    let email_user = stmt
        .bind(&[normalized_user.email.clone().into()])?
        .first::<User>(None)
        .await?;

    if let Some(user) = email_user {
        // Link this provider to the existing account
        let create_data = CreateUserData {
            email: normalized_user.email,
            name: normalized_user.name,
            avatar_url: normalized_user.avatar_url,
            oauth_provider: normalized_user.provider,
            oauth_id: normalized_user.provider_id,
        };

        let updated_user = queries::create_or_update_user(db, create_data, &user.org_id).await?;

        let org = queries::get_org_by_id(db, &user.org_id)
            .await?
            .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

        return Ok((updated_user, org));
    }

    // Step 3: new user — check if signups are enabled (first user is always allowed)
    let user_count = queries::get_user_count(db).await?;
    if user_count > 0 {
        let signups_enabled = queries::get_setting(db, "signups_enabled")
            .await?
            .unwrap_or_else(|| "true".to_string());
        if signups_enabled != "true" {
            return Err(Error::RustError("SIGNUPS_DISABLED".to_string()));
        }
    }

    // Derive org name from display name or email prefix
    let org_name = normalized_user
        .name
        .clone()
        .map(|n| format!("{}'s Organization", n))
        .unwrap_or_else(|| {
            let email_prefix = normalized_user
                .email
                .split('@')
                .next()
                .unwrap_or("user")
                .to_string();
            format!("{}'s Organization", email_prefix)
        });

    // Create organization first (with temp user ID)
    let temp_user_id = uuid::Uuid::new_v4().to_string();
    let org = queries::create_default_org(db, &temp_user_id, &org_name).await?;

    // Now create the user with the actual org_id
    let create_data = CreateUserData {
        email: normalized_user.email.clone(),
        name: normalized_user.name.clone(),
        avatar_url: normalized_user.avatar_url.clone(),
        oauth_provider: normalized_user.provider.clone(),
        oauth_id: normalized_user.provider_id.clone(),
    };
    let user = queries::create_or_update_user(db, create_data, &org.id).await?;

    // Update the billing account owner to the actual user ID
    queries::update_billing_account_owner(db, org.billing_account_id.as_ref().unwrap(), &user.id)
        .await?;

    Ok((user, org))
}

// Helper module for URL encoding
mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
