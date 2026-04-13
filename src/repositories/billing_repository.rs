/// Billing Repository
///
/// Data access layer for billing accounts, subscriptions, and webhook records.
/// This is the first population of this repository — additional functions will be
/// added when the Billing domain (Step 14) and Organization domain (Step 12) are extracted.
use crate::utils::now_timestamp;
use worker::Result;
use worker::d1::D1Database;

pub struct BillingRepository;

impl BillingRepository {
    pub fn new() -> Self {
        Self
    }

    /// Get all subscriptions with pending_cancellation that have expired.
    /// Returns provider_subscription_id, billing_account_id, and current_period_end for each.
    pub async fn get_expired_pending_cancellations(
        &self,
        db: &D1Database,
        now: i64,
    ) -> Result<Vec<serde_json::Value>> {
        let stmt = db.prepare(
            "SELECT provider_subscription_id, billing_account_id, current_period_end
             FROM subscriptions
             WHERE pending_cancellation = 1
               AND current_period_end < ?1
             LIMIT 1000",
        );
        let results = stmt.bind(&[(now as f64).into()])?.all().await?;
        results.results::<serde_json::Value>()
    }

    /// Update billing account tier.
    /// This will affect all organizations linked to this billing account.
    pub async fn update_billing_account_tier(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        new_tier: &str,
    ) -> Result<()> {
        let stmt = db.prepare("UPDATE billing_accounts SET tier = ?1 WHERE id = ?2");
        stmt.bind(&[new_tier.into(), billing_account_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Finalize an expired subscription after downgrading the tier.
    /// Sets status to 'canceled' and clears the pending_cancellation flag.
    pub async fn finalize_expired_subscription(
        &self,
        db: &D1Database,
        provider_subscription_id: &str,
        now: i64,
    ) -> Result<()> {
        let stmt = db.prepare(
            "UPDATE subscriptions
             SET status = 'canceled',
                 pending_cancellation = 0,
                 canceled_at = ?1,
                 updated_at = ?1
             WHERE provider_subscription_id = ?2",
        );
        stmt.bind(&[(now as f64).into(), provider_subscription_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Delete expired webhook records (for cleanup cron job).
    /// Removes all processed_webhooks rows where expires_at < now.
    pub async fn cleanup_expired_webhooks(&self, db: &D1Database) -> Result<()> {
        let now = now_timestamp();
        let stmt = db.prepare(
            "DELETE FROM processed_webhooks
             WHERE expires_at < ?1",
        );
        stmt.bind(&[(now as f64).into()])?.run().await?;
        Ok(())
    }
}

impl Default for BillingRepository {
    fn default() -> Self {
        Self::new()
    }
}
