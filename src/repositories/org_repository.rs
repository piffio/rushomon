/// Organization Repository
///
/// Data access layer for organization records, memberships, invitations,
/// and org-level settings in D1.
use crate::models::{
    OrgInvitation, OrgMember, OrgMemberWithUser, OrgWithRole, Organization, link::Link,
};
use crate::repositories::BillingRepository;
use crate::utils::now_timestamp;
use worker::Result;
use worker::d1::D1Database;

pub struct OrgRepository;

impl OrgRepository {
    pub fn new() -> Self {
        Self
    }

    // ─── Organization CRUD ────────────────────────────────────────────────────────

    /// Get organization by ID
    pub async fn get_by_id(&self, db: &D1Database, org_id: &str) -> Result<Option<Organization>> {
        let stmt = db.prepare(
            "SELECT id, name, slug, created_at, created_by, billing_account_id
             FROM organizations
             WHERE id = ?1",
        );
        stmt.bind(&[org_id.into()])?
            .first::<Organization>(None)
            .await
    }

    /// Create an organization linked to a specific billing account
    pub async fn create_with_billing_account(
        &self,
        db: &D1Database,
        org_name: &str,
        created_by: &str,
        billing_account_id: &str,
    ) -> Result<Organization> {
        let org_id = uuid::Uuid::new_v4().to_string();
        let slug = self.generate_unique_slug(db, org_name).await?;
        let now = now_timestamp();

        let stmt = db.prepare(
            "INSERT INTO organizations (id, name, slug, created_at, created_by, billing_account_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        );

        stmt.bind(&[
            org_id.clone().into(),
            org_name.into(),
            slug.clone().into(),
            (now as f64).into(),
            created_by.into(),
            billing_account_id.into(),
        ])?
        .run()
        .await?;

        Ok(Organization {
            id: org_id,
            name: org_name.to_string(),
            slug,
            created_at: now,
            created_by: created_by.to_string(),
            billing_account_id: Some(billing_account_id.to_string()),
        })
    }

    /// Create a default organization for a new user with a new billing account.
    /// Uses the default tier from settings.
    pub async fn create_default(
        &self,
        db: &D1Database,
        user_id: &str,
        org_name: &str,
    ) -> Result<Organization> {
        let org_id = uuid::Uuid::new_v4().to_string();
        let slug = self.generate_unique_slug(db, org_name).await?;
        let now = now_timestamp();

        // Read the default tier from settings
        let settings_repo = crate::repositories::SettingsRepository::new();
        let tier = settings_repo
            .get_setting(db, "default_user_tier")
            .await?
            .unwrap_or_else(|| "free".to_string());

        // Create billing account for the user
        let billing_repo = BillingRepository::new();
        let billing_account = billing_repo.create(db, user_id, &tier).await?;

        let stmt = db.prepare(
            "INSERT INTO organizations (id, name, slug, created_at, created_by, billing_account_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        );

        stmt.bind(&[
            org_id.clone().into(),
            org_name.to_string().into(),
            slug.clone().into(),
            (now as f64).into(),
            user_id.into(),
            billing_account.id.clone().into(),
        ])?
        .run()
        .await?;

        Ok(Organization {
            id: org_id,
            name: org_name.to_string(),
            slug,
            created_at: now,
            created_by: user_id.to_string(),
            billing_account_id: Some(billing_account.id),
        })
    }

    /// Update an org's display name
    pub async fn update_name(&self, db: &D1Database, org_id: &str, name: &str) -> Result<()> {
        let stmt = db.prepare("UPDATE organizations SET name = ?1 WHERE id = ?2");
        stmt.bind(&[name.into(), org_id.into()])?.run().await?;
        Ok(())
    }

    /// Delete an organization and all related data
    /// IMPORTANT: Links and analytics must be handled BEFORE calling this function
    pub async fn delete(&self, db: &D1Database, org_id: &str) -> Result<()> {
        // Delete pending invitations
        let stmt = db.prepare("DELETE FROM org_invitations WHERE org_id = ?1");
        stmt.bind(&[org_id.into()])?.run().await?;

        // Delete org members
        let stmt = db.prepare("DELETE FROM org_members WHERE org_id = ?1");
        stmt.bind(&[org_id.into()])?.run().await?;

        // Delete the organization
        let stmt = db.prepare("DELETE FROM organizations WHERE id = ?1");
        stmt.bind(&[org_id.into()])?.run().await?;

        Ok(())
    }

    // ─── Org Membership ─────────────────────────────────────────────────────────────

    /// Get all organizations a user belongs to (via org_members junction table)
    pub async fn get_user_orgs(&self, db: &D1Database, user_id: &str) -> Result<Vec<OrgWithRole>> {
        let stmt = db.prepare(
            "SELECT o.id, o.name, m.role, m.joined_at
             FROM organizations o
             JOIN org_members m ON o.id = m.org_id
             WHERE m.user_id = ?1
             ORDER BY m.joined_at ASC",
        );
        let results = stmt.bind(&[user_id.into()])?.all().await?;
        let rows = results.results::<serde_json::Value>()?;

        let orgs: Vec<OrgWithRole> = rows
            .iter()
            .filter_map(|row| {
                Some(OrgWithRole {
                    id: row["id"].as_str()?.to_string(),
                    name: row["name"].as_str()?.to_string(),
                    role: row["role"].as_str()?.to_string(),
                    joined_at: row["joined_at"].as_f64()? as i64,
                })
            })
            .collect();

        Ok(orgs)
    }

    /// Get the membership record for a specific user in a specific org
    /// Falls back to users.org_id ownership check and auto-inserts the row to self-heal
    pub async fn get_member(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
    ) -> Result<Option<OrgMember>> {
        let stmt = db.prepare(
            "SELECT org_id, user_id, role, joined_at
             FROM org_members
             WHERE org_id = ?1 AND user_id = ?2",
        );
        let existing = stmt
            .bind(&[org_id.into(), user_id.into()])?
            .first::<OrgMember>(None)
            .await?;

        if existing.is_some() {
            return Ok(existing);
        }

        // Not in org_members — check if org_id matches users.org_id (pre-migration user)
        let user_row = db
            .prepare("SELECT org_id, created_at FROM users WHERE id = ?1")
            .bind(&[user_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        let Some(user_row) = user_row else {
            return Ok(None);
        };

        if user_row["org_id"].as_str() != Some(org_id) {
            return Ok(None);
        }

        let joined_at = user_row["created_at"].as_f64().unwrap_or(0.0) as i64;

        // Auto-heal: insert the missing membership row
        db.prepare(
            "INSERT INTO org_members (org_id, user_id, role, joined_at)
             VALUES (?1, ?2, 'owner', ?3)
             ON CONFLICT(org_id, user_id) DO NOTHING",
        )
        .bind(&[org_id.into(), user_id.into(), (joined_at as f64).into()])?
        .run()
        .await?;

        Ok(Some(OrgMember {
            org_id: org_id.to_string(),
            user_id: user_id.to_string(),
            role: "owner".to_string(),
            joined_at,
        }))
    }

    /// Get all members of an org with user details
    pub async fn get_members(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<Vec<OrgMemberWithUser>> {
        let stmt = db.prepare(
            "SELECT u.id as user_id, u.email, u.name, u.avatar_url, m.role, m.joined_at
             FROM org_members m
             JOIN users u ON u.id = m.user_id
             WHERE m.org_id = ?1
             ORDER BY m.joined_at ASC",
        );
        let results = stmt.bind(&[org_id.into()])?.all().await?;
        let rows = results.results::<serde_json::Value>()?;
        let members = rows
            .iter()
            .filter_map(|row| {
                Some(OrgMemberWithUser {
                    user_id: row["user_id"].as_str()?.to_string(),
                    email: row["email"].as_str()?.to_string(),
                    name: row["name"].as_str().map(|s| s.to_string()),
                    avatar_url: row["avatar_url"].as_str().map(|s| s.to_string()),
                    role: row["role"].as_str()?.to_string(),
                    joined_at: row["joined_at"].as_f64()? as i64,
                })
            })
            .collect();
        Ok(members)
    }

    /// Add a user to an org
    pub async fn add_member(
        &self,
        db: &D1Database,
        org_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<()> {
        let now = now_timestamp();
        let stmt = db.prepare(
            "INSERT INTO org_members (org_id, user_id, role, joined_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(org_id, user_id) DO NOTHING",
        );
        stmt.bind(&[
            org_id.into(),
            user_id.into(),
            role.into(),
            (now as f64).into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Remove a user from an org
    pub async fn remove_member(&self, db: &D1Database, org_id: &str, user_id: &str) -> Result<()> {
        let stmt = db.prepare("DELETE FROM org_members WHERE org_id = ?1 AND user_id = ?2");
        stmt.bind(&[org_id.into(), user_id.into()])?.run().await?;
        Ok(())
    }

    /// Count owners of an org (to prevent removing the last owner)
    pub async fn count_owners(&self, db: &D1Database, org_id: &str) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT COUNT(*) as count FROM org_members WHERE org_id = ?1 AND role = 'owner'",
        );
        let result = stmt
            .bind(&[org_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    /// Count all members in an organization (including owner)
    pub async fn count_members(&self, db: &D1Database, org_id: &str) -> Result<i64> {
        let stmt = db.prepare("SELECT COUNT(*) as count FROM org_members WHERE org_id = ?1");
        let result = stmt
            .bind(&[org_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    /// Count organizations where a user is an owner
    pub async fn count_user_owned_orgs(&self, db: &D1Database, user_id: &str) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT COUNT(*) as count FROM org_members WHERE user_id = ?1 AND role = 'owner'",
        );
        let result = stmt
            .bind(&[user_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    // ─── Org Invitations ───────────────────────────────────────────────────────────

    /// Create a new org invitation (token = UUID = id)
    pub async fn create_invitation(
        &self,
        db: &D1Database,
        org_id: &str,
        invited_by: &str,
        email: &str,
        role: &str,
    ) -> Result<OrgInvitation> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_timestamp();
        let expires_at = now + 7 * 24 * 3600; // 7 days

        let stmt = db.prepare(
            "INSERT INTO org_invitations (id, org_id, invited_by, email, role, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        );
        stmt.bind(&[
            id.clone().into(),
            org_id.into(),
            invited_by.into(),
            email.into(),
            role.into(),
            (now as f64).into(),
            (expires_at as f64).into(),
        ])?
        .run()
        .await?;

        Ok(OrgInvitation {
            id,
            org_id: org_id.to_string(),
            invited_by: invited_by.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            created_at: now,
            expires_at,
            accepted_at: None,
        })
    }

    /// Look up an invitation by its token (UUID = id)
    pub async fn get_invitation_by_token(
        &self,
        db: &D1Database,
        token: &str,
    ) -> Result<Option<OrgInvitation>> {
        let stmt = db.prepare(
            "SELECT id, org_id, invited_by, email, role, created_at, expires_at, accepted_at
             FROM org_invitations
             WHERE id = ?1",
        );
        stmt.bind(&[token.into()])?
            .first::<OrgInvitation>(None)
            .await
    }

    /// List pending (not yet accepted, not expired) invitations for an org
    pub async fn list_pending_invitations(
        &self,
        db: &D1Database,
        org_id: &str,
    ) -> Result<Vec<OrgInvitation>> {
        let now = now_timestamp();
        let stmt = db.prepare(
            "SELECT id, org_id, invited_by, email, role, created_at, expires_at, accepted_at
             FROM org_invitations
             WHERE org_id = ?1 AND accepted_at IS NULL AND expires_at > ?2
             ORDER BY created_at DESC",
        );
        let results = stmt
            .bind(&[org_id.into(), (now as f64).into()])?
            .all()
            .await?;
        results.results::<OrgInvitation>()
    }

    /// Mark an invitation as accepted
    pub async fn accept_invitation(&self, db: &D1Database, token: &str) -> Result<()> {
        let now = now_timestamp();
        let stmt = db.prepare("UPDATE org_invitations SET accepted_at = ?1 WHERE id = ?2");
        stmt.bind(&[(now as f64).into(), token.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Delete an invitation (revoke)
    pub async fn revoke_invitation(&self, db: &D1Database, invitation_id: &str) -> Result<()> {
        let stmt = db.prepare("DELETE FROM org_invitations WHERE id = ?1");
        stmt.bind(&[invitation_id.into()])?.run().await?;
        Ok(())
    }

    /// Count pending (not yet accepted) invitations for an org
    pub async fn count_pending_invitations(&self, db: &D1Database, org_id: &str) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT COUNT(*) as count FROM org_invitations
             WHERE org_id = ?1 AND accepted_at IS NULL AND expires_at > ?2",
        );
        let now = now_timestamp();
        let result = stmt
            .bind(&[org_id.into(), (now as f64).into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    /// Check whether a pending (non-expired) invite for this email already exists in the org
    pub async fn pending_invite_exists(
        &self,
        db: &D1Database,
        org_id: &str,
        email: &str,
    ) -> Result<bool> {
        let now = now_timestamp();
        let stmt = db.prepare(
            "SELECT 1 FROM org_invitations
             WHERE org_id = ?1 AND email = ?2 AND accepted_at IS NULL AND expires_at > ?3
             LIMIT 1",
        );
        Ok(stmt
            .bind(&[org_id.into(), email.into(), (now as f64).into()])?
            .first::<serde_json::Value>(None)
            .await?
            .is_some())
    }

    // ─── Org Settings ─────────────────────────────────────────────────────────────

    /// Get the org-level forward_query_params default setting (0 or 1)
    pub async fn get_forward_query_params(&self, db: &D1Database, org_id: &str) -> Result<bool> {
        let stmt = db.prepare(
            "SELECT COALESCE(forward_query_params, 0) as forward_query_params
             FROM organizations
             WHERE id = ?1",
        );
        let result = stmt
            .bind(&[org_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result
            .and_then(|r| r["forward_query_params"].as_f64())
            .map(|v| v != 0.0)
            .unwrap_or(false))
    }

    /// Update the org-level forward_query_params default
    pub async fn set_forward_query_params(
        &self,
        db: &D1Database,
        org_id: &str,
        forward: bool,
    ) -> Result<()> {
        let stmt = db.prepare("UPDATE organizations SET forward_query_params = ?1 WHERE id = ?2");
        let value: i64 = if forward { 1 } else { 0 };
        stmt.bind(&[(value as f64).into(), org_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Get the org logo_url (nullable)
    pub async fn get_logo_url(&self, db: &D1Database, org_id: &str) -> Result<Option<String>> {
        let stmt = db.prepare("SELECT logo_url FROM organizations WHERE id = ?1");
        let result = stmt
            .bind(&[org_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|r| r["logo_url"].as_str().map(|s| s.to_string())))
    }

    /// Set (or clear) the org logo_url
    pub async fn set_logo_url(
        &self,
        db: &D1Database,
        org_id: &str,
        logo_url: Option<&str>,
    ) -> Result<()> {
        let stmt = db.prepare("UPDATE organizations SET logo_url = ?1 WHERE id = ?2");
        stmt.bind(&[
            logo_url
                .map(|s| s.into())
                .unwrap_or(wasm_bindgen::JsValue::NULL),
            org_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    // ─── Org Links (for delete/migrate operations) ─────────────────────────────────

    /// Count active links in an organization
    pub async fn count_links(&self, db: &D1Database, org_id: &str) -> Result<i64> {
        let stmt = db
            .prepare("SELECT COUNT(*) as count FROM links WHERE org_id = ?1 AND status = 'active'");
        let result = stmt
            .bind(&[org_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
    }

    /// Get all link IDs for an organization (for KV cleanup)
    pub async fn get_link_ids(&self, db: &D1Database, org_id: &str) -> Result<Vec<String>> {
        let stmt = db.prepare("SELECT id FROM links WHERE org_id = ?1");
        let results = stmt.bind(&[org_id.into()])?.all().await?;

        let link_ids: Vec<String> = results
            .results::<serde_json::Value>()?
            .into_iter()
            .filter_map(|v| v["id"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(link_ids)
    }

    /// Get all links for an organization
    #[allow(dead_code)]
    pub async fn get_links(&self, db: &D1Database, org_id: &str) -> Result<Vec<Link>> {
        let stmt = db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by,
                    created_at, updated_at, expires_at, status, click_count,
                    utm_params, forward_query_params, redirect_type
             FROM links
             WHERE org_id = ?1",
        );
        let results = stmt.bind(&[org_id.into()])?.all().await?;
        results.results::<Link>()
    }

    /// Migrate all links from one org to another (updates both active and inactive links)
    pub async fn migrate_links(
        &self,
        db: &D1Database,
        from_org_id: &str,
        to_org_id: &str,
    ) -> Result<()> {
        let stmt = db.prepare("UPDATE links SET org_id = ?1 WHERE org_id = ?2");
        stmt.bind(&[to_org_id.into(), from_org_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Hard delete all links in an organization (after KV cleanup)
    /// Deletes analytics events first to satisfy FK constraints
    pub async fn delete_all_links(&self, db: &D1Database, org_id: &str) -> Result<()> {
        // First delete analytics events (FK constraint: analytics_events.link_id -> links.id)
        let analytics_stmt = db.prepare("DELETE FROM analytics_events WHERE org_id = ?1");
        analytics_stmt.bind(&[org_id.into()])?.run().await?;

        // Then delete the links themselves
        let stmt = db.prepare("DELETE FROM links WHERE org_id = ?1");
        stmt.bind(&[org_id.into()])?.run().await?;

        Ok(())
    }

    // ─── Helper Functions ─────────────────────────────────────────────────────────

    /// Generate a unique slug for an organization name
    /// Always adds random 5-character suffix for consistency and collision prevention
    async fn generate_unique_slug(&self, _db: &D1Database, org_name: &str) -> Result<String> {
        // Generate base slug using existing logic
        let base_slug = org_name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();

        let uuid_str = uuid::Uuid::new_v4().to_string().replace('-', "");
        let random_suffix = &uuid_str[..5];

        let slug = format!("{}-{}", base_slug, random_suffix);

        Ok(slug)
    }
}

impl Default for OrgRepository {
    fn default() -> Self {
        Self::new()
    }
}
