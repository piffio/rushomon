/// Analytics Service
///
/// Business logic for analytics gating and time range parsing.
/// Moved from api/analytics.rs to the services layer.
use crate::models::{Tier, TimeRange};

/// Analytics gating result
#[derive(Debug, Clone)]
pub struct AnalyticsGatingResult {
    pub gated: bool,
    pub reason: Option<String>,
    pub adjusted_start: i64,
    #[allow(dead_code)]
    pub original_start: i64,
}

/// Apply tier-based analytics retention gating
/// Returns adjusted start date and gating information
pub fn apply_analytics_gating(
    tier: Tier,
    start: i64,
    _end: i64,
    now: i64,
) -> AnalyticsGatingResult {
    let limits = tier.limits();
    let mut gated = false;
    let mut gated_reason: Option<String> = None;
    let mut adjusted_start = start;

    // Enforce analytics retention window — clamp start date
    if let Some(retention_days) = limits.analytics_retention_days {
        let retention_start = now - retention_days * 24 * 60 * 60;
        if start < retention_start {
            adjusted_start = retention_start;
            gated = true;
            gated_reason = Some("retention_limited".to_string());
        }
    }

    AnalyticsGatingResult {
        gated,
        reason: gated_reason,
        adjusted_start,
        original_start: start,
    }
}

/// Parse time range from query string
/// Supports both ?days=N and ?start=UNIX&end=UNIX formats
pub fn parse_time_range_from_query(query: &str) -> TimeRange {
    let now = crate::models::analytics::now_timestamp();
    parse_time_range_from_query_with_now(query, now)
}

/// Parse time range from query string with provided timestamp for testing
/// Supports both ?days=N and ?start=UNIX&end=UNIX formats
pub fn parse_time_range_from_query_with_now(query: &str, now: i64) -> TimeRange {
    // Try to parse days parameter first
    if let Some(days_str) = query
        .split('&')
        .find(|s| s.starts_with("days="))
        .and_then(|s| s.split('=').nth(1))
    {
        let days = days_str.parse::<i64>().unwrap_or(7);
        return TimeRange::Days { value: days };
    }

    // Fall back to custom start/end parameters
    let start_param = query
        .split('&')
        .find(|s| s.starts_with("start="))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| now - 7 * 24 * 60 * 60);
    let end_param = query
        .split('&')
        .find(|s| s.starts_with("end="))
        .and_then(|s| s.split('=').nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(now);

    TimeRange::Custom {
        start: start_param,
        end: end_param,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed timestamp for consistent testing
    const TEST_NOW: i64 = 1640995200; // 2022-01-01 00:00:00 UTC

    #[test]
    fn test_parse_time_range_days_parameter() {
        let time_range = parse_time_range_from_query_with_now("days=30", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }

        let time_range = parse_time_range_from_query_with_now("days=90&other=value", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 90),
            _ => panic!("Expected Days variant"),
        }

        let time_range = parse_time_range_from_query_with_now("days=0", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 0),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_parse_time_range_custom_parameters() {
        let start = 1640995200;
        let end = 1643673600;
        let time_range =
            parse_time_range_from_query_with_now(&format!("start={}&end={}", start, end), TEST_NOW);

        match time_range {
            TimeRange::Custom { start: s, end: e } => {
                assert_eq!(s, start);
                assert_eq!(e, end);
            }
            _ => panic!("Expected Custom variant"),
        }

        let time_range = parse_time_range_from_query_with_now(
            &format!("other=value&start={}&end={}&more=data", start, end),
            TEST_NOW,
        );

        match time_range {
            TimeRange::Custom { start: s, end: e } => {
                assert_eq!(s, start);
                assert_eq!(e, end);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_parse_time_range_missing_params_defaults() {
        let time_range = parse_time_range_from_query_with_now("", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(end, TEST_NOW);
                assert_eq!(start, TEST_NOW - 7 * 24 * 60 * 60);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_parse_time_range_invalid_days_fallback() {
        let time_range = parse_time_range_from_query_with_now("days=invalid", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 7),
            _ => panic!("Expected Days variant"),
        }

        let time_range = parse_time_range_from_query_with_now("days=30abc", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 7),
            _ => panic!("Expected Days variant"),
        }

        let time_range = parse_time_range_from_query_with_now("days=", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 7),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_parse_time_range_mixed_parameters_priority() {
        let time_range =
            parse_time_range_from_query_with_now("days=30&start=123&end=456", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant (days should take priority)"),
        }

        let time_range =
            parse_time_range_from_query_with_now("start=123&days=90&end=456", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 90),
            _ => panic!("Expected Days variant (days should take priority)"),
        }
    }

    #[test]
    fn test_parse_time_range_partial_custom_parameters() {
        let time_range = parse_time_range_from_query_with_now("start=1640995200", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, 1640995200);
                assert_eq!(end, TEST_NOW);
            }
            _ => panic!("Expected Custom variant"),
        }

        let time_range = parse_time_range_from_query_with_now("end=1643673600", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(end, 1643673600);
                assert_eq!(start, TEST_NOW - 7 * 24 * 60 * 60);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_parse_time_range_invalid_custom_parameters() {
        let time_range =
            parse_time_range_from_query_with_now("start=invalid&end=1643673600", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, TEST_NOW - 7 * 24 * 60 * 60);
                assert_eq!(end, 1643673600);
            }
            _ => panic!("Expected Custom variant"),
        }

        let time_range =
            parse_time_range_from_query_with_now("start=1640995200&end=invalid", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, 1640995200);
                assert_eq!(end, TEST_NOW);
            }
            _ => panic!("Expected Custom variant"),
        }

        let time_range =
            parse_time_range_from_query_with_now("start=invalid&end=invalid", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, TEST_NOW - 7 * 24 * 60 * 60);
                assert_eq!(end, TEST_NOW);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_parse_time_range_edge_cases() {
        let time_range = parse_time_range_from_query_with_now("days=-7", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, -7),
            _ => panic!("Expected Days variant"),
        }

        let time_range = parse_time_range_from_query_with_now("days=3650", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 3650),
            _ => panic!("Expected Days variant"),
        }

        let time_range = parse_time_range_from_query_with_now("start=0&end=0", TEST_NOW);
        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, 0);
                assert_eq!(end, 0);
            }
            _ => panic!("Expected Custom variant"),
        }

        let time_range = parse_time_range_from_query_with_now("&days=30&&&", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }

        let time_range = parse_time_range_from_query_with_now("days=30&other=value=test", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_parse_time_range_parameter_order_independence() {
        let queries = [
            "days=30",
            "days=30&other=value",
            "other=value&days=30",
            "days=30&start=123&end=456",
            "start=123&days=30&end=456",
            "start=123&end=456&days=30",
        ];

        for query in queries.iter() {
            let time_range = parse_time_range_from_query_with_now(query, TEST_NOW);
            match time_range {
                TimeRange::Days { value } => assert_eq!(value, 30),
                _ => panic!("Expected Days variant for query: {}", query),
            }
        }
    }

    #[test]
    fn test_parse_time_range_url_encoded_characters() {
        let time_range = parse_time_range_from_query_with_now("days=30%20", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 7),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_parse_time_range_multiple_same_parameters() {
        let time_range = parse_time_range_from_query_with_now("days=30&days=90", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }

        let time_range =
            parse_time_range_from_query_with_now("start=123&start=456&end=789", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, 123);
                assert_eq!(end, 789);
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    // Tier-based gating logic tests
    #[test]
    fn test_analytics_gating_free_tier() {
        let now = TEST_NOW;
        let start_8_days_ago = now - 8 * 24 * 60 * 60;
        let start_6_days_ago = now - 6 * 24 * 60 * 60;

        let result = apply_analytics_gating(Tier::Free, start_8_days_ago, now, now);

        assert!(result.gated);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert_eq!(result.adjusted_start, now - 7 * 24 * 60 * 60);
        assert_eq!(result.original_start, start_8_days_ago);

        let result = apply_analytics_gating(Tier::Free, start_6_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, start_6_days_ago);
        assert_eq!(result.original_start, start_6_days_ago);
    }

    #[test]
    fn test_analytics_gating_pro_tier() {
        let now = TEST_NOW;
        let start_400_days_ago = now - 400 * 24 * 60 * 60;
        let start_300_days_ago = now - 300 * 24 * 60 * 60;

        let result = apply_analytics_gating(Tier::Pro, start_400_days_ago, now, now);

        assert!(result.gated);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert_eq!(result.adjusted_start, now - 365 * 24 * 60 * 60);
        assert_eq!(result.original_start, start_400_days_ago);

        let result = apply_analytics_gating(Tier::Pro, start_300_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, start_300_days_ago);
        assert_eq!(result.original_start, start_300_days_ago);
    }

    #[test]
    fn test_analytics_gating_business_tier() {
        let now = TEST_NOW;
        let start_1000_days_ago = now - 1000 * 24 * 60 * 60;

        let result = apply_analytics_gating(Tier::Business, start_1000_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, start_1000_days_ago);
        assert_eq!(result.original_start, start_1000_days_ago);
    }

    #[test]
    fn test_analytics_gating_unlimited_tier() {
        let now = TEST_NOW;
        let start_5000_days_ago = now - 5000 * 24 * 60 * 60;

        let result = apply_analytics_gating(Tier::Unlimited, start_5000_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, start_5000_days_ago);
        assert_eq!(result.original_start, start_5000_days_ago);
    }

    #[test]
    fn test_analytics_gating_edge_cases() {
        let now = TEST_NOW;

        let exactly_7_days_ago = now - 7 * 24 * 60 * 60;
        let result = apply_analytics_gating(Tier::Free, exactly_7_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, exactly_7_days_ago);

        let exactly_365_days_ago = now - 365 * 24 * 60 * 60;
        let result = apply_analytics_gating(Tier::Pro, exactly_365_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, exactly_365_days_ago);

        let future_start = now + 24 * 60 * 60;
        let result = apply_analytics_gating(Tier::Free, future_start, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, future_start);

        let result = apply_analytics_gating(Tier::Free, now, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, now);
    }

    #[test]
    fn test_analytics_gating_zero_and_negative_ranges() {
        let now = TEST_NOW;

        let result = apply_analytics_gating(Tier::Free, 0, now, now);

        assert!(result.gated);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert_eq!(result.adjusted_start, now - 7 * 24 * 60 * 60);
        assert_eq!(result.original_start, 0);

        let result = apply_analytics_gating(Tier::Free, -86400, now, now);

        assert!(result.gated);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert_eq!(result.adjusted_start, now - 7 * 24 * 60 * 60);
        assert_eq!(result.original_start, -86400);
    }

    #[test]
    fn test_analytics_gating_result_structure() {
        let now = TEST_NOW;
        let start = now - 10 * 24 * 60 * 60;

        let result = apply_analytics_gating(Tier::Free, start, now, now);

        assert_eq!(result.gated, true);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert!(result.adjusted_start > result.original_start);
        assert_eq!(result.original_start, start);

        let cloned = result.clone();
        assert_eq!(cloned.gated, result.gated);
        assert_eq!(cloned.reason, result.reason);
        assert_eq!(cloned.adjusted_start, result.adjusted_start);
        assert_eq!(cloned.original_start, result.original_start);
    }

    #[test]
    fn test_analytics_gating_consistency_across_tiers() {
        let now = TEST_NOW;
        let start = now - 10 * 24 * 60 * 60;

        let business_result = apply_analytics_gating(Tier::Business, start, now, now);
        let unlimited_result = apply_analytics_gating(Tier::Unlimited, start, now, now);

        assert!(!business_result.gated);
        assert!(!unlimited_result.gated);
        assert_eq!(business_result.reason, None);
        assert_eq!(unlimited_result.reason, None);
        assert_eq!(business_result.adjusted_start, start);
        assert_eq!(unlimited_result.adjusted_start, start);

        let free_result = apply_analytics_gating(Tier::Free, start, now, now);
        let pro_result = apply_analytics_gating(Tier::Pro, start, now, now);

        assert!(free_result.gated);
        assert!(!pro_result.gated);
    }
}
