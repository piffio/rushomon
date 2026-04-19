/// Report service - Business logic for link report operations
///
/// Handles resolving reports for a link when its status changes.
/// Orchestrates ReportRepository and LinkRepository.
use crate::repositories::{LinkRepository, ReportRepository};
use crate::utils::AppError;
use worker::console_log;
use worker::d1::D1Database;

/// Service for report-related business logic
#[derive(Default)]
pub struct ReportService;

impl ReportService {
    pub fn new() -> Self {
        Self
    }

    /// Update a link's status and, if the new status is "disabled" or "blocked",
    /// auto-resolve all pending reports for that link.
    ///
    /// KV sync must be performed by the caller after this call.
    pub async fn update_link_status_and_resolve_reports(
        &self,
        db: &D1Database,
        link_id: &str,
        status: &str,
        acted_by: &str,
    ) -> Result<(), AppError> {
        let link_repo = LinkRepository::new();
        link_repo.update_status_by_id(db, link_id, status).await?;

        if status == "disabled" || status == "blocked" {
            let report_repo = ReportRepository::new();
            if let Err(e) = report_repo
                .resolve_for_link(
                    db,
                    link_id,
                    "reviewed",
                    &format!("Action taken: Link {}", status),
                    acted_by,
                )
                .await
            {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "reports_resolve_failed",
                        "link_id": link_id,
                        "error": e.to_string(),
                        "level": "error"
                    })
                );
            }
        }

        Ok(())
    }
}
