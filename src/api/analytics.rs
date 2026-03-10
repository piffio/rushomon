use crate::auth;
use crate::db;
use crate::models::{Tier, TimeRange};
use worker::d1::D1Database;
use worker::*;

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

/// Handle getting org-level aggregate analytics: GET /api/analytics/org
pub async fn handle_get_org_analytics(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let url = req.url()?;
    let query = url.query().unwrap_or("");

    // Parse time range: ?days=N, ?start=UNIX&end=UNIX
    let time_range = parse_time_range_from_query(query);

    let (mut start, end) = time_range.calculate_timestamps();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Resolve tier from billing account
    let org = db::get_org_by_id(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    let tier = if let Some(ref billing_account_id) = org.billing_account_id {
        db::get_billing_account(&db, billing_account_id)
            .await?
            .and_then(|ba| Tier::from_str_value(&ba.tier))
            .unwrap_or(Tier::Free)
    } else {
        Tier::Free
    };

    let now = crate::models::analytics::now_timestamp();
    let gating_result = apply_analytics_gating(tier, start, end, now);

    // Use the adjusted start date for queries
    start = gating_result.adjusted_start;

    // Run queries sequentially (D1 limitation)
    let total_clicks = db::get_org_total_clicks_in_range(&db, org_id, start, end).await?;
    let unique_links_clicked = db::get_org_unique_links_clicked(&db, org_id, start, end).await?;
    let clicks_over_time = db::get_org_clicks_over_time(&db, org_id, start, end).await?;
    let top_links = db::get_org_top_links(&db, org_id, start, end, 10).await?;
    let top_referrers = db::get_org_top_referrers(&db, org_id, start, end, 10).await?;
    let top_countries = db::get_org_top_countries(&db, org_id, start, end, 10).await?;
    let top_user_agents = db::get_org_top_user_agents(&db, org_id, start, end, 20).await?;

    let response = crate::models::analytics::OrgAnalyticsResponse {
        total_clicks,
        unique_links_clicked,
        clicks_over_time,
        top_links,
        top_referrers,
        top_countries,
        top_user_agents,
        analytics_gated: if gating_result.gated {
            Some(true)
        } else {
            None
        },
        gated_reason: gating_result.reason,
    };

    Response::from_json(&response)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed timestamp for consistent testing
    const TEST_NOW: i64 = 1640995200; // 2022-01-01 00:00:00 UTC

    #[test]
    fn test_parse_time_range_days_parameter() {
        // Test valid days parameter
        let time_range = parse_time_range_from_query_with_now("days=30", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }

        // Test days parameter with other params
        let time_range = parse_time_range_from_query_with_now("days=90&other=value", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 90),
            _ => panic!("Expected Days variant"),
        }

        // Test days=0 (all time)
        let time_range = parse_time_range_from_query_with_now("days=0", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 0),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_parse_time_range_custom_parameters() {
        // Test valid custom range
        let start = 1640995200; // 2022-01-01
        let end = 1643673600; // 2022-02-01
        let time_range =
            parse_time_range_from_query_with_now(&format!("start={}&end={}", start, end), TEST_NOW);

        match time_range {
            TimeRange::Custom { start: s, end: e } => {
                assert_eq!(s, start);
                assert_eq!(e, end);
            }
            _ => panic!("Expected Custom variant"),
        }

        // Test custom range with other params
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
        // Test empty query string
        let time_range = parse_time_range_from_query_with_now("", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(end, TEST_NOW);
                assert_eq!(start, TEST_NOW - 7 * 24 * 60 * 60); // 7 days ago
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_parse_time_range_invalid_days_fallback() {
        // Test non-numeric days parameter
        let time_range = parse_time_range_from_query_with_now("days=invalid", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 7), // Should fallback to 7
            _ => panic!("Expected Days variant"),
        }

        // Test partially numeric days parameter
        let time_range = parse_time_range_from_query_with_now("days=30abc", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 7), // Should fallback to 7
            _ => panic!("Expected Days variant"),
        }

        // Test empty days parameter
        let time_range = parse_time_range_from_query_with_now("days=", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 7), // Should fallback to 7
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_parse_time_range_mixed_parameters_priority() {
        // Test that days parameter takes priority over custom range
        let time_range =
            parse_time_range_from_query_with_now("days=30&start=123&end=456", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant (days should take priority)"),
        }

        // Test that days parameter takes priority even in different order
        let time_range =
            parse_time_range_from_query_with_now("start=123&days=90&end=456", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 90),
            _ => panic!("Expected Days variant (days should take priority)"),
        }
    }

    #[test]
    fn test_parse_time_range_partial_custom_parameters() {
        // Test only start parameter (missing end)
        let time_range = parse_time_range_from_query_with_now("start=1640995200", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, 1640995200);
                assert_eq!(end, TEST_NOW); // Should default to now
            }
            _ => panic!("Expected Custom variant"),
        }

        // Test only end parameter (missing start)
        let time_range = parse_time_range_from_query_with_now("end=1643673600", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(end, 1643673600);
                assert_eq!(start, TEST_NOW - 7 * 24 * 60 * 60); // Should default to 7 days ago
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_parse_time_range_invalid_custom_parameters() {
        // Test invalid start parameter
        let time_range =
            parse_time_range_from_query_with_now("start=invalid&end=1643673600", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, TEST_NOW - 7 * 24 * 60 * 60); // Should default to 7 days ago
                assert_eq!(end, 1643673600);
            }
            _ => panic!("Expected Custom variant"),
        }

        // Test invalid end parameter
        let time_range =
            parse_time_range_from_query_with_now("start=1640995200&end=invalid", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, 1640995200);
                assert_eq!(end, TEST_NOW); // Should default to now
            }
            _ => panic!("Expected Custom variant"),
        }

        // Test both invalid parameters
        let time_range =
            parse_time_range_from_query_with_now("start=invalid&end=invalid", TEST_NOW);

        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, TEST_NOW - 7 * 24 * 60 * 60); // Should default to 7 days ago
                assert_eq!(end, TEST_NOW); // Should default to now
            }
            _ => panic!("Expected Custom variant"),
        }
    }

    #[test]
    fn test_parse_time_range_edge_cases() {
        // Test negative days
        let time_range = parse_time_range_from_query_with_now("days=-7", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, -7),
            _ => panic!("Expected Days variant"),
        }

        // Test very large days
        let time_range = parse_time_range_from_query_with_now("days=3650", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 3650),
            _ => panic!("Expected Days variant"),
        }

        // Test zero timestamps
        let time_range = parse_time_range_from_query_with_now("start=0&end=0", TEST_NOW);
        match time_range {
            TimeRange::Custom { start, end } => {
                assert_eq!(start, 0);
                assert_eq!(end, 0);
            }
            _ => panic!("Expected Custom variant"),
        }

        // Test malformed query string
        let time_range = parse_time_range_from_query_with_now("&days=30&&&", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }

        // Test query with equals signs in values (should still work)
        let time_range = parse_time_range_from_query_with_now("days=30&other=value=test", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_parse_time_range_parameter_order_independence() {
        // Test different parameter orders
        let queries = [
            "days=30",
            "days=30&other=value",
            "other=value&days=30",
            "days=30&start=123&end=456", // days should take priority
            "start=123&days=30&end=456", // days should take priority
            "start=123&end=456&days=30", // days should take priority
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
        // Test URL-encoded characters (though our simple parser doesn't decode them)
        let time_range = parse_time_range_from_query_with_now("days=30%20", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 7), // Should fallback due to %20
            _ => panic!("Expected Days variant"),
        }
    }

    #[test]
    fn test_parse_time_range_multiple_same_parameters() {
        // Test multiple days parameters (first one should be used)
        let time_range = parse_time_range_from_query_with_now("days=30&days=90", TEST_NOW);
        match time_range {
            TimeRange::Days { value } => assert_eq!(value, 30),
            _ => panic!("Expected Days variant"),
        }

        // Test multiple start parameters (first one should be used)
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
        let start_8_days_ago = now - 8 * 24 * 60 * 60; // 8 days ago
        let start_6_days_ago = now - 6 * 24 * 60 * 60; // 6 days ago

        // Test free tier (7-day retention)
        let result = apply_analytics_gating(Tier::Free, start_8_days_ago, now, now);

        assert!(result.gated);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert_eq!(result.adjusted_start, now - 7 * 24 * 60 * 60); // Clamped to 7 days ago
        assert_eq!(result.original_start, start_8_days_ago);

        // Test free tier with valid range (no gating)
        let result = apply_analytics_gating(Tier::Free, start_6_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, start_6_days_ago); // Unchanged
        assert_eq!(result.original_start, start_6_days_ago);
    }

    #[test]
    fn test_analytics_gating_pro_tier() {
        let now = TEST_NOW;
        let start_400_days_ago = now - 400 * 24 * 60 * 60; // 400 days ago
        let start_300_days_ago = now - 300 * 24 * 60 * 60; // 300 days ago

        // Test pro tier (365-day retention)
        let result = apply_analytics_gating(Tier::Pro, start_400_days_ago, now, now);

        assert!(result.gated);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert_eq!(result.adjusted_start, now - 365 * 24 * 60 * 60); // Clamped to 365 days ago
        assert_eq!(result.original_start, start_400_days_ago);

        // Test pro tier with valid range (no gating)
        let result = apply_analytics_gating(Tier::Pro, start_300_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, start_300_days_ago); // Unchanged
        assert_eq!(result.original_start, start_300_days_ago);
    }

    #[test]
    fn test_analytics_gating_business_tier() {
        let now = TEST_NOW;
        let start_1000_days_ago = now - 1000 * 24 * 60 * 60; // 1000 days ago

        // Test business tier (unlimited retention)
        let result = apply_analytics_gating(Tier::Business, start_1000_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, start_1000_days_ago); // Unchanged
        assert_eq!(result.original_start, start_1000_days_ago);
    }

    #[test]
    fn test_analytics_gating_unlimited_tier() {
        let now = TEST_NOW;
        let start_5000_days_ago = now - 5000 * 24 * 60 * 60; // 5000 days ago

        // Test unlimited tier (unlimited retention)
        let result = apply_analytics_gating(Tier::Unlimited, start_5000_days_ago, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, start_5000_days_ago); // Unchanged
        assert_eq!(result.original_start, start_5000_days_ago);
    }

    #[test]
    fn test_analytics_gating_edge_cases() {
        let now = TEST_NOW;

        // Test exactly at retention limit (free tier, 7 days)
        let exactly_7_days_ago = now - 7 * 24 * 60 * 60;
        let result = apply_analytics_gating(Tier::Free, exactly_7_days_ago, now, now);

        assert!(!result.gated); // Should not be gated at exactly the limit
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, exactly_7_days_ago);

        // Test exactly at retention limit (pro tier, 365 days)
        let exactly_365_days_ago = now - 365 * 24 * 60 * 60;
        let result = apply_analytics_gating(Tier::Pro, exactly_365_days_ago, now, now);

        assert!(!result.gated); // Should not be gated at exactly the limit
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, exactly_365_days_ago);

        // Test start date in the future (should not be gated)
        let future_start = now + 24 * 60 * 60; // 1 day in future
        let result = apply_analytics_gating(Tier::Free, future_start, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, future_start);

        // Test start date equals now (should not be gated)
        let result = apply_analytics_gating(Tier::Free, now, now, now);

        assert!(!result.gated);
        assert_eq!(result.reason, None);
        assert_eq!(result.adjusted_start, now);
    }

    #[test]
    fn test_analytics_gating_zero_and_negative_ranges() {
        let now = TEST_NOW;

        // Test start = 0 (epoch) with free tier
        let result = apply_analytics_gating(Tier::Free, 0, now, now);

        assert!(result.gated);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert_eq!(result.adjusted_start, now - 7 * 24 * 60 * 60);
        assert_eq!(result.original_start, 0);

        // Test negative start (before epoch) with free tier
        let result = apply_analytics_gating(Tier::Free, -86400, now, now); // 1 day before epoch

        assert!(result.gated);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert_eq!(result.adjusted_start, now - 7 * 24 * 60 * 60);
        assert_eq!(result.original_start, -86400);
    }

    #[test]
    fn test_analytics_gating_result_structure() {
        let now = TEST_NOW;
        let start = now - 10 * 24 * 60 * 60; // 10 days ago

        let result = apply_analytics_gating(Tier::Free, start, now, now);

        // Test that all fields are properly set
        assert_eq!(result.gated, true);
        assert_eq!(result.reason, Some("retention_limited".to_string()));
        assert!(result.adjusted_start > result.original_start); // Should be clamped forward
        assert_eq!(result.original_start, start);

        // Test cloning works
        let cloned = result.clone();
        assert_eq!(cloned.gated, result.gated);
        assert_eq!(cloned.reason, result.reason);
        assert_eq!(cloned.adjusted_start, result.adjusted_start);
        assert_eq!(cloned.original_start, result.original_start);
    }

    #[test]
    fn test_analytics_gating_consistency_across_tiers() {
        let now = TEST_NOW;
        let start = now - 10 * 24 * 60 * 60; // 10 days ago

        // Test that unlimited tiers never gate
        let business_result = apply_analytics_gating(Tier::Business, start, now, now);
        let unlimited_result = apply_analytics_gating(Tier::Unlimited, start, now, now);

        assert!(!business_result.gated);
        assert!(!unlimited_result.gated);
        assert_eq!(business_result.reason, None);
        assert_eq!(unlimited_result.reason, None);
        assert_eq!(business_result.adjusted_start, start);
        assert_eq!(unlimited_result.adjusted_start, start);

        // Test that limited tiers gate appropriately
        let free_result = apply_analytics_gating(Tier::Free, start, now, now);
        let pro_result = apply_analytics_gating(Tier::Pro, start, now, now);

        assert!(free_result.gated); // 10 days > 7-day limit
        assert!(!pro_result.gated); // 10 days < 365-day limit
    }
}
