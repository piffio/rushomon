/// Report service - Business logic for report operations
///
/// Handles resolving reports when link status changes or admin takes action.
/// Orchestrates ReportRepository.
use crate::repositories::ReportRepository;
use crate::utils::AppError;
use worker::d1::D1Database;

/// Service for report-related business logic
pub struct ReportService;

impl ReportService {
    pub fn new() -> Self {
        Self
    }

    /// Resolve all pending reports for a link when its status changes.
    ///
    /// This is called when an admin changes a link's status to "disabled" or "blocked",
    /// which typically indicates the link was abusive and the reports should be auto-resolved.
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `link_id` - ID of the link whose reports should be resolved
    /// * `resolution` - Resolution status (e.g., "resolved", "dismissed")
    /// * `resolution_note` - Note explaining why reports were resolved
    /// * `acted_by` - User ID who took the action
    pub async fn resolve_reports_for_link(
        &self,
        db: &D1Database,
        link_id: &str,
        resolution: &str,
        resolution_note: &str,
        acted_by: &str,
    ) -> Result<(), AppError> {
        let report_repo = ReportRepository::new();

        report_repo
            .resolve_for_link(db, link_id, resolution, resolution_note, acted_by)
            .await?;

        Ok(())
    }
}
