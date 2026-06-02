/// Domain management service
///
/// Handles tier-based domain limits and downgrade enforcement.
use crate::models::Tier;
use crate::models::custom_domain::{STATUS_ACTIVE, STATUS_INACTIVE_DOWNGRADE};
use crate::repositories::CustomDomainRepository;
use worker::Result;
use worker::d1::D1Database;

pub struct DomainService {
    repo: CustomDomainRepository,
}

impl DomainService {
    pub fn new() -> Self {
        Self {
            repo: CustomDomainRepository::new(),
        }
    }

    /// Apply tier downgrade enforcement for custom domains
    ///
    /// When a user downgrades, we:
    /// 1. Keep the N oldest active domains (where N = max_custom_domains for new tier)
    /// 2. Mark excess domains as inactive_downgrade
    /// 3. Return the list of deactivated domains for notification/email
    ///
    /// Downgrade scenarios:
    /// - Business → Pro: Keep 1 oldest domain, deactivate rest (Business has 3, Pro has 1)
    /// - Pro → Free: Keep 1 oldest domain, deactivate rest (Pro has 1, Free has 0... but we keep 1 as grace)
    /// - Business → Free: Keep 1 oldest domain, deactivate rest
    pub async fn apply_downgrade(
        &self,
        db: &D1Database,
        org_id: &str,
        old_tier: Tier,
        new_tier: Tier,
    ) -> Result<Vec<String>> {
        let old_limits = old_tier.limits();
        let new_limits = new_tier.limits();

        // Get max domains for each tier (None = unlimited)
        let old_max = old_limits.max_custom_domains.unwrap_or(u32::MAX);
        let new_max = new_limits.max_custom_domains.unwrap_or(u32::MAX);

        // If new tier allows equal or more domains, no action needed
        if new_max >= old_max {
            return Ok(vec![]);
        }

        // Get all active domains ordered by created_at (oldest first)
        let active_domains = self.repo.get_active_ordered(db, org_id).await?;

        // Calculate how many domains to keep vs deactivate
        let keep_count = new_max as usize;
        let total_active = active_domains.len();

        // Special case: Free tier gets 1 domain as grace (Option C from plan)
        // Even though Free tier limit is 0, we keep 1 domain active
        let effective_keep_count = if new_tier == Tier::Free && keep_count == 0 {
            1
        } else {
            keep_count
        };

        if total_active <= effective_keep_count {
            // No domains need to be deactivated
            return Ok(vec![]);
        }

        // Deactivate excess domains (newest ones, since we keep oldest)
        let mut deactivated = Vec::new();
        for domain in active_domains.iter().skip(effective_keep_count) {
            self.repo.deactivate_for_downgrade(db, &domain.id).await?;
            deactivated.push(domain.hostname.clone());
        }

        Ok(deactivated)
    }

    /// Reactivate domains when user upgrades
    ///
    /// When a user upgrades, we can reactivate domains that were previously
    /// deactivated due to downgrade, up to the new tier limit.
    #[allow(dead_code)]
    pub async fn apply_upgrade(
        &self,
        db: &D1Database,
        org_id: &str,
        old_tier: Tier,
        new_tier: Tier,
    ) -> Result<Vec<String>> {
        let old_limits = old_tier.limits();
        let new_limits = new_tier.limits();

        let old_max = old_limits.max_custom_domains.unwrap_or(u32::MAX);
        let new_max = new_limits.max_custom_domains.unwrap_or(u32::MAX);

        // If new tier doesn't allow more domains, no action needed
        if new_max <= old_max {
            return Ok(vec![]);
        }

        // Get all domains for the org
        let all_domains = self.repo.get_by_org(db, org_id).await?;

        // Find inactive domains that were deactivated due to downgrade
        let mut inactive_domains: Vec<_> = all_domains
            .into_iter()
            .filter(|d| d.status == STATUS_INACTIVE_DOWNGRADE)
            .collect();

        // Sort by created_at (oldest first - these were kept active during downgrade)
        inactive_domains.sort_by_key(|d| d.created_at);

        // Calculate how many domains we can reactivate
        let active_count = self.repo.get_active_for_org(db, org_id).await?.len();
        let can_activate = (new_max as usize).saturating_sub(active_count);

        if can_activate == 0 || inactive_domains.is_empty() {
            return Ok(vec![]);
        }

        // Reactivate up to can_activate domains
        let mut reactivated = Vec::new();
        for domain in inactive_domains.into_iter().take(can_activate) {
            self.repo.reactivate(db, &domain.id).await?;
            reactivated.push(domain.hostname);
        }

        Ok(reactivated)
    }

    /// Check if a domain can be used for creating new links
    ///
    /// Returns true only if the domain is in "active" status
    #[allow(dead_code)]
    pub async fn can_use_domain_for_links(
        &self,
        db: &D1Database,
        org_id: &str,
        hostname: &str,
    ) -> Result<bool> {
        let domain = self
            .repo
            .get_by_hostname_and_org(db, hostname, org_id)
            .await?;

        match domain {
            Some(d) => Ok(d.status == STATUS_ACTIVE),
            None => Ok(false),
        }
    }

    /// Get domain status information for frontend display
    ///
    /// Returns status and whether the domain is usable for new links
    #[allow(dead_code)]
    pub async fn get_domain_status(
        &self,
        db: &D1Database,
        org_id: &str,
        hostname: &str,
    ) -> Result<Option<DomainStatusInfo>> {
        let domain = self
            .repo
            .get_by_hostname_and_org(db, hostname, org_id)
            .await?;

        match domain {
            Some(d) => {
                let info = DomainStatusInfo {
                    status: d.status.clone(),
                    is_usable: d.status == STATUS_ACTIVE,
                    is_inactive_downgrade: d.status == STATUS_INACTIVE_DOWNGRADE,
                };
                Ok(Some(info))
            }
            None => Ok(None),
        }
    }
}

/// Domain status information for frontend display
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DomainStatusInfo {
    pub status: String,
    pub is_usable: bool,
    pub is_inactive_downgrade: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_status_info() {
        let info = DomainStatusInfo {
            status: STATUS_ACTIVE.to_string(),
            is_usable: true,
            is_inactive_downgrade: false,
        };
        assert!(info.is_usable);
        assert!(!info.is_inactive_downgrade);

        let info2 = DomainStatusInfo {
            status: STATUS_INACTIVE_DOWNGRADE.to_string(),
            is_usable: false,
            is_inactive_downgrade: true,
        };
        assert!(!info2.is_usable);
        assert!(info2.is_inactive_downgrade);
    }
}
