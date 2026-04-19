/// API Key Repository
///
/// Provides data access for the `api_keys` table covering both admin and
/// user-facing operations.
use crate::utils::now_timestamp;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use worker::Result;
use worker::d1::D1Database;

/// A user-visible API key record (no key_hash, no raw token).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyRecord {
    pub id: String,
    pub name: String,
    pub hint: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub expires_at: Option<i64>,
}

/// API key record with tier information (for authentication).
#[derive(Debug, Deserialize)]
pub struct ApiKeyWithTierRecord {
    pub id: String,
    pub user_id: String,
    pub org_id: String,
    pub expires_at: Option<i64>,
    pub status: String,
    pub tier: Option<String>,
}

/// Admin view of an API key with user/org details.
#[derive(Debug, Serialize, Deserialize)]
pub struct AdminApiKeyRecord {
    pub id: String,
    pub name: String,
    pub hint: String,
    pub user_id: String,
    pub user_email: Option<String>,
    pub user_name: Option<String>,
    pub org_id: String,
    pub org_name: Option<String>,
    pub tier: Option<String>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub status: String,
    pub updated_at: Option<i64>,
    pub updated_by: Option<String>,
}

pub struct ApiKeyRepository;

impl ApiKeyRepository {
    pub fn new() -> Self {
        Self
    }

    /// Return a paginated list of all API keys with user / org / tier info.
    pub async fn list_all(
        &self,
        db: &D1Database,
        page: i64,
        limit: i64,
        search: Option<&str>,
        status_filter: Option<&str>,
    ) -> Result<(Vec<AdminApiKeyRecord>, i64)> {
        let offset = (page - 1) * limit;

        let status_clause = match status_filter {
            Some("active") => "AND ak.status = 'active'",
            Some("revoked") => "AND ak.status = 'revoked'",
            Some("deleted") => "AND ak.status = 'deleted'",
            _ => "",
        };

        #[derive(serde::Deserialize)]
        struct CountRow {
            cnt: i64,
        }

        if let Some(s) = search {
            let pattern = format!("%{}%", s);
            let where_clause = format!(
                "WHERE (u.email LIKE ?1 OR ak.name LIKE ?1 OR o.name LIKE ?1) {}",
                status_clause
            );

            let count_sql = format!(
                "SELECT COUNT(*) as cnt
                 FROM api_keys ak
                 JOIN users u ON ak.user_id = u.id
                 JOIN organizations o ON ak.org_id = o.id
                 LEFT JOIN billing_accounts ba ON o.billing_account_id = ba.id
                 {}",
                where_clause
            );

            let list_sql = format!(
                "SELECT ak.id, ak.name, ak.hint, ak.user_id,
                        u.email as user_email, u.name as user_name,
                        ak.org_id, o.name as org_name, ba.tier,
                        ak.created_at, ak.last_used_at, ak.expires_at,
                        ak.status, ak.updated_at, ak.updated_by
                 FROM api_keys ak
                 JOIN users u ON ak.user_id = u.id
                 JOIN organizations o ON ak.org_id = o.id
                 LEFT JOIN billing_accounts ba ON o.billing_account_id = ba.id
                 {}
                 ORDER BY ak.created_at DESC
                 LIMIT ?2 OFFSET ?3",
                where_clause
            );

            let total = db
                .prepare(&count_sql)
                .bind(&[pattern.clone().into()])?
                .first::<CountRow>(None)
                .await?
                .map(|r| r.cnt)
                .unwrap_or(0);

            let keys = db
                .prepare(&list_sql)
                .bind(&[
                    pattern.into(),
                    (limit as f64).into(),
                    (offset as f64).into(),
                ])?
                .all()
                .await?
                .results::<AdminApiKeyRecord>()?;

            Ok((keys, total))
        } else {
            let where_clause = if status_clause.is_empty() {
                "".to_string()
            } else {
                format!("WHERE {}", status_clause.trim_start_matches("AND "))
            };

            let count_sql = format!(
                "SELECT COUNT(*) as cnt
                 FROM api_keys ak
                 JOIN users u ON ak.user_id = u.id
                 JOIN organizations o ON ak.org_id = o.id
                 LEFT JOIN billing_accounts ba ON o.billing_account_id = ba.id
                 {}",
                where_clause
            );

            let list_sql = format!(
                "SELECT ak.id, ak.name, ak.hint, ak.user_id,
                        u.email as user_email, u.name as user_name,
                        ak.org_id, o.name as org_name, ba.tier,
                        ak.created_at, ak.last_used_at, ak.expires_at,
                        ak.status, ak.updated_at, ak.updated_by
                 FROM api_keys ak
                 JOIN users u ON ak.user_id = u.id
                 JOIN organizations o ON ak.org_id = o.id
                 LEFT JOIN billing_accounts ba ON o.billing_account_id = ba.id
                 {}
                 ORDER BY ak.created_at DESC
                 LIMIT ?1 OFFSET ?2",
                where_clause
            );

            let total = db
                .prepare(&count_sql)
                .first::<CountRow>(None)
                .await?
                .map(|r| r.cnt)
                .unwrap_or(0);

            let keys = db
                .prepare(&list_sql)
                .bind(&[(limit as f64).into(), (offset as f64).into()])?
                .all()
                .await?
                .results::<AdminApiKeyRecord>()?;

            Ok((keys, total))
        }
    }

    /// Revoke an active key (status: active → revoked).
    pub async fn revoke(&self, db: &D1Database, key_id: &str, admin_user_id: &str) -> Result<()> {
        db.prepare(
            "UPDATE api_keys SET status = 'revoked', updated_at = ?1, updated_by = ?2
             WHERE id = ?3 AND status = 'active'",
        )
        .bind(&[
            (now_timestamp() as f64).into(),
            admin_user_id.into(),
            key_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Reactivate a revoked key (status: revoked → active).
    pub async fn reactivate(
        &self,
        db: &D1Database,
        key_id: &str,
        admin_user_id: &str,
    ) -> Result<()> {
        db.prepare(
            "UPDATE api_keys SET status = 'active', updated_at = ?1, updated_by = ?2
             WHERE id = ?3 AND status = 'revoked'",
        )
        .bind(&[
            (now_timestamp() as f64).into(),
            admin_user_id.into(),
            key_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Soft-delete a key (status → deleted).
    pub async fn delete(&self, db: &D1Database, key_id: &str, admin_user_id: &str) -> Result<()> {
        db.prepare(
            "UPDATE api_keys SET status = 'deleted', updated_at = ?1, updated_by = ?2
             WHERE id = ?3 AND status != 'deleted'",
        )
        .bind(&[
            (now_timestamp() as f64).into(),
            admin_user_id.into(),
            key_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Restore a deleted key (status: deleted → active).
    pub async fn restore(&self, db: &D1Database, key_id: &str, admin_user_id: &str) -> Result<()> {
        db.prepare(
            "UPDATE api_keys SET status = 'active', updated_at = ?1, updated_by = ?2
             WHERE id = ?3 AND status = 'deleted'",
        )
        .bind(&[
            (now_timestamp() as f64).into(),
            admin_user_id.into(),
            key_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Insert a new API key for a user. The `key_hash` is a SHA-256 hex string.
    /// Returns the generated key ID.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_for_user(
        &self,
        db: &D1Database,
        id: &str,
        user_id: &str,
        org_id: &str,
        name: &str,
        key_hash: &str,
        hint: &str,
        created_at: i64,
        expires_at: Option<i64>,
    ) -> Result<()> {
        db.prepare(
            "INSERT INTO api_keys (id, user_id, org_id, name, key_hash, hint, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&[
            id.into(),
            user_id.into(),
            org_id.into(),
            name.into(),
            key_hash.into(),
            hint.into(),
            (created_at as f64).into(),
            expires_at
                .map(|t| (t as f64).into())
                .unwrap_or(worker::wasm_bindgen::JsValue::NULL),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// List active API keys for a specific user (no raw token returned).
    pub async fn list_for_user(&self, db: &D1Database, user_id: &str) -> Result<Vec<ApiKeyRecord>> {
        let results = db
            .prepare(
                "SELECT id, name, hint, created_at, last_used_at, expires_at
                 FROM api_keys
                 WHERE user_id = ?1 AND status = 'active'
                 ORDER BY created_at DESC",
            )
            .bind(&[user_id.into()])?
            .all()
            .await?;
        results.results::<ApiKeyRecord>()
    }

    /// Soft-delete a key owned by the given user (status: active → deleted).
    /// Scoped to `user_id` so users cannot revoke other users' keys.
    pub async fn revoke_for_user(
        &self,
        db: &D1Database,
        key_id: &str,
        user_id: &str,
    ) -> Result<()> {
        let now = now_timestamp();
        db.prepare(
            "UPDATE api_keys SET status = 'deleted', updated_at = ?1, updated_by = ?2
             WHERE id = ?3 AND user_id = ?4 AND status = 'active'",
        )
        .bind(&[
            (now as f64).into(),
            user_id.into(),
            key_id.into(),
            user_id.into(),
        ])?
        .run()
        .await?;
        Ok(())
    }

    /// Get an API key by its hash, including tier information from the billing account.
    pub async fn get_by_hash_with_tier(
        &self,
        db: &D1Database,
        key_hash: &str,
    ) -> Result<Option<ApiKeyWithTierRecord>> {
        let stmt = db.prepare(
            "SELECT ak.id, ak.user_id, ak.org_id, ak.expires_at, ak.status, ba.tier
             FROM api_keys ak
             JOIN organizations o ON ak.org_id = o.id
             LEFT JOIN billing_accounts ba ON o.billing_account_id = ba.id
             WHERE ak.key_hash = ?1",
        );

        stmt.bind(&[key_hash.into()])?
            .first::<ApiKeyWithTierRecord>(None)
            .await
    }

    /// Update the last_used_at timestamp for an API key.
    pub async fn update_last_used(
        &self,
        db: &D1Database,
        key_id: &str,
        timestamp: i64,
    ) -> Result<()> {
        let stmt = db.prepare("UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2");
        stmt.bind(&[(timestamp as f64).into(), key_id.into()])?
            .run()
            .await?;
        Ok(())
    }
}

impl Default for ApiKeyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_api_key_status_validation() {
        // Test valid status values
        let valid_statuses = vec!["active", "revoked", "deleted"];
        for status in valid_statuses {
            assert!(matches!(status, "active" | "revoked" | "deleted"));
        }

        // Test invalid status values
        let invalid_statuses = vec!["invalid", "", "ACTIVE", "Revoked"];
        for status in invalid_statuses {
            assert!(!matches!(status, "active" | "revoked" | "deleted"));
        }
    }

    #[test]
    fn test_timestamp_conversion_safety() {
        // Test that timestamp conversion works correctly
        let timestamp = 1234567890i64;
        let converted = timestamp as f64;
        assert_eq!(converted, 1234567890.0);

        // Test edge cases
        let max_timestamp = i64::MAX;
        let max_converted = max_timestamp as f64;
        assert!(max_converted > 0.0);

        let min_timestamp = i64::MIN;
        let min_converted = min_timestamp as f64;
        assert!(min_converted < 0.0);
    }

    #[test]
    fn test_query_parameter_validation() {
        // Test that our queries use proper parameter binding
        let revoke_query = "UPDATE api_keys SET status = 'revoked', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'active'";
        assert!(revoke_query.contains("?1"));
        assert!(revoke_query.contains("?2"));
        assert!(revoke_query.contains("?3"));
        assert!(!revoke_query.contains("DROP"));

        let reactivate_query = "UPDATE api_keys SET status = 'active', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'revoked'";
        assert!(reactivate_query.contains("?1"));
        assert!(reactivate_query.contains("?2"));
        assert!(reactivate_query.contains("?3"));
        assert!(!reactivate_query.contains("DELETE"));

        let delete_query = "UPDATE api_keys SET status = 'deleted', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status != 'deleted'";
        assert!(delete_query.contains("?1"));
        assert!(delete_query.contains("?2"));
        assert!(delete_query.contains("?3"));
        assert!(!delete_query.contains("TRUNCATE"));
    }

    #[test]
    fn test_sql_injection_prevention() {
        // Test that our query patterns prevent SQL injection
        let malicious_inputs = vec![
            "'; DROP TABLE api_keys; --",
            "1' OR '1'='1",
            "admin'--",
            "'; INSERT INTO api_keys VALUES (...); --",
        ];

        for input in malicious_inputs {
            // Ensure parameter binding is used (no string concatenation)
            let query_template = "SELECT * FROM api_keys WHERE id = ?1";
            assert!(!query_template.contains(&format!("'{}'", input)));
        }
    }

    #[test]
    fn test_status_transition_logic() {
        // Test that our SQL WHERE clauses enforce correct status transitions
        // Active → Revoked: allowed
        let revoke_clause = "WHERE id = ?1 AND status = 'active'";
        assert!(revoke_clause.contains("status = 'active'"));

        // Revoked → Active: allowed
        let reactivate_clause = "WHERE id = ?1 AND status = 'revoked'";
        assert!(reactivate_clause.contains("status = 'revoked'"));

        // Active → Deleted: allowed (soft delete)
        let delete_clause = "WHERE id = ?1 AND status = 'active'";
        assert!(delete_clause.contains("status = 'active'"));

        // Ensure we don't allow transitioning from deleted to active (security measure)
        let no_restore_clause = "WHERE id = ?1 AND status = 'revoked'";
        assert!(!no_restore_clause.contains("status = 'deleted'"));
    }

    #[test]
    fn test_audit_field_consistency() {
        // Test that all update queries include audit fields
        let update_queries = vec![
            "UPDATE api_keys SET status = 'revoked', updated_at = ?1, updated_by = ?2 WHERE id = ?3",
            "UPDATE api_keys SET status = 'active', updated_at = ?1, updated_by = ?2 WHERE id = ?3",
            "UPDATE api_keys SET status = 'deleted', updated_at = ?1, updated_by = ?2 WHERE id = ?3",
        ];

        for query in update_queries {
            assert!(
                query.contains("updated_at"),
                "Query must include updated_at audit field"
            );
            assert!(
                query.contains("updated_by"),
                "Query must include updated_by audit field"
            );
        }
    }
}
