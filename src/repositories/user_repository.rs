/// User Repository
///
/// Data access layer for user records in D1.
/// Note: Session data is stored in KV and managed via auth::session.
use crate::models::link::Link;
use crate::models::user::User;
use crate::utils::now_timestamp;
use worker::Result;
use worker::d1::D1Database;

/// Extended user info for admin panel, includes billing account details.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UserWithBillingInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub oauth_provider: String,
    pub oauth_id: String,
    pub org_id: String,
    pub role: String,
    pub created_at: i64,
    pub suspended_at: Option<i64>,
    pub suspension_reason: Option<String>,
    pub suspended_by: Option<String>,
    pub billing_account_id: Option<String>,
    pub billing_account_tier: Option<String>,
}

pub struct UserRepository;

impl UserRepository {
    pub fn new() -> Self {
        Self
    }

    /// Get a user by their ID.
    pub async fn get_user_by_id(&self, db: &D1Database, user_id: &str) -> Result<Option<User>> {
        let stmt = db.prepare(
            "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at,
                    suspended_at, suspension_reason, suspended_by
             FROM users
             WHERE id = ?1",
        );
        stmt.bind(&[user_id.into()])?.first::<User>(None).await
    }

    /// Get a user by their email address.
    pub async fn get_by_email(&self, db: &D1Database, email: &str) -> Result<Option<User>> {
        let stmt = db.prepare(
            "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at,
                    suspended_at, suspension_reason, suspended_by
             FROM users
             WHERE email = ?1",
        );
        stmt.bind(&[email.into()])?.first::<User>(None).await
    }

    /// Return all users with billing tier info, paginated.
    pub async fn list_with_billing_info(
        &self,
        db: &D1Database,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserWithBillingInfo>> {
        let rows = db
            .prepare(
                "SELECT u.id, u.email, u.name, u.avatar_url, u.oauth_provider, u.oauth_id,
                        u.org_id, u.role, u.created_at, u.suspended_at, u.suspension_reason, u.suspended_by,
                        o.billing_account_id, ba.tier as billing_account_tier
                 FROM users u
                 LEFT JOIN organizations o ON u.org_id = o.id
                 LEFT JOIN billing_accounts ba ON o.billing_account_id = ba.id
                 ORDER BY u.created_at ASC
                 LIMIT ?1 OFFSET ?2",
            )
            .bind(&[(limit as f64).into(), (offset as f64).into()])?
            .all()
            .await?
            .results::<serde_json::Value>()?;

        let users = rows
            .iter()
            .filter_map(|row| {
                Some(UserWithBillingInfo {
                    id: row["id"].as_str()?.to_string(),
                    email: row["email"].as_str()?.to_string(),
                    name: row["name"].as_str().map(|s| s.to_string()),
                    avatar_url: row["avatar_url"].as_str().map(|s| s.to_string()),
                    oauth_provider: row["oauth_provider"].as_str()?.to_string(),
                    oauth_id: row["oauth_id"].as_str()?.to_string(),
                    org_id: row["org_id"].as_str()?.to_string(),
                    role: row["role"].as_str()?.to_string(),
                    created_at: row["created_at"].as_f64()? as i64,
                    suspended_at: row["suspended_at"].as_f64().map(|v| v as i64),
                    suspension_reason: row["suspension_reason"].as_str().map(|s| s.to_string()),
                    suspended_by: row["suspended_by"].as_str().map(|s| s.to_string()),
                    billing_account_id: row["billing_account_id"].as_str().map(|s| s.to_string()),
                    billing_account_tier: row["billing_account_tier"]
                        .as_str()
                        .map(|s| s.to_string()),
                })
            })
            .collect();

        Ok(users)
    }

    /// Total number of users on the instance.
    pub async fn count(&self, db: &D1Database) -> Result<i64> {
        let result = db
            .prepare("SELECT COUNT(*) as count FROM users")
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result
            .map(|v| v["count"].as_f64().unwrap_or(0.0) as i64)
            .unwrap_or(0))
    }

    /// Number of users with the `admin` role.
    pub async fn admin_count(&self, db: &D1Database) -> Result<i64> {
        let result = db
            .prepare("SELECT COUNT(*) as count FROM users WHERE role = 'admin'")
            .first::<serde_json::Value>(None)
            .await?;
        Ok(result
            .map(|v| v["count"].as_f64().unwrap_or(0.0) as i64)
            .unwrap_or(0))
    }

    /// Returns true if `user_id` is the only admin remaining in `org_id`.
    pub async fn is_last_admin_in_org(
        &self,
        db: &D1Database,
        user_id: &str,
        org_id: &str,
    ) -> Result<bool> {
        let result = db
            .prepare(
                "SELECT COUNT(*) as admin_count FROM users
                 WHERE org_id = ?1 AND role = 'admin' AND id != ?2",
            )
            .bind(&[org_id.into(), user_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        let count = result
            .as_ref()
            .and_then(|v| v.get("admin_count"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0);
        Ok(count == 0)
    }

    /// Update a user's instance-level role.
    pub async fn update_role(&self, db: &D1Database, user_id: &str, new_role: &str) -> Result<()> {
        db.prepare("UPDATE users SET role = ?1 WHERE id = ?2")
            .bind(&[new_role.into(), user_id.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Suspend a user.
    pub async fn suspend(
        &self,
        db: &D1Database,
        user_id: &str,
        reason: &str,
        suspended_by: &str,
    ) -> Result<()> {
        let now = now_timestamp();
        db.prepare(
            "UPDATE users SET suspended_at = ?1, suspension_reason = ?2, suspended_by = ?3 WHERE id = ?4",
        )
        .bind(&[
            (now as f64).into(),
            reason.into(),
            suspended_by.into(),
            user_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Unsuspend a user.
    pub async fn unsuspend(&self, db: &D1Database, user_id: &str) -> Result<()> {
        db.prepare(
            "UPDATE users SET suspended_at = NULL, suspension_reason = NULL, suspended_by = NULL WHERE id = ?1",
        )
        .bind(&[user_id.into()])?
        .run()
        .await?;
        Ok(())
    }

    /// Delete a user and all their associated data.
    /// Returns `(user_count, links_count, analytics_count)` for audit purposes.
    pub async fn delete(&self, db: &D1Database, user_id: &str) -> Result<(usize, usize, usize)> {
        let analytics_count = db
            .prepare(
                "DELETE FROM analytics_events WHERE link_id IN (SELECT id FROM links WHERE created_by = ?1)",
            )
            .bind(&[user_id.into()])?
            .run()
            .await?
            .meta()?
            .and_then(|m| m.changes)
            .unwrap_or(0);

        db.prepare("DELETE FROM link_reports WHERE reporter_user_id = ?1 OR reviewed_by = ?2")
            .bind(&[user_id.into(), user_id.into()])?
            .run()
            .await?;

        let links_count = db
            .prepare("DELETE FROM links WHERE created_by = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?
            .meta()?
            .and_then(|m| m.changes)
            .unwrap_or(0);

        db.prepare("DELETE FROM org_members WHERE user_id = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?;

        db.prepare("DELETE FROM org_invitations WHERE invited_by = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?;

        db.prepare("DELETE FROM destination_blacklist WHERE created_by = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?;

        let user_count = db
            .prepare("DELETE FROM users WHERE id = ?1")
            .bind(&[user_id.into()])?
            .run()
            .await?
            .meta()?
            .and_then(|m| m.changes)
            .unwrap_or(0);

        Ok((user_count, links_count, analytics_count))
    }

    /// All links created by a specific user (for KV cleanup before deletion).
    pub async fn get_links_by_creator(&self, db: &D1Database, user_id: &str) -> Result<Vec<Link>> {
        db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by,
                    created_at, updated_at, expires_at, status, click_count,
                    utm_params, forward_query_params, redirect_type
             FROM links
             WHERE created_by = ?1",
        )
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results::<Link>()
    }

    /// All links belonging to an org (for KV cleanup on suspend).
    pub async fn get_links_by_org(&self, db: &D1Database, org_id: &str) -> Result<Vec<Link>> {
        db.prepare(
            "SELECT id, org_id, short_code, destination_url, title, created_by,
                    created_at, updated_at, expires_at, status, click_count,
                    utm_params, forward_query_params, redirect_type
             FROM links
             WHERE org_id = ?1",
        )
        .bind(&[org_id.into()])?
        .all()
        .await?
        .results::<Link>()
    }

    /// Soft-disable all active links for an org; returns the number of rows changed.
    pub async fn disable_all_links_for_org(&self, db: &D1Database, org_id: &str) -> Result<i64> {
        let now = now_timestamp();
        let result = db
            .prepare(
                "UPDATE links SET status = 'disabled', updated_at = ?1
                 WHERE org_id = ?2 AND status = 'active'",
            )
            .bind(&[(now as f64).into(), org_id.into()])?
            .run()
            .await?;
        Ok(result
            .meta()?
            .and_then(|m| m.changes)
            .map(|c| c as i64)
            .unwrap_or(0))
    }
}

impl Default for UserRepository {
    fn default() -> Self {
        Self::new()
    }
}
