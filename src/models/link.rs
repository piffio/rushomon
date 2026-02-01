use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// The data stored in KV for fast redirect lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMapping {
    pub destination_url: String,
    pub link_id: String,
    pub expires_at: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
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
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
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
            .as_secs() as i64 + 3600; // 1 hour in future

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
