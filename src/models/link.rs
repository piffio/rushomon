use crate::utils::now_timestamp;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Link {
    pub id: String,
    pub org_id: String,
    pub short_code: String,
    pub destination_url: String,
    pub title: Option<String>,
    pub created_by: String,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub is_active: bool,
    pub click_count: i64,
}

impl<'de> Deserialize<'de> for Link {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct LinkHelper {
            id: String,
            org_id: String,
            short_code: String,
            destination_url: String,
            title: Option<String>,
            created_by: String,
            created_at: i64,
            updated_at: Option<i64>,
            expires_at: Option<i64>,
            is_active: i64, // D1 returns numbers, so use i64 directly
            click_count: i64,
        }

        let helper = LinkHelper::deserialize(deserializer)?;

        // Convert integer to boolean (0 → false, non-zero → true)
        let is_active = helper.is_active != 0;

        Ok(Link {
            id: helper.id,
            org_id: helper.org_id,
            short_code: helper.short_code,
            destination_url: helper.destination_url,
            title: helper.title,
            created_by: helper.created_by,
            created_at: helper.created_at,
            updated_at: helper.updated_at,
            expires_at: helper.expires_at,
            is_active,
            click_count: helper.click_count,
        })
    }
}

/// The data stored in KV for fast redirect lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMapping {
    pub destination_url: String,
    pub link_id: String,
    pub expires_at: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateLinkRequest {
    pub destination_url: String,
    pub short_code: Option<String>,
    pub title: Option<String>,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateLinkRequest {
    pub destination_url: Option<String>,
    pub title: Option<String>,
    pub is_active: Option<bool>,
    pub expires_at: Option<i64>,
}

impl Link {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = now_timestamp();
            return now > expires_at;
        }
        false
    }

    pub fn to_mapping(&self) -> LinkMapping {
        LinkMapping {
            destination_url: self.destination_url.clone(),
            link_id: self.id.clone(),
            expires_at: self.expires_at,
            is_active: self.is_active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_is_expired_returns_false_when_no_expiration() {
        let link = Link {
            id: "test-id".to_string(),
            org_id: "org-id".to_string(),
            short_code: "abc123".to_string(),
            destination_url: "https://example.com".to_string(),
            title: None,
            created_by: "user-id".to_string(),
            created_at: 1000000,
            updated_at: None,
            expires_at: None, // No expiration
            is_active: true,
            click_count: 0,
        };
        assert!(!link.is_expired());
    }

    #[test]
    fn test_link_is_expired_returns_false_when_not_expired() {
        let future_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 3600; // 1 hour in future

        let link = Link {
            id: "test-id".to_string(),
            org_id: "org-id".to_string(),
            short_code: "abc123".to_string(),
            destination_url: "https://example.com".to_string(),
            title: None,
            created_by: "user-id".to_string(),
            created_at: 1000000,
            updated_at: None,
            expires_at: Some(future_timestamp),
            is_active: true,
            click_count: 0,
        };
        assert!(!link.is_expired());
    }

    #[test]
    fn test_link_is_expired_returns_true_when_expired() {
        let past_timestamp = 1000000; // Very old timestamp (Jan 1970)

        let link = Link {
            id: "test-id".to_string(),
            org_id: "org-id".to_string(),
            short_code: "abc123".to_string(),
            destination_url: "https://example.com".to_string(),
            title: None,
            created_by: "user-id".to_string(),
            created_at: 1000000,
            updated_at: None,
            expires_at: Some(past_timestamp),
            is_active: true,
            click_count: 0,
        };
        assert!(link.is_expired());
    }

    #[test]
    fn test_link_to_mapping_conversion() {
        let link = Link {
            id: "link-123".to_string(),
            org_id: "org-456".to_string(),
            short_code: "abc123".to_string(),
            destination_url: "https://example.com/path".to_string(),
            title: Some("Test Link".to_string()),
            created_by: "user-789".to_string(),
            created_at: 1000000,
            updated_at: None,
            expires_at: Some(2000000),
            is_active: true,
            click_count: 42,
        };

        let mapping = link.to_mapping();

        assert_eq!(mapping.destination_url, "https://example.com/path");
        assert_eq!(mapping.link_id, "link-123");
        assert_eq!(mapping.expires_at, Some(2000000));
        assert_eq!(mapping.is_active, true);
    }

    #[test]
    fn test_link_to_mapping_preserves_all_fields() {
        let link = Link {
            id: "id-1".to_string(),
            org_id: "org-1".to_string(),
            short_code: "test".to_string(),
            destination_url: "https://test.com".to_string(),
            title: None,
            created_by: "user-1".to_string(),
            created_at: 123456,
            updated_at: Some(789012),
            expires_at: None,
            is_active: false,
            click_count: 100,
        };

        let mapping = link.to_mapping();

        assert_eq!(mapping.destination_url, link.destination_url);
        assert_eq!(mapping.link_id, link.id);
        assert_eq!(mapping.expires_at, link.expires_at);
        assert_eq!(mapping.is_active, link.is_active);
    }
}
