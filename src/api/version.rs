use serde::Serialize;
use utoipa::ToSchema;
use worker::*;

#[derive(Serialize, ToSchema)]
pub struct VersionResponse {
    #[schema(example = "0.6.2")]
    pub version: String,
    #[schema(example = "rushomon")]
    pub name: String,
    #[schema(example = "2024-02-23T22:30:00Z")]
    pub build_timestamp: String,
    #[schema(example = "abc123def")]
    pub git_commit: String,
}

/// Handle version endpoint: GET /api/version
/// Returns the current application version and build information
pub async fn handle_version(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let version = env!("CARGO_PKG_VERSION");

    let response = VersionResponse {
        version: version.to_string(),
        name: env!("CARGO_PKG_NAME").to_string(),
        build_timestamp: option_env!("BUILD_TIMESTAMP")
            .unwrap_or("unknown")
            .to_string(),
        git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_string(),
    };

    Response::from_json(&response)
}
