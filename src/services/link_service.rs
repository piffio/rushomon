/// Link service - Business logic for link operations
///
/// Handles quota enforcement, blacklist checks, and tag limit validation.
/// Orchestrates BillingRepository, BlacklistRepository, and TagRepository.
use crate::models::Tier;
use crate::repositories::{BillingRepository, BlacklistRepository, TagRepository};
use crate::utils::AppError;
use chrono::Datelike;
use worker::d1::D1Database;

/// Context returned after a successful quota check, carrying billing info
/// needed for subsequent operations (tag checks, link creation).
pub struct QuotaContext {
    pub billing_account_id: String,
    pub tier: Option<Tier>,
}

impl QuotaContext {
    pub fn tier_limits(&self) -> Option<crate::models::tier::TierLimits> {
        self.tier.as_ref().map(|t| t.limits())
    }

    pub fn is_pro_or_above(&self) -> bool {
        matches!(
            self.tier.as_ref(),
            Some(Tier::Pro) | Some(Tier::Business) | Some(Tier::Unlimited)
        )
    }
}

/// Service for link-related business logic
#[derive(Default)]
pub struct LinkService;

impl LinkService {
    pub fn new() -> Self {
        Self
    }

    /// Load billing account for org, increment the monthly counter if a limit applies,
    /// and return a QuotaContext for downstream checks.
    ///
    /// Returns Err(AppError::Forbidden) if the monthly limit has been reached.
    /// Returns Err(AppError::InternalError) if there is no billing account.
    pub async fn check_quota(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<QuotaContext, AppError> {
        let billing_repo = BillingRepository::new();
        let billing_account = billing_repo.get_for_org(db, org_id).await?.ok_or_else(|| {
            AppError::Internal("No billing account found for organization".to_string())
        })?;

        let tier = Tier::from_str_value(&billing_account.tier);
        let limits = tier.as_ref().map(|t| t.limits());

        if let Some(ref tier_limits) = limits
            && let Some(max_links) = tier_limits.max_links_per_month
        {
            let now = chrono::Utc::now();
            let year_month = format!("{}-{:02}", now.year(), now.month());

            let can_create = billing_repo
                .increment_monthly_counter(db, &billing_account.id, &year_month, max_links)
                .await?;

            if !can_create {
                let current_count = billing_repo
                    .get_monthly_counter(db, &billing_account.id, &year_month)
                    .await?;
                let remaining = max_links.saturating_sub(current_count);
                let message = if remaining > 0 {
                    format!(
                        "You can create {} more short links this month across all organizations.",
                        remaining
                    )
                } else {
                    "You have reached your monthly link limit across all organizations. Upgrade your plan to create more links.".to_string()
                };
                return Err(AppError::Forbidden(message));
            }
        }

        Ok(QuotaContext {
            billing_account_id: billing_account.id,
            tier,
        })
    }

    /// Check whether a destination URL is blacklisted.
    ///
    /// Returns Err(AppError::Forbidden) if blocked.
    pub async fn check_blacklist(&self, db: &D1Database, url: &str) -> Result<(), AppError> {
        let blacklist_repo = BlacklistRepository::new();
        if blacklist_repo.is_blacklisted(db, url).await? {
            return Err(AppError::Forbidden(
                "Destination URL is blocked".to_string(),
            ));
        }
        Ok(())
    }

    /// Check whether adding the given new tags would exceed the billing account's tag limit.
    ///
    /// Returns Err(AppError::Forbidden) with a user-facing message if the limit would be exceeded.
    pub async fn check_tag_limit(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        new_tags: &[String],
        max_tags: i64,
    ) -> Result<(), AppError> {
        let current_tag_count = TagRepository::new()
            .count_distinct_tags_for_billing_account(db, billing_account_id)
            .await?;

        let new_tag_count = if !new_tags.is_empty() {
            let existing_tags_result = db
                .prepare(
                    "SELECT DISTINCT tag_name
                     FROM link_tags lt
                     JOIN organizations o ON lt.org_id = o.id
                     WHERE o.billing_account_id = ?1",
                )
                .bind(&[billing_account_id.into()])?
                .all()
                .await?;
            let existing_tags_set: std::collections::HashSet<String> = existing_tags_result
                .results::<serde_json::Value>()?
                .iter()
                .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
                .collect();

            new_tags
                .iter()
                .filter(|tag| !existing_tags_set.contains(*tag))
                .count() as i64
        } else {
            0
        };

        if current_tag_count + new_tag_count > max_tags {
            let remaining = max_tags.saturating_sub(current_tag_count);
            let message = if remaining > 0 {
                format!(
                    "You can create {} more tag{} across all organizations. Upgrade your plan to add more tags.",
                    remaining,
                    if remaining == 1 { "" } else { "s" }
                )
            } else {
                "You have reached your tag limit across all organizations. Upgrade your plan to create more tags.".to_string()
            };
            return Err(AppError::Forbidden(message));
        }

        Ok(())
    }
}
