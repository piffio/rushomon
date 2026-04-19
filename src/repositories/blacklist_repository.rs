/// Blacklist Repository
///
/// Admin-only data access for the `destination_blacklist` table.
use crate::utils::normalize_url_for_blacklist;
use crate::utils::now_timestamp;
use worker::Result;
use worker::d1::D1Database;

/// A single blacklist entry.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BlacklistEntry {
    pub id: String,
    pub destination: String,
    pub match_type: String,
    pub reason: String,
    pub created_by: String,
    pub created_at: i64,
}

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

    /// Check if a destination is blacklisted (exact or domain match).
    pub async fn is_blacklisted(&self, db: &D1Database, destination: &str) -> Result<bool> {
        // Normalize the destination URL for comparison
        let normalized_destination = match normalize_url_for_blacklist(destination) {
            Ok(url) => url,
            Err(_) => {
                // If URL parsing fails, fall back to exact string comparison
                destination.to_string()
            }
        };

        // First check exact match against normalized blacklist entries
        let exact_stmt = db.prepare(
            "SELECT 1 FROM destination_blacklist
             WHERE destination = ?1 AND match_type = 'exact'
             LIMIT 1",
        );
        if let Ok(Some(_)) = exact_stmt
            .bind(&[normalized_destination.clone().into()])?
            .first::<serde_json::Value>(None)
            .await
        {
            return Ok(true);
        }

        // Then check domain match (still uses original domain extraction logic)
        let url = match url::Url::parse(destination) {
            Ok(u) => u,
            Err(_) => return Ok(false),
        };

        let domain = url.host_str().unwrap_or("");
        let domain_stmt = db.prepare(
            "SELECT 1 FROM destination_blacklist
             WHERE ?1 LIKE '%' || destination || '%' AND match_type = 'domain'
             LIMIT 1",
        );
        if let Ok(Some(_)) = domain_stmt
            .bind(&[domain.into()])?
            .first::<serde_json::Value>(None)
            .await
        {
            return Ok(true);
        }

        // Finally, check if any normalized blacklist entries match our normalized destination
        // This handles cases where blocked URLs have different forms but same normalized form
        let all_exact_entries = db
            .prepare("SELECT destination FROM destination_blacklist WHERE match_type = 'exact'")
            .all()
            .await?
            .results::<serde_json::Value>()?;
        for entry in all_exact_entries {
            if let Some(dest) = entry.get("destination").and_then(|d| d.as_str())
                && let Ok(normalized_entry) = normalize_url_for_blacklist(dest)
                && normalized_entry == normalized_destination
            {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl Default for BlacklistRepository {
    fn default() -> Self {
        Self::new()
    }
}
