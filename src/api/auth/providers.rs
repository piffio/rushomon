/// Auth providers handler
///
/// GET /api/auth/providers — lists OAuth providers enabled on this instance.
use crate::auth;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/auth/providers",
    tag = "Authentication",
    summary = "List enabled OAuth providers",
    description = "Returns the list of OAuth providers configured and enabled on this instance",
    responses(
        (status = 200, description = "List of enabled providers"),
    )
)]
pub async fn handle_list_auth_providers(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner(req, ctx).await)
}

async fn inner(req: Request, ctx: RouteContext<()>) -> Response {
    use serde_json::json;

    let env = &ctx.env;
    let mut providers = Vec::new();

    if auth::providers::GITHUB.is_enabled(env) {
        providers.push(json!({ "name": "github", "label": "GitHub" }));
    }
    if auth::providers::GOOGLE.is_enabled(env) {
        providers.push(json!({ "name": "google", "label": "Google" }));
    }

    let origin = req.headers().get("Origin").ok().flatten();
    match Response::from_json(&json!({ "providers": providers })) {
        Ok(response) => crate::add_cors_headers(response, origin, env),
        Err(e) => {
            Response::error(e.to_string(), 500).unwrap_or_else(|_| Response::empty().unwrap())
        }
    }
}
