/// Blacklist Repository
///
/// Admin-only data access for the `destination_blacklist` table.
/// `is_destination_blacklisted` remains in `db/queries.rs` — it is still called
/// from link create/update/import handlers not yet extracted from `router.rs`.
use crate::db::queries::BlacklistEntry;
use crate::utils::now_timestamp;
use worker::Result;
use worker::d1::D1Database;

pub struct BlacklistRepository;

impl BlacklistRepository {
    pub fn new() -> Self {
        Self
    }

    /// Check whether an entry with the same destination + match_type already exists.
    pub async fn is_duplicate(
        &self,
        db: &D1Database,
        destination: &str,
        match_type: &str,
    ) -> Result<bool> {
        let stmt = db.prepare(
            "SELECT 1 FROM destination_blacklist
             WHERE destination = ?1 AND match_type = ?2
             LIMIT 1",
        );
        if let Ok(Some(_)) = stmt
            .bind(&[destination.into(), match_type.into()])?
            .first::<serde_json::Value>(None)
            .await
        {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Insert a new blacklist entry and return its generated ID.
    pub async fn add(
        &self,
        db: &D1Database,
        destination: &str,
        match_type: &str,
        reason: &str,
        created_by: &str,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_timestamp();
        db.prepare(
            "INSERT INTO destination_blacklist (id, destination, match_type, reason, created_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&[
            id.into(),
            destination.into(),
            match_type.into(),
            reason.into(),
            created_by.into(),
            (now as f64).into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Hard-delete a blacklist entry by ID.
    pub async fn remove(&self, db: &D1Database, id: &str) -> Result<()> {
        db.prepare("DELETE FROM destination_blacklist WHERE id = ?1")
            .bind(&[id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Return all blacklist entries ordered by creation date descending.
    pub async fn list_all(&self, db: &D1Database) -> Result<Vec<BlacklistEntry>> {
        let results = db
            .prepare(
                "SELECT id, destination, match_type, reason, created_by, created_at
                 FROM destination_blacklist
                 ORDER BY created_at DESC",
            )
            .all()
            .await?;
        results.results::<BlacklistEntry>()
    }

    /// Return all active/disabled links — used to scan for newly-blacklisted destinations.
    pub async fn get_candidate_links(&self, db: &D1Database) -> Result<Vec<crate::models::Link>> {
        let results = db
            .prepare(
                "SELECT id, org_id, short_code, destination_url, title, created_by,
                        created_at, updated_at, expires_at, status, click_count,
                        utm_params, forward_query_params, redirect_type
                 FROM links
                 WHERE status IN ('active', 'disabled')",
            )
            .all()
            .await?;
        results.results::<crate::models::Link>()
    }
}

impl Default for BlacklistRepository {
    fn default() -> Self {
        Self::new()
    }
}
