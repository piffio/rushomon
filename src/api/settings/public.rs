/// GET /api/settings (public)
///
/// Returns non-sensitive public settings needed by the frontend,
/// including founder pricing status, active discount amounts, and
/// minimum short code lengths.
use crate::services::SettingsService;
use worker::*;

#[utoipa::path(
    get,
    path = "/api/settings/public",
    tag = "Settings",
    summary = "Get public settings",
    description = "Returns non-sensitive public settings needed by the frontend, including founder pricing status, active discount amounts for each plan tier, and minimum short code lengths",
    responses(
        (status = 200, description = "Public settings object"),
    )
)]
pub async fn handle_get_public_settings(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;
    let settings_service = SettingsService::new();
    let public_settings = settings_service.get_public_settings(&db, &ctx.env).await?;
    Response::from_json(&public_settings)
}
