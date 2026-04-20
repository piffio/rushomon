/// Org service - Business logic for organization operations
///
/// Handles org limit enforcement and member limit checks.
/// Orchestrates BillingRepository and OrgRepository.
use crate::models::{Organization, Tier};
use crate::repositories::{BillingRepository, OrgRepository};
use crate::utils::AppError;
use worker::d1::D1Database;

/// Service for organization-related business logic
#[derive(Default)]
pub struct OrgService;

impl OrgService {
    pub fn new() -> Self {
        Self
    }

    /// Check whether the user's billing account has capacity to create another organization.
    ///
    /// Returns Err(AppError::Forbidden) if the org limit for their tier has been reached.
    /// Returns Err(AppError::Internal) if no billing account is found.
    pub async fn check_org_limit(&self, db: &D1Database, user_id: &str) -> Result<(), AppError> {
        let billing_repo = BillingRepository::new();
        let billing_account = billing_repo
            .get_for_user(db, user_id)
            .await?
            .ok_or_else(|| AppError::Internal("No billing account found".to_string()))?;

        let tier = Tier::from_str_value(&billing_account.tier).unwrap_or(Tier::Free);
        let limits = tier.limits();

        if let Some(max_orgs) = limits.max_orgs {
            let orgs_in_billing_account = billing_repo.count_orgs(db, &billing_account.id).await?;
            if orgs_in_billing_account >= max_orgs {
                return Err(AppError::Forbidden(format!(
                    "Organization limit reached ({}/{}). Upgrade your plan to create more organizations.",
                    orgs_in_billing_account, max_orgs
                )));
            }
        }

        Ok(())
    }

    /// Check whether an org has capacity for a new member (considering pending invitations).
    ///
    /// Returns Err(AppError::Forbidden) if the member limit for the org's tier has been reached.
    pub async fn check_member_limit(&self, db: &D1Database, org_id: &str) -> Result<(), AppError> {
        let repo = OrgRepository::new();
        let billing_repo = BillingRepository::new();

        let org = repo
            .get_by_id(db, org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        let tier = if let Some(ref ba_id) = org.billing_account_id {
            if let Ok(Some(ba)) = billing_repo.get_by_id(db, ba_id).await {
                Tier::from_str_value(&ba.tier).unwrap_or(Tier::Free)
            } else {
                Tier::Free
            }
        } else {
            Tier::Free
        };

        let limits = tier.limits();

        if let Some(max_members) = limits.max_members {
            let current_members = repo.count_members(db, org_id).await?;
            let pending_invites = repo.count_pending_invitations(db, org_id).await?;
            if current_members + pending_invites >= max_members {
                return Err(AppError::Forbidden(format!(
                    "Member limit reached ({}/{})",
                    current_members + pending_invites,
                    max_members
                )));
            }
        }

        Ok(())
    }

    /// Get the effective billing tier for an organization.
    ///
    /// Looks up the billing account linked to the org and returns its tier.
    /// Returns `Tier::Free` if no billing account is found.
    pub async fn get_org_tier(&self, db: &D1Database, org: &Organization) -> Tier {
        let billing_repo = BillingRepository::new();
        if let Some(ref billing_account_id) = org.billing_account_id
            && let Ok(Some(billing_account)) = billing_repo.get_by_id(db, billing_account_id).await
        {
            return Tier::from_str_value(&billing_account.tier).unwrap_or(Tier::Free);
        }
        Tier::Free
    }

    /// List all organizations a user belongs to, enriched with billing tier.
    ///
    /// Returns a JSON-serializable list of org summaries including tier, role, and joined_at.
    pub async fn list_user_orgs_with_tier(
        &self,
        db: &D1Database,
        user_id: &str,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        let repo = OrgRepository::new();
        let billing_repo = BillingRepository::new();
        let orgs = repo.get_user_orgs(db, user_id).await?;

        let mut result = Vec::with_capacity(orgs.len());
        for org in orgs {
            let tier = if let Ok(Some(org_details)) = repo.get_by_id(db, &org.id).await {
                if let Some(ref billing_account_id) = org_details.billing_account_id {
                    if let Ok(Some(ba)) = billing_repo.get_by_id(db, billing_account_id).await {
                        ba.tier
                    } else {
                        "free".to_string()
                    }
                } else {
                    "free".to_string()
                }
            } else {
                "free".to_string()
            };

            result.push(serde_json::json!({
                "id": org.id,
                "name": org.name,
                "tier": tier,
                "role": org.role,
                "joined_at": org.joined_at,
            }));
        }
        Ok(result)
    }
}
