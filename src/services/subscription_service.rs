/// Subscription Service
///
/// Business logic for scheduled subscription lifecycle management:
/// - Downgrading expired subscriptions to the free tier
/// - Cleaning up stale webhook records
use crate::repositories::BillingRepository;
use crate::utils::now_timestamp;
use worker::d1::D1Database;
use worker::*;

pub struct SubscriptionService;

impl SubscriptionService {
    pub fn new() -> Self {
        Self
    }

    /// Downgrades all subscriptions whose pending-cancellation period has ended.
    /// For each expired subscription: sets billing account tier to "free" then
    /// marks the subscription as fully canceled.
    /// Returns (success_count, error_count).
    pub async fn downgrade_expired(&self, db: &D1Database) -> (usize, usize) {
        let now = now_timestamp();
        let repo = BillingRepository::new();

        let expired = match repo.get_expired_pending_cancellations(db, now).await {
            Ok(rows) => rows,
            Err(e) => {
                console_error!("[cron] Failed to query expired subscriptions: {}", e);
                return (0, 0);
            }
        };

        console_log!(
            "[cron] Found {} expired subscriptions to process",
            expired.len()
        );

        let mut success_count = 0usize;
        let mut error_count = 0usize;

        for sub in expired {
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

            if let Err(e) = repo
                .update_billing_account_tier(db, billing_account_id, "free")
                .await
            {
                console_error!(
                    "[cron] Failed to downgrade tier for billing account {}: {}",
                    billing_account_id,
                    e
                );
                error_count += 1;
                continue;
            }

            if let Err(e) = repo
                .finalize_expired_subscription(db, subscription_id, now)
                .await
            {
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

        (success_count, error_count)
    }

    /// Deletes processed_webhooks rows whose expires_at is in the past.
    pub async fn cleanup_webhooks(&self, db: &D1Database) {
        let repo = BillingRepository::new();
        match repo.cleanup_expired_webhooks(db).await {
            Ok(_) => {
                console_log!("[cron] Webhook cleanup complete");
            }
            Err(e) => {
                console_error!("[cron] Webhook cleanup failed: {}", e);
            }
        }
    }
}

impl Default for SubscriptionService {
    fn default() -> Self {
        Self::new()
    }
}
