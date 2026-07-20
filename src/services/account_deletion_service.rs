/// Account Deletion Service
///
/// Business logic for self-service account deletion with a grace period.
/// Users can schedule deletion (7-day grace), cancel within that window,
/// and a cron job permanently deletes expired accounts.
use crate::kv;
use crate::models::Tier;
use crate::repositories::{ApiKeyRepository, BillingRepository, OrgRepository, UserRepository};
use crate::utils::{AppError, now_timestamp};
use worker::d1::D1Database;
use worker::{Env, KvStore, Result, console_log, console_warn};

/// Grace period in seconds before permanent deletion (7 days).
const DELETION_GRACE_PERIOD_SECONDS: i64 = 7 * 24 * 60 * 60;

pub struct AccountDeletionService {
    user_repo: UserRepository,
    org_repo: OrgRepository,
    billing_repo: BillingRepository,
    api_key_repo: ApiKeyRepository,
}

#[derive(Debug, serde::Serialize)]
pub struct DeletionScheduledResult {
    pub scheduled_deletion_at: i64,
    pub grace_period_seconds: i64,
}

impl AccountDeletionService {
    pub fn new() -> Self {
        Self {
            user_repo: UserRepository::new(),
            org_repo: OrgRepository::new(),
            billing_repo: BillingRepository::new(),
            api_key_repo: ApiKeyRepository::new(),
        }
    }

    /// Schedule account deletion for the authenticated user.
    ///
    /// Checks:
    /// - User exists
    /// - User is not already pending deletion (returns current schedule if so)
    /// - User is not the last owner of any org with multiple members (paid tiers)
    ///
    /// Solo free/pro-tier orgs (max_members == 1) are allowed — they will be
    /// auto-deleted during hard deletion. Only orgs with multiple members
    /// (Business/Unlimited) block the operation.
    ///
    /// Sets `pending_deletion_at` to now + 7 days.
    pub async fn request_deletion(
        &self,
        db: &D1Database,
        user_id: &str,
    ) -> std::result::Result<DeletionScheduledResult, AppError> {
        let user = self
            .user_repo
            .get_user_by_id(db, user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to load user: {}", e)))?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if let Some(existing) = user.pending_deletion_at {
            return Ok(DeletionScheduledResult {
                scheduled_deletion_at: existing,
                grace_period_seconds: DELETION_GRACE_PERIOD_SECONDS,
            });
        }

        let orgs = self
            .org_repo
            .get_user_orgs(db, user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to load user orgs: {}", e)))?;

        let mut blocking_orgs: Vec<String> = Vec::new();
        for org in &orgs {
            if org.role == "owner" {
                let member_count = self
                    .org_repo
                    .count_members(db, &org.id)
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to count members: {}", e)))?;

                if member_count > 1 {
                    blocking_orgs.push(org.name.clone());
                    continue;
                }

                let billing_account =
                    self.billing_repo
                        .get_for_org(db, &org.id)
                        .await
                        .map_err(|e| {
                            AppError::Internal(format!("Failed to load billing account: {}", e))
                        })?;

                if let Some(ba) = billing_account {
                    let tier = Tier::from_str_value(&ba.tier).unwrap_or(Tier::Free);
                    let limits = tier.limits();
                    if limits.max_members.unwrap_or(1) > 1 {
                        blocking_orgs.push(org.name.clone());
                    }
                }
            }
        }

        if !blocking_orgs.is_empty() {
            return Err(AppError::BadRequest(format!(
                "Cannot delete account: you are the last owner of {} organization(s) \
                 with multiple members: {}. Please transfer ownership or remove members \
                 before deleting your account.",
                blocking_orgs.len(),
                blocking_orgs.join(", ")
            )));
        }

        let now = now_timestamp();
        let deletion_at = now + DELETION_GRACE_PERIOD_SECONDS;

        self.user_repo
            .schedule_deletion(db, user_id, deletion_at)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to schedule deletion: {}", e)))?;

        Ok(DeletionScheduledResult {
            scheduled_deletion_at: deletion_at,
            grace_period_seconds: DELETION_GRACE_PERIOD_SECONDS,
        })
    }

    /// Cancel a pending account deletion.
    pub async fn cancel_deletion(
        &self,
        db: &D1Database,
        user_id: &str,
    ) -> std::result::Result<(), AppError> {
        let user = self
            .user_repo
            .get_user_by_id(db, user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to load user: {}", e)))?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if user.pending_deletion_at.is_none() {
            return Err(AppError::BadRequest(
                "No pending account deletion to cancel".to_string(),
            ));
        }

        self.user_repo
            .cancel_deletion(db, user_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to cancel deletion: {}", e)))?;

        Ok(())
    }

    /// Process all users whose grace period has expired.
    /// Called by the daily cron job. Performs permanent hard deletion.
    ///
    /// For each user:
    /// 1. Delete solo orgs owned by the user (links, org, billing account)
    /// 2. Cancel Polar subscriptions for paid billing accounts
    /// 3. Revoke all API keys
    /// 4. Delete user's KV link mappings
    /// 5. Delete the user record
    pub async fn process_expired_deletions(
        &self,
        db: &D1Database,
        kv: &KvStore,
        env: &Env,
    ) -> Result<()> {
        let now = now_timestamp();
        let users = self.user_repo.get_users_due_for_deletion(db, now).await?;

        let polar_client = crate::billing::polar::polar_client_from_env(env).ok();

        for user in &users {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "account_deletion_processing",
                    "user_id": user.id,
                    "scheduled_at": user.pending_deletion_scheduled_at,
                    "level": "info"
                })
            );

            let user_links = match self.user_repo.get_links_by_creator(db, &user.id).await {
                Ok(links) => links,
                Err(e) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "account_deletion_kv_lookup_failed",
                            "user_id": user.id,
                            "error": e.to_string(),
                            "level": "error"
                        })
                    );
                    continue;
                }
            };

            for link in &user_links {
                if let Err(e) = kv::delete_link_mapping(kv, &link.org_id, &link.short_code).await {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "account_deletion_kv_delete_failed",
                            "user_id": user.id,
                            "short_code": link.short_code,
                            "error": e.to_string(),
                            "level": "warn"
                        })
                    );
                }
            }

            if let Err(e) = self.api_key_repo.revoke_all_for_user(db, &user.id).await {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "account_deletion_api_key_revoke_failed",
                        "user_id": user.id,
                        "error": e.to_string(),
                        "level": "warn"
                    })
                );
            }

            let orgs = match self.org_repo.get_user_orgs(db, &user.id).await {
                Ok(o) => o,
                Err(e) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "account_deletion_org_lookup_failed",
                            "user_id": user.id,
                            "error": e.to_string(),
                            "level": "error"
                        })
                    );
                    continue;
                }
            };

            for org in &orgs {
                if org.role != "owner" {
                    continue;
                }

                let member_count = match self.org_repo.count_members(db, &org.id).await {
                    Ok(c) => c,
                    Err(e) => {
                        console_log!(
                            "{}",
                            serde_json::json!({
                                "event": "account_deletion_member_count_failed",
                                "user_id": user.id,
                                "org_id": org.id,
                                "error": e.to_string(),
                                "level": "warn"
                            })
                        );
                        continue;
                    }
                };

                if member_count > 1 {
                    continue;
                }

                let billing_account = self
                    .billing_repo
                    .get_for_org(db, &org.id)
                    .await
                    .ok()
                    .flatten();

                let is_solo_eligible = if let Some(ref ba) = billing_account {
                    let tier = Tier::from_str_value(&ba.tier).unwrap_or(Tier::Free);
                    let limits = tier.limits();
                    limits.max_members.unwrap_or(1) <= 1
                } else {
                    true
                };

                if !is_solo_eligible {
                    continue;
                }

                if let Err(e) = self.org_repo.delete_all_links(db, &org.id).await {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "account_deletion_org_links_delete_failed",
                            "user_id": user.id,
                            "org_id": org.id,
                            "error": e.to_string(),
                            "level": "warn"
                        })
                    );
                }

                if let Err(e) = self.org_repo.delete(db, &org.id).await {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "account_deletion_org_delete_failed",
                            "user_id": user.id,
                            "org_id": org.id,
                            "error": e.to_string(),
                            "level": "warn"
                        })
                    );
                    continue;
                }

                if let Some(ref ba) = billing_account {
                    if let Some(ref polar) = polar_client
                        && let Err(e) = polar.delete_customer_by_external_id(&ba.id, true).await
                    {
                        console_warn!(
                            "{}",
                            serde_json::json!({
                                "event": "account_deletion_polar_cancel_failed",
                                "user_id": user.id,
                                "billing_account_id": ba.id,
                                "error": e.to_string(),
                                "level": "warn"
                            })
                        );
                    }

                    if let Err(e) = self.billing_repo.delete(db, &ba.id).await {
                        console_log!(
                            "{}",
                            serde_json::json!({
                                "event": "account_deletion_billing_delete_failed",
                                "user_id": user.id,
                                "billing_account_id": ba.id,
                                "error": e.to_string(),
                                "level": "warn"
                            })
                        );
                    }
                }
            }

            match self.user_repo.delete(db, &user.id).await {
                Ok((user_count, links_count, analytics_count)) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "account_deletion_completed",
                            "user_id": user.id,
                            "deleted_users": user_count,
                            "deleted_links": links_count,
                            "deleted_analytics": analytics_count,
                            "level": "info"
                        })
                    );
                }
                Err(e) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "account_deletion_db_failed",
                            "user_id": user.id,
                            "error": e.to_string(),
                            "level": "error"
                        })
                    );
                }
            }
        }

        Ok(())
    }
}

impl Default for AccountDeletionService {
    fn default() -> Self {
        Self::new()
    }
}
