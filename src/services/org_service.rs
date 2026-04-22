/// Org service - Business logic for organization operations
///
/// Handles org limit enforcement and member limit checks.
/// Orchestrates BillingRepository and OrgRepository.
use crate::models::{OrgMember, Organization, Tier};
use crate::repositories::{BillingRepository, LinkRepository, OrgRepository};
use crate::utils::AppError;
use chrono::Datelike;
use worker::d1::D1Database;
use worker::kv::KvStore;

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

    // ─── Membership Guards ────────────────────────────────────────────────────

    /// Verify user is an owner or admin of the org.
    ///
    /// Returns Ok(member) on success, Err(Forbidden) if member but wrong role,
    /// Err(NotFound) if not a member.
    pub async fn require_owner_or_admin(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
        forbidden_msg: &str,
    ) -> Result<OrgMember, AppError> {
        let repo = OrgRepository::new();
        let member = repo.get_member(db, org_id, user_id).await?;
        match member {
            Some(m) if m.role == "owner" || m.role == "admin" => Ok(m),
            Some(_) => Err(AppError::Forbidden(forbidden_msg.to_string())),
            None => Err(AppError::NotFound("Organization not found".to_string())),
        }
    }

    /// Verify user is an owner of the org.
    ///
    /// Returns Ok(member) on success, Err(Forbidden) if member but not owner,
    /// Err(NotFound) if not a member.
    pub async fn require_owner(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
        forbidden_msg: &str,
    ) -> Result<OrgMember, AppError> {
        let repo = OrgRepository::new();
        let member = repo.get_member(db, org_id, user_id).await?;
        match member {
            Some(m) if m.role == "owner" => Ok(m),
            Some(_) => Err(AppError::Forbidden(forbidden_msg.to_string())),
            None => Err(AppError::NotFound("Organization not found".to_string())),
        }
    }

    // ─── Member Management ────────────────────────────────────────────────────

    /// Remove a member from an org, enforcing role-based permission rules.
    ///
    /// Rules:
    /// - Any member may remove themselves (self-removal).
    /// - Owners may remove anyone.
    /// - Admins may remove regular members only (not owners or other admins).
    /// - Regular members may only perform self-removal.
    /// - The last owner of an org cannot be removed.
    pub async fn remove_member(
        &self,
        db: &D1Database,
        org_id: &str,
        requester_id: &str,
        target_user_id: &str,
    ) -> Result<(), AppError> {
        let repo = OrgRepository::new();

        let requester = repo
            .get_member(db, org_id, requester_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        let target = repo
            .get_member(db, org_id, target_user_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound("Member not found in this organization".to_string())
            })?;

        let is_self_removal = target_user_id == requester_id;

        if !is_self_removal {
            match requester.role.as_str() {
                "owner" => {}
                "admin" => {
                    if target.role == "owner" {
                        return Err(AppError::Forbidden(
                            "Admins cannot remove owners".to_string(),
                        ));
                    }
                    if target.role == "admin" {
                        return Err(AppError::Forbidden(
                            "Admins cannot remove other admins".to_string(),
                        ));
                    }
                }
                _ => {
                    return Err(AppError::Forbidden(
                        "Only org owners and admins can remove members".to_string(),
                    ));
                }
            }
        }

        if target.role == "owner" {
            let owner_count = repo.count_owners(db, org_id).await?;
            if owner_count <= 1 {
                return Err(AppError::BadRequest(
                    "Cannot remove the last owner. Transfer ownership first.".to_string(),
                ));
            }
        }

        repo.remove_member(db, org_id, target_user_id).await?;
        Ok(())
    }

    // ─── Org Settings ─────────────────────────────────────────────────────────

    /// Get org settings (forward_query_params) with membership check.
    pub async fn get_org_settings(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
    ) -> Result<bool, AppError> {
        let repo = OrgRepository::new();
        if repo.get_member(db, org_id, user_id).await?.is_none() {
            return Err(AppError::NotFound("Organization not found".to_string()));
        }
        let forward_query_params = repo.get_forward_query_params(db, org_id).await?;
        Ok(forward_query_params)
    }

    /// Update org settings (forward_query_params) with owner/admin and tier checks.
    pub async fn update_org_settings(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
        forward_query_params: bool,
    ) -> Result<bool, AppError> {
        let repo = OrgRepository::new();

        self.require_owner_or_admin(
            db,
            org_id,
            user_id,
            "Only org owners and admins can change organization settings",
        )
        .await?;

        let org = repo
            .get_by_id(db, org_id)
            .await?
            .ok_or_else(|| AppError::Internal("Organization not found".to_string()))?;

        let tier = self.get_org_tier(db, &org).await;
        let is_pro_or_above = matches!(tier, Tier::Pro | Tier::Business | Tier::Unlimited);

        if forward_query_params && !is_pro_or_above {
            return Err(AppError::Forbidden(
                "Query parameter forwarding requires a Pro plan or above.".to_string(),
            ));
        }

        repo.set_forward_query_params(db, org_id, forward_query_params)
            .await?;
        let updated = repo.get_forward_query_params(db, org_id).await?;
        Ok(updated)
    }

    // ─── Org Logo ─────────────────────────────────────────────────────────────

    /// Check org logo upload permissions (owner/admin + Pro+).
    ///
    /// Returns Ok(()) if the caller may upload/delete a logo.
    pub async fn check_logo_permission(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
    ) -> Result<(), AppError> {
        self.require_owner_or_admin(
            db,
            org_id,
            user_id,
            "Only org owners and admins can upload a logo",
        )
        .await?;

        let repo = OrgRepository::new();
        let org = repo
            .get_by_id(db, org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

        let tier = self.get_org_tier(db, &org).await;
        if !matches!(tier, Tier::Pro | Tier::Business | Tier::Unlimited) {
            return Err(AppError::Forbidden(
                "Custom org logo requires a Pro plan or above.".to_string(),
            ));
        }
        Ok(())
    }

    /// Check delete logo permission (owner/admin only, no tier check).
    pub async fn check_delete_logo_permission(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
    ) -> Result<(), AppError> {
        self.require_owner_or_admin(
            db,
            org_id,
            user_id,
            "Only org owners and admins can delete the logo",
        )
        .await?;
        Ok(())
    }

    /// Update the stored logo URL in D1.
    pub async fn set_logo_url(
        &self,
        db: &D1Database,
        org_id: &str,
        logo_url: Option<&str>,
    ) -> Result<(), AppError> {
        OrgRepository::new()
            .set_logo_url(db, org_id, logo_url)
            .await
            .map_err(AppError::from)
    }

    // ─── Org CRUD ─────────────────────────────────────────────────────────────

    /// Create a new organization linked to the user's billing account and add them as owner.
    pub async fn create_org_with_billing(
        &self,
        db: &D1Database,
        user_id: &str,
        name: &str,
    ) -> Result<Organization, AppError> {
        let billing_repo = BillingRepository::new();
        let billing_account = billing_repo
            .get_for_user(db, user_id)
            .await?
            .ok_or_else(|| AppError::Internal("No billing account found".to_string()))?;

        let repo = OrgRepository::new();
        let org = repo
            .create_with_billing_account(db, name, user_id, &billing_account.id)
            .await?;
        repo.add_member(db, &org.id, user_id, "owner").await?;

        Ok(org)
    }

    /// Get org with membership verification.
    ///
    /// Returns the org + member record, or Err(NotFound/Forbidden) if user is not a member.
    pub async fn get_org_as_member(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
    ) -> Result<(Organization, OrgMember), AppError> {
        let repo = OrgRepository::new();
        let member = repo
            .get_member(db, org_id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
        let org = repo
            .get_by_id(db, org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
        Ok((org, member))
    }

    /// Delete an organization: handle links (delete or migrate to target org), then delete org.
    ///
    /// `action` must be `"delete"` or `"migrate"`. When migrating, `target_org_id` must be provided
    /// and the caller must be an owner of the target org.
    #[allow(clippy::too_many_arguments)]
    pub async fn delete_org(
        &self,
        db: &D1Database,
        kv: &KvStore,
        org_id: &str,
        user_id: &str,
        action: &str,
        target_org_id: Option<&str>,
    ) -> Result<(), AppError> {
        let repo = OrgRepository::new();
        let link_repo = LinkRepository::new();
        let billing_repo = BillingRepository::new();

        match action {
            "delete" => {
                let link_ids = repo.get_link_ids(db, org_id).await?;
                for link_id in &link_ids {
                    if let Some(link) = link_repo.get_by_id(db, link_id, org_id).await? {
                        let _ = kv.delete(&link.short_code).await;
                    }
                }
                repo.delete_all_links(db, org_id).await?;
            }
            "migrate" => {
                let target_id = target_org_id.ok_or_else(|| {
                    AppError::BadRequest(
                        "target_org_id is required when action is migrate".to_string(),
                    )
                })?;

                let target_member = repo.get_member(db, target_id, user_id).await?;
                match &target_member {
                    Some(m) if m.role == "owner" => {}
                    Some(_) => {
                        return Err(AppError::Forbidden(
                            "You must be an owner of the target organization".to_string(),
                        ));
                    }
                    None => {
                        return Err(AppError::NotFound(
                            "Target organization not found".to_string(),
                        ));
                    }
                }

                let target_org = repo.get_by_id(db, target_id).await?.ok_or_else(|| {
                    AppError::NotFound("Target organization not found".to_string())
                })?;

                let target_billing_account_id =
                    target_org.billing_account_id.as_deref().ok_or_else(|| {
                        AppError::Internal("Target organization has no billing account".to_string())
                    })?;
                let target_billing_account = billing_repo
                    .get_by_id(db, target_billing_account_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("Billing account not found".to_string()))?;

                let target_tier =
                    Tier::from_str_value(&target_billing_account.tier).unwrap_or(Tier::Free);
                let target_limits = target_tier.limits();
                let source_link_count = repo.count_links(db, org_id).await?;

                if let Some(max_links) = target_limits.max_links_per_month {
                    let now = chrono::Utc::now();
                    let year_month = format!("{}-{:02}", now.year(), now.month());
                    let target_current_usage = billing_repo
                        .get_monthly_counter(db, target_billing_account_id, &year_month)
                        .await?;

                    let target_available = max_links - target_current_usage;
                    if source_link_count > target_available {
                        return Err(AppError::BadRequest(format!(
                            "Target billing account has insufficient capacity. Available slots: {}, Required: {}",
                            target_available, source_link_count
                        )));
                    }

                    billing_repo
                        .increment_monthly_counter(
                            db,
                            target_billing_account_id,
                            &year_month,
                            source_link_count,
                        )
                        .await?;
                }

                repo.migrate_links(db, org_id, target_id).await?;

                let migrated_link_ids = repo.get_link_ids(db, target_id).await?;
                for link_id in &migrated_link_ids {
                    if let Some(link) = link_repo.get_by_id(db, link_id, target_id).await? {
                        let resolved_forward = link.forward_query_params.unwrap_or(false);
                        let mapping = link.to_mapping(resolved_forward);
                        if let Ok(put_builder) = kv.put(&link.short_code, mapping) {
                            let _ = put_builder.execute().await;
                        }
                    }
                }
            }
            _ => {
                return Err(AppError::BadRequest(
                    "Invalid action. Must be 'delete' or 'migrate'".to_string(),
                ));
            }
        }

        repo.delete(db, org_id).await?;
        Ok(())
    }
}
