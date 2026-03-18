//! Scheduled cron job to downgrade expired subscriptions.
//!
//! This handler runs daily at midnight UTC via Cloudflare Cron Triggers.
//! It finds subscriptions where:
//! - pending_cancellation = 1 (user canceled at period end)
//! - current_period_end < now (the period has ended)
//!
//! For each expired subscription, it:
//! - Downgrades the billing account tier to "free"
//! - Clears the pending_cancellation flag
//! - Updates the subscription status to "canceled"

use crate::db;
use crate::utils::now_timestamp;
use worker::d1::D1Database;
use worker::*;

/// Scheduled handler for downgrading expired subscriptions.
/// Called by Cloudflare Cron Triggers.
#[event(scheduled)]
pub async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_log!("[cron] Starting expired subscription downgrade job");

    let db = match env.get_binding::<D1Database>("rushomon") {
        Ok(db) => db,
        Err(e) => {
            console_error!("[cron] Failed to get database binding: {}", e);
            return;
        }
    };

    let now = now_timestamp();

    // Find expired subscriptions
    match db::get_expired_pending_cancellations(&db, now).await {
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
                    db::update_billing_account_tier(&db, billing_account_id, "free").await
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
                if let Err(e) = db::finalize_expired_subscription(&db, subscription_id, now).await {
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
                "[cron] Completed: {} successful, {} errors",
                success_count,
                error_count
            );
        }
        Err(e) => {
            console_error!("[cron] Failed to query expired subscriptions: {}", e);
        }
    }
}
