use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Free,
    Pro,
    Business,
    /// Unlimited tier for self-hosters - no limits at all
    Unlimited,
}

impl Tier {
    pub fn from_str_value(s: &str) -> Option<Tier> {
        match s {
            "free" => Some(Tier::Free),
            "pro" => Some(Tier::Pro),
            "business" => Some(Tier::Business),
            "unlimited" => Some(Tier::Unlimited),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Tier::Free => "free",
            Tier::Pro => "pro",
            Tier::Business => "business",
            Tier::Unlimited => "unlimited",
        }
    }

    pub fn limits(&self) -> TierLimits {
        match self {
            // Free: 1 user, 1 org, 15 links/month, 7-day analytics, 5 tags, no API keys
            Tier::Free => TierLimits {
                max_links_per_month: Some(15),
                analytics_retention_days: Some(7),
                allow_custom_short_code: false,
                allow_utm_parameters: false,
                allow_query_forwarding: false,
                allow_device_routing: false,
                allow_api_keys: false,
                max_members: Some(1),
                max_orgs: Some(1),
                max_tags: Some(5),
            },
            // Pro ($9): 1 user, 1 org, 1000 links/month, 1-year analytics, custom codes, 25 tags, API keys
            Tier::Pro => TierLimits {
                max_links_per_month: Some(1000),
                analytics_retention_days: Some(365),
                allow_custom_short_code: true,
                allow_utm_parameters: true,
                allow_query_forwarding: true,
                allow_device_routing: false,
                allow_api_keys: true,
                max_members: Some(1),
                max_orgs: Some(1),
                max_tags: Some(25),
            },
            // Business ($29): 20 users, 3 orgs, 10000 links/month, unlimited analytics, unlimited tags, API keys
            Tier::Business => TierLimits {
                max_links_per_month: Some(10000),
                analytics_retention_days: None,
                allow_custom_short_code: true,
                allow_utm_parameters: true,
                allow_query_forwarding: true,
                allow_device_routing: true,
                allow_api_keys: true,
                max_members: Some(20),
                max_orgs: Some(3),
                max_tags: None,
            },
            // Unlimited (self-hosted): no limits
            Tier::Unlimited => TierLimits {
                max_links_per_month: None,
                analytics_retention_days: None,
                allow_custom_short_code: true,
                allow_utm_parameters: true,
                allow_query_forwarding: true,
                allow_device_routing: true,
                allow_api_keys: true,
                max_members: None,
                max_orgs: None,
                max_tags: None,
            },
        }
    }
}

impl fmt::Display for Tier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct TierLimits {
    /// Maximum links per calendar month. None = unlimited.
    /// When exceeded, link creation is blocked with a clear error message.
    pub max_links_per_month: Option<i64>,
    /// Analytics data retention in days. None = unlimited.
    /// Enforced at the API level (data is kept, but filtered by date window).
    pub analytics_retention_days: Option<i64>,
    /// Whether custom short codes are allowed for this tier.
    pub allow_custom_short_code: bool,
    /// Whether UTM parameters are allowed for this tier.
    pub allow_utm_parameters: bool,
    /// Whether query parameter forwarding is allowed for this tier.
    pub allow_query_forwarding: bool,
    /// Whether device-based routing (iOS/Android/Desktop URLs) is allowed for this tier.
    pub allow_device_routing: bool,
    /// Whether API key creation and usage is allowed for this tier.
    pub allow_api_keys: bool,
    /// Maximum members per organization (including owner). None = unlimited.
    pub max_members: Option<i64>,
    /// Maximum organizations a user can own. None = unlimited.
    pub max_orgs: Option<i64>,
    /// Maximum distinct tag names across all orgs in the billing account. None = unlimited.
    pub max_tags: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_str() {
        assert_eq!(Tier::from_str_value("free"), Some(Tier::Free));
        assert_eq!(Tier::from_str_value("pro"), Some(Tier::Pro));
        assert_eq!(Tier::from_str_value("business"), Some(Tier::Business));
        assert_eq!(Tier::from_str_value("unlimited"), Some(Tier::Unlimited));
        assert_eq!(Tier::from_str_value("invalid"), None);
    }

    #[test]
    fn test_tier_as_str() {
        assert_eq!(Tier::Free.as_str(), "free");
        assert_eq!(Tier::Pro.as_str(), "pro");
        assert_eq!(Tier::Business.as_str(), "business");
        assert_eq!(Tier::Unlimited.as_str(), "unlimited");
    }

    #[test]
    fn test_free_tier_has_limits() {
        let limits = Tier::Free.limits();
        assert_eq!(limits.max_links_per_month, Some(15));
        assert_eq!(limits.analytics_retention_days, Some(7));
        assert!(!limits.allow_custom_short_code);
        assert!(!limits.allow_api_keys);
        assert_eq!(limits.max_members, Some(1));
        assert_eq!(limits.max_orgs, Some(1));
        assert_eq!(limits.max_tags, Some(5));
    }

    #[test]
    fn test_pro_tier_limits() {
        let limits = Tier::Pro.limits();
        assert_eq!(limits.max_links_per_month, Some(1000));
        assert_eq!(limits.analytics_retention_days, Some(365));
        assert!(limits.allow_custom_short_code);
        assert!(limits.allow_api_keys);
        assert_eq!(limits.max_members, Some(1));
        assert_eq!(limits.max_orgs, Some(1));
        assert_eq!(limits.max_tags, Some(25));
    }

    #[test]
    fn test_business_tier_limits() {
        let limits = Tier::Business.limits();
        assert_eq!(limits.max_links_per_month, Some(10000));
        assert!(limits.analytics_retention_days.is_none());
        assert!(limits.allow_custom_short_code);
        assert!(limits.allow_api_keys);
        assert_eq!(limits.max_members, Some(20));
        assert_eq!(limits.max_orgs, Some(3));
        assert!(limits.max_tags.is_none());
    }

    #[test]
    fn test_unlimited_tier_has_no_limits() {
        let limits = Tier::Unlimited.limits();
        assert!(limits.max_links_per_month.is_none());
        assert!(limits.analytics_retention_days.is_none());
        assert!(limits.allow_custom_short_code);
        assert!(limits.allow_api_keys);
        assert!(limits.max_members.is_none());
        assert!(limits.max_orgs.is_none());
        assert!(limits.max_tags.is_none());
    }

    #[test]
    fn test_tier_display() {
        assert_eq!(format!("{}", Tier::Free), "free");
        assert_eq!(format!("{}", Tier::Pro), "pro");
        assert_eq!(format!("{}", Tier::Business), "business");
        assert_eq!(format!("{}", Tier::Unlimited), "unlimited");
    }

    #[test]
    fn test_tier_serialization() {
        assert_eq!(serde_json::to_string(&Tier::Free).unwrap(), "\"free\"");
        assert_eq!(serde_json::to_string(&Tier::Pro).unwrap(), "\"pro\"");
        assert_eq!(
            serde_json::to_string(&Tier::Business).unwrap(),
            "\"business\""
        );
        assert_eq!(
            serde_json::to_string(&Tier::Unlimited).unwrap(),
            "\"unlimited\""
        );
    }
}
