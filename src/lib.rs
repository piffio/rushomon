use worker::*;

mod api;
pub mod auth;
mod billing;
mod kv;
mod middleware;
mod models;
pub mod openapi;
mod repositories;
mod scheduled;
mod services;
pub mod utils;

use middleware::{add_cors_headers, add_security_headers};

#[event(fetch)]
async fn main(req: Request, env: Env, worker_ctx: Context) -> Result<Response> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    // Validate JWT secret at startup - fail fast if misconfigured
    // This prevents production deployment with weak secrets
    let jwt_secret = env.secret("JWT_SECRET")?.to_string();
    auth::validate_jwt_secret(&jwt_secret)?;

    // Extract origin and URL scheme for CORS and security headers (before router consumes request)
    let origin = req.headers().get("Origin").ok().flatten();
    let is_https = req.url().map(|u| u.scheme() == "https").unwrap_or(false);

    // With run_worker_first = true in wrangler config, the Worker runs before static assets.
    // Determine if this request is to the frontend domain (vs redirect domain like rush.mn).
    // Only serve static assets on the frontend domain; redirect domain goes straight to router.
    let url = req.url()?;
    let path = url.path();

    let request_authority = match url.port() {
        Some(port) => format!("{}:{}", url.host_str().unwrap_or(""), port),
        None => url.host_str().unwrap_or("").to_string(),
    };
    let frontend_url_str = crate::utils::get_frontend_url(&env);
    let frontend_authority = Url::parse(&frontend_url_str)
        .ok()
        .map(|u| match u.port() {
            Some(port) => format!("{}:{}", u.host_str().unwrap_or(""), port),
            None => u.host_str().unwrap_or("").to_string(),
        })
        .unwrap_or_default();
    let is_frontend_domain = request_authority == frontend_authority;

    // Serve static assets on the frontend domain for non-API routes.
    // With not_found_handling = "none", ASSETS returns 404 for unknown paths, letting us
    // fall through to the router for short code redirects before applying SPA fallback.
    if !path.starts_with("/api/")
        && is_frontend_domain
        && let Ok(assets) = env.get_binding::<worker::Fetcher>("ASSETS")
    {
        let asset_url = url.to_string();
        match assets.fetch(asset_url, None).await {
            Ok(asset_response) if asset_response.status_code() < 400 => {
                let response_with_headers = add_security_headers(asset_response, is_https);
                return Ok(add_cors_headers(response_with_headers, origin, &env));
            }
            _ => {
                // Asset not found or fetch failed, continue to router
            }
        }
    }

    // Handle CORS preflight for all API routes with a single early-return.
    if req.method() == Method::Options && path.starts_with("/api/") {
        let response = Response::empty()?;
        return Ok(add_cors_headers(response, origin, &env));
    }

    let response = api::router::run(req, env.clone(), is_frontend_domain).await?;

    // Execute deferred analytics via wait_until (non-blocking, runs after response is sent)
    // This avoids blocking the redirect response on D1 writes (~15-50ms savings)
    let deferred = api::router::DEFERRED_ANALYTICS.with(|cell| cell.borrow_mut().take());
    if let Some(analytics_future) = deferred {
        worker_ctx.wait_until(analytics_future);
    }

    // SPA fallback: if router returned 404 on the frontend domain, serve fallback.html.
    // This enables client-side routing for paths like /dashboard, /auth/callback, etc.
    // Short code redirects are already handled by the router above (returning 301/302).
    if response.status_code() == 404
        && is_frontend_domain
        && !path.starts_with("/api/")
        && let Ok(assets) = env.get_binding::<worker::Fetcher>("ASSETS")
    {
        let fallback_url = format!("{}://{}/fallback.html", url.scheme(), &request_authority);
        if let Ok(fallback_response) = assets.fetch(fallback_url, None).await
            && fallback_response.status_code() < 400
        {
            let response_with_headers = add_security_headers(fallback_response, is_https);
            return Ok(add_cors_headers(response_with_headers, origin, &env));
        }
    }

    // Add security headers and CORS headers to all responses
    let response = add_security_headers(response, is_https);
    Ok(add_cors_headers(response, origin, &env))
}
