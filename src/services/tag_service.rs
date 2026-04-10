/// Tag service - Business logic for tag operations
///
/// Handles tag validation, business rules, and orchestrates the tag repository.
use crate::repositories::TagRepository;
use crate::repositories::tag_repository::OrgTag;
use crate::utils::normalize_tag;
use worker::d1::D1Database;
use worker::*;

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

    /// Get all tags for an organization with usage counts
    pub async fn get_org_tags(&self, db: &D1Database, org_id: &str) -> Result<Vec<OrgTag>> {
        self.repository.get_org_tags(db, org_id).await
    }

    /// Delete a tag from an organization. Returns true if the tag was deleted, false if it didn't exist.
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

    /// Rename a tag within an organization
    pub async fn rename_tag(
        &self,
        db: &D1Database,
        org_id: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<Vec<OrgTag>> {
        // Validate old tag name
        if old_name.trim().is_empty() {
            return Err(worker::Error::RustError(
                "Old tag name cannot be empty".to_string(),
            ));
        }

        // Normalize the new tag name
        let normalized_new_name = normalize_tag(new_name).ok_or_else(|| {
            worker::Error::RustError(
                "Invalid new tag name. Tag names must be non-empty and at most 50 characters."
                    .to_string(),
            )
        })?;

        // Rename the tag
        self.repository
            .rename_tag_for_org(db, org_id, old_name, &normalized_new_name)
            .await?;

        // Return updated tag list
        self.repository.get_org_tags(db, org_id).await
    }
}
