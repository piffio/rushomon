use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Free,
    Unlimited,
}

impl Tier {
    pub fn from_str_value(s: &str) -> Option<Tier> {
        match s {
            "free" => Some(Tier::Free),
            "unlimited" => Some(Tier::Unlimited),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Tier::Free => "free",
            Tier::Unlimited => "unlimited",
        }
    }

    pub fn limits(&self) -> TierLimits {
        match self {
            Tier::Free => TierLimits {
                max_links_per_month: Some(25),
                max_tracked_clicks_per_month: Some(1_000),
                analytics_retention_days: Some(7),
            },
            Tier::Unlimited => TierLimits {
                max_links_per_month: None,
                max_tracked_clicks_per_month: None,
                analytics_retention_days: None,
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
    /// Maximum links that can be created per calendar month. None = unlimited.
    pub max_links_per_month: Option<i64>,
    /// Maximum tracked clicks per calendar month. None = unlimited.
    /// When exceeded, analytics are still recorded but the UI is gated.
    pub max_tracked_clicks_per_month: Option<i64>,
    /// Analytics data retention in days. None = unlimited.
    /// Enforced at the API level (data is kept, but filtered by date window).
    pub analytics_retention_days: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_str() {
        assert_eq!(Tier::from_str_value("free"), Some(Tier::Free));
        assert_eq!(Tier::from_str_value("unlimited"), Some(Tier::Unlimited));
        assert_eq!(Tier::from_str_value("invalid"), None);
    }

    #[test]
    fn test_tier_as_str() {
        assert_eq!(Tier::Free.as_str(), "free");
        assert_eq!(Tier::Unlimited.as_str(), "unlimited");
    }

    #[test]
    fn test_free_tier_has_limits() {
        let limits = Tier::Free.limits();
        assert_eq!(limits.max_links_per_month, Some(25));
        assert_eq!(limits.max_tracked_clicks_per_month, Some(1_000));
        assert_eq!(limits.analytics_retention_days, Some(7));
    }

    #[test]
    fn test_unlimited_tier_has_no_limits() {
        let limits = Tier::Unlimited.limits();
        assert!(limits.max_links_per_month.is_none());
        assert!(limits.max_tracked_clicks_per_month.is_none());
        assert!(limits.analytics_retention_days.is_none());
    }

    #[test]
    fn test_tier_display() {
        assert_eq!(format!("{}", Tier::Free), "free");
        assert_eq!(format!("{}", Tier::Unlimited), "unlimited");
    }

    #[test]
    fn test_tier_serialization() {
        assert_eq!(serde_json::to_string(&Tier::Free).unwrap(), "\"free\"");
        assert_eq!(
            serde_json::to_string(&Tier::Unlimited).unwrap(),
            "\"unlimited\""
        );
    }
}
