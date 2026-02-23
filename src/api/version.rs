use worker::*;

/// Handle version endpoint: GET /api/version
/// Returns the current application version and build information
pub async fn handle_version(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let version = env!("CARGO_PKG_VERSION");

    let response = serde_json::json!({
        "version": version,
        "name": env!("CARGO_PKG_NAME"),
        "build_timestamp": option_env!("BUILD_TIMESTAMP").unwrap_or("unknown"),
        "git_commit": option_env!("GIT_COMMIT").unwrap_or("unknown")
    });

    Response::from_json(&response)
}
