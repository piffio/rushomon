/// Report service - Business logic for report operations
///
/// Handles resolving reports when link status changes or admin takes action.
/// Orchestrates ReportRepository and LinkRepository.
use crate::models::link::LinkStatus;
use crate::repositories::report_repository::{LinkReport, LinkReportWithLink};
use crate::repositories::{LinkRepository, ReportRepository};
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

    /// List abuse reports with optional status filter and pagination.
    pub async fn list_reports(
        &self,
        db: &D1Database,
        page: u32,
        limit: u32,
        status_filter: Option<&str>,
    ) -> Result<(Vec<LinkReportWithLink>, i64), AppError> {
        let repo = ReportRepository::new();
        Ok(repo.list_paginated(db, page, limit, status_filter).await?)
    }

    /// Get a single abuse report by ID.
    pub async fn get_report(
        &self,
        db: &D1Database,
        report_id: &str,
    ) -> Result<LinkReport, AppError> {
        let repo = ReportRepository::new();
        repo.get_by_id(db, report_id)
            .await
            .map_err(|_| AppError::NotFound("Report not found".to_string()))
    }

    /// Update the status of a report (reviewed or dismissed).
    ///
    /// Returns Err(AppError::BadRequest) if the status value is not allowed.
    pub async fn update_report_status(
        &self,
        db: &D1Database,
        report_id: &str,
        status: &str,
        admin_user_id: &str,
        admin_notes: Option<&str>,
    ) -> Result<(), AppError> {
        if status != "reviewed" && status != "dismissed" {
            return Err(AppError::BadRequest(
                "Invalid status. Must be 'reviewed' or 'dismissed'".to_string(),
            ));
        }
        let repo = ReportRepository::new();
        Ok(repo
            .update_status(db, report_id, status, admin_user_id, admin_notes)
            .await?)
    }

    /// Count pending (unreviewed) reports.
    pub async fn count_pending_reports(&self, db: &D1Database) -> Result<i64, AppError> {
        let repo = ReportRepository::new();
        Ok(repo.count_pending(db).await?)
    }

    /// Submit an abuse report for a link.
    ///
    /// Validates the link exists and is reportable, checks for duplicate submissions,
    /// persists the report, and logs the event with hashed PII.
    ///
    /// Returns Err(AppError::NotFound) if the link doesn't exist.
    /// Returns Err(AppError::BadRequest) if the link is already disabled/blocked.
    /// Returns Err(AppError::TooManyRequests) on duplicate report within 24 h.
    pub async fn submit_link_report(
        &self,
        db: &D1Database,
        link_id: &str,
        reason: &str,
        reporter_user_id: Option<&str>,
        reporter_email: Option<&str>,
    ) -> Result<LinkReport, AppError> {
        let link_repo = LinkRepository::new();
        let link = match link_repo.get_active_by_short_code(db, link_id).await {
            Ok(Some(link)) => link,
            Ok(None) => match link_repo.get_by_id_no_auth_all(db, link_id).await {
                Ok(Some(link)) => link,
                Ok(None) => {
                    return Err(AppError::NotFound("Link not found or removed".to_string()));
                }
                Err(e) => return Err(AppError::Internal(format!("Database error: {}", e))),
            },
            Err(e) => return Err(AppError::Internal(format!("Database error: {}", e))),
        };

        if matches!(link.status, LinkStatus::Blocked | LinkStatus::Disabled) {
            return Err(AppError::BadRequest(
                "This link has already been disabled and cannot be reported.".to_string(),
            ));
        }

        let actual_link_id = link.id.clone();
        let repo = ReportRepository::new();

        if repo
            .is_duplicate(
                db,
                &actual_link_id,
                reason,
                reporter_user_id,
                reporter_email,
            )
            .await
            .unwrap_or(false)
        {
            return Err(AppError::TooManyRequests(
                "You have already reported this link for the same reason within the last 24 hours"
                    .to_string(),
            ));
        }

        let report = repo
            .create(
                db,
                &actual_link_id,
                reason,
                reporter_user_id,
                reporter_email,
            )
            .await?;

        Ok(report)
    }
}
