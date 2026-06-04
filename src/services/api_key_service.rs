/// API Key Service
///
/// Business logic for user-facing API key management:
/// - Tier gating (Pro+ required)
/// - Token generation and hashing
/// - Ownership-scoped revocation
/// - Org-scope management (set/update which orgs a key can act on behalf of)
use crate::models::Tier;
use crate::repositories::api_key_repository::ApiKeyRecord;
use crate::repositories::{ApiKeyRepository, BillingRepository, OrgRepository};
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
    /// `org_ids` is the list of orgs this key is allowed to act on behalf of.
    /// When empty, defaults to all orgs the user currently belongs to.
    ///
    /// Returns `(key_id, raw_token, hint, created_at, expires_at, org_ids)`.
    /// The raw token is shown exactly once — callers must surface it to the user immediately.
    pub async fn create(
        &self,
        db: &D1Database,
        user_id: &str,
        org_id: &str,
        name: &str,
        expires_in_days: Option<i64>,
        requested_org_ids: Vec<String>,
    ) -> Result<(String, String, String, i64, Option<i64>, Vec<String>)> {
        // --- Tier gate (check against the current session org) ---
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

        // --- Resolve org scope ---
        // If the caller supplied a list, validate all are orgs the user actually belongs to.
        // If the caller supplied nothing, default to all of the user's orgs.
        let user_org_ids: Vec<String> = OrgRepository::new()
            .get_user_orgs(db, user_id)
            .await?
            .into_iter()
            .map(|o| o.id)
            .collect();

        let final_org_ids: Vec<String> = if requested_org_ids.is_empty() {
            user_org_ids
        } else {
            // Validate all requested orgs are in the user's membership.
            for req_org in &requested_org_ids {
                if !user_org_ids.contains(req_org) {
                    return Err(worker::Error::RustError(format!(
                        "Organization '{}' is not in your membership list",
                        req_org
                    )));
                }
            }
            requested_org_ids
        };

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

        let repo = ApiKeyRepository::new();

        // --- Persist key ---
        repo.create_for_user(
            db, &key_id, user_id, org_id, name, &key_hash, &hint, now, expires_at,
        )
        .await?;

        // --- Persist org scope ---
        let org_id_refs: Vec<&str> = final_org_ids.iter().map(|s| s.as_str()).collect();
        repo.set_key_orgs(db, &key_id, &org_id_refs).await?;

        Ok((key_id, raw_token, hint, now, expires_at, final_org_ids))
    }

    /// List active API keys for the given user (includes org_ids per key).
    pub async fn list(&self, db: &D1Database, user_id: &str) -> Result<Vec<ApiKeyRecord>> {
        ApiKeyRepository::new().list_for_user(db, user_id).await
    }

    /// Revoke an API key that must be owned by `user_id`.
    pub async fn revoke(&self, db: &D1Database, key_id: &str, user_id: &str) -> Result<()> {
        ApiKeyRepository::new()
            .revoke_for_user(db, key_id, user_id)
            .await
    }

    /// Update the org scope of an existing key owned by `user_id`.
    ///
    /// Validates that all supplied org IDs are in the user's current membership.
    /// The org list must be non-empty (use `revoke` to deactivate a key instead).
    pub async fn update_orgs(
        &self,
        db: &D1Database,
        key_id: &str,
        user_id: &str,
        new_org_ids: Vec<String>,
    ) -> Result<()> {
        if new_org_ids.is_empty() {
            return Err(worker::Error::RustError(
                "org_ids must contain at least one organization".to_string(),
            ));
        }

        // Verify ownership.
        let owner = ApiKeyRepository::new().get_owner(db, key_id).await?;
        match owner {
            Some(owner_id) if owner_id == user_id => {}
            _ => {
                return Err(worker::Error::RustError(
                    "API key not found or not owned by you".to_string(),
                ));
            }
        }

        // Validate membership for each requested org.
        let user_org_ids: Vec<String> = OrgRepository::new()
            .get_user_orgs(db, user_id)
            .await?
            .into_iter()
            .map(|o| o.id)
            .collect();

        for req_org in &new_org_ids {
            if !user_org_ids.contains(req_org) {
                return Err(worker::Error::RustError(format!(
                    "Organization '{}' is not in your membership list",
                    req_org
                )));
            }
        }

        let org_refs: Vec<&str> = new_org_ids.iter().map(|s| s.as_str()).collect();
        ApiKeyRepository::new()
            .set_key_orgs(db, key_id, &org_refs)
            .await
    }

    /// Handle a user being removed from an org.
    ///
    /// Removes `org_id` from that user's key scopes; keys that end up with zero
    /// allowed orgs are automatically revoked with `updated_by = 'system'`.
    pub async fn handle_user_removed_from_org(
        &self,
        db: &D1Database,
        user_id: &str,
        org_id: &str,
    ) -> Result<()> {
        let repo = ApiKeyRepository::new();
        let keys_to_revoke = repo.remove_org_from_user_keys(db, user_id, org_id).await?;
        if !keys_to_revoke.is_empty() {
            repo.revoke_keys_system(db, &keys_to_revoke).await?;
        }
        Ok(())
    }
}

impl Default for ApiKeyService {
    fn default() -> Self {
        Self::new()
    }
}
