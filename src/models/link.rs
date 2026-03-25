use crate::utils::now_timestamp;
use serde::{Deserialize, Deserializer, Serialize};

/// Standard Google UTM parameters attached to a link.
/// All fields are optional; only non-empty values are appended to the destination URL.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UtmParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_campaign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_term: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_ref: Option<String>,
}

impl UtmParams {
    /// Returns true if at least one field is set to a non-empty value.
    pub fn is_empty(&self) -> bool {
        self.utm_source.as_deref().unwrap_or("").is_empty()
            && self.utm_medium.as_deref().unwrap_or("").is_empty()
            && self.utm_campaign.as_deref().unwrap_or("").is_empty()
            && self.utm_term.as_deref().unwrap_or("").is_empty()
            && self.utm_content.as_deref().unwrap_or("").is_empty()
            && self.utm_ref.as_deref().unwrap_or("").is_empty()
    }

    /// Serialize to a JSON string for DB storage.
    pub fn to_json_string(&self) -> Option<String> {
        if self.is_empty() {
            None
        } else {
            serde_json::to_string(self).ok()
        }
    }

    /// Deserialize from a JSON string from DB.
    pub fn from_json_str(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LinkStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "blocked")]
    Blocked,
}

impl LinkStatus {
    pub fn as_str(&self) -> &str {
        match self {
            LinkStatus::Active => "active",
            LinkStatus::Disabled => "disabled",
            LinkStatus::Blocked => "blocked",
        }
    }
}

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
    pub status: LinkStatus,
    pub click_count: i64,
    pub tags: Vec<String>,
    /// UTM parameters baked into this link (Pro+ only).
    pub utm_params: Option<UtmParams>,
    /// Whether to forward visitor query params to the destination (Pro+ only).
    /// None = use org default (resolved at KV write time).
    pub forward_query_params: Option<bool>,
    /// HTTP redirect type: 301 (permanent) or 307 (temporary).
    /// Default is 301 for SEO, 307 available on Pro+ plans.
    pub redirect_type: String,
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
            status: String, // D1 returns TEXT
            click_count: i64,
            utm_params: Option<String>,        // JSON string from D1
            forward_query_params: Option<i64>, // 0/1/NULL from D1
            redirect_type: String,             // "301" or "307"
        }

        let helper = LinkHelper::deserialize(deserializer)?;

        // Parse status string
        let status = match helper.status.as_str() {
            "active" => LinkStatus::Active,
            "disabled" => LinkStatus::Disabled,
            "blocked" => LinkStatus::Blocked,
            _ => LinkStatus::Disabled, // Default to disabled for unknown values
        };

        // Parse UTM params from JSON string
        let utm_params = helper
            .utm_params
            .as_deref()
            .and_then(UtmParams::from_json_str);

        // Parse forward_query_params: 1 = true, 0 = false, NULL = None
        let forward_query_params = helper.forward_query_params.map(|v| v != 0);

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
            status,
            click_count: helper.click_count,
            tags: Vec::new(), // Populated separately via get_tags_for_links
            utm_params,
            forward_query_params,
            redirect_type: helper.redirect_type,
        })
    }
}

/// The data stored in KV for fast redirect lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMapping {
    pub destination_url: String,
    pub link_id: String,
    pub expires_at: Option<i64>,
    pub status: LinkStatus,
    /// Resolved UTM params to append on every redirect (Pro+ only).
    /// Missing in old KV entries = no UTM params (safe default).
    #[serde(default)]
    pub utm_params: Option<UtmParams>,
    /// Resolved forwarding flag — org default has already been applied at write time.
    /// Missing in old KV entries = false (safe default: no forwarding).
    #[serde(default)]
    pub forward_query_params: bool,
    /// HTTP redirect type: 301 (permanent) or 307 (temporary).
    /// Missing in old KV entries = "301" (safe default: permanent for SEO).
    #[serde(default = "default_redirect_type")]
    pub redirect_type: String,
}

fn default_redirect_type() -> String {
    "301".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateLinkRequest {
    pub destination_url: String,
    pub short_code: Option<String>,
    pub title: Option<String>,
    pub expires_at: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub utm_params: Option<UtmParams>,
    pub forward_query_params: Option<bool>,
    /// HTTP redirect type: 301 (permanent) or 307 (temporary).
    /// Default is 301, available on Pro+ plans.
    #[serde(default = "default_redirect_type")]
    pub redirect_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateLinkRequest {
    pub destination_url: Option<String>,
    pub title: Option<String>,
    pub status: Option<LinkStatus>,
    pub expires_at: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub utm_params: Option<UtmParams>,
    pub forward_query_params: Option<bool>,
    /// HTTP redirect type: 301 (permanent) or 307 (temporary).
    /// Available on Pro+ plans.
    pub redirect_type: Option<String>,
}

impl Link {
    #[allow(dead_code)] // Used in tests and reserved for future expiration checks
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = now_timestamp();
            return now > expires_at;
        }
        false
    }

    /// Build a `LinkMapping` for KV storage.
    /// `resolved_forward` is the effective forwarding flag after applying the org default
    /// when `self.forward_query_params` is `None`.
    pub fn to_mapping(&self, resolved_forward: bool) -> LinkMapping {
        LinkMapping {
            destination_url: self.destination_url.clone(),
            link_id: self.id.clone(),
            expires_at: self.expires_at,
            status: self.status.clone(),
            utm_params: self.utm_params.clone(),
            forward_query_params: resolved_forward,
            redirect_type: self.redirect_type.clone(),
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
            status: LinkStatus::Active,
            click_count: 0,
            tags: Vec::new(),
            utm_params: None,
            forward_query_params: None,
            redirect_type: "301".to_string(),
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
            status: LinkStatus::Active,
            click_count: 0,
            tags: Vec::new(),
            utm_params: None,
            forward_query_params: None,
            redirect_type: "301".to_string(),
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
            status: LinkStatus::Active,
            click_count: 0,
            tags: Vec::new(),
            utm_params: None,
            forward_query_params: None,
            redirect_type: "301".to_string(),
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
            status: LinkStatus::Active,
            click_count: 42,
            tags: Vec::new(),
            utm_params: None,
            forward_query_params: None,
            redirect_type: "301".to_string(),
        };

        let mapping = link.to_mapping(false);

        assert_eq!(mapping.destination_url, "https://example.com/path");
        assert_eq!(mapping.link_id, "link-123");
        assert_eq!(mapping.expires_at, Some(2000000));
        assert!(matches!(mapping.status, LinkStatus::Active));
        assert!(!mapping.forward_query_params);
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
            status: LinkStatus::Disabled,
            click_count: 100,
            tags: Vec::new(),
            utm_params: None,
            forward_query_params: None,
            redirect_type: "301".to_string(),
        };

        let mapping = link.to_mapping(false);

        assert_eq!(mapping.destination_url, link.destination_url);
        assert_eq!(mapping.link_id, link.id);
        assert_eq!(mapping.expires_at, link.expires_at);
        assert!(matches!(mapping.status, LinkStatus::Disabled));
    }

    #[test]
    fn test_utm_params_is_empty() {
        let empty = UtmParams::default();
        assert!(empty.is_empty());

        let non_empty = UtmParams {
            utm_source: Some("twitter".to_string()),
            ..Default::default()
        };
        assert!(!non_empty.is_empty());
    }

    #[test]
    fn test_utm_params_to_json_string() {
        let params = UtmParams {
            utm_source: Some("google".to_string()),
            utm_medium: Some("cpc".to_string()),
            utm_campaign: None,
            utm_term: None,
            utm_content: None,
            utm_ref: None,
        };
        let json = params.to_json_string();
        assert!(json.is_some());
        let parsed: UtmParams = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(parsed.utm_source, Some("google".to_string()));
        assert_eq!(parsed.utm_medium, Some("cpc".to_string()));
    }

    #[test]
    fn test_link_to_mapping_with_utm_and_forwarding() {
        let utm = UtmParams {
            utm_source: Some("twitter".to_string()),
            utm_medium: Some("social".to_string()),
            utm_campaign: None,
            utm_term: None,
            utm_content: None,
            utm_ref: None,
        };
        let link = Link {
            id: "id-2".to_string(),
            org_id: "org-2".to_string(),
            short_code: "test2".to_string(),
            destination_url: "https://example.com".to_string(),
            title: None,
            created_by: "user-2".to_string(),
            created_at: 123456,
            updated_at: None,
            expires_at: None,
            status: LinkStatus::Active,
            click_count: 0,
            tags: Vec::new(),
            utm_params: Some(utm.clone()),
            forward_query_params: Some(true),
            redirect_type: "301".to_string(),
        };

        let mapping = link.to_mapping(true);
        assert!(mapping.forward_query_params);
        assert_eq!(mapping.utm_params, Some(utm));
    }
}

#[cfg(test)]
mod redirect_type_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_create_link_request_default_redirect_type() {
        let json = r#"{"destination_url": "https://example.com"}"#;
        let request: CreateLinkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.redirect_type, "301");
    }

    #[test]
    fn test_create_link_request_explicit_301() {
        let json = r#"{"destination_url": "https://example.com", "redirect_type": "301"}"#;
        let request: CreateLinkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.redirect_type, "301");
    }

    #[test]
    fn test_create_link_request_explicit_307() {
        let json = r#"{"destination_url": "https://example.com", "redirect_type": "307"}"#;
        let request: CreateLinkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.redirect_type, "307");
    }

    #[test]
    fn test_create_link_request_invalid_redirect_type() {
        let json = r#"{"destination_url": "https://example.com", "redirect_type": "404"}"#;
        let request: CreateLinkRequest = serde_json::from_str(json).unwrap();
        // Should accept any string, validation happens at runtime
        assert_eq!(request.redirect_type, "404");
    }

    #[test]
    fn test_update_link_request_optional_redirect_type() {
        let json = r#"{"destination_url": "https://example.com"}"#;
        let request: UpdateLinkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.redirect_type, None);
    }

    #[test]
    fn test_update_link_request_with_redirect_type() {
        let json = r#"{"destination_url": "https://example.com", "redirect_type": "307"}"#;
        let request: UpdateLinkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.redirect_type, Some("307".to_string()));
    }

    #[test]
    fn test_link_serialization_includes_redirect_type() {
        let link = Link {
            id: "test-id".to_string(),
            org_id: "org-1".to_string(),
            short_code: "test".to_string(),
            destination_url: "https://example.com".to_string(),
            title: Some("Test".to_string()),
            created_by: "user-1".to_string(),
            created_at: 1234567890,
            updated_at: Some(1234567890),
            expires_at: None,
            status: LinkStatus::Active,
            click_count: 0,
            tags: vec![],
            utm_params: None,
            forward_query_params: None,
            redirect_type: "301".to_string(),
        };

        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("\"redirect_type\":\"301\""));
    }

    #[test]
    fn test_link_deserialization_with_redirect_type() {
        let json = r#"{
            "id": "test-id",
            "org_id": "org-1",
            "short_code": "test",
            "destination_url": "https://example.com",
            "title": "Test",
            "created_by": "user-1",
            "created_at": 1234567890,
            "updated_at": 1234567890,
            "expires_at": null,
            "status": "active",
            "click_count": 0,
            "tags": [],
            "utm_params": null,
            "forward_query_params": null,
            "redirect_type": "307"
        }"#;

        let link: Link = serde_json::from_str(json).unwrap();
        assert_eq!(link.redirect_type, "307");
    }
}
