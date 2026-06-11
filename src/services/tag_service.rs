/// Tag service - Business logic for tag operations
///
/// Handles tag validation, business rules, and orchestrates the tag repository.
use crate::repositories::TagRepository;
use crate::repositories::tag_repository::{OrgTag, SimilarTagGroup};
use crate::utils::normalize_tag;
use worker::d1::D1Database;
use worker::*;

/// Analytics data for tags
#[derive(Debug, serde::Serialize)]
pub struct TagAnalytics {
    pub total_tags: i64,
    pub used_tags: i64,
    pub unused_tags: i64,
    pub top_tags: Vec<OrgTag>,
    pub unused_tag_names: Vec<String>,
    pub similar_tag_groups: Vec<SimilarTagGroup>,
}

/// Request to merge multiple tags
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct MergeTagsRequest {
    pub source_tags: Vec<String>,
    pub destination_tag: String,
}

/// Result of a merge operation
#[derive(Debug, serde::Serialize)]
pub struct MergeResult {
    pub affected_links: i64,
    pub merged_tags: Vec<String>,
    pub destination_tag: String,
}

/// Service for tag operations
#[derive(Default)]
pub struct TagService {
    repository: TagRepository,
}

impl TagService {
    /// Create a new tag service instance
    pub fn new() -> Self {
        Self {
            repository: TagRepository::new(),
        }
    }

    /// Get all tags for an organization with full statistics
    pub async fn get_org_tags(&self, db: &D1Database, org_id: &str) -> Result<Vec<OrgTag>> {
        self.repository.get_org_tags(db, org_id).await
    }

    /// Create a new tag manually (without a link).
    /// Returns the updated tag list and whether the tag was newly created.
    pub async fn create_tag(
        &self,
        db: &D1Database,
        org_id: &str,
        tag_name: &str,
        color_index: Option<i32>,
    ) -> Result<(Vec<OrgTag>, bool)> {
        // Validate and normalize tag name
        let normalized = normalize_tag(tag_name).ok_or_else(|| {
            worker::Error::RustError(
                "Invalid tag name. Tag names must be non-empty and at most 50 characters."
                    .to_string(),
            )
        })?;

        // Create the tag
        let created = self
            .repository
            .create_tag(db, org_id, &normalized, color_index)
            .await?;

        // Return updated tag list
        let tags = self.repository.get_org_tags(db, org_id).await?;
        Ok((tags, created))
    }

    /// Delete a tag from an organization.
    /// Only allows deletion if the tag has no links (unused tags only).
    pub async fn delete_tag(&self, db: &D1Database, org_id: &str, tag_name: &str) -> Result<bool> {
        // Validate tag name
        if tag_name.trim().is_empty() {
            return Err(worker::Error::RustError(
                "Tag name cannot be empty".to_string(),
            ));
        }

        self.repository
            .delete_tag_for_org(db, org_id, tag_name)
            .await
    }

    /// Update a tag's name and/or color.
    /// At least one of new_name or color_index must be provided.
    pub async fn update_tag(
        &self,
        db: &D1Database,
        org_id: &str,
        tag_name: &str,
        new_name: Option<String>,
        color_index: Option<i32>,
    ) -> Result<Vec<OrgTag>> {
        // Validate tag name
        if tag_name.trim().is_empty() {
            return Err(worker::Error::RustError(
                "Tag name cannot be empty".to_string(),
            ));
        }

        // Track the effective name for color updates (may change if renamed)
        let mut effective_name = tag_name.to_string();

        // Handle rename if new_name is provided
        if let Some(name) = new_name {
            let normalized_new_name = normalize_tag(&name).ok_or_else(|| {
                worker::Error::RustError(
                    "Invalid new tag name. Tag names must be non-empty and at most 50 characters."
                        .to_string(),
                )
            })?;

            // Only rename if name is actually different
            if tag_name != normalized_new_name {
                self.repository
                    .rename_tag_for_org(db, org_id, tag_name, &normalized_new_name)
                    .await?;
                effective_name = normalized_new_name;
            }
        }

        // Update color if provided
        if let Some(color) = color_index {
            self.repository
                .update_tag_metadata(db, org_id, &effective_name, Some(color))
                .await?;
        }

        // Return updated tag list
        self.repository.get_org_tags(db, org_id).await
    }

    /// Merge multiple source tags into a destination tag.
    /// Creates the destination tag if it doesn't exist.
    /// Returns the number of affected links.
    pub async fn merge_tags(
        &self,
        db: &D1Database,
        org_id: &str,
        request: MergeTagsRequest,
    ) -> Result<MergeResult> {
        let MergeTagsRequest {
            source_tags,
            destination_tag,
        } = request;

        // Validate we have source tags
        if source_tags.is_empty() {
            return Err(worker::Error::RustError(
                "At least one source tag is required".to_string(),
            ));
        }

        // Validate destination tag
        let normalized_dest = normalize_tag(&destination_tag).ok_or_else(|| {
            worker::Error::RustError(
                "Invalid destination tag name. Tag names must be non-empty and at most 50 characters."
                    .to_string(),
            )
        })?;

        // Normalize and validate source tags
        let mut normalized_sources = Vec::new();
        for tag in &source_tags {
            let normalized = normalize_tag(tag).ok_or_else(|| {
                worker::Error::RustError(format!("Invalid source tag name: '{}'", tag))
            })?;

            // Don't allow a tag to merge into itself
            if normalized == normalized_dest {
                return Err(worker::Error::RustError(format!(
                    "Cannot merge tag '{}' into itself",
                    tag
                )));
            }

            normalized_sources.push(normalized);
        }

        // Remove duplicates from source tags
        normalized_sources.sort();
        normalized_sources.dedup();

        // Perform the merge
        let affected_links = self
            .repository
            .merge_tags_for_org(db, org_id, &normalized_sources, &normalized_dest)
            .await?;

        Ok(MergeResult {
            affected_links,
            merged_tags: normalized_sources,
            destination_tag: normalized_dest,
        })
    }

    /// Get comprehensive analytics for tags in an organization.
    pub async fn get_tag_analytics(&self, db: &D1Database, org_id: &str) -> Result<TagAnalytics> {
        // Get all tags with stats
        let all_tags = self
            .repository
            .get_org_tags(db, org_id)
            .await
            .map_err(|e| {
                console_error!("get_org_tags failed: {:?}", e);
                e
            })?;

        let total_tags = all_tags.len() as i64;
        let used_tags = all_tags.iter().filter(|t| t.count > 0).count() as i64;
        let unused_tags = total_tags - used_tags;

        // Get top 10 tags
        let top_tags = self
            .repository
            .get_top_tags(db, org_id, 10)
            .await
            .map_err(|e| {
                console_error!("get_top_tags failed: {:?}", e);
                e
            })?;

        // Get unused tags for cleanup suggestions
        let unused_tag_names = self
            .repository
            .get_unused_tags(db, org_id)
            .await
            .map_err(|e| {
                console_error!("get_unused_tags failed: {:?}", e);
                e
            })?;

        // Get similar tags for merge suggestions
        let similar_tag_groups = self
            .repository
            .find_similar_tags(db, org_id)
            .await
            .map_err(|e| {
                console_error!("find_similar_tags failed: {:?}", e);
                e
            })?;

        Ok(TagAnalytics {
            total_tags,
            used_tags,
            unused_tags,
            top_tags,
            unused_tag_names,
            similar_tag_groups,
        })
    }

    /// Get similar tag groups for merge suggestions.
    #[allow(dead_code)]
    pub async fn get_similar_tags(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<Vec<SimilarTagGroup>> {
        self.repository.find_similar_tags(db, org_id).await
    }

    /// Get unused tags (with 0 links) for cleanup.
    #[allow(dead_code)]
    pub async fn get_unused_tags(&self, db: &D1Database, org_id: &str) -> Result<Vec<String>> {
        self.repository.get_unused_tags(db, org_id).await
    }
}
