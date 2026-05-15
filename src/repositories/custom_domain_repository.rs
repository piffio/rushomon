use crate::models::CustomDomain;
use crate::models::custom_domain::{STATUS_ACTIVE, STATUS_PENDING};
use crate::utils::now_timestamp;
use wasm_bindgen::JsValue;
use worker::Result;
use worker::d1::D1Database;

pub struct CustomDomainRepository;

impl CustomDomainRepository {
    pub fn new() -> Self {
        Self
    }

    /// Insert a new custom domain record (status = pending)
    pub async fn create(&self, db: &D1Database, domain: &CustomDomain) -> Result<()> {
        db.prepare(
            "INSERT INTO custom_domains (id, org_id, hostname, status, cf_hostname_id, ssl_status, created_at, verified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&[
            domain.id.as_str().into(),
            domain.org_id.as_str().into(),
            domain.hostname.as_str().into(),
            domain.status.as_str().into(),
            domain
                .cf_hostname_id
                .as_deref()
                .map(|s| s.into())
                .unwrap_or(JsValue::NULL),
            domain.ssl_status.as_str().into(),
            (domain.created_at as f64).into(),
            domain
                .verified_at
                .map(|v| (v as f64).into())
                .unwrap_or(JsValue::NULL),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Get all custom domains for an organization
    pub async fn get_by_org(&self, db: &D1Database, org_id: &str) -> Result<Vec<CustomDomain>> {
        let results = db
            .prepare(
                "SELECT id, org_id, hostname, status, cf_hostname_id, ssl_status, created_at, verified_at
                 FROM custom_domains
                 WHERE org_id = ?1
                 ORDER BY created_at ASC",
            )
            .bind(&[org_id.into()])?
            .all()
            .await?;

        results.results::<CustomDomain>()
    }

    /// Get a custom domain by hostname (used during redirect to resolve org)
    pub async fn get_by_hostname(
        &self,
        db: &D1Database,
        hostname: &str,
    ) -> Result<Option<CustomDomain>> {
        db.prepare(
            "SELECT id, org_id, hostname, status, cf_hostname_id, ssl_status, created_at, verified_at
             FROM custom_domains
             WHERE hostname = ?1",
        )
        .bind(&[hostname.into()])?
        .first::<CustomDomain>(None)
        .await
    }

    /// Get a custom domain by ID (must belong to org for authorization)
    #[allow(dead_code)]
    pub async fn get_by_id_and_org(
        &self,
        db: &D1Database,
        id: &str,
        org_id: &str,
    ) -> Result<Option<CustomDomain>> {
        db.prepare(
            "SELECT id, org_id, hostname, status, cf_hostname_id, ssl_status, created_at, verified_at
             FROM custom_domains
             WHERE id = ?1 AND org_id = ?2",
        )
        .bind(&[id.into(), org_id.into()])?
        .first::<CustomDomain>(None)
        .await
    }

    /// Get a custom domain by hostname that belongs to a specific org
    pub async fn get_by_hostname_and_org(
        &self,
        db: &D1Database,
        hostname: &str,
        org_id: &str,
    ) -> Result<Option<CustomDomain>> {
        db.prepare(
            "SELECT id, org_id, hostname, status, cf_hostname_id, ssl_status, created_at, verified_at
             FROM custom_domains
             WHERE hostname = ?1 AND org_id = ?2",
        )
        .bind(&[hostname.into(), org_id.into()])?
        .first::<CustomDomain>(None)
        .await
    }

    /// Count custom domains for an org (all statuses, excluding failed)
    pub async fn count_non_failed_for_org(&self, db: &D1Database, org_id: &str) -> Result<u32> {
        let result = db
            .prepare(
                "SELECT COUNT(*) as count FROM custom_domains
                 WHERE org_id = ?1 AND status != 'failed'",
            )
            .bind(&[org_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as u32)
    }

    /// Get all active custom domains for an org (used for KV dual-write)
    pub async fn get_active_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<Vec<CustomDomain>> {
        let results = db
            .prepare(
                "SELECT id, org_id, hostname, status, cf_hostname_id, ssl_status, created_at, verified_at
                 FROM custom_domains
                 WHERE org_id = ?1 AND status = 'active'",
            )
            .bind(&[org_id.into()])?
            .all()
            .await?;

        results.results::<CustomDomain>()
    }

    /// Update domain status and optionally set cf_hostname_id and verified_at
    pub async fn update_status(
        &self,
        db: &D1Database,
        id: &str,
        status: &str,
        cf_hostname_id: Option<&str>,
        verified_at: Option<i64>,
    ) -> Result<()> {
        db.prepare(
            "UPDATE custom_domains
             SET status = ?1, cf_hostname_id = COALESCE(?2, cf_hostname_id), verified_at = ?3
             WHERE id = ?4",
        )
        .bind(&[
            status.into(),
            cf_hostname_id.map(|s| s.into()).unwrap_or(JsValue::NULL),
            verified_at
                .map(|v| (v as f64).into())
                .unwrap_or(JsValue::NULL),
            id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Update SSL certificate status (separate from hostname status)
    pub async fn update_ssl_status(
        &self,
        db: &D1Database,
        id: &str,
        ssl_status: &str,
    ) -> Result<()> {
        db.prepare("UPDATE custom_domains SET ssl_status = ?1 WHERE id = ?2")
            .bind(&[ssl_status.into(), id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Delete a custom domain record
    pub async fn delete(&self, db: &D1Database, id: &str, org_id: &str) -> Result<()> {
        db.prepare("DELETE FROM custom_domains WHERE id = ?1 AND org_id = ?2")
            .bind(&[id.into(), org_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Get all custom domains across all orgs (admin only)
    pub async fn get_all(&self, db: &D1Database) -> Result<Vec<CustomDomain>> {
        let results = db
            .prepare(
                "SELECT id, org_id, hostname, status, cf_hostname_id, ssl_status, created_at, verified_at
                 FROM custom_domains
                 ORDER BY created_at ASC",
            )
            .bind(&[])?
            .all()
            .await?;

        results.results::<CustomDomain>()
    }

    /// Get all pending custom domains across all orgs (used by scheduled status poller)
    pub async fn get_all_pending(&self, db: &D1Database) -> Result<Vec<CustomDomain>> {
        let results = db
            .prepare(
                "SELECT id, org_id, hostname, status, cf_hostname_id, ssl_status, created_at, verified_at
                 FROM custom_domains
                 WHERE status = 'pending'
                 ORDER BY created_at ASC",
            )
            .bind(&[])?
            .all()
            .await?;

        results.results::<CustomDomain>()
    }

    /// Activate a domain: set status=active, verified_at=now, and update cf_hostname_id
    #[allow(dead_code)]
    pub async fn activate(&self, db: &D1Database, id: &str, cf_hostname_id: &str) -> Result<()> {
        let now = now_timestamp();
        self.update_status(db, id, STATUS_ACTIVE, Some(cf_hostname_id), Some(now))
            .await
    }

    /// Mark a domain as pending (reset after failed)
    #[allow(dead_code)]
    pub async fn set_pending(&self, db: &D1Database, id: &str) -> Result<()> {
        self.update_status(db, id, STATUS_PENDING, None, None).await
    }
}
