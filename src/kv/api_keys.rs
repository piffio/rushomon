use crate::models::Tier;
use serde::{Deserialize, Serialize};
use worker::{Result, kv::KvStore};

/// KV cache entry for API key validation
/// Contains minimal data needed for fast authentication decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyValidation {
    pub user_id: String,
    pub org_id: String,
    pub tier: String,
    pub expires_at: Option<i64>,
}

impl ApiKeyValidation {
    /// Check if this API key is currently valid
    pub fn is_valid(&self) -> bool {
        // Check expiration
        if let Some(expires_at) = self.expires_at {
            let now = chrono::Utc::now().timestamp();
            if now > expires_at {
                return false;
            }
        }

        // Check tier allows API keys
        match Tier::from_str_value(&self.tier) {
            Some(tier) => tier.limits().allow_api_keys,
            None => false, // Unknown tier = deny
        }
    }
}

/// KV key format for API key validation cache
fn make_cache_key(key_hash: &str) -> String {
    format!("api_key:{}", key_hash)
}

/// Store API key validation data in KV cache
pub async fn store_api_key_validation(
    kv: &KvStore,
    key_hash: &str,
    validation: &ApiKeyValidation,
) -> Result<()> {
    let key = make_cache_key(key_hash);

    // Set TTL based on key expiration or default 24 hours
    let mut put = kv.put(&key, validation)?;

    let ttl = if let Some(expires_at) = validation.expires_at {
        let now = chrono::Utc::now().timestamp();
        (expires_at - now).max(0) as u64
    } else {
        // 24 hours for non-expiring keys
        24 * 60 * 60
    };

    put = put.expiration_ttl(ttl);
    put.execute().await?;
    Ok(())
}

/// Get API key validation data from KV cache
pub async fn get_api_key_validation(
    kv: &KvStore,
    key_hash: &str,
) -> Result<Option<ApiKeyValidation>> {
    let key = make_cache_key(key_hash);
    kv.get(&key)
        .json::<ApiKeyValidation>()
        .await
        .map_err(|e| worker::Error::RustError(format!("KV cache error: {:?}", e)))
}

/// Delete API key validation data from KV cache
pub async fn delete_api_key_validation(kv: &KvStore, key_hash: &str) -> Result<()> {
    let key = make_cache_key(key_hash);
    kv.delete(&key).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_key_validation_is_valid() {
        let validation = ApiKeyValidation {
            user_id: "user123".to_string(),
            org_id: "org123".to_string(),
            tier: "pro".to_string(),
            expires_at: None,
        };

        assert!(validation.is_valid());
    }

    #[test]
    fn test_api_key_validation_free_tier_invalid() {
        let validation = ApiKeyValidation {
            user_id: "user123".to_string(),
            org_id: "org123".to_string(),
            tier: "free".to_string(),
            expires_at: None,
        };

        assert!(!validation.is_valid());
    }

    #[test]
    fn test_api_key_validation_expired() {
        let past_timestamp = chrono::Utc::now().timestamp() - 3600; // 1 hour ago
        let validation = ApiKeyValidation {
            user_id: "user123".to_string(),
            org_id: "org123".to_string(),
            tier: "pro".to_string(),
            expires_at: Some(past_timestamp),
        };

        assert!(!validation.is_valid());
    }

    #[test]
    fn test_api_key_validation_future_expiration() {
        let future_timestamp = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
        let validation = ApiKeyValidation {
            user_id: "user123".to_string(),
            org_id: "org123".to_string(),
            tier: "pro".to_string(),
            expires_at: Some(future_timestamp),
        };

        assert!(validation.is_valid());
    }

    #[test]
    fn test_api_key_validation_unknown_tier() {
        let validation = ApiKeyValidation {
            user_id: "user123".to_string(),
            org_id: "org123".to_string(),
            tier: "unknown".to_string(),
            expires_at: None,
        };

        assert!(!validation.is_valid());
    }

    #[test]
    fn test_make_cache_key() {
        let key = make_cache_key("abc123");
        assert_eq!(key, "api_key:abc123");
    }

    // Helper function for testing
    fn make_cache_key(key_hash: &str) -> String {
        format!("api_key:{}", key_hash)
    }
}
