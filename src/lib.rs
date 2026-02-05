use worker::*;

mod api;
pub mod auth;
mod db;
mod kv;
mod models;
mod router;
mod utils;

/// Add CORS headers to a response
fn add_cors_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    let _ = headers.set("Access-Control-Allow-Origin", "http://localhost:5173");
    let _ = headers.set("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS");
    let _ = headers.set(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization",
    );
    let _ = headers.set("Access-Control-Allow-Credentials", "true");
    response
}

/// Handle CORS preflight requests
async fn handle_cors_preflight(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let response = Response::empty()?;
    Ok(add_cors_headers(response))
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();

    // Create router
    let router = Router::new();

    router
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
        .options_async("/api/auth/logout", handle_cors_preflight)
        .options_async("/api/links", handle_cors_preflight)
        .options_async("/api/links/:id", handle_cors_preflight)
        // Auth routes (public)
        .get_async("/api/auth/github", router::handle_github_login)
        .get_async("/api/auth/callback", router::handle_oauth_callback)
        // API routes - authentication required
        .get_async("/api/auth/me", router::handle_get_current_user)
        .post_async("/api/auth/logout", router::handle_logout)
        .post_async("/api/links", router::handle_create_link)
        .get_async("/api/links", router::handle_list_links)
        .get_async("/api/links/:id", router::handle_get_link)
        .delete_async("/api/links/:id", router::handle_delete_link)
        // Health check
        .get("/", |_, _| Response::ok("Rushomon URL Shortener API"))
        .run(req, env)
        .await
}
