/// API Key Repository
///
/// Provides data access for the `api_keys` table covering both admin and
/// user-facing operations.
use crate::db::queries::AdminApiKeyRecord;
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
}

impl Default for ApiKeyRepository {
    fn default() -> Self {
        Self::new()
    }
}
