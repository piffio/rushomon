/// Tag repository - Data access for tags
///
/// Handles all database operations related to tags.
/// Schema: tags table holds metadata, link_tags is the join table to links.
use worker::d1::D1Database;
use worker::*;

/// Tag with full statistics for organization display
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OrgTag {
    pub name: String,
    pub count: i64,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub color_index: Option<i32>,
}

/// Similar tag group for analytics
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SimilarTagGroup {
    pub tags: Vec<String>,
    pub suggestion: String,
}

/// Tag usage over time for analytics
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct TagUsage {
    pub date: String,
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

    /// Get all tags for an organization with full statistics,
    /// sorted by count desc then name asc.
    pub async fn get_org_tags(&self, db: &D1Database, org_id: &str) -> Result<Vec<OrgTag>> {
        let stmt = db.prepare(
            "SELECT
                t.tag_name as name,
                COUNT(lt.link_id) as count,
                t.created_at,
                MAX(lt.created_at) as last_used_at,
                t.color_index
             FROM tags t
             LEFT JOIN link_tags lt ON t.org_id = lt.org_id AND t.tag_name = lt.tag_name
             WHERE t.org_id = ?1
             GROUP BY t.tag_name, t.created_at, t.color_index
             ORDER BY count DESC, t.tag_name ASC",
        );
        let results = stmt.bind(&[org_id.into()])?.all().await?;
        let rows = results.results::<serde_json::Value>()?;
        let tags = rows
            .iter()
            .filter_map(|row| {
                let name = row["name"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                let created_at = row["created_at"].as_f64()? as i64;
                let last_used_at = row["last_used_at"].as_f64().map(|f| f as i64);
                let color_index = row["color_index"].as_f64().map(|f| f as i32);
                Some(OrgTag {
                    name,
                    count,
                    created_at,
                    last_used_at,
                    color_index,
                })
            })
            .collect();
        Ok(tags)
    }

    /// Create a new tag in the organization.
    /// Returns true if created, false if tag already exists.
    pub async fn create_tag(
        &self,
        db: &D1Database,
        org_id: &str,
        tag_name: &str,
        color_index: Option<i32>,
    ) -> Result<bool> {
        // Check if tag already exists
        let check_stmt =
            db.prepare("SELECT 1 FROM tags WHERE org_id = ?1 AND tag_name = ?2 LIMIT 1");
        let exists = check_stmt
            .bind(&[org_id.into(), tag_name.into()])?
            .first::<serde_json::Value>(None)
            .await?
            .is_some();

        if exists {
            return Ok(false);
        }

        // Insert the new tag
        let insert_stmt = db.prepare(
            "INSERT INTO tags (org_id, tag_name, created_at, color_index)
             VALUES (?1, ?2, strftime('%s', 'now'), ?3)",
        );

        let color_js_value: wasm_bindgen::JsValue = match color_index {
            Some(c) => (c as f64).into(),
            None => wasm_bindgen::JsValue::NULL,
        };

        insert_stmt
            .bind(&[org_id.into(), tag_name.into(), color_js_value])?
            .run()
            .await?;

        Ok(true)
    }

    /// Delete a tag from an organization.
    /// This removes the tag from all associated links via link_tags,
    /// then deletes the tag from the tags table.
    /// Returns true if the tag was deleted, false if tag doesn't exist.
    pub async fn delete_tag_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
        tag_name: &str,
    ) -> Result<bool> {
        // First check if the tag exists
        let check_stmt = db.prepare("SELECT 1 FROM tags WHERE org_id = ?1 AND tag_name = ?2");

        let check_result = check_stmt
            .bind(&[org_id.into(), tag_name.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        // If no result, tag doesn't exist
        if check_result.is_none() {
            return Ok(false);
        }

        // Delete associations from link_tags first
        let unlink_stmt = db.prepare(
            "DELETE FROM link_tags
             WHERE org_id = ?1 AND tag_name = ?2",
        );

        unlink_stmt
            .bind(&[org_id.into(), tag_name.into()])?
            .run()
            .await?;

        // Delete the tag from tags table
        let delete_stmt = db.prepare(
            "DELETE FROM tags
             WHERE org_id = ?1 AND tag_name = ?2",
        );

        let result = delete_stmt
            .bind(&[org_id.into(), tag_name.into()])?
            .run()
            .await?;

        let affected_rows = result.meta()?.and_then(|m| m.changes).unwrap_or(0);

        Ok(affected_rows > 0)
    }

    /// Rename a tag within an organization.
    /// If the new name already exists, this merges the tags.
    pub async fn rename_tag_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<()> {
        // Check if destination tag already exists
        let check_dest =
            db.prepare("SELECT 1 FROM tags WHERE org_id = ?1 AND tag_name = ?2 LIMIT 1");
        let dest_exists = check_dest
            .bind(&[org_id.into(), new_name.into()])?
            .first::<serde_json::Value>(None)
            .await?
            .is_some();

        if dest_exists {
            // Merge case: destination exists, move all links from old to new
            // First, update link_tags to point to new name
            // Use INSERT OR REPLACE to handle duplicates (same link having both tags)
            let update_stmt = db.prepare(
                "UPDATE OR REPLACE link_tags
                 SET tag_name = ?1
                 WHERE org_id = ?2 AND tag_name = ?3",
            );
            update_stmt
                .bind(&[new_name.into(), org_id.into(), old_name.into()])?
                .run()
                .await?;

            // Delete the old tag from tags table
            let delete_stmt = db.prepare("DELETE FROM tags WHERE org_id = ?1 AND tag_name = ?2");
            delete_stmt
                .bind(&[org_id.into(), old_name.into()])?
                .run()
                .await?;
        } else {
            // Simple rename case: destination doesn't exist
            // Update link_tags first
            let update_links = db.prepare(
                "UPDATE link_tags
                 SET tag_name = ?1
                 WHERE org_id = ?2 AND tag_name = ?3",
            );
            update_links
                .bind(&[new_name.into(), org_id.into(), old_name.into()])?
                .run()
                .await?;

            // Update tags table
            let update_tags = db.prepare(
                "UPDATE tags
                 SET tag_name = ?1
                 WHERE org_id = ?2 AND tag_name = ?3",
            );
            update_tags
                .bind(&[new_name.into(), org_id.into(), old_name.into()])?
                .run()
                .await?;
        }

        Ok(())
    }

    /// Merge multiple source tags into a destination tag.
    /// Creates the destination tag if it doesn't exist.
    pub async fn merge_tags_for_org(
        &self,
        db: &D1Database,
        org_id: &str,
        source_tags: &[String],
        dest_tag: &str,
    ) -> Result<i64> {
        // First ensure destination tag exists
        let ensure_dest = db.prepare(
            "INSERT OR IGNORE INTO tags (org_id, tag_name, created_at)
             VALUES (?1, ?2, strftime('%s', 'now'))",
        );
        ensure_dest
            .bind(&[org_id.into(), dest_tag.into()])?
            .run()
            .await?;

        // Update all link_tags from source tags to destination
        // Use a single UPDATE with IN clause for efficiency
        let placeholders: Vec<String> = (0..source_tags.len())
            .map(|i| format!("?{}", i + 3))
            .collect();
        let query = format!(
            "UPDATE link_tags
             SET tag_name = ?1
             WHERE org_id = ?2 AND tag_name IN ({})",
            placeholders.join(", ")
        );

        let mut params: Vec<wasm_bindgen::JsValue> = vec![dest_tag.into(), org_id.into()];
        params.extend(source_tags.iter().map(|t| t.clone().into()));

        let update_stmt = db.prepare(&query);
        let affected_rows = update_stmt
            .bind(&params)?
            .run()
            .await?
            .meta()?
            .and_then(|m| m.changes)
            .unwrap_or(0);

        // Delete source tags from tags table (they're now merged)
        // This cascades and cleans up any orphaned link_tags entries
        let delete_placeholders: Vec<String> = (0..source_tags.len())
            .map(|i| format!("?{}", i + 2))
            .collect();
        let delete_query = format!(
            "DELETE FROM tags
             WHERE org_id = ?1 AND tag_name IN ({})",
            delete_placeholders.join(", ")
        );

        let mut delete_params: Vec<wasm_bindgen::JsValue> = vec![org_id.into()];
        delete_params.extend(source_tags.iter().map(|t| t.clone().into()));

        let delete_stmt = db.prepare(&delete_query);
        delete_stmt.bind(&delete_params)?.run().await?;

        Ok(affected_rows as i64)
    }

    /// Get tags that are similar to each other (potential duplicates).
    /// Uses simple string similarity - tags that are substrings of each other.
    pub async fn find_similar_tags(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<Vec<SimilarTagGroup>> {
        // Fetch all tags for this org and do similarity matching in Rust
        // This avoids complex self-joins with LIKE patterns that can cause issues
        let stmt = db.prepare("SELECT tag_name FROM tags WHERE org_id = ?1 ORDER BY tag_name");

        let results = stmt.bind(&[org_id.into()])?.all().await?;
        let rows = results.results::<serde_json::Value>()?;

        let tags: Vec<String> = rows
            .iter()
            .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
            .collect();

        // Find similar tags in Rust
        let mut groups: Vec<SimilarTagGroup> = Vec::new();
        for i in 0..tags.len() {
            for j in (i + 1)..tags.len() {
                let tag1 = &tags[i];
                let tag2 = &tags[j];

                // Check if tags are similar (case-insensitive match or one is prefix of other)
                let tag1_lower = tag1.to_lowercase();
                let tag2_lower = tag2.to_lowercase();

                let similar = tag1_lower == tag2_lower
                    || tag1_lower.starts_with(&tag2_lower)
                    || tag2_lower.starts_with(&tag1_lower);

                if similar {
                    // Find or create a group
                    if let Some(group) = groups
                        .iter_mut()
                        .find(|g| g.tags.contains(tag1) || g.tags.contains(tag2))
                    {
                        if !group.tags.contains(tag1) {
                            group.tags.push(tag1.clone());
                        }
                        if !group.tags.contains(tag2) {
                            group.tags.push(tag2.clone());
                        }
                    } else {
                        groups.push(SimilarTagGroup {
                            tags: vec![tag1.clone(), tag2.clone()],
                            suggestion: if tag1.len() < tag2.len() {
                                tag1.clone()
                            } else {
                                tag2.clone()
                            },
                        });
                    }
                }
            }
        }

        // Sort tags within each group and set suggestion to the shortest one
        for group in &mut groups {
            group.tags.sort();
            // Use the shortest tag as suggestion (likely the root word)
            if let Some(shortest) = group.tags.iter().min_by_key(|t| t.len()) {
                group.suggestion = shortest.clone();
            }
        }

        Ok(groups)
    }

    /// Get unused tags (tags with 0 links) for cleanup suggestions.
    pub async fn get_unused_tags(&self, db: &D1Database, org_id: &str) -> Result<Vec<String>> {
        let stmt = db.prepare(
            "SELECT t.tag_name
             FROM tags t
             LEFT JOIN link_tags lt ON t.org_id = lt.org_id AND t.tag_name = lt.tag_name
             WHERE t.org_id = ?1
             GROUP BY t.tag_name
             HAVING COUNT(lt.link_id) = 0
             ORDER BY t.tag_name",
        );

        let results = stmt.bind(&[org_id.into()])?.all().await?;
        let rows = results.results::<serde_json::Value>()?;

        let tags = rows
            .iter()
            .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(tags)
    }

    /// Get most used tags (top N) for analytics.
    pub async fn get_top_tags(
        &self,
        db: &D1Database,
        org_id: &str,
        limit: i64,
    ) -> Result<Vec<OrgTag>> {
        let stmt = db.prepare(
            "SELECT
                t.tag_name as name,
                COUNT(lt.link_id) as count,
                t.created_at,
                MAX(lt.created_at) as last_used_at,
                t.color_index
             FROM tags t
             LEFT JOIN link_tags lt ON t.org_id = lt.org_id AND t.tag_name = lt.tag_name
             WHERE t.org_id = ?1
             GROUP BY t.tag_name, t.created_at, t.color_index
             ORDER BY count DESC, t.tag_name ASC
             LIMIT ?2",
        );

        // Convert limit to f64 to avoid bigint issue with D1
        let results = stmt
            .bind(&[org_id.into(), (limit as f64).into()])?
            .all()
            .await?;
        let rows = results.results::<serde_json::Value>()?;

        let tags = rows
            .iter()
            .filter_map(|row| {
                let name = row["name"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                let created_at = row["created_at"].as_f64()? as i64;
                let last_used_at = row["last_used_at"].as_f64().map(|f| f as i64);
                let color_index = row["color_index"].as_f64().map(|f| f as i32);
                Some(OrgTag {
                    name,
                    count,
                    created_at,
                    last_used_at,
                    color_index,
                })
            })
            .collect();

        Ok(tags)
    }

    /// Count distinct tag names across all organizations in a billing account.
    /// Used to enforce BA-level tag limits.
    pub async fn count_distinct_tags_for_billing_account(
        &self,
        db: &D1Database,
        billing_account_id: &str,
    ) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT COUNT(DISTINCT t.tag_name) as count
             FROM tags t
             JOIN organizations o ON t.org_id = o.id
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

    /// Update tag metadata (currently only color_index).
    /// Returns true if the tag was updated, false if tag doesn't exist.
    pub async fn update_tag_metadata(
        &self,
        db: &D1Database,
        org_id: &str,
        tag_name: &str,
        color_index: Option<i32>,
    ) -> Result<bool> {
        let color_js_value: wasm_bindgen::JsValue = match color_index {
            Some(c) => (c as f64).into(),
            None => wasm_bindgen::JsValue::NULL,
        };

        let stmt = db.prepare(
            "UPDATE tags
             SET color_index = ?1
             WHERE org_id = ?2 AND tag_name = ?3",
        );

        let result = stmt
            .bind(&[color_js_value, org_id.into(), tag_name.into()])?
            .run()
            .await?;

        let affected_rows = result.meta()?.and_then(|m| m.changes).unwrap_or(0);

        Ok(affected_rows > 0)
    }
}
