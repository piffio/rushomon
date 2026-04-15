/// API Key Service
///
/// Business logic for user-facing API key management:
/// - Tier gating (Pro+ required)
/// - Token generation and hashing
/// - Ownership-scoped revocation
use crate::models::Tier;
use crate::repositories::api_key_repository::ApiKeyRecord;
use crate::repositories::{ApiKeyRepository, BillingRepository};
use crate::utils::{generate_short_code_with_length, now_timestamp};
use hex;
use sha2::{Digest, Sha256};
use worker::Result;
use worker::d1::D1Database;

pub struct ApiKeyService;

impl ApiKeyService {
    pub fn new() -> Self {
        Self
    }

    /// Create a new API key for the given user/org.
    ///
    /// Performs the tier check, generates the token + hash, and persists the record.
    /// Returns `(key_id, raw_token, hint, created_at, expires_at)`.
    /// The raw token is shown exactly once — callers must surface it to the user immediately.
    pub async fn create(
        &self,
        db: &D1Database,
        user_id: &str,
        org_id: &str,
        name: &str,
        expires_in_days: Option<i64>,
    ) -> Result<(String, String, String, i64, Option<i64>)> {
        // --- Tier gate ---
        let billing_repo = BillingRepository::new();
        let tier = match billing_repo.get_billing_account_for_org(db, org_id).await? {
            Some(ba) => Tier::from_str_value(&ba.tier).unwrap_or(Tier::Free),
            None => Tier::Free,
        };

        if !tier.limits().allow_api_keys {
            return Err(worker::Error::RustError(
                "API keys are not available on your current plan. Upgrade to Pro or higher to use API keys.".to_string(),
            ));
        }

        // --- Generate token ---
        let now = now_timestamp();
        let expires_at = expires_in_days.map(|days| now + (days * 24 * 60 * 60));
        let raw_token = format!("ro_pat_{}", generate_short_code_with_length(32));
        let hint = format!("ro_pat_...{}", &raw_token[raw_token.len() - 4..]);

        // Hash for storage
        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        let key_hash = hex::encode(hasher.finalize());

        let key_id = uuid::Uuid::new_v4().to_string();

        // --- Persist ---
        ApiKeyRepository::new()
            .create_for_user(
                db, &key_id, user_id, org_id, name, &key_hash, &hint, now, expires_at,
            )
            .await?;

        Ok((key_id, raw_token, hint, now, expires_at))
    }

    /// List active API keys for the given user.
    pub async fn list(&self, db: &D1Database, user_id: &str) -> Result<Vec<ApiKeyRecord>> {
        ApiKeyRepository::new().list_for_user(db, user_id).await
    }

    /// Revoke an API key that must be owned by `user_id`.
    pub async fn revoke(&self, db: &D1Database, key_id: &str, user_id: &str) -> Result<()> {
        ApiKeyRepository::new()
            .revoke_for_user(db, key_id, user_id)
            .await
    }
}

impl Default for ApiKeyService {
    fn default() -> Self {
        Self::new()
    }
}
