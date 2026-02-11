use worker::*;

mod api;
pub mod auth;
mod db;
mod kv;
mod middleware;
mod models;
mod router;
mod utils;

/// Check if an origin is allowed for CORS
fn is_allowed_origin(origin: &str, env: &Env) -> bool {
    // Get allowed origins from environment variable (comma-separated)
    // Format: "https://rushomon-ui.pages.dev,http://localhost:5173,http://localhost:5174"
    if let Ok(allowed_origins) = env.var("ALLOWED_ORIGINS") {
        let allowed_str = allowed_origins.to_string();
        for allowed_origin in allowed_str.split(',') {
            if allowed_origin.trim() == origin {
                return true;
            }
        }
    }

    // Fallback: Allow local development ONLY in debug builds
    // In production (release builds), localhost is NOT allowed for security
    #[cfg(debug_assertions)]
    if origin == "http://localhost:5173" || origin == "http://localhost:5174" {
        return true;
    }

    // Check for ephemeral environment pattern from env var
    // Format: "pr-{}.rushomon-ui.pages.dev" where {} is replaced with PR number
    if let Ok(ephemeral_pattern) = env.var("EPHEMERAL_ORIGIN_PATTERN") {
        let pattern = ephemeral_pattern.to_string();

        // Check if pattern contains placeholder
        if let Some(prefix_end) = pattern.find("{}") {
            let prefix = &pattern[..prefix_end];
            let suffix = &pattern[prefix_end + 2..];

            if origin.starts_with(prefix) && origin.ends_with(suffix) {
                // Extract the value between prefix and suffix
                let value = &origin[prefix.len()..origin.len() - suffix.len()];
                // Validate it's numeric (for security - prevent arbitrary subdomains)
                return !value.is_empty() && value.chars().all(|c| c.is_numeric());
            }
        }
    }

    false
}

/// Add security headers to all responses
///
/// Security headers applied:
/// - X-Content-Type-Options: nosniff - Prevents MIME type sniffing
/// - X-Frame-Options: DENY - Prevents clickjacking attacks
/// - X-XSS-Protection: 0 - Disables legacy XSS filter (modern CSP preferred)
/// - Strict-Transport-Security: Forces HTTPS (production only)
/// - Referrer-Policy: Controls referrer information leakage
/// - Permissions-Policy: Restricts access to browser features
fn add_security_headers(mut response: Response, is_https: bool) -> Response {
    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    let _ = headers.set("X-Content-Type-Options", "nosniff");

    // Prevent clickjacking
    let _ = headers.set("X-Frame-Options", "DENY");

    // Disable legacy XSS filter (CSP is better)
    let _ = headers.set("X-XSS-Protection", "0");

    // Force HTTPS for all future requests (only in production)
    if is_https {
        let _ = headers.set(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        );
    }

    // Control referrer information
    let _ = headers.set("Referrer-Policy", "strict-origin-when-cross-origin");

    // Restrict dangerous browser features
    let _ = headers.set(
        "Permissions-Policy",
        "geolocation=(), microphone=(), camera=()",
    );

    response
}

/// Add CORS headers to a response based on the request origin
fn add_cors_headers(mut response: Response, origin: Option<String>, env: &Env) -> Response {
    if let Some(origin_value) = origin
        && is_allowed_origin(&origin_value, env)
    {
        let headers = response.headers_mut();
        let _ = headers.set("Access-Control-Allow-Origin", &origin_value);
        let _ = headers.set(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, OPTIONS",
        );
        let _ = headers.set(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, Cookie",
        );
        let _ = headers.set("Access-Control-Allow-Credentials", "true");
        let _ = headers.set("Access-Control-Max-Age", "86400"); // 24 hours
    }
    response
}

/// Handle CORS preflight requests
async fn handle_cors_preflight(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let origin = req.headers().get("Origin").ok().flatten();
    let response = Response::empty()?;
    Ok(add_cors_headers(response, origin, &ctx.env))
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    // Extract origin and URL scheme for CORS and security headers (before router consumes request)
    let origin = req.headers().get("Origin").ok().flatten();
    let is_https = req.url().map(|u| u.scheme() == "https").unwrap_or(false);

    // Try to serve static assets first (for ephemeral unified Worker deployment)
    // This only works when ASSETS binding is present (ephemeral environment)
    let url = req.url()?;
    let path = url.path();

    // If not an API route and ASSETS binding exists, try to serve static file
    if !path.starts_with("/api/") {
        // Check if ASSETS binding exists (only in ephemeral unified deployments)
        if let Ok(assets) = env.get_binding::<worker::Fetcher>("ASSETS") {
            // Try to serve the request from static assets
            // Fetch expects a URL string, not a Request object
            let asset_url = url.to_string();
            match assets.fetch(asset_url, None).await {
                Ok(asset_response) => {
                    // Check if asset was found (status 200-299)
                    if asset_response.status_code() >= 200 && asset_response.status_code() < 300 {
                        // Static asset found, return it with security headers
                        let response_with_headers = add_security_headers(asset_response, is_https);
                        return Ok(add_cors_headers(response_with_headers, origin, &env));
                    }
                    // Asset not found (404) or error (5xx), continue to router
                }
                Err(_) => {
                    // Asset fetch failed, continue to router (might be a short code redirect)
                }
            }
        }
    }

    // Create router
    let router = Router::new();

    let response = router
        // Public redirect routes - must come first to catch short codes
        .get_async("/:code", |req, route_ctx| async move {
            let code = route_ctx
                .param("code")
                .ok_or_else(|| Error::RustError("Missing short code".to_string()))?
                .to_string();

            // Skip if it looks like an API route
            if code.starts_with("api") {
                return Response::error("Not found", 404);
            }

            router::handle_redirect(req, route_ctx, code).await
        })
        .head_async("/:code", |req, route_ctx| async move {
            let code = route_ctx
                .param("code")
                .ok_or_else(|| Error::RustError("Missing short code".to_string()))?
                .to_string();

            // Skip if it looks like an API route
            if code.starts_with("api") {
                return Response::error("Not found", 404);
            }

            router::handle_redirect(req, route_ctx, code).await
        })
        // CORS preflight handlers for API routes
        .options_async("/api/auth/github", handle_cors_preflight)
        .options_async("/api/auth/callback", handle_cors_preflight)
        .options_async("/api/auth/me", handle_cors_preflight)
        .options_async("/api/auth/refresh", handle_cors_preflight)
        .options_async("/api/auth/logout", handle_cors_preflight)
        .options_async("/api/links", handle_cors_preflight)
        .options_async("/api/links/:id", handle_cors_preflight)
        .options_async("/api/admin/users", handle_cors_preflight)
        .options_async("/api/admin/users/:id", handle_cors_preflight)
        .options_async("/api/admin/settings", handle_cors_preflight)
        // Auth routes (public)
        .get_async("/api/auth/github", router::handle_github_login)
        .get_async("/api/auth/callback", router::handle_oauth_callback)
        // API routes - authentication required
        .get_async("/api/auth/me", router::handle_get_current_user)
        .post_async("/api/auth/refresh", router::handle_token_refresh)
        .post_async("/api/auth/logout", router::handle_logout)
        .post_async("/api/links", router::handle_create_link)
        .get_async("/api/links", router::handle_list_links)
        .get_async("/api/links/:id", router::handle_get_link)
        .put_async("/api/links/:id", router::handle_update_link)
        .delete_async("/api/links/:id", router::handle_delete_link)
        // Admin routes - admin authentication required
        .get_async("/api/admin/users", router::handle_admin_list_users)
        .get_async("/api/admin/users/:id", router::handle_admin_get_user)
        .put_async("/api/admin/users/:id", router::handle_admin_update_user)
        .get_async("/api/admin/settings", router::handle_admin_get_settings)
        .put_async("/api/admin/settings", router::handle_admin_update_setting)
        // Health check
        .get("/", |_, _| Response::ok("Rushomon URL Shortener API"))
        .run(req, env.clone())
        .await?;

    // Add security headers and CORS headers to all responses
    let response = add_security_headers(response, is_https);
    Ok(add_cors_headers(response, origin, &env))
}
