/// Report Repository
///
/// Data access layer for the `link_reports` table.
use crate::repositories::link_repository::{
    AdminLink, AdminLinkBase, serialize_optional_int_as_bool,
};
use crate::utils::now_timestamp;
use uuid::Uuid;
use wasm_bindgen::JsValue;
use worker::Result;
use worker::d1::D1Database;

/// A link abuse report.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct LinkReport {
    pub id: String,
    pub link_id: String,
    pub reason: String,
    pub reporter_user_id: Option<String>,
    pub reporter_email: Option<String>,
    pub status: String,
    pub admin_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<i64>,
    pub created_at: i64,
}

/// A report with joined link information.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct LinkReportWithLink {
    pub id: String,
    pub link_id: String,
    pub link: AdminLink,
    pub reason: String,
    pub reporter_user_id: Option<String>,
    pub reporter_email: Option<String>,
    pub status: String,
    pub admin_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<i64>,
    pub created_at: i64,
    pub report_count: i64, // For grouping
}

/// Helper struct for flat query results from the database.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct LinkReportQueryResult {
    pub id: String,
    pub link_id: String,
    pub reason: String,
    pub reporter_user_id: Option<String>,
    pub reporter_email: Option<String>,
    pub status: String,
    pub admin_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<i64>,
    pub created_at: i64,
    pub link__id: String,
    pub link__org_id: String,
    pub link__short_code: String,
    pub link__destination_url: String,
    pub link__title: Option<String>,
    pub link__created_by: String,
    pub link__created_at: i64,
    pub link__updated_at: Option<i64>,
    pub link__expires_at: Option<i64>,
    pub link__status: String,
    pub link__click_count: i64,
    pub link__utm_params: Option<String>,
    #[serde(serialize_with = "serialize_optional_int_as_bool")]
    pub link__forward_query_params: Option<i64>, // Stored as INTEGER in D1 (0/1/NULL)
    pub link__redirect_type: String,
    pub link__ios_url: Option<String>,
    pub link__android_url: Option<String>,
    pub link__desktop_url: Option<String>,
    pub link__creator_email: String,
    pub link__org_name: String,
    pub report_count: i64,
}

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
                l.ios_url as link__ios_url, l.android_url as link__android_url, l.desktop_url as link__desktop_url,
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
                        ios_url: qr.link__ios_url,
                        android_url: qr.link__android_url,
                        desktop_url: qr.link__desktop_url,
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

    /// Resolve all pending reports for a specific link.
    pub async fn resolve_for_link(
        &self,
        db: &D1Database,
        link_id: &str,
        status: &str,
        admin_notes: &str,
        admin_user_id: &str,
    ) -> Result<()> {
        let now = now_timestamp();

        let stmt = db.prepare(
            "UPDATE link_reports
             SET status = ?1, admin_notes = ?2, reviewed_by = ?3, reviewed_at = ?4
             WHERE link_id = ?5 AND status = 'pending'",
        );

        stmt.bind(&[
            status.into(),
            admin_notes.into(),
            admin_user_id.into(),
            (now as f64).into(),
            link_id.into(),
        ])?
        .run()
        .await?;

        Ok(())
    }
}

impl Default for ReportRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod link_report_serialization_tests {
    use super::*;

    #[test]
    fn test_link_report_query_result_deserializes_i64_as_bool() {
        // Test that LinkReportQueryResult can deserialize link__forward_query_params as i64
        let json = r#"{
            "id": "report-1",
            "link_id": "link-1",
            "reason": "spam",
            "reporter_user_id": null,
            "reporter_email": "reporter@example.com",
            "status": "pending",
            "admin_notes": null,
            "reviewed_by": null,
            "reviewed_at": null,
            "created_at": 1234567890,
            "link__id": "link-1",
            "link__org_id": "org-1",
            "link__short_code": "abc123",
            "link__destination_url": "https://example.com",
            "link__title": null,
            "link__created_by": "user-1",
            "link__created_at": 1234567890,
            "link__updated_at": null,
            "link__expires_at": null,
            "link__status": "active",
            "link__click_count": 0,
            "link__utm_params": null,
            "link__forward_query_params": 1,
            "link__redirect_type": "301",
            "link__ios_url": null,
            "link__android_url": null,
            "link__desktop_url": null,
            "link__creator_email": "creator@example.com",
            "link__org_name": "Test Org",
            "report_count": 1
        }"#;

        let result: LinkReportQueryResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.link__forward_query_params, Some(1));
    }

    #[test]
    fn test_link_report_query_result_serializes_i64_as_bool() {
        // Test that LinkReportQueryResult serializes link__forward_query_params as bool
        let result = LinkReportQueryResult {
            id: "report-1".to_string(),
            link_id: "link-1".to_string(),
            reason: "spam".to_string(),
            reporter_user_id: None,
            reporter_email: Some("reporter@example.com".to_string()),
            status: "pending".to_string(),
            admin_notes: None,
            reviewed_by: None,
            reviewed_at: None,
            created_at: 1234567890,
            link__id: "link-1".to_string(),
            link__org_id: "org-1".to_string(),
            link__short_code: "abc123".to_string(),
            link__destination_url: "https://example.com".to_string(),
            link__title: None,
            link__created_by: "user-1".to_string(),
            link__created_at: 1234567890,
            link__updated_at: None,
            link__expires_at: None,
            link__status: "active".to_string(),
            link__click_count: 0,
            link__utm_params: None,
            link__forward_query_params: Some(1),
            link__redirect_type: "301".to_string(),
            link__ios_url: None,
            link__android_url: None,
            link__desktop_url: None,
            link__creator_email: "creator@example.com".to_string(),
            link__org_name: "Test Org".to_string(),
            report_count: 1,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        // The serializer should convert i64 to bool
        assert_eq!(parsed["link__forward_query_params"], true);
    }
}
