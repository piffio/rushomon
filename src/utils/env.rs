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

/// Determine the scheme (http/https) based on domain
pub fn get_scheme(env: &Env) -> String {
    let domain = get_domain(env);
    if domain.starts_with("localhost") {
        "http".to_string()
    } else {
        "https".to_string()
    }
}
