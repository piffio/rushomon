/// Tag repository - Data access for tags
///
/// Handles all database operations related to tags.
use worker::d1::D1Database;
use worker::*;

/// Tag with usage count for organization display
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OrgTag {
    pub name: String,
    pub count: i64,
}

/// Repository for tag operations
#[derive(Default)]
pub struct TagRepository;

impl TagRepository {
    /// Create a new tag repository instance
    pub fn new() -> Self {
        Self
    }

    /// Get all tags for an organization with usage counts,
    /// sorted by count desc then name asc.
    pub async fn get_org_tags(&self, db: &D1Database, org_id: &str) -> Result<Vec<OrgTag>> {
        let stmt = db.prepare(
            "SELECT tag_name, COUNT(*) as count
             FROM link_tags
             WHERE org_id = ?1
             GROUP BY tag_name
             ORDER BY count DESC, tag_name ASC",
        );
        let results = stmt.bind(&[org_id.into()])?.all().await?;
        let rows = results.results::<serde_json::Value>()?;
        let tags = rows
            .iter()
            .filter_map(|row| {
                let name = row["tag_name"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(OrgTag { name, count })
            })
            .collect();
        Ok(tags)
    }

    /// Delete a tag from an organization. Returns true if rows were deleted, false if tag doesn't exist.
    pub async fn delete_tag_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
        tag_name: &str,
    ) -> Result<bool> {
        // First check if the tag exists in this org
        let check_stmt = db.prepare(
            "SELECT 1 FROM link_tags
             WHERE org_id = ?1 AND tag_name = ?2
             LIMIT 1",
        );

        let exists = check_stmt
            .bind(&[org_id.into(), tag_name.into()])?
            .first::<serde_json::Value>(None)
            .await?
            .is_some();

        if !exists {
            return Ok(false);
        }

        // Delete the tag
        let delete_stmt = db.prepare(
            "DELETE FROM link_tags
             WHERE org_id = ?1 AND tag_name = ?2",
        );

        delete_stmt
            .bind(&[org_id.into(), tag_name.into()])?
            .run()
            .await?;

        Ok(true)
    }

    /// Rename a tag within an organization. Updates all links that use the tag.
    /// If the new name already exists in the org, this will effectively merge the tags.
    pub async fn rename_tag_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<()> {
        // Use INSERT OR REPLACE to handle the case where new_name already exists
        // This effectively merges the old tag into the existing one
        let stmt = db.prepare(
            "UPDATE link_tags
             SET tag_name = ?1
             WHERE org_id = ?2 AND tag_name = ?3",
        );

        stmt.bind(&[new_name.into(), org_id.into(), old_name.into()])?
            .run()
            .await?;

        Ok(())
    }

    /// Count distinct tag names across all organizations in a billing account.
    /// Used to enforce BA-level tag limits.
    pub async fn count_distinct_tags_for_billing_account(
        &self,
        db: &D1Database,
        billing_account_id: &str,
    ) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT COUNT(DISTINCT lt.tag_name) as count
             FROM link_tags lt
             JOIN organizations o ON lt.org_id = o.id
             WHERE o.billing_account_id = ?1",
        );

        let result = stmt
            .bind(&[billing_account_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        if let Some(result) = result {
            Ok(result["count"].as_f64().unwrap_or(0.0) as i64)
        } else {
            Ok(0)
        }
    }
}
