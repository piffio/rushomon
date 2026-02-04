use worker::*;

mod api;
pub mod auth;
mod db;
mod kv;
mod models;
mod router;
mod utils;

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
