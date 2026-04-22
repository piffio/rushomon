/// Billing Repository
///
/// Data access layer for billing accounts, subscriptions, and webhook records.
use crate::models::{BillingAccount, User};
use crate::utils::now_timestamp;
use wasm_bindgen::JsValue;
use worker::Result;
use worker::d1::D1Database;

/// Response type for billing account list with stats
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BillingAccountWithStats {
    pub id: String,
    pub owner_user_id: String,
    pub owner_email: String,
    pub owner_name: Option<String>,
    pub tier: String,
    pub org_count: i64,
    pub total_members: i64,
    pub links_created_this_month: i64,
    pub created_at: i64,
}

/// Response type for org within a billing account
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OrgWithMembersCount {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub member_count: i64,
    pub link_count: i64,
    pub created_at: i64,
}

/// Usage stats for billing account
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UsageStats {
    pub links_created_this_month: i64,
    pub max_links_per_month: Option<i64>,
    pub year_month: String,
}

/// Response type for detailed billing account view
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BillingAccountDetails {
    pub account: BillingAccount,
    pub owner: User,
    pub organizations: Vec<OrgWithMembersCount>,
    pub usage: UsageStats,
    pub subscription: Option<serde_json::Value>,
}

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

    /// Get the billing account for the organization that contains the user.
    /// Returns None if the organization has no billing account associated.
    pub async fn get_billing_account_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<Option<BillingAccount>> {
        db.prepare(
            "SELECT ba.id, ba.owner_user_id, ba.tier, ba.provider_customer_id, ba.created_at
             FROM billing_accounts ba
             JOIN organizations o ON o.billing_account_id = ba.id
             WHERE o.id = ?1",
        )
        .bind(&[org_id.into()])?
        .first::<BillingAccount>(None)
        .await
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

    /// Get billing account by ID.
    pub async fn get_by_id(&self, db: &D1Database, id: &str) -> Result<Option<BillingAccount>> {
        db.prepare(
            "SELECT id, owner_user_id, tier, provider_customer_id, created_at
             FROM billing_accounts
             WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<BillingAccount>(None)
        .await
    }

    /// Get the billing account of the user's primary organization.
    pub async fn get_for_user(
        &self,
        db: &D1Database,
        user_id: &str,
    ) -> Result<Option<BillingAccount>> {
        db.prepare(
            "SELECT ba.id, ba.owner_user_id, ba.tier, ba.provider_customer_id, ba.created_at
             FROM billing_accounts ba
             INNER JOIN organizations o ON o.billing_account_id = ba.id
             INNER JOIN users u ON u.org_id = o.id
             WHERE u.id = ?1",
        )
        .bind(&[user_id.into()])?
        .first::<BillingAccount>(None)
        .await
    }

    /// Get billing account for an organization.
    pub async fn get_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<Option<BillingAccount>> {
        db.prepare(
            "SELECT ba.id, ba.owner_user_id, ba.tier, ba.provider_customer_id, ba.created_at
             FROM billing_accounts ba
             INNER JOIN organizations o ON o.billing_account_id = ba.id
             WHERE o.id = ?1",
        )
        .bind(&[org_id.into()])?
        .first::<BillingAccount>(None)
        .await
    }

    /// Create a new billing account owned by the given user.
    #[allow(dead_code)]
    pub async fn create(
        &self,
        db: &D1Database,
        owner_user_id: &str,
        tier: &str,
    ) -> Result<BillingAccount> {
        let id = BillingAccount::generate_id();
        let now = now_timestamp();
        db.prepare(
            "INSERT INTO billing_accounts (id, owner_user_id, tier, created_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&[
            id.clone().into(),
            owner_user_id.into(),
            tier.into(),
            (now as f64).into(),
        ])?
        .run()
        .await?;
        Ok(BillingAccount {
            id,
            owner_user_id: owner_user_id.to_string(),
            tier: tier.to_string(),
            provider_customer_id: None,
            created_at: now,
        })
    }

    /// Get monthly usage counter for a billing account.
    pub async fn get_monthly_counter(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        year_month: &str,
    ) -> Result<i64> {
        let result = db
            .prepare(
                "SELECT links_created
                 FROM monthly_counters
                 WHERE billing_account_id = ?1 AND year_month = ?2",
            )
            .bind(&[billing_account_id.into(), year_month.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result
            .and_then(|v| v["links_created"].as_f64())
            .unwrap_or(0.0) as i64)
    }

    /// Increment monthly counter. Returns true if under limit, false if limit exceeded.
    #[allow(dead_code)]
    pub async fn increment_monthly_counter(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        year_month: &str,
        max_value: i64,
    ) -> Result<bool> {
        let current = self
            .get_monthly_counter(db, billing_account_id, year_month)
            .await?;
        if current >= max_value {
            return Ok(false);
        }
        let now = now_timestamp();
        db.prepare(
            "INSERT INTO monthly_counters (billing_account_id, year_month, links_created, updated_at)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(billing_account_id, year_month)
             DO UPDATE SET links_created = links_created + 1",
        )
        .bind(&[
            billing_account_id.into(),
            year_month.into(),
            (now as f64).into(),
        ])?
        .run()
        .await?;
        Ok(true)
    }

    /// Reset monthly counter for a billing account (admin only, for testing).
    pub async fn reset_monthly_counter(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        year_month: &str,
    ) -> Result<()> {
        let stmt = db.prepare(
            "DELETE FROM monthly_counters
             WHERE billing_account_id = ?1 AND year_month = ?2",
        );
        stmt.bind(&[billing_account_id.into(), year_month.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Count organizations linked to a billing account.
    pub async fn count_orgs(&self, db: &D1Database, billing_account_id: &str) -> Result<i64> {
        let result = db
            .prepare(
                "SELECT COUNT(*) as count
                 FROM organizations
                 WHERE billing_account_id = ?1",
            )
            .bind(&[billing_account_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    /// Reset a billing account to free tier: delete all subscriptions and clear provider_customer_id.
    pub async fn reset_to_free(&self, db: &D1Database, billing_account_id: &str) -> Result<()> {
        db.prepare("DELETE FROM subscriptions WHERE billing_account_id = ?1")
            .bind(&[billing_account_id.into()])?
            .run()
            .await?;
        db.prepare(
            "UPDATE billing_accounts SET tier = 'free', provider_customer_id = NULL WHERE id = ?1",
        )
        .bind(&[billing_account_id.into()])?
        .run()
        .await?;
        Ok(())
    }

    /// Update billing account tier.
    pub async fn update_tier(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        new_tier: &str,
    ) -> Result<()> {
        db.prepare("UPDATE billing_accounts SET tier = ?1 WHERE id = ?2")
            .bind(&[new_tier.into(), billing_account_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Update billing account owner.
    pub async fn update_owner(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        new_owner_id: &str,
    ) -> Result<()> {
        db.prepare("UPDATE billing_accounts SET owner_user_id = ?1 WHERE id = ?2")
            .bind(&[new_owner_id.into(), billing_account_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Store/update the Polar customer ID on a billing account.
    pub async fn update_provider_customer_id(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        provider_customer_id: &str,
    ) -> Result<()> {
        db.prepare("UPDATE billing_accounts SET provider_customer_id = ?1 WHERE id = ?2")
            .bind(&[provider_customer_id.into(), billing_account_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Look up a billing_account_id by provider customer ID (fallback for webhooks).
    pub async fn get_id_by_provider_customer(
        &self,
        db: &D1Database,
        provider_customer_id: &str,
    ) -> Result<Option<String>> {
        let result = db
            .prepare("SELECT id FROM billing_accounts WHERE provider_customer_id = ?1 LIMIT 1")
            .bind(&[provider_customer_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["id"].as_str().map(|s| s.to_string())))
    }

    /// Get billing account ID for an organization.
    #[allow(dead_code)]
    pub async fn get_id_for_org(&self, db: &D1Database, org_id: &str) -> Result<Option<String>> {
        let result = db
            .prepare("SELECT billing_account_id FROM organizations WHERE id = ?1")
            .bind(&[org_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["billing_account_id"].as_str().map(|s| s.to_string())))
    }

    /// Get the current active subscription for a billing account.
    pub async fn get_subscription(
        &self,
        db: &D1Database,
        billing_account_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        db.prepare(
            "SELECT id, billing_account_id, status, plan, interval,
                    provider_subscription_id, provider_customer_id, provider_price_id,
                    current_period_start, current_period_end,
                    cancel_at_period_end, canceled_at, created_at, updated_at,
                    amount_cents, currency, discount_name, pending_cancellation
             FROM subscriptions
             WHERE billing_account_id = ?1
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(&[billing_account_id.into()])?
        .first::<serde_json::Value>(None)
        .await
    }

    /// Insert or update a subscription record.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_subscription(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        provider_subscription_id: &str,
        provider_customer_id: &str,
        status: &str,
        plan: &str,
        interval: &str,
        provider_price_id: &str,
        current_period_start: i64,
        current_period_end: i64,
        cancel_at_period_end: bool,
        amount_cents: Option<i64>,
        currency: &str,
        discount_name: Option<&str>,
        ends_at: Option<i64>,
        now: i64,
    ) -> Result<()> {
        let sub_id = format!("sub_{}", crate::utils::generate_short_code_with_length(16));
        let cancel_flag: i64 = if cancel_at_period_end { 1 } else { 0 };
        db.prepare(
            "INSERT INTO subscriptions (
               id, billing_account_id, status, plan, interval,
               provider_subscription_id, provider_customer_id, provider_price_id,
               current_period_start, current_period_end, ends_at,
               cancel_at_period_end, amount_cents, currency, discount_name,
               created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
             ON CONFLICT(provider_subscription_id) DO UPDATE SET
               status = excluded.status,
               plan = excluded.plan,
               interval = excluded.interval,
               provider_price_id = excluded.provider_price_id,
               current_period_start = excluded.current_period_start,
               current_period_end = excluded.current_period_end,
               ends_at = excluded.ends_at,
               cancel_at_period_end = excluded.cancel_at_period_end,
               amount_cents = excluded.amount_cents,
               currency = excluded.currency,
               discount_name = excluded.discount_name,
               updated_at = excluded.updated_at",
        )
        .bind(&[
            sub_id.into(),
            billing_account_id.into(),
            status.into(),
            plan.into(),
            interval.into(),
            provider_subscription_id.into(),
            provider_customer_id.into(),
            provider_price_id.into(),
            (current_period_start as f64).into(),
            (current_period_end as f64).into(),
            ends_at
                .map(|v| (v as f64).into())
                .unwrap_or(JsValue::from_str("")),
            (cancel_flag as f64).into(),
            (amount_cents.unwrap_or(0) as f64).into(),
            currency.into(),
            discount_name.unwrap_or("").into(),
            (now as f64).into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Mark a subscription as canceled.
    pub async fn mark_subscription_canceled(
        &self,
        db: &D1Database,
        provider_subscription_id: &str,
        now: i64,
    ) -> Result<()> {
        db.prepare(
            "UPDATE subscriptions
             SET status = 'canceled', canceled_at = ?1, updated_at = ?1
             WHERE provider_subscription_id = ?2",
        )
        .bind(&[(now as f64).into(), provider_subscription_id.into()])?
        .run()
        .await?;
        Ok(())
    }

    /// Set subscription as pending cancellation (cancel_at_period_end = true).
    pub async fn set_subscription_pending_cancellation(
        &self,
        db: &D1Database,
        provider_subscription_id: &str,
        current_period_end: i64,
    ) -> Result<()> {
        db.prepare(
            "UPDATE subscriptions
             SET pending_cancellation = 1,
                 cancel_at_period_end = 1,
                 updated_at = ?1
             WHERE provider_subscription_id = ?2",
        )
        .bind(&[
            (current_period_end as f64).into(),
            provider_subscription_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Clear pending cancellation flag.
    pub async fn clear_subscription_pending_cancellation(
        &self,
        db: &D1Database,
        provider_subscription_id: &str,
    ) -> Result<()> {
        db.prepare(
            "UPDATE subscriptions
             SET pending_cancellation = 0,
                 cancel_at_period_end = 0,
                 updated_at = (SELECT strftime('%s', 'now'))
             WHERE provider_subscription_id = ?1",
        )
        .bind(&[provider_subscription_id.into()])?
        .run()
        .await?;
        Ok(())
    }

    /// Update subscription status (admin only).
    pub async fn update_subscription_status(
        &self,
        db: &D1Database,
        subscription_id: &str,
        status: &str,
        now: i64,
    ) -> Result<()> {
        db.prepare(
            "UPDATE subscriptions
             SET status = ?1, updated_at = ?2
             WHERE id = ?3",
        )
        .bind(&[status.into(), (now as f64).into(), subscription_id.into()])?
        .run()
        .await?;
        Ok(())
    }

    /// Check if a webhook has already been processed (idempotency).
    pub async fn webhook_already_processed(
        &self,
        db: &D1Database,
        provider: &str,
        webhook_id: &str,
    ) -> Result<bool> {
        let result = db
            .prepare(
                "SELECT 1 FROM processed_webhooks
                 WHERE provider = ?1 AND webhook_id = ?2
                 LIMIT 1",
            )
            .bind(&[provider.into(), webhook_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.is_some())
    }

    /// Mark a webhook as processed (idempotency).
    pub async fn mark_webhook_processed(
        &self,
        db: &D1Database,
        provider: &str,
        webhook_id: &str,
        event_type: &str,
        ttl_seconds: i64,
    ) -> Result<()> {
        let now = now_timestamp();
        let expires_at = now + ttl_seconds;
        let record_id = format!("{}_{}_{}", provider, webhook_id, now);
        db.prepare(
            "INSERT INTO processed_webhooks
             (id, provider, webhook_id, event_type, processed_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&[
            record_id.into(),
            provider.into(),
            webhook_id.into(),
            event_type.into(),
            (now as f64).into(),
            (expires_at as f64).into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// List all billing accounts with stats (admin, paginated, filterable).
    pub async fn list_for_admin(
        &self,
        db: &D1Database,
        page: i64,
        limit: i64,
        search: Option<&str>,
        tier_filter: Option<&str>,
    ) -> Result<(Vec<BillingAccountWithStats>, i64)> {
        let offset = (page - 1) * limit;
        let current_month = chrono::Utc::now().format("%Y-%m").to_string();

        let mut where_clauses: Vec<&str> = vec![];
        let mut bind_values: Vec<JsValue> = vec![];

        if let Some(search_term) = search {
            where_clauses.push("u.email LIKE ?");
            bind_values.push(format!("%{}%", search_term).into());
        }
        if let Some(tier) = tier_filter {
            where_clauses.push("ba.tier = ?");
            bind_values.push(tier.into());
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!(
            "SELECT COUNT(DISTINCT ba.id) as count
             FROM billing_accounts ba
             LEFT JOIN users u ON u.id = ba.owner_user_id
             {}",
            where_sql
        );
        let count_result = if bind_values.is_empty() {
            db.prepare(&count_sql)
                .first::<serde_json::Value>(None)
                .await?
        } else {
            db.prepare(&count_sql)
                .bind(&bind_values)?
                .first::<serde_json::Value>(None)
                .await?
        };
        let total = count_result
            .and_then(|v| v["count"].as_f64())
            .unwrap_or(0.0) as i64;

        let mut bind_with_limit = bind_values.clone();
        bind_with_limit.push(current_month.clone().into());
        bind_with_limit.push((limit as f64).into());
        bind_with_limit.push((offset as f64).into());

        let query_sql = format!(
            "SELECT ba.id, ba.owner_user_id, ba.tier, ba.created_at,
                    u.email as owner_email, u.name as owner_name,
                    COUNT(DISTINCT o.id) as org_count,
                    COUNT(DISTINCT om.user_id) as total_members,
                    COALESCE(mc.links_created, 0) as links_created_this_month
             FROM billing_accounts ba
             LEFT JOIN users u ON u.id = ba.owner_user_id
             LEFT JOIN organizations o ON o.billing_account_id = ba.id
             LEFT JOIN org_members om ON om.org_id = o.id
             LEFT JOIN monthly_counters mc ON mc.billing_account_id = ba.id AND mc.year_month = ?
             {}
             GROUP BY ba.id
             ORDER BY ba.created_at DESC
             LIMIT ? OFFSET ?",
            where_sql
        );

        let rows = db
            .prepare(&query_sql)
            .bind(&bind_with_limit)?
            .all()
            .await?
            .results::<serde_json::Value>()?;

        let accounts = rows
            .iter()
            .filter_map(|row| {
                Some(BillingAccountWithStats {
                    id: row["id"].as_str()?.to_string(),
                    owner_user_id: row["owner_user_id"].as_str()?.to_string(),
                    owner_email: row["owner_email"].as_str()?.to_string(),
                    owner_name: row["owner_name"].as_str().map(|s| s.to_string()),
                    tier: row["tier"].as_str()?.to_string(),
                    org_count: row["org_count"].as_f64()? as i64,
                    total_members: row["total_members"].as_f64()? as i64,
                    links_created_this_month: row["links_created_this_month"].as_f64()? as i64,
                    created_at: row["created_at"].as_f64()? as i64,
                })
            })
            .collect();

        Ok((accounts, total))
    }

    /// Get detailed view of a single billing account with orgs, usage, subscription.
    pub async fn get_details(
        &self,
        db: &D1Database,
        billing_account_id: &str,
    ) -> Result<Option<BillingAccountDetails>> {
        let account = match self.get_by_id(db, billing_account_id).await? {
            Some(acc) => acc,
            None => return Ok(None),
        };

        let owner = match db
            .prepare("SELECT * FROM users WHERE id = ?1")
            .bind(&[account.owner_user_id.clone().into()])?
            .first::<User>(None)
            .await?
        {
            Some(u) => u,
            None => return Ok(None),
        };

        let orgs_rows = db
            .prepare(
                "SELECT o.id, o.name, o.slug, o.created_at,
                        COUNT(DISTINCT om.user_id) as member_count,
                        COUNT(DISTINCT l.id) as link_count
                 FROM organizations o
                 LEFT JOIN org_members om ON om.org_id = o.id
                 LEFT JOIN links l ON l.org_id = o.id AND l.status = 'active'
                 WHERE o.billing_account_id = ?1
                 GROUP BY o.id
                 ORDER BY o.created_at ASC",
            )
            .bind(&[billing_account_id.into()])?
            .all()
            .await?
            .results::<serde_json::Value>()?;

        let organizations = orgs_rows
            .iter()
            .filter_map(|row| {
                Some(OrgWithMembersCount {
                    id: row["id"].as_str()?.to_string(),
                    name: row["name"].as_str()?.to_string(),
                    slug: row["slug"].as_str()?.to_string(),
                    member_count: row["member_count"].as_f64()? as i64,
                    link_count: row["link_count"].as_f64()? as i64,
                    created_at: row["created_at"].as_f64()? as i64,
                })
            })
            .collect();

        let current_month = chrono::Utc::now().format("%Y-%m").to_string();
        let counter = self
            .get_monthly_counter(db, billing_account_id, &current_month)
            .await?;
        let tier =
            crate::models::Tier::from_str_value(&account.tier).unwrap_or(crate::models::Tier::Free);
        let limits = tier.limits();

        let usage = UsageStats {
            links_created_this_month: counter,
            max_links_per_month: limits.max_links_per_month,
            year_month: current_month,
        };

        let subscription = self.get_subscription(db, billing_account_id).await?;

        Ok(Some(BillingAccountDetails {
            account,
            owner,
            organizations,
            usage,
            subscription,
        }))
    }
}

impl Default for BillingRepository {
    fn default() -> Self {
        Self::new()
    }
}
