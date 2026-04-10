/// GET /api/settings (public)
///
/// Returns non-sensitive public settings needed by the frontend,
/// including founder pricing status and active discount amounts.
use crate::services::SettingsService;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/settings/public",
    tag = "Settings",
    summary = "Get public settings",
    description = "Returns non-sensitive public settings needed by the frontend, including founder pricing status and active discount amounts for each plan tier",
    responses(
        (status = 200, description = "Public settings object"),
    )
)]
pub async fn handle_get_public_settings(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;
    let settings_service = SettingsService::new();
    let public_settings = settings_service.get_public_settings(&db).await?;
    Response::from_json(&public_settings)
}
