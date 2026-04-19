/// Blacklist service - Business logic for blacklist cascade operations
///
/// Handles checking URLs against the blacklist and cascading blocks to
/// all existing links that match a newly-added blacklist entry.
/// Orchestrates BlacklistRepository, LinkRepository, and ReportRepository.
use crate::repositories::{BlacklistRepository, LinkRepository, ReportRepository};
use crate::utils::AppError;
use worker::console_log;
use worker::d1::D1Database;
use worker::kv::KvStore;

/// Service for blacklist-related business logic
#[derive(Default)]
pub struct BlacklistService;

impl BlacklistService {
    pub fn new() -> Self {
        Self
    }

    /// Block all existing active/disabled links whose destination matches the blacklist,
    /// remove them from KV, and auto-resolve any open reports for those links.
    ///
    /// Returns the number of links that were blocked.
    pub async fn block_matching_links(
        &self,
        db: &D1Database,
        kv: &KvStore,
        acted_by: &str,
        match_type: &str,
        destination: &str,
    ) -> Result<i64, AppError> {
        let blacklist_repo = BlacklistRepository::new();
        let link_repo = LinkRepository::new();
        let report_repo = ReportRepository::new();

        let candidate_links = blacklist_repo.get_candidate_links(db).await?;
        let mut blocked_count = 0i64;

        for link in candidate_links {
            if !blacklist_repo
                .is_blacklisted(db, &link.destination_url)
                .await?
            {
                continue;
            }

            link_repo
                .update_status_by_id(db, &link.id, "blocked")
                .await?;
            blocked_count += 1;

            if let Err(e) = report_repo
                .resolve_for_link(
                    db,
                    &link.id,
                    "reviewed",
                    &format!("Action taken: Blocked {} ({})", match_type, destination),
                    acted_by,
                )
                .await
            {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "reports_resolve_failed",
                        "link_id": link.id,
                        "error": e.to_string(),
                        "level": "error"
                    })
                );
            }

            match crate::kv::delete_link_mapping(kv, &link.org_id, &link.short_code).await {
                Ok(()) => {}
                Err(e) => {
                    console_log!(
                        "{}",
                        serde_json::json!({
                            "event": "admin_block_destination_kv_delete_failed",
                            "short_code": link.short_code,
                            "link_id": link.id,
                            "error": e.to_string(),
                            "level": "error"
                        })
                    );
                }
            }
        }

        Ok(blocked_count)
    }
}
