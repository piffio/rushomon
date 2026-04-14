use crate::db::queries::{AdminLink, AdminLinkBase};
/// Report Repository
///
/// Data access layer for the `link_reports` table.
use crate::db::queries::{LinkReport, LinkReportQueryResult, LinkReportWithLink};
use crate::utils::now_timestamp;
use uuid::Uuid;
use wasm_bindgen::JsValue;
use worker::Result;
use worker::d1::D1Database;

pub struct ReportRepository;

impl ReportRepository {
    pub fn new() -> Self {
        Self
    }

    /// Create a new link abuse report. Returns the created report.
    pub async fn create(
        &self,
        db: &D1Database,
        link_id: &str,
        reason: &str,
        reporter_user_id: Option<&str>,
        reporter_email: Option<&str>,
    ) -> Result<LinkReport> {
        let id = Uuid::new_v4().to_string();
        let now = now_timestamp();

        let stmt = db.prepare(
            "INSERT INTO link_reports (id, link_id, reason, reporter_user_id, reporter_email, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        );

        stmt.bind(&[
            id.clone().into(),
            link_id.into(),
            reason.into(),
            reporter_user_id.map(|s| s.into()).unwrap_or(JsValue::NULL),
            reporter_email.map(|s| s.into()).unwrap_or(JsValue::NULL),
            "pending".into(),
            (now as f64).into(),
        ])?
        .run()
        .await?;

        self.get_by_id(db, &id).await
    }

    /// Fetch a single report by ID.
    pub async fn get_by_id(&self, db: &D1Database, report_id: &str) -> Result<LinkReport> {
        let stmt = db.prepare(
            "SELECT id, link_id, reason, reporter_user_id, reporter_email, status,
                    admin_notes, reviewed_by, reviewed_at, created_at
             FROM link_reports WHERE id = ?1",
        );

        match stmt
            .bind(&[report_id.into()])?
            .first::<LinkReport>(None)
            .await?
        {
            Some(r) => Ok(r),
            None => Err(worker::Error::RustError("Report not found".to_string())),
        }
    }

    /// Return a paginated list of reports with joined link and user info.
    pub async fn list_paginated(
        &self,
        db: &D1Database,
        page: u32,
        limit: u32,
        status_filter: Option<&str>,
    ) -> Result<(Vec<LinkReportWithLink>, i64)> {
        let offset = (page - 1) * limit;

        let count_query = format!(
            "SELECT COUNT(*) as count FROM link_reports lr {}",
            if status_filter.is_some() {
                "WHERE lr.status = ?1"
            } else {
                ""
            }
        );

        let total = if let Some(filter) = status_filter {
            db.prepare(&count_query)
                .bind(&[filter.into()])?
                .first::<serde_json::Value>(None)
                .await?
        } else {
            db.prepare(&count_query)
                .first::<serde_json::Value>(None)
                .await?
        }
        .and_then(|v| v["count"].as_f64())
        .unwrap_or(0.0) as i64;

        let reports_query = format!(
            "SELECT
                lr.id, lr.link_id, lr.reason, lr.reporter_user_id, lr.reporter_email,
                lr.status, lr.admin_notes, lr.reviewed_by, lr.reviewed_at, lr.created_at,
                l.id as link__id, l.org_id as link__org_id, l.short_code as link__short_code,
                l.destination_url as link__destination_url, l.title as link__title,
                l.created_by as link__created_by, l.created_at as link__created_at,
                l.updated_at as link__updated_at, l.expires_at as link__expires_at,
                l.status as link__status, l.click_count as link__click_count,
                l.utm_params as link__utm_params, l.forward_query_params as link__forward_query_params,
                l.redirect_type as link__redirect_type,
                u.email as link__creator_email, o.name as link__org_name,
                COUNT(lr_sub.id) as report_count
             FROM link_reports lr
             LEFT JOIN links l ON lr.link_id = l.id
             LEFT JOIN users u ON l.created_by = u.id
             LEFT JOIN organizations o ON l.org_id = o.id
             LEFT JOIN link_reports lr_sub ON lr.link_id = lr_sub.link_id
             {}
             GROUP BY lr.id, l.id, u.id, o.id
             ORDER BY lr.created_at DESC
             LIMIT {} OFFSET {}",
            if status_filter.is_some() {
                "WHERE lr.status = ?1"
            } else {
                ""
            },
            if status_filter.is_some() { "?2" } else { "?1" },
            if status_filter.is_some() { "?3" } else { "?2" }
        );

        let query_results: Vec<LinkReportQueryResult> = if let Some(filter) = status_filter {
            db.prepare(&reports_query)
                .bind(&[filter.into(), (limit as f64).into(), (offset as f64).into()])?
                .all()
                .await?
        } else {
            db.prepare(&reports_query)
                .bind(&[(limit as f64).into(), (offset as f64).into()])?
                .all()
                .await?
        }
        .results()?;

        let reports = query_results
            .into_iter()
            .map(|qr| LinkReportWithLink {
                id: qr.id,
                link_id: qr.link_id,
                link: AdminLink {
                    base: AdminLinkBase {
                        id: qr.link__id,
                        org_id: qr.link__org_id,
                        short_code: qr.link__short_code,
                        destination_url: qr.link__destination_url,
                        title: qr.link__title,
                        created_by: qr.link__created_by,
                        created_at: qr.link__created_at,
                        updated_at: qr.link__updated_at,
                        expires_at: qr.link__expires_at,
                        status: qr.link__status,
                        click_count: qr.link__click_count,
                        utm_params: qr.link__utm_params,
                        forward_query_params: qr.link__forward_query_params,
                        redirect_type: qr.link__redirect_type,
                        creator_email: qr.link__creator_email,
                        org_name: qr.link__org_name,
                    },
                    kv_sync_status: "unknown".to_string(),
                    kv_exists: false,
                },
                reason: qr.reason,
                reporter_user_id: qr.reporter_user_id,
                reporter_email: qr.reporter_email,
                status: qr.status,
                admin_notes: qr.admin_notes,
                reviewed_by: qr.reviewed_by,
                reviewed_at: qr.reviewed_at,
                created_at: qr.created_at,
                report_count: qr.report_count,
            })
            .collect();

        Ok((reports, total))
    }

    /// Update a report's status and optionally add admin notes.
    pub async fn update_status(
        &self,
        db: &D1Database,
        report_id: &str,
        status: &str,
        admin_user_id: &str,
        admin_notes: Option<&str>,
    ) -> Result<()> {
        let now = now_timestamp();
        let stmt = db.prepare(
            "UPDATE link_reports
             SET status = ?1, admin_notes = ?2, reviewed_by = ?3, reviewed_at = ?4
             WHERE id = ?5",
        );
        stmt.bind(&[
            status.into(),
            admin_notes.map(|s| s.into()).unwrap_or(JsValue::NULL),
            admin_user_id.into(),
            (now as f64).into(),
            report_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Count all pending reports (for admin badge).
    pub async fn count_pending(&self, db: &D1Database) -> Result<i64> {
        let result = db
            .prepare("SELECT COUNT(*) as count FROM link_reports WHERE status = 'pending'")
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    /// Return true if a duplicate report exists for (link, reason, reporter) within 24 h.
    pub async fn is_duplicate(
        &self,
        db: &D1Database,
        link_id: &str,
        reason: &str,
        reporter_user_id: Option<&str>,
        reporter_email: Option<&str>,
    ) -> Result<bool> {
        let cutoff = now_timestamp() - 24 * 60 * 60;

        let query = if reporter_user_id.is_some() {
            "SELECT COUNT(*) as count FROM link_reports
             WHERE link_id = ?1 AND reason = ?2 AND reporter_user_id = ?3 AND created_at > ?4"
        } else {
            "SELECT COUNT(*) as count FROM link_reports
             WHERE link_id = ?1 AND reason = ?2 AND reporter_email = ?3 AND created_at > ?4"
        };

        let reporter = reporter_user_id.or(reporter_email).unwrap_or("");
        let result = db
            .prepare(query)
            .bind(&[
                link_id.into(),
                reason.into(),
                reporter.into(),
                (cutoff as f64).into(),
            ])?
            .first::<serde_json::Value>(None)
            .await?;

        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64 > 0)
    }
}

impl Default for ReportRepository {
    fn default() -> Self {
        Self::new()
    }
}
