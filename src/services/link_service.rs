use crate::models::Tier;
/// Link service - Business logic for link operations
///
/// Handles quota enforcement, blacklist checks, and tag limit validation.
/// Orchestrates BillingRepository, BlacklistRepository, and TagRepository.
use crate::models::link::{Link, LinkStatus, UtmParams};
use crate::repositories::{BillingRepository, BlacklistRepository, LinkRepository, TagRepository};
use crate::utils::AppError;
use chrono::Datelike;
use worker::d1::D1Database;
use worker::kv::KvStore;

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

    // ─── CRUD Operations ────────────────────────────────────────────────────

    /// Get a single link by ID with its tags.
    pub async fn get_link(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
    ) -> Result<Option<Link>, AppError> {
        let repo = LinkRepository::new();
        let mut link = repo.get_by_id(db, link_id, org_id).await?;
        if let Some(ref mut l) = link {
            l.tags = repo.get_tags(db, &l.id).await?;
        }
        Ok(link)
    }

    /// Get a link by short code.
    pub async fn get_link_by_code(
        &self,
        db: &D1Database,
        short_code: &str,
        org_id: &str,
    ) -> Result<Option<Link>, AppError> {
        let repo = LinkRepository::new();
        repo.get_by_short_code(db, short_code, org_id)
            .await
            .map_err(AppError::from)
    }

    /// List links with filtering, sorting, and pagination.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_links(
        &self,
        db: &D1Database,
        org_id: &str,
        search: Option<&str>,
        status_filter: Option<&str>,
        sort: &str,
        limit: i64,
        offset: i64,
        tags_filter: Option<&[String]>,
    ) -> Result<(Vec<Link>, i64, serde_json::Value), AppError> {
        let repo = LinkRepository::new();

        let total = repo
            .count_filtered(db, org_id, search, status_filter, tags_filter)
            .await?;

        let mut links = repo
            .list_filtered(
                db,
                org_id,
                search,
                status_filter,
                sort,
                limit,
                offset,
                tags_filter,
            )
            .await?;

        let stats = repo.get_dashboard_stats(db, org_id).await?;
        let stats_json = serde_json::to_value(&stats)
            .map_err(|e| AppError::Internal(format!("Failed to serialize stats: {}", e)))?;

        // Attach tags to links
        let link_ids: Vec<String> = links.iter().map(|l| l.id.clone()).collect();
        let tags_map = repo.get_tags_for_links(db, &link_ids).await?;
        for link in &mut links {
            link.tags = tags_map.get(&link.id).cloned().unwrap_or_default();
        }

        Ok((links, total, stats_json))
    }

    /// Update a link with new values.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_link(
        &self,
        db: &D1Database,
        kv: &KvStore,
        link_id: &str,
        org_id: &str,
        destination_url: Option<String>,
        title: Option<String>,
        expires_at: Option<Option<i64>>,
        tags: Option<Vec<String>>,
        utm_params: Option<Option<UtmParams>>,
        forward_query_params: Option<Option<bool>>,
        status: Option<String>,
        redirect_type: Option<String>,
    ) -> Result<Link, AppError> {
        let repo = LinkRepository::new();

        // Verify link exists and belongs to org
        let existing = repo
            .get_by_id(db, link_id, org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Link not found".to_string()))?;

        // Convert UTM params to JSON string if provided
        let utm_string: Option<Option<String>> =
            utm_params.map(|u| u.map(|p| p.to_json_string().unwrap_or_default()));
        let utm_ref: Option<Option<&str>> =
            utm_string.as_ref().map(|o| o.as_ref().map(|s| s.as_str()));

        // Update the link (single call handles all fields)
        let updated = repo
            .update(
                db,
                link_id,
                org_id,
                destination_url.as_deref(),
                title.as_deref(),
                status.as_deref(),
                expires_at,
                utm_ref, // utm_params as Option<Option<&str>>
                forward_query_params,
                redirect_type.as_deref(),
            )
            .await?;

        // Sync KV if status changed
        if let Some(ref new_status_str) = status {
            let new_status = match new_status_str.as_str() {
                "active" => LinkStatus::Active,
                "disabled" => LinkStatus::Disabled,
                "blocked" => LinkStatus::Blocked,
                _ => return Ok(updated),
            };

            if new_status != existing.status {
                // Get org default for forward_query_params if needed
                let org_repo = crate::repositories::OrgRepository::new();
                let resolved_forward = if updated.forward_query_params.is_none() {
                    org_repo
                        .get_forward_query_params(db, org_id)
                        .await
                        .unwrap_or(false)
                } else {
                    updated.forward_query_params.unwrap_or(false)
                };

                if new_status == LinkStatus::Active {
                    // Re-enable in KV by storing the mapping
                    let mapping = updated.to_mapping(resolved_forward);
                    crate::kv::store_link_mapping(kv, org_id, &updated.short_code, &mapping)
                        .await?;
                } else {
                    // Disable in KV
                    crate::kv::delete_link_mapping(kv, org_id, &updated.short_code).await?;
                }
            }
        }

        // Update tags if provided
        if let Some(new_tags) = tags {
            let normalized_tags =
                crate::repositories::tag_repository::validate_and_normalize_tags(&new_tags)?;
            repo.set_tags(db, link_id, org_id, &normalized_tags).await?;
        }

        // Return updated link with tags
        let mut result = updated;
        result.tags = repo.get_tags(db, link_id).await?;

        Ok(result)
    }

    /// Delete a link and its KV mapping.
    pub async fn delete_link(
        &self,
        db: &D1Database,
        kv: &KvStore,
        link_id: &str,
        org_id: &str,
    ) -> Result<(), AppError> {
        let repo = LinkRepository::new();

        // Get link first to retrieve short_code for KV deletion
        let link = repo
            .get_by_id(db, link_id, org_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Link not found".to_string()))?;

        // Delete from D1
        repo.hard_delete(db, link_id, org_id).await?;

        // Delete from KV
        crate::kv::delete_link_mapping(kv, org_id, &link.short_code).await?;

        Ok(())
    }

    /// Get all links for export.
    pub async fn export_links(&self, db: &D1Database, org_id: &str) -> Result<Vec<Link>, AppError> {
        let repo = LinkRepository::new();
        let links = repo.get_all_for_export(db, org_id).await?;

        // Attach tags to each link
        let link_ids: Vec<String> = links.iter().map(|l| l.id.clone()).collect();
        let tags_map = repo.get_tags_for_links(db, &link_ids).await?;

        let mut result = links;
        for link in &mut result {
            link.tags = tags_map.get(&link.id).cloned().unwrap_or_default();
        }

        Ok(result)
    }

    // ─── Admin Operations ────────────────────────────────────────────────────

    /// List all links for admin with filtering.
    #[allow(clippy::too_many_arguments)]
    pub async fn admin_list_links(
        &self,
        db: &D1Database,
        kv: &KvStore,
        page: i64,
        limit: i64,
        org_filter: Option<&str>,
        email_filter: Option<&str>,
        domain_filter: Option<&str>,
    ) -> Result<(Vec<crate::repositories::link_repository::AdminLink>, i64), AppError> {
        let repo = LinkRepository::new();
        let offset = (page - 1) * limit;

        let links = repo
            .list_admin(
                db,
                kv,
                limit,
                offset,
                org_filter,
                email_filter,
                domain_filter,
            )
            .await?;
        let total = repo
            .count_admin(db, org_filter, email_filter, domain_filter)
            .await?;

        Ok((links, total))
    }

    /// Update link status (admin only).
    pub async fn admin_update_link_status(
        &self,
        db: &D1Database,
        kv: &KvStore,
        link_id: &str,
        status: LinkStatus,
    ) -> Result<(), AppError> {
        let repo = LinkRepository::new();

        // Get link without org check (admin operation)
        let link = repo
            .get_by_id_no_auth_all(db, link_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Link not found".to_string()))?;

        // Update status in D1
        repo.update_status_by_id(db, link_id, status.as_str())
            .await?;

        // Sync KV based on new status
        if status == LinkStatus::Active {
            // Re-enable in KV by storing the mapping
            let mapping = crate::models::LinkMapping {
                destination_url: link.destination_url.clone(),
                link_id: link.id.clone(),
                expires_at: link.expires_at,
                status: crate::models::link::LinkStatus::Active,
                forward_query_params: link.forward_query_params.unwrap_or(false),
                utm_params: link.utm_params.clone(),
                redirect_type: link.redirect_type.clone(),
            };
            crate::kv::store_link_mapping(kv, &link.org_id, &link.short_code, &mapping).await?;
        } else {
            // Disable in KV
            crate::kv::delete_link_mapping(kv, &link.org_id, &link.short_code).await?;
        }

        // Resolve any pending reports for this link (system resolution on status change)
        let report_service = crate::services::ReportService::new();
        report_service
            .resolve_reports_for_link(
                db,
                link_id,
                "resolved",
                "Link status changed by admin",
                "system",
            )
            .await?;

        Ok(())
    }

    /// Delete a link as admin.
    pub async fn admin_delete_link(
        &self,
        db: &D1Database,
        kv: &KvStore,
        link_id: &str,
    ) -> Result<(), AppError> {
        let repo = LinkRepository::new();

        // Get link without org check
        let link = repo
            .get_by_id_no_auth_all(db, link_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Link not found".to_string()))?;

        // Delete from D1
        repo.hard_delete(db, link_id, &link.org_id).await?;

        // Delete from KV
        crate::kv::delete_link_mapping(kv, &link.org_id, &link.short_code).await?;

        Ok(())
    }

    /// Create a new link with all associated operations.
    ///
    /// Creates the link in D1, sets tags, stores KV mapping, and updates org timestamp.
    pub async fn create_link(
        &self,
        db: &D1Database,
        kv: &KvStore,
        link: &Link,
        tags: &[String],
        org_id: &str,
    ) -> Result<(), AppError> {
        let repo = LinkRepository::new();
        let org_repo = crate::repositories::OrgRepository::new();

        // Create link in D1
        repo.create(db, link).await?;

        // Set tags if any
        if !tags.is_empty() {
            repo.set_tags(db, &link.id, org_id, tags).await?;
        }

        // Resolve forward_query_params from org default if not set on link
        let resolved_forward = if link.forward_query_params.is_none() {
            org_repo
                .get_forward_query_params(db, org_id)
                .await
                .unwrap_or(false)
        } else {
            link.forward_query_params.unwrap_or(false)
        };

        // Store in KV
        let mapping = link.to_mapping(resolved_forward);
        crate::kv::store_link_mapping(kv, org_id, &link.short_code, &mapping).await?;

        Ok(())
    }

    /// Re-sync a link's KV entry based on its current status (admin only).
    pub async fn admin_sync_link_kv(
        &self,
        db: &D1Database,
        kv: &KvStore,
        link_id: &str,
    ) -> Result<(), AppError> {
        let repo = LinkRepository::new();

        let link = repo
            .get_by_id_no_auth_all(db, link_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Link not found".to_string()))?;

        match link.status.as_str() {
            "active" => {
                let link_model = crate::models::Link {
                    id: link.id.clone(),
                    org_id: link.org_id.clone(),
                    short_code: link.short_code.clone(),
                    destination_url: link.destination_url.clone(),
                    title: link.title.clone(),
                    created_by: link.created_by.clone(),
                    created_at: link.created_at,
                    updated_at: link.updated_at,
                    expires_at: link.expires_at,
                    status: crate::models::link::LinkStatus::Active,
                    click_count: link.click_count,
                    tags: link.tags,
                    utm_params: link.utm_params,
                    forward_query_params: link.forward_query_params,
                    redirect_type: link.redirect_type.clone(),
                };
                let org_repo = crate::repositories::OrgRepository::new();
                let resolved_forward = if let Some(forward) = link.forward_query_params {
                    forward
                } else {
                    org_repo
                        .get_forward_query_params(db, &link.org_id)
                        .await
                        .unwrap_or(false)
                };
                let mapping = link_model.to_mapping(resolved_forward);
                crate::kv::store_link_mapping(kv, &link.org_id, &link.short_code, &mapping).await?;
            }
            "blocked" | "disabled" => {
                crate::kv::delete_link_mapping(kv, &link.org_id, &link.short_code).await?;
            }
            _ => {
                return Err(AppError::BadRequest(
                    "Cannot sync link with unknown status".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Check whether the org's billing tier allows Pro-only link features.
    ///
    /// Returns the org's billing account tier. Returns Err(AppError::Forbidden)
    /// with a user-facing message if pro features are requested but the tier is Free.
    pub async fn check_pro_features_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
        wants_redirect_type: bool,
        wants_utm_or_forward: bool,
    ) -> Result<(String, Option<Tier>), AppError> {
        let billing_repo = BillingRepository::new();
        let billing_account = billing_repo.get_for_org(db, org_id).await?.ok_or_else(|| {
            AppError::Internal("No billing account found for organization".to_string())
        })?;
        let tier = Tier::from_str_value(&billing_account.tier);
        let is_pro_or_above = matches!(
            tier.as_ref(),
            Some(Tier::Pro) | Some(Tier::Business) | Some(Tier::Unlimited)
        );

        if (wants_redirect_type || wants_utm_or_forward) && !is_pro_or_above {
            let error_msg = if wants_redirect_type {
                "Custom redirect types (307) require a Pro plan or above."
            } else {
                "UTM parameters and query parameter forwarding require a Pro plan or above."
            };
            return Err(AppError::Forbidden(error_msg.to_string()));
        }

        Ok((billing_account.id, tier))
    }

    /// Check tag limits for a link update operation, accounting for tags being removed.
    ///
    /// Returns Err(AppError::Forbidden) if adding the new tags would exceed the billing account limit.
    pub async fn check_tag_limit_for_update(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        link_id: &str,
        new_tags: &[String],
        max_tags: i64,
    ) -> Result<(), AppError> {
        let tag_repo = TagRepository::new();
        let current_tag_count = tag_repo
            .count_distinct_tags_for_billing_account(db, billing_account_id)
            .await?;

        let existing_link_tags = LinkRepository::new().get_tags(db, link_id).await?;
        let existing_tags_set: std::collections::HashSet<String> =
            existing_link_tags.into_iter().collect();
        let new_tags_set: std::collections::HashSet<String> = new_tags.iter().cloned().collect();

        let existing_ba_tags_result = db
            .prepare(
                "SELECT DISTINCT tag_name
                 FROM link_tags lt
                 JOIN organizations o ON lt.org_id = o.id
                 WHERE o.billing_account_id = ?1",
            )
            .bind(&[billing_account_id.into()])?
            .all()
            .await?;
        let existing_ba_tags_set: std::collections::HashSet<String> = existing_ba_tags_result
            .results::<serde_json::Value>()?
            .iter()
            .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
            .collect();

        let tags_being_removed: std::collections::HashSet<String> = existing_tags_set
            .difference(&new_tags_set)
            .cloned()
            .collect();

        let tags_being_added: std::collections::HashSet<String> = new_tags_set
            .difference(&existing_tags_set)
            .cloned()
            .collect();

        let mut disappearing_count = 0i64;
        for tag in &tags_being_removed {
            let usage_result = db
                .prepare(
                    "SELECT COUNT(*) as count
                     FROM link_tags lt
                     JOIN organizations o ON lt.org_id = o.id
                     WHERE o.billing_account_id = ?1 AND lt.tag_name = ?2 AND lt.link_id != ?3",
                )
                .bind(&[
                    billing_account_id.into(),
                    tag.as_str().into(),
                    link_id.into(),
                ])?
                .first::<serde_json::Value>(None)
                .await?;
            let usage_count = usage_result
                .and_then(|r| r["count"].as_f64())
                .unwrap_or(0.0) as i64;
            if usage_count == 0 {
                disappearing_count += 1;
            }
        }

        let new_to_ba_count = tags_being_added
            .iter()
            .filter(|tag| !existing_ba_tags_set.contains(*tag))
            .count() as i64;

        let net_change = new_to_ba_count - disappearing_count;

        if current_tag_count + net_change > max_tags {
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

    /// Import multiple links in bulk.
    ///
    /// Returns detailed results with created count, skipped count, failed count,
    /// and detailed errors/warnings for each row.
    pub async fn import_links(
        &self,
        db: &D1Database,
        kv: &KvStore,
        org_id: &str,
        _user_id: &str,
        links: Vec<crate::models::link::Link>,
        tags_list: Vec<Vec<String>>,
    ) -> Result<ImportResult, AppError> {
        let mut created = 0;
        let mut skipped = 0;
        let mut failed = 0;
        let mut errors: Vec<ImportError> = Vec::new();
        let mut warnings: Vec<ImportWarning> = Vec::new();

        for (idx, (link, tags)) in links.into_iter().zip(tags_list).enumerate() {
            let row_num = idx + 1;

            // Check if short code already exists
            if crate::kv::links::short_code_exists(kv, &link.short_code).await? {
                skipped += 1;
                warnings.push(ImportWarning {
                    row: row_num,
                    destination_url: link.destination_url.clone(),
                    reason: format!("Short code '{}' already exists", link.short_code),
                });
                continue;
            }

            // Create the link
            if let Err(e) = self.create_link(db, kv, &link, &tags, org_id).await {
                failed += 1;
                errors.push(ImportError {
                    row: row_num,
                    destination_url: link.destination_url.clone(),
                    reason: e.to_string(),
                });
            } else {
                created += 1;
            }
        }

        Ok(ImportResult {
            created,
            skipped,
            failed,
            errors,
            warnings,
        })
    }
}

/// Result of a bulk import operation.
#[derive(Debug)]
pub struct ImportResult {
    pub created: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<ImportError>,
    pub warnings: Vec<ImportWarning>,
}

/// Error for a single import row.
#[derive(Debug)]
pub struct ImportError {
    pub row: usize,
    pub destination_url: String,
    pub reason: String,
}

/// Warning for a single import row.
#[derive(Debug)]
pub struct ImportWarning {
    pub row: usize,
    pub destination_url: String,
    pub reason: String,
}
