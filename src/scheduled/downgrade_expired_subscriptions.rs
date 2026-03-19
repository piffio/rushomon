//! Scheduled cron job handler for subscription management and webhook cleanup.
//!
//! This handler runs two separate jobs via Cloudflare Cron Triggers:
//!
//! 1. subscription_downgrade (daily at midnight UTC):
//!    - Finds subscriptions where pending_cancellation = 1 AND current_period_end < now
//!    - Downgrades billing account tier to "free"
//!    - Marks subscription as canceled
//!
//! 2. webhook_cleanup (daily at 4 AM UTC):
//!    - Deletes webhook records older than 30 days from processed_webhooks table

use crate::db;
use crate::utils::now_timestamp;
use worker::d1::D1Database;
use worker::*;

/// Scheduled handler for subscription downgrade and webhook cleanup.
/// Called by Cloudflare Cron Triggers with names "subscription_downgrade" or "webhook_cleanup".
#[event(scheduled)]
pub async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let cron_name = event.cron();

    let db = match env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[cron] Failed to get database binding: {}", e);
            return;
        }
    };

    match cron_name.as_str() {
        "subscription_downgrade" => {
            console_log!("[cron] Starting subscription downgrade job");
            run_subscription_downgrade(&db).await;
        }
        "webhook_cleanup" => {
            console_log!("[cron] Starting webhook cleanup job");
            run_webhook_cleanup(&db).await;
        }
        other => {
            console_warn!("[cron] Unknown cron trigger: {}", other);
        }
    }

    console_log!("[cron] Scheduled job complete");
}

/// Downgrades expired subscriptions to free tier.
async fn run_subscription_downgrade(db: &D1Database) {
    let now = now_timestamp();

    // Find expired subscriptions
    match db::get_expired_pending_cancellations(db, now).await {
        Ok(expired_subscriptions) => {
            console_log!(
                "[cron] Found {} expired subscriptions to process",
                expired_subscriptions.len()
            );

            let mut success_count = 0;
            let mut error_count = 0;

            for sub in expired_subscriptions {
                let subscription_id = sub
                    .get("provider_subscription_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let billing_account_id = sub
                    .get("billing_account_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let current_period_end = sub
                    .get("current_period_end")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                console_log!(
                    "[cron] Processing expired subscription: {} (billing_account: {}, period_end: {})",
                    subscription_id,
                    billing_account_id,
                    current_period_end
                );

                // Downgrade tier to free
                if let Err(e) =
                    db::update_billing_account_tier(db, billing_account_id, "free").await
                {
                    console_error!(
                        "[cron] Failed to downgrade tier for billing account {}: {}",
                        billing_account_id,
                        e
                    );
                    error_count += 1;
                    continue;
                }

                // Mark subscription as fully canceled
                if let Err(e) = db::finalize_expired_subscription(db, subscription_id, now).await {
                    console_error!(
                        "[cron] Failed to finalize subscription {}: {}",
                        subscription_id,
                        e
                    );
                    error_count += 1;
                    continue;
                }

                success_count += 1;
                console_log!(
                    "[cron] Successfully downgraded subscription {} to free tier",
                    subscription_id
                );
            }

            console_log!(
                "[cron] Subscription downgrade complete: {} successful, {} errors",
                success_count,
                error_count
            );
        }
        Err(e) => {
            console_error!("[cron] Failed to query expired subscriptions: {}", e);
        }
    }
}

/// Cleans up expired webhook records (older than 30 days).
async fn run_webhook_cleanup(db: &D1Database) {
    match db::cleanup_expired_webhooks(db).await {
        Ok(_) => {
            console_log!("[cron] Webhook cleanup complete");
        }
        Err(e) => {
            console_error!("[cron] Webhook cleanup failed: {}", e);
        }
    }
}
