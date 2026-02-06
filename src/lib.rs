use worker::*;

mod api;
pub mod auth;
mod db;
mod kv;
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

    // Fallback: Allow local development if no env var set
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

/// Add CORS headers to a response based on the request origin
fn add_cors_headers(mut response: Response, origin: Option<String>, env: &Env) -> Response {
    if let Some(origin_value) = origin
        && is_allowed_origin(&origin_value, env)
    {
        let headers = response.headers_mut();
        let _ = headers.set("Access-Control-Allow-Origin", &origin_value);
        let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS");
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

    // Extract origin for CORS (before router consumes request)
    let origin = req.headers().get("Origin").ok().flatten();

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
        .delete_async("/api/links/:id", router::handle_delete_link)
        // Health check
        .get("/", |_, _| Response::ok("Rushomon URL Shortener API"))
        .run(req, env.clone())
        .await?;

    // Add CORS headers to all responses
    Ok(add_cors_headers(response, origin, &env))
}
