//! Scheduled cron job handler for subscription management and webhook cleanup.
//!
//! This handler runs two separate jobs via Cloudflare Cron Triggers.
//! The job to run is determined by the cron expression that triggered the event.

use crate::services::SubscriptionService;
use worker::d1::D1Database;
use worker::*;

/// Scheduled handler for subscription downgrade and webhook cleanup.
/// Called by Cloudflare Cron Triggers.
/// Determines which job to run based on the cron expression.
#[event(scheduled)]
pub async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let db = match env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[cron] Failed to get database binding: {}", e);
            return;
        }
    };

    let service = SubscriptionService::new();
    let cron_expr = event.cron();

    match cron_expr.as_str() {
        "0 0 * * *" => {
            console_log!("[cron] Starting subscription downgrade job (midnight UTC)");
            service.downgrade_expired(&db).await;
        }
        "0 4 * * *" => {
            console_log!("[cron] Starting webhook cleanup job (4 AM UTC)");
            service.cleanup_webhooks(&db).await;
        }
        other => {
            console_warn!("[cron] Unexpected cron expression: {}", other);
        }
    }

    console_log!("[cron] Scheduled job complete");
}
