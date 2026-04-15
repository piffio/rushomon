/// Link repository — all SQL operations for the links domain.
///
/// Covers:
/// - User-facing link CRUD (create, list, get, update, delete)
/// - Admin link listing and KV sync status
/// - Tag management per link (get, set, delete)
/// - Analytics event logging and click-count increment
/// - Export helpers
/// - Dashboard statistics
use crate::db;
use crate::models::{AnalyticsEvent, Link, link::LinkStatus};
use crate::utils::now_timestamp;
use serde::Serializer;
use wasm_bindgen::JsValue;
use worker::d1::D1Database;
use worker::*;

/// Serialize Option<i64> as Option<bool> for JSON responses (0 = false, 1 = true, NULL = None)
pub fn serialize_optional_int_as_bool<S>(
    value: &Option<i64>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_some(&(*v != 0)),
        None => serializer.serialize_none(),
    }
}

// ─── Structs ──────────────────────────────────────────────────────────────────

/// Dashboard statistics for an organization
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DashboardStats {
    pub total_links: i64,
    pub active_links: i64,
    pub total_clicks: i64,
}

/// Link with admin-only fields (creator email, org name)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminLinkBase {
    pub id: String,
    pub org_id: String,
    pub short_code: String,
    pub destination_url: String,
    pub title: Option<String>,
    pub created_by: String,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub status: String,
    pub click_count: i64,
    pub utm_params: Option<String>,
    #[serde(serialize_with = "serialize_optional_int_as_bool")]
    pub forward_query_params: Option<i64>, // Stored as INTEGER in D1 (0/1/NULL)
    pub redirect_type: String,
    pub creator_email: String,
    pub org_name: String,
}

/// Admin link with KV sync status
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AdminLink {
    #[serde(flatten)]
    pub base: AdminLinkBase,
    pub kv_sync_status: String, // "synced", "missing", "mismatched"
    pub kv_exists: bool,
}

// ─── Repository ───────────────────────────────────────────────────────────────

pub struct LinkRepository;

#[allow(dead_code)]
impl LinkRepository {
    pub fn new() -> Self {
        Self
    }

    // ─── User-facing CRUD ────────────────────────────────────────────────────

    /// Insert a new link into D1
    pub async fn create(&self, db: &D1Database, link: &Link) -> Result<()> {
        let utm_json = link.utm_params.as_ref().and_then(|u| u.to_json_string());

        let stmt = db.prepare(
            "INSERT INTO links (id, org_id, short_code, destination_url, title, created_by, created_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
        );

        stmt.bind(&[
            link.id.clone().into(),
            link.org_id.clone().into(),
            link.short_code.clone().into(),
            link.destination_url.clone().into(),
            link.title
                .clone()
                .map(|t| t.into())
                .unwrap_or(JsValue::NULL),
            link.created_by.clone().into(),
            (link.created_at as f64).into(),
            link.expires_at
                .map(|t| (t as f64).into())
                .unwrap_or(JsValue::NULL),
            link.status.as_str().into(),
            (link.click_count as f64).into(),
            utm_json.map(|s| s.into()).unwrap_or(JsValue::NULL),
            link.forward_query_params
                .map(|v| (if v { 1i64 } else { 0i64 } as f64).into())
                .unwrap_or(JsValue::NULL),
            link.redirect_type.clone().into(),
        ])?
        .run()
        .await?;

        Ok(())
    }

    /// Get a link by ID scoped to an org (active or disabled only)
    pub async fn get_by_id(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
    ) -> Result<Option<Link>> {
        let stmt = db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
             FROM links
             WHERE id = ?1
             AND org_id = ?2
             AND status IN ('active', 'disabled')"
        );
        stmt.bind(&[link_id.into(), org_id.into()])?
            .first::<Link>(None)
            .await
    }

    /// Get a link by ID without org check — active only (public redirects)
    pub async fn get_by_id_no_auth(&self, db: &D1Database, link_id: &str) -> Result<Option<Link>> {
        let stmt = db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
             FROM links
             WHERE id = ?1
             AND status = 'active'"
        );
        stmt.bind(&[link_id.into()])?.first::<Link>(None).await
    }

    /// Get a link by ID without org check — all statuses (admin operations)
    pub async fn get_by_id_no_auth_all(
        &self,
        db: &D1Database,
        link_id: &str,
    ) -> Result<Option<Link>> {
        let stmt = db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
             FROM links
             WHERE id = ?1"
        );
        stmt.bind(&[link_id.into()])?.first::<Link>(None).await
    }

    /// Get a link by short_code scoped to an org (active or disabled)
    pub async fn get_by_short_code(
        &self,
        db: &D1Database,
        short_code: &str,
        org_id: &str,
    ) -> Result<Option<Link>> {
        let stmt = db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
             FROM links
             WHERE short_code = ?1
             AND org_id = ?2
             AND status IN ('active', 'disabled')"
        );
        stmt.bind(&[short_code.into(), org_id.into()])?
            .first::<Link>(None)
            .await
    }

    /// Get an active link by short_code (public reporting)
    pub async fn get_active_by_short_code(
        &self,
        db: &D1Database,
        short_code: &str,
    ) -> Result<Option<Link>> {
        let stmt = db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
             FROM links
             WHERE short_code = ?1
             AND status = 'active'"
        );
        stmt.bind(&[short_code.into()])?.first::<Link>(None).await
    }

    /// Get links for an org with search/filter/sort/tag-filter options
    #[allow(clippy::too_many_arguments)]
    pub async fn list_filtered(
        &self,
        db: &D1Database,
        org_id: &str,
        search: Option<&str>,
        status_filter: Option<&str>,
        sort: &str,
        limit: i64,
        offset: i64,
        tags_filter: Option<&[String]>,
    ) -> Result<Vec<Link>> {
        let mut query = String::from(
            "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
             FROM links
             WHERE org_id = ?1"
        );

        let mut params: Vec<JsValue> = vec![org_id.into()];

        if let Some(status) = status_filter {
            query.push_str(&format!(" AND status = ?{}", params.len() + 1));
            params.push(status.into());
        } else {
            query.push_str(" AND status IN ('active', 'disabled')");
        }

        if let Some(search_term) = search {
            let term = format!("%{}%", search_term.replace('%', "\\%").replace('_', "\\_"));
            query.push_str(&format!(
                " AND (LOWER(title) LIKE LOWER(?{}) OR LOWER(short_code) LIKE LOWER(?{}) OR LOWER(destination_url) LIKE LOWER(?{}))",
                params.len() + 1,
                params.len() + 2,
                params.len() + 3
            ));
            params.push(term.clone().into());
            params.push(term.clone().into());
            params.push(term.into());
        }

        if let Some(tags) = tags_filter {
            if tags.len() == 1 {
                query.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM link_tags lt WHERE lt.link_id = id AND lt.tag_name = ?{})",
                    params.len() + 1
                ));
                params.push(tags[0].as_str().into());
            } else if !tags.is_empty() {
                let start_param = params.len() + 1;
                let placeholders: Vec<String> = (0..tags.len())
                    .map(|i| format!("?{}", start_param + i))
                    .collect();
                query.push_str(&format!(
                    " AND id IN (SELECT DISTINCT link_id FROM link_tags WHERE tag_name IN ({}))",
                    placeholders.join(", ")
                ));
                for tag in tags {
                    params.push(tag.as_str().into());
                }
            }
        }

        let order_clause = match sort {
            "clicks" => " ORDER BY click_count DESC",
            "updated" => " ORDER BY updated_at DESC NULLS LAST",
            "title" => " ORDER BY title ASC NULLS LAST",
            "code" => " ORDER BY short_code ASC",
            _ => " ORDER BY created_at DESC",
        };
        query.push_str(order_clause);

        query.push_str(&format!(
            " LIMIT ?{} OFFSET ?{}",
            params.len() + 1,
            params.len() + 2
        ));
        params.push((limit as f64).into());
        params.push((offset as f64).into());

        let stmt = db.prepare(&query);
        let results = stmt.bind(&params)?.all().await?;
        results.results::<Link>()
    }

    /// Count links for an org with search/status/tag filters
    pub async fn count_filtered(
        &self,
        db: &D1Database,
        org_id: &str,
        search: Option<&str>,
        status_filter: Option<&str>,
        tags_filter: Option<&[String]>,
    ) -> Result<i64> {
        let mut query = String::from("SELECT COUNT(*) as count FROM links WHERE org_id = ?1");

        let mut params: Vec<JsValue> = vec![org_id.into()];

        if let Some(status) = status_filter {
            query.push_str(&format!(" AND status = ?{}", params.len() + 1));
            params.push(status.into());
        } else {
            query.push_str(" AND status IN ('active', 'disabled')");
        }

        if let Some(search_term) = search {
            let term = format!("%{}%", search_term.replace('%', "\\%").replace('_', "\\_"));
            query.push_str(&format!(
                " AND (LOWER(title) LIKE LOWER(?{}) OR LOWER(short_code) LIKE LOWER(?{}) OR LOWER(destination_url) LIKE LOWER(?{}))",
                params.len() + 1,
                params.len() + 2,
                params.len() + 3
            ));
            params.push(term.clone().into());
            params.push(term.clone().into());
            params.push(term.into());
        }

        if let Some(tags) = tags_filter {
            if tags.len() == 1 {
                query.push_str(&format!(
                    " AND EXISTS (SELECT 1 FROM link_tags lt WHERE lt.link_id = id AND lt.tag_name = ?{})",
                    params.len() + 1
                ));
                params.push(tags[0].as_str().into());
            } else if !tags.is_empty() {
                let start_param = params.len() + 1;
                let placeholders: Vec<String> = (0..tags.len())
                    .map(|i| format!("?{}", start_param + i))
                    .collect();
                query.push_str(&format!(
                    " AND id IN (SELECT DISTINCT link_id FROM link_tags WHERE tag_name IN ({}))",
                    placeholders.join(", ")
                ));
                for tag in tags {
                    params.push(tag.as_str().into());
                }
            }
        }

        let stmt = db.prepare(&query);
        let result = stmt.bind(&params)?.first::<serde_json::Value>(None).await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    /// Update a link. Only provided fields are changed.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
        destination_url: Option<&str>,
        title: Option<&str>,
        status: Option<&str>,
        expires_at: Option<i64>,
        utm_params: Option<Option<&str>>,
        forward_query_params: Option<Option<bool>>,
        redirect_type: Option<&str>,
    ) -> Result<Link> {
        let now = now_timestamp();

        let mut query = String::from("UPDATE links SET updated_at = ?1");
        let mut params: Vec<JsValue> = vec![(now as f64).into()];
        let mut param_count = 2;

        if let Some(url) = destination_url {
            query.push_str(&format!(", destination_url = ?{}", param_count));
            params.push(url.into());
            param_count += 1;
        }

        if let Some(t) = title {
            query.push_str(&format!(", title = ?{}", param_count));
            params.push(t.into());
            param_count += 1;
        }

        if let Some(s) = status {
            query.push_str(&format!(", status = ?{}", param_count));
            params.push(s.into());
            param_count += 1;
        }

        if expires_at.is_some() {
            query.push_str(&format!(", expires_at = ?{}", param_count));
            params.push(expires_at.map(|t| t as f64).into());
            param_count += 1;
        }

        if let Some(utm_val) = utm_params {
            query.push_str(&format!(", utm_params = ?{}", param_count));
            params.push(utm_val.map(|s| s.into()).unwrap_or(JsValue::NULL));
            param_count += 1;
        }

        if let Some(fwd_val) = forward_query_params {
            query.push_str(&format!(", forward_query_params = ?{}", param_count));
            params.push(
                fwd_val
                    .map(|v| (if v { 1i64 } else { 0i64 } as f64).into())
                    .unwrap_or(JsValue::NULL),
            );
            param_count += 1;
        }

        if let Some(rt) = redirect_type {
            query.push_str(&format!(", redirect_type = ?{}", param_count));
            params.push(rt.into());
            param_count += 1;
        }

        query.push_str(&format!(
            " WHERE id = ?{} AND org_id = ?{}",
            param_count,
            param_count + 1
        ));
        params.push(link_id.into());
        params.push(org_id.into());

        let stmt = db.prepare(&query);
        stmt.bind(&params)?.run().await?;

        self.get_by_id(db, link_id, org_id)
            .await?
            .ok_or_else(|| worker::Error::RustError("Link not found after update".to_string()))
    }

    /// Update link status by ID (admin operations — no org scope)
    pub async fn update_status_by_id(
        &self,
        db: &D1Database,
        link_id: &str,
        status: &str,
    ) -> Result<()> {
        let now = now_timestamp();
        let stmt = db.prepare("UPDATE links SET status = ?1, updated_at = ?2 WHERE id = ?3");
        stmt.bind(&[status.into(), (now as f64).into(), link_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Hard-delete a link and all its related data
    pub async fn hard_delete(&self, db: &D1Database, link_id: &str, org_id: &str) -> Result<()> {
        // Delete analytics events (FK constraint)
        let analytics_stmt = db.prepare("DELETE FROM analytics_events WHERE link_id = ?1");
        analytics_stmt.bind(&[link_id.into()])?.run().await?;

        // Delete link reports
        let reports_stmt = db.prepare("DELETE FROM link_reports WHERE link_id = ?1");
        reports_stmt.bind(&[link_id.into()])?.run().await?;

        // Delete link tag associations
        let tags_stmt = db.prepare("DELETE FROM link_tags WHERE link_id = ?1");
        tags_stmt.bind(&[link_id.into()])?.run().await?;

        // Delete the link itself
        let stmt = db.prepare("DELETE FROM links WHERE id = ?1 AND org_id = ?2");
        stmt.bind(&[link_id.into(), org_id.into()])?.run().await?;

        Ok(())
    }

    // ─── Analytics ────────────────────────────────────────────────────────────

    /// Increment the click counter for a link
    pub async fn increment_click_count(&self, db: &D1Database, link_id: &str) -> Result<()> {
        let stmt = db.prepare("UPDATE links SET click_count = click_count + 1 WHERE id = ?1");
        stmt.bind(&[link_id.into()])?.run().await?;
        Ok(())
    }

    /// Log an analytics event
    pub async fn log_analytics_event(&self, db: &D1Database, event: &AnalyticsEvent) -> Result<()> {
        let stmt = db.prepare(
            "INSERT INTO analytics_events (link_id, org_id, timestamp, referrer, user_agent, country, city)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        );
        stmt.bind(&[
            event.link_id.clone().into(),
            event.org_id.clone().into(),
            (event.timestamp as f64).into(),
            event
                .referrer
                .clone()
                .map(|t| t.into())
                .unwrap_or(JsValue::NULL),
            event
                .user_agent
                .clone()
                .map(|t| t.into())
                .unwrap_or(JsValue::NULL),
            event
                .country
                .clone()
                .map(|t| t.into())
                .unwrap_or(JsValue::NULL),
            event
                .city
                .clone()
                .map(|t| t.into())
                .unwrap_or(JsValue::NULL),
        ])?
        .run()
        .await?;
        Ok(())
    }

    // ─── Tags ─────────────────────────────────────────────────────────────────

    /// Get all tags for a single link, sorted alphabetically
    pub async fn get_tags(&self, db: &D1Database, link_id: &str) -> Result<Vec<String>> {
        let stmt =
            db.prepare("SELECT tag_name FROM link_tags WHERE link_id = ?1 ORDER BY tag_name ASC");
        let results = stmt.bind(&[link_id.into()])?.all().await?;
        let rows = results.results::<serde_json::Value>()?;
        Ok(rows
            .iter()
            .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
            .collect())
    }

    /// Get tags for multiple links in a single query. Returns link_id → tags map.
    pub async fn get_tags_for_links(
        &self,
        db: &D1Database,
        link_ids: &[String],
    ) -> Result<std::collections::HashMap<String, Vec<String>>> {
        let mut map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        if link_ids.is_empty() {
            return Ok(map);
        }

        let placeholders: Vec<String> = (1..=link_ids.len()).map(|i| format!("?{}", i)).collect();
        let query = format!(
            "SELECT link_id, tag_name FROM link_tags WHERE link_id IN ({}) ORDER BY link_id, tag_name ASC",
            placeholders.join(", ")
        );

        let params: Vec<JsValue> = link_ids.iter().map(|id| id.as_str().into()).collect();
        let stmt = db.prepare(&query);
        let results = stmt.bind(&params)?.all().await?;
        let rows = results.results::<serde_json::Value>()?;

        for row in &rows {
            if let (Some(link_id), Some(tag_name)) =
                (row["link_id"].as_str(), row["tag_name"].as_str())
            {
                map.entry(link_id.to_string())
                    .or_default()
                    .push(tag_name.to_string());
            }
        }
        Ok(map)
    }

    /// Replace all tags for a link atomically (delete existing, insert new)
    pub async fn set_tags(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
        tags: &[String],
    ) -> Result<()> {
        let delete_stmt = db.prepare("DELETE FROM link_tags WHERE link_id = ?1");
        delete_stmt.bind(&[link_id.into()])?.run().await?;

        for tag in tags {
            let insert_stmt =
                db.prepare("INSERT INTO link_tags (link_id, tag_name, org_id) VALUES (?1, ?2, ?3)");
            insert_stmt
                .bind(&[link_id.into(), tag.as_str().into(), org_id.into()])?
                .run()
                .await?;
        }
        Ok(())
    }

    /// Delete all tags for a link (called on link deletion)
    pub async fn delete_tags(&self, db: &D1Database, link_id: &str) -> Result<()> {
        let stmt = db.prepare("DELETE FROM link_tags WHERE link_id = ?1");
        stmt.bind(&[link_id.into()])?.run().await?;
        Ok(())
    }

    // ─── Dashboard ────────────────────────────────────────────────────────────

    /// Get dashboard statistics for an organization
    pub async fn get_dashboard_stats(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<DashboardStats> {
        let stmt = db.prepare(
            "SELECT
                COUNT(*) as total_links,
                SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) as active_links,
                SUM(click_count) as total_clicks
             FROM links
             WHERE org_id = ?1
             AND status IN ('active', 'disabled')",
        );

        let result = stmt
            .bind(&[org_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        match result {
            Some(val) => Ok(DashboardStats {
                total_links: val["total_links"].as_f64().unwrap_or(0.0) as i64,
                active_links: val["active_links"].as_f64().unwrap_or(0.0) as i64,
                total_clicks: val["total_clicks"].as_f64().unwrap_or(0.0) as i64,
            }),
            None => Ok(DashboardStats {
                total_links: 0,
                active_links: 0,
                total_clicks: 0,
            }),
        }
    }

    // ─── Export ───────────────────────────────────────────────────────────────

    /// Get all active/disabled links for an org (for CSV/JSON export)
    pub async fn get_all_for_export(&self, db: &D1Database, org_id: &str) -> Result<Vec<Link>> {
        let stmt = db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params
             FROM links
             WHERE org_id = ?1
             AND status IN ('active', 'disabled')
             ORDER BY created_at DESC",
        );
        let results = stmt.bind(&[org_id.into()])?.all().await?;
        results.results::<Link>()
    }

    // ─── Admin ────────────────────────────────────────────────────────────────

    /// Get paginated admin link listing (base data, no KV status)
    pub async fn list_admin_base(
        &self,
        db: &D1Database,
        limit: i64,
        offset: i64,
        org_filter: Option<&str>,
        email_filter: Option<&str>,
        domain_filter: Option<&str>,
    ) -> Result<Vec<AdminLinkBase>> {
        let mut query = String::from(
            "SELECT l.id, l.org_id, l.short_code, l.destination_url, l.title, l.created_by, l.created_at, l.updated_at, l.expires_at, l.status, l.click_count, l.utm_params, l.forward_query_params, l.redirect_type, u.email as creator_email, o.name as org_name
             FROM links l
             JOIN users u ON l.created_by = u.id
             JOIN organizations o ON l.org_id = o.id
             WHERE l.status IN ('active', 'disabled', 'blocked')"
        );

        let mut params: Vec<JsValue> = vec![];
        let mut param_count = 1;

        if let Some(org_id) = org_filter {
            query.push_str(&format!(" AND l.org_id = ?{}", param_count));
            params.push(org_id.into());
            param_count += 1;
        }

        if let Some(email) = email_filter {
            query.push_str(&format!(" AND u.email LIKE ?{}", param_count));
            params.push(format!("%{}%", email).into());
            param_count += 1;
        }

        if let Some(domain) = domain_filter {
            query.push_str(&format!(" AND l.destination_url LIKE ?{}", param_count));
            params.push(format!("%{}%", domain).into());
            param_count += 1;
        }

        query.push_str(&format!(
            " ORDER BY l.created_at DESC LIMIT ?{} OFFSET ?{}",
            param_count,
            param_count + 1
        ));
        params.push((limit as f64).into());
        params.push((offset as f64).into());

        let stmt = db.prepare(&query);
        let results = stmt.bind(&params)?.all().await?;
        results.results::<AdminLinkBase>()
    }

    /// Get paginated admin link listing with KV sync status
    #[allow(clippy::too_many_arguments)]
    pub async fn list_admin(
        &self,
        db: &D1Database,
        kv: &worker::kv::KvStore,
        limit: i64,
        offset: i64,
        org_filter: Option<&str>,
        email_filter: Option<&str>,
        domain_filter: Option<&str>,
    ) -> Result<Vec<AdminLink>> {
        let base_links = self
            .list_admin_base(db, limit, offset, org_filter, email_filter, domain_filter)
            .await?;

        let mut links_with_kv = Vec::new();
        for base_link in base_links {
            let (kv_sync_status, kv_exists) = self.check_kv_sync(kv, &base_link).await?;
            links_with_kv.push(AdminLink {
                base: base_link,
                kv_sync_status,
                kv_exists,
            });
        }
        Ok(links_with_kv)
    }

    /// Count admin links with filters
    pub async fn count_admin(
        &self,
        db: &D1Database,
        org_filter: Option<&str>,
        email_filter: Option<&str>,
        domain_filter: Option<&str>,
    ) -> Result<i64> {
        let mut query = String::from(
            "SELECT COUNT(*) as count FROM links l
             JOIN users u ON l.created_by = u.id
             WHERE l.status IN ('active', 'disabled', 'blocked')",
        );

        let mut params: Vec<JsValue> = vec![];
        let mut param_count = 1;

        if let Some(org_id) = org_filter {
            query.push_str(&format!(" AND l.org_id = ?{}", param_count));
            params.push(org_id.into());
            param_count += 1;
        }

        if let Some(email) = email_filter {
            query.push_str(&format!(" AND u.email LIKE ?{}", param_count));
            params.push(format!("%{}%", email).into());
            param_count += 1;
        }

        if let Some(domain) = domain_filter {
            query.push_str(&format!(" AND l.destination_url LIKE ?{}", param_count));
            params.push(format!("%{}%", domain).into());
        }

        let stmt = db.prepare(&query);
        let result = stmt.bind(&params)?.first::<serde_json::Value>(None).await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    /// Check KV sync status for a link
    pub async fn check_kv_sync(
        &self,
        kv: &worker::kv::KvStore,
        link: &AdminLinkBase,
    ) -> Result<(String, bool)> {
        use crate::kv;

        let kv_mapping = kv::get_link_mapping(kv, &link.short_code).await?;

        match kv_mapping {
            Some(mapping) => {
                let should_exist = link.status == "active";
                let is_active = mapping.status == LinkStatus::Active;

                if should_exist && is_active || !should_exist && !is_active {
                    Ok(("synced".to_string(), true))
                } else {
                    Ok(("mismatched".to_string(), true))
                }
            }
            None => {
                let should_exist = link.status == "active";
                if should_exist {
                    Ok(("missing".to_string(), false))
                } else {
                    Ok(("synced".to_string(), false))
                }
            }
        }
    }

    /// Resolve all pending reports for a specific link
    pub async fn resolve_reports_for_link(
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

    // ─── KV sync helpers ──────────────────────────────────────────────────────

    /// Resolve whether forward_query_params should be applied for a link,
    /// falling back to the org-level setting if not set on the link itself.
    pub async fn resolved_forward_for_link(&self, db: &D1Database, link: &Link) -> bool {
        if let Some(value) = link.forward_query_params {
            value
        } else {
            db::get_org_forward_query_params(db, &link.org_id)
                .await
                .unwrap_or(false)
        }
    }

    /// Sync a link's KV mapping from its D1 state.
    /// Used after status changes (suspend, block) to keep KV consistent.
    pub async fn sync_kv_from_link(
        &self,
        db: &D1Database,
        kv_store: &worker::kv::KvStore,
        link: &Link,
    ) -> Result<()> {
        use crate::kv;
        match link.status {
            LinkStatus::Blocked => {
                kv::delete_link_mapping(kv_store, &link.org_id, &link.short_code).await
            }
            LinkStatus::Active | LinkStatus::Disabled => {
                let resolved_forward = self.resolved_forward_for_link(db, link).await;
                let mapping = link.to_mapping(resolved_forward);
                kv::update_link_mapping(kv_store, &link.short_code, &mapping).await
            }
        }
    }
}

impl Default for LinkRepository {
    fn default() -> Self {
        Self::new()
    }
}
