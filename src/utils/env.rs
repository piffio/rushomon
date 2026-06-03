/// Environment variable utilities
use worker::Env;

/// Get the frontend URL from environment, with fallback to localhost
pub fn get_frontend_url(env: &Env) -> String {
    env.var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
}

/// Get the domain from environment
pub fn get_domain(env: &Env) -> String {
    env.var("DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "localhost:8787".to_string())
}

/// Get the fallback domain for custom domain CNAMEs
/// Falls back to DOMAIN if not configured
pub fn get_fallback_domain(env: &Env) -> String {
    env.var("FALLBACK_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| get_domain(env))
}

/// Determine the scheme (http/https) based on domain
pub fn get_scheme(env: &Env) -> String {
    let domain = get_domain(env);
    if domain.starts_with("localhost") {
        "http".to_string()
    } else {
        "https".to_string()
    }
}

/// Returns true when the minimum Mailgun environment variables are present.
///
/// Used to conditionally enable email features (sending emails, surfacing
/// notification preference toggles in the UI, etc.).
/// Requires both `MAILGUN_API_KEY` and `MAILGUN_DOMAIN` to be non-empty.
pub fn is_mailgun_configured(env: &Env) -> bool {
    let api_key = env
        .var("MAILGUN_API_KEY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let domain = env
        .var("MAILGUN_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();
    !api_key.is_empty() && !domain.is_empty()
}
