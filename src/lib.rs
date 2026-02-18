use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use worker::*;

mod api;
pub mod auth;
mod db;
mod kv;
mod middleware;
mod models;
mod router;
pub mod utils;

// Thread-local storage for deferred analytics futures from redirect handlers.
// Workers are single-threaded, so thread_local is safe and avoids passing Context through the Router.
thread_local! {
    static DEFERRED_ANALYTICS: RefCell<Option<Pin<Box<dyn Future<Output = ()> + 'static>>>> = RefCell::new(None);
}

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
/// - Content-Security-Policy: Comprehensive XSS protection
/// - X-Content-Type-Options: nosniff - Prevents MIME type sniffing
/// - X-Frame-Options: DENY - Prevents clickjacking attacks
/// - X-XSS-Protection: 0 - Disables legacy XSS filter (modern CSP preferred)
/// - Strict-Transport-Security: Forces HTTPS (production only)
/// - Referrer-Policy: Controls referrer information leakage
/// - Permissions-Policy: Restricts access to browser features
fn add_security_headers(mut response: Response, is_https: bool) -> Response {
    let headers = response.headers_mut();

    // Content Security Policy - Defense in depth against XSS
    // - default-src 'self': Only load resources from same origin by default
    // - script-src 'self': Only execute scripts from same origin (no inline scripts)
    // - style-src 'self' 'unsafe-inline': Allow same-origin styles + inline (needed for SvelteKit)
    // - img-src 'self' data: https:: Allow images from same origin, data URIs, and HTTPS
    // - font-src 'self': Only load fonts from same origin
    // - connect-src 'self': Only allow API calls to same origin
    // - frame-ancestors 'none': Prevent embedding in iframes (redundant with X-Frame-Options)
    // - base-uri 'self': Prevent base tag injection
    // - form-action 'self': Restrict form submissions to same origin
    // - upgrade-insecure-requests: Automatically upgrade HTTP to HTTPS
    let csp = if is_https {
        "default-src 'self'; \
         script-src 'self'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; \
         font-src 'self'; \
         connect-src 'self'; \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         form-action 'self'; \
         upgrade-insecure-requests"
    } else {
        // Development mode (no upgrade-insecure-requests for localhost)
        "default-src 'self'; \
         script-src 'self'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; \
         font-src 'self'; \
         connect-src 'self'; \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         form-action 'self'"
    };
    let _ = headers.set("Content-Security-Policy", csp);

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
async fn main(req: Request, env: Env, worker_ctx: Context) -> Result<Response> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

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
    let frontend_url_str = router::get_frontend_url(&env);
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

    // Create router
    let router = Router::new();

    let response = router
        // Public redirect routes - must come first to catch short codes
        .get_async("/:code", move |req, route_ctx| async move {
            let code = route_ctx
                .param("code")
                .ok_or_else(|| Error::RustError("Missing short code".to_string()))?
                .to_string();

            // Skip API routes and known frontend routes on the frontend domain.
            // Frontend routes (dashboard, auth, settings, admin, 404) must not be
            // treated as short codes — they should fall through to the SPA fallback.
            // Without this, /404 would redirect to /404 in an infinite loop.
            if code.starts_with("api") {
                return Response::error("Not found", 404);
            }
            if is_frontend_domain
                && matches!(
                    code.as_str(),
                    "dashboard" | "auth" | "settings" | "admin" | "404"
                )
            {
                return Response::error("Not found", 404);
            }

            let result = router::handle_redirect(req, route_ctx, code).await?;
            if let Some(future) = result.analytics_future {
                DEFERRED_ANALYTICS.with(|cell| cell.replace(Some(future)));
            }
            Ok(result.response)
        })
        .head_async("/:code", move |req, route_ctx| async move {
            let code = route_ctx
                .param("code")
                .ok_or_else(|| Error::RustError("Missing short code".to_string()))?
                .to_string();

            if code.starts_with("api") {
                return Response::error("Not found", 404);
            }
            if is_frontend_domain
                && matches!(
                    code.as_str(),
                    "dashboard" | "auth" | "settings" | "admin" | "404"
                )
            {
                return Response::error("Not found", 404);
            }

            let result = router::handle_redirect(req, route_ctx, code).await?;
            if let Some(future) = result.analytics_future {
                DEFERRED_ANALYTICS.with(|cell| cell.replace(Some(future)));
            }
            Ok(result.response)
        })
        // CORS preflight handlers for API routes
        .options_async("/api/auth/github", handle_cors_preflight)
        .options_async("/api/auth/callback", handle_cors_preflight)
        .options_async("/api/auth/me", handle_cors_preflight)
        .options_async("/api/auth/refresh", handle_cors_preflight)
        .options_async("/api/auth/logout", handle_cors_preflight)
        .options_async("/api/links", handle_cors_preflight)
        .options_async("/api/links/by-code/:code", handle_cors_preflight)
        .options_async("/api/links/:id", handle_cors_preflight)
        .options_async("/api/links/:id/analytics", handle_cors_preflight)
        .options_async("/api/usage", handle_cors_preflight)
        .options_async("/api/admin/users", handle_cors_preflight)
        .options_async("/api/admin/users/:id", handle_cors_preflight)
        .options_async("/api/admin/settings", handle_cors_preflight)
        .options_async("/api/admin/orgs/:id/tier", handle_cors_preflight)
        .options_async("/api/admin/orgs/:id/reset-counter", handle_cors_preflight)
        .options_async("/api/admin/links", handle_cors_preflight)
        .options_async("/api/admin/links/:id", handle_cors_preflight)
        .options_async("/api/admin/blacklist", handle_cors_preflight)
        .options_async("/api/admin/blacklist/:id", handle_cors_preflight)
        .options_async("/api/admin/users/:id/suspend", handle_cors_preflight)
        .options_async("/api/admin/users/:id/unsuspend", handle_cors_preflight)
        .options_async("/api/reports/links", handle_cors_preflight)
        // Auth routes (public)
        .get_async("/api/auth/github", router::handle_github_login)
        .get_async("/api/auth/callback", router::handle_oauth_callback)
        // API routes - authentication required
        .get_async("/api/auth/me", router::handle_get_current_user)
        .post_async("/api/auth/refresh", router::handle_token_refresh)
        .post_async("/api/auth/logout", router::handle_logout)
        .get_async("/api/usage", router::handle_get_usage)
        .post_async("/api/links", router::handle_create_link)
        .get_async("/api/links", router::handle_list_links)
        .get_async("/api/links/by-code/:code", router::handle_get_link_by_code)
        .get_async(
            "/api/links/:id/analytics",
            router::handle_get_link_analytics,
        )
        .get_async("/api/links/:id", router::handle_get_link)
        .put_async("/api/links/:id", router::handle_update_link)
        .delete_async("/api/links/:id", router::handle_delete_link)
        // Admin routes - admin authentication required
        .get_async("/api/admin/users", router::handle_admin_list_users)
        .get_async("/api/admin/users/:id", router::handle_admin_get_user)
        .put_async("/api/admin/users/:id", router::handle_admin_update_user)
        .get_async("/api/admin/settings", router::handle_admin_get_settings)
        .put_async("/api/admin/settings", router::handle_admin_update_setting)
        .put_async(
            "/api/admin/orgs/:id/tier",
            router::handle_admin_update_org_tier,
        )
        .post_async(
            "/api/admin/orgs/:id/reset-counter",
            router::handle_admin_reset_monthly_counter,
        )
        // Admin moderation routes
        .get_async("/api/admin/links", router::handle_admin_list_links)
        .put_async(
            "/api/admin/links/:id",
            router::handle_admin_update_link_status,
        )
        .delete_async("/api/admin/links/:id", router::handle_admin_delete_link)
        .post_async(
            "/api/admin/blacklist",
            router::handle_admin_block_destination,
        )
        .get_async("/api/admin/blacklist", router::handle_admin_get_blacklist)
        .delete_async(
            "/api/admin/blacklist/:id",
            router::handle_admin_remove_blacklist,
        )
        .put_async(
            "/api/admin/users/:id/suspend",
            router::handle_admin_suspend_user,
        )
        .put_async(
            "/api/admin/users/:id/unsuspend",
            router::handle_admin_unsuspend_user,
        )
        // Abuse report route (public, can be called by anyone)
        .post_async("/api/reports/links", router::handle_report_link)
        // Root redirect: redirect to frontend (e.g., rush.mn/ → rushomon.cc/)
        .get_async("/", |_req, ctx| async move {
            let url = Url::parse(&router::get_frontend_url(&ctx.env))?;
            Response::redirect_with_status(url, 301)
        })
        .run(req, env.clone())
        .await?;

    // Execute deferred analytics via wait_until (non-blocking, runs after response is sent)
    // This avoids blocking the redirect response on D1 writes (~15-50ms savings)
    let deferred = DEFERRED_ANALYTICS.with(|cell| cell.borrow_mut().take());
    if let Some(analytics_future) = deferred {
        worker_ctx.wait_until(analytics_future);
    }

    // SPA fallback: if router returned 404 on the frontend domain, serve index.html.
    // This enables client-side routing for paths like /dashboard, /auth/callback, etc.
    // Short code redirects are already handled by the router above (returning 301/302).
    if response.status_code() == 404
        && is_frontend_domain
        && !path.starts_with("/api/")
        && let Ok(assets) = env.get_binding::<worker::Fetcher>("ASSETS")
    {
        let fallback_url = format!("{}://{}/index.html", url.scheme(), &request_authority);
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
