use crate::models::Link;
use serde::{Deserialize, Serialize};

/// Time range specification for analytics queries
/// Extensible design that can grow to support custom ranges, timezones, comparisons, etc.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TimeRange {
    /// Simple days-based range (e.g., "last 7 days", "last 30 days")
    Days { value: i64 },
    /// Custom date range with specific timestamps
    Custom { start: i64, end: i64 },
}

impl TimeRange {
    /// Calculate the actual start and end timestamps based on the range specification
    /// All time calculations are done on the backend to eliminate clock skew issues
    pub fn calculate_timestamps(&self) -> (i64, i64) {
        let now = now_timestamp();

        match self {
            TimeRange::Days { value } => {
                if *value == 0 {
                    // All time - start from epoch
                    (0, now)
                } else {
                    // Last N days
                    let start = now - value * 24 * 60 * 60;
                    (start, now)
                }
            }
            TimeRange::Custom { start, end } => (*start, *end),
        }
    }
}

/// Get current timestamp for analytics calculations
pub fn now_timestamp() -> i64 {
    (js_sys::Date::now() / 1000.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_time_range_days_calculation() {
        let now = now_timestamp();

        // Test 7 days range
        let time_range = TimeRange::Days { value: 7 };
        let (start, end) = time_range.calculate_timestamps();

        assert_eq!(end, now);
        assert_eq!(start, now - 7 * 24 * 60 * 60);
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_time_range_all_time() {
        let now = now_timestamp();

        // Test all time (0 days)
        let time_range = TimeRange::Days { value: 0 };
        let (start, end) = time_range.calculate_timestamps();

        assert_eq!(end, now);
        assert_eq!(start, 0); // Epoch start
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_time_range_custom() {
        let start = 1640995200; // 2022-01-01 00:00:00 UTC
        let end = 1643673600; // 2022-02-01 00:00:00 UTC

        let time_range = TimeRange::Custom { start, end };
        let (calculated_start, calculated_end) = time_range.calculate_timestamps();

        assert_eq!(calculated_start, start);
        assert_eq!(calculated_end, end);
    }

    #[test]
    fn test_time_range_json_serialization() {
        let time_range = TimeRange::Days { value: 30 };
        let json = serde_json::to_string(&time_range).unwrap();

        // Should serialize to tagged format
        assert!(json.contains(r#""type":"Days""#));
        assert!(json.contains(r#""value":30"#));

        // Test deserialization
        let deserialized: TimeRange = serde_json::from_str(&json).unwrap();
        match deserialized {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_time_range_custom_json_serialization() {
        let start = 1640995200;
        let end = 1643673600;
        let time_range = TimeRange::Custom { start, end };
        let json = serde_json::to_string(&time_range).unwrap();

        // Should serialize to tagged format
        assert!(json.contains(r#""type":"Custom""#));
        assert!(json.contains(&format!(r#""start":{}"#, start)));
        assert!(json.contains(&format!(r#""end":{}"#, end)));

        // Test deserialization
        let deserialized: TimeRange = serde_json::from_str(&json).unwrap();
        match deserialized {
            TimeRange::Custom {
                start: d_start,
                end: d_end,
            } => {
                assert_eq!(d_start, start);
                assert_eq!(d_end, end);
            }
            _ => panic!("Expected Custom variant"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub id: Option<i64>,
    pub link_id: String,
    pub org_id: String,
    pub timestamp: i64,
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyClicks {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReferrerCount {
    pub referrer: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountryCount {
    pub country: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserAgentCount {
    pub user_agent: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct LinkAnalyticsResponse {
    pub link: Link,
    pub total_clicks_in_range: i64,
    pub clicks_over_time: Vec<DailyClicks>,
    pub top_referrers: Vec<ReferrerCount>,
    pub top_countries: Vec<CountryCount>,
    pub top_user_agents: Vec<UserAgentCount>,
    /// Whether analytics data is gated due to tier limits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analytics_gated: Option<bool>,
    /// Reason analytics are gated (e.g., "click_limit_exceeded", "retention_limited")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gated_reason: Option<String>,
}
