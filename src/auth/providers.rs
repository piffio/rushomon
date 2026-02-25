/// OAuth provider configuration - all URL and credential env var keys for a provider.
pub struct OAuthProviderConfig {
    pub name: &'static str,
    pub authorize_url_env: &'static str,
    pub token_url_env: &'static str,
    pub user_url_env: &'static str,
    pub default_authorize_url: &'static str,
    pub default_token_url: &'static str,
    pub default_user_url: &'static str,
    pub scopes: &'static str,
    pub client_id_env: &'static str,
    pub client_secret_env: &'static str,
}

impl OAuthProviderConfig {
    /// Get the authorize URL, with env var override support (used for testing)
    pub fn authorize_url(&self, env: &worker::Env) -> String {
        env.var(self.authorize_url_env)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| self.default_authorize_url.to_string())
    }

    /// Get the token URL, with env var override support
    pub fn token_url(&self, env: &worker::Env) -> String {
        env.var(self.token_url_env)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| self.default_token_url.to_string())
    }

    /// Get the user info URL, with env var override support
    pub fn user_url(&self, env: &worker::Env) -> String {
        env.var(self.user_url_env)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| self.default_user_url.to_string())
    }

    /// Get the client ID from environment variables
    pub fn client_id(&self, env: &worker::Env) -> worker::Result<String> {
        Ok(env.var(self.client_id_env)?.to_string())
    }

    /// Get the client secret from worker secrets
    pub fn client_secret(&self, env: &worker::Env) -> worker::Result<String> {
        Ok(env.secret(self.client_secret_env)?.to_string())
    }

    /// Returns true if this provider is configured (client ID env var is present)
    pub fn is_enabled(&self, env: &worker::Env) -> bool {
        env.var(self.client_id_env)
            .map(|v| !v.to_string().is_empty())
            .unwrap_or(false)
    }
}

/// GitHub OAuth provider configuration
pub static GITHUB: OAuthProviderConfig = OAuthProviderConfig {
    name: "github",
    authorize_url_env: "GITHUB_AUTHORIZE_URL",
    token_url_env: "GITHUB_TOKEN_URL",
    user_url_env: "GITHUB_USER_URL",
    default_authorize_url: "https://github.com/login/oauth/authorize",
    default_token_url: "https://github.com/login/oauth/access_token",
    default_user_url: "https://api.github.com/user",
    scopes: "user:email",
    client_id_env: "GITHUB_CLIENT_ID",
    client_secret_env: "GITHUB_CLIENT_SECRET",
};

/// Google OAuth provider configuration
pub static GOOGLE: OAuthProviderConfig = OAuthProviderConfig {
    name: "google",
    authorize_url_env: "GOOGLE_AUTHORIZE_URL",
    token_url_env: "GOOGLE_TOKEN_URL",
    user_url_env: "GOOGLE_USER_URL",
    default_authorize_url: "https://accounts.google.com/o/oauth2/v2/auth",
    default_token_url: "https://oauth2.googleapis.com/token",
    default_user_url: "https://openidconnect.googleapis.com/v1/userinfo",
    scopes: "openid email profile",
    client_id_env: "GOOGLE_CLIENT_ID",
    client_secret_env: "GOOGLE_CLIENT_SECRET",
};

/// Normalized user profile — common across all providers
#[derive(Debug)]
pub struct NormalizedUser {
    pub provider: String,
    pub provider_id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}
