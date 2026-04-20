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

/// Get usage information for an organization.
///
/// Returns tier, limits, current monthly usage, tag count, and next reset time.
#[allow(dead_code)]
pub async fn get_usage(
    db: &worker::d1::D1Database,
    org_id: &str,
) -> Result<UsageInfo, crate::utils::AppError> {
    use crate::models::Tier;
    use crate::repositories::{
        AnalyticsRepository, BillingRepository, OrgRepository, TagRepository,
    };
    use chrono::{Datelike, TimeZone};

    let org_repo = OrgRepository::new();
    let billing_repo = BillingRepository::new();

    let org = org_repo
        .get_by_id(db, org_id)
        .await?
        .ok_or_else(|| crate::utils::AppError::NotFound("Organization not found".to_string()))?;

    // Get billing account for usage tracking
    let billing_account_id = org.billing_account_id.as_ref().ok_or_else(|| {
        crate::utils::AppError::Internal("Organization has no billing account".to_string())
    })?;
    let billing_account = billing_repo
        .get_by_id(db, billing_account_id)
        .await?
        .ok_or_else(|| crate::utils::AppError::NotFound("Billing account not found".to_string()))?;

    let tier = Tier::from_str_value(&billing_account.tier).unwrap_or(Tier::Free);
    let limits = tier.limits();

    // Use billing account monthly counter for efficiency
    let now = chrono::Utc::now();
    let year_month = format!("{}-{:02}", now.year(), now.month());
    let analytics_repo = AnalyticsRepository::new();
    let links_created_this_month = analytics_repo
        .get_monthly_counter_for_billing_account(db, &billing_account.id, &year_month)
        .await?;

    // Get tag count for the billing account
    let tags_count = TagRepository::new()
        .count_distinct_tags_for_billing_account(db, &billing_account.id)
        .await?;

    // Calculate next reset time (first day of next month at midnight UTC)
    let now = chrono::Utc::now();
    let next_reset = chrono::Utc
        .with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
        .single()
        .unwrap_or_else(chrono::Utc::now);
    let next_reset_timestamp = next_reset.timestamp();

    Ok(UsageInfo {
        tier: tier.as_str().to_string(),
        limits,
        links_created_this_month,
        tags_count,
        next_reset_utc: next_reset.to_rfc3339(),
        next_reset_timestamp,
    })
}

/// Usage information for an organization.
#[derive(Debug)]
#[allow(dead_code)]
pub struct UsageInfo {
    pub tier: String,
    pub limits: crate::models::tier::TierLimits,
    pub links_created_this_month: i64,
    pub tags_count: i64,
    pub next_reset_utc: String,
    pub next_reset_timestamp: i64,
}

/// Get link-level analytics.
///
/// Returns click analytics for a single link with tier-based gating applied.
#[allow(dead_code)]
pub async fn get_link_analytics(
    db: &worker::d1::D1Database,
    link_id: &str,
    org_id: &str,
    time_range: crate::models::TimeRange,
) -> Result<LinkAnalyticsResult, crate::utils::AppError> {
    use crate::models::Tier;
    use crate::repositories::{
        AnalyticsRepository, BillingRepository, LinkRepository, OrgRepository,
    };

    let link_repo = LinkRepository::new();
    let org_repo = OrgRepository::new();
    let billing_repo = BillingRepository::new();
    let analytics_repo = AnalyticsRepository::new();

    // Verify link exists and belongs to org
    let link = link_repo
        .get_by_id(db, link_id, org_id)
        .await?
        .ok_or_else(|| crate::utils::AppError::NotFound("Link not found".to_string()))?;

    // Get tier for gating
    let org = org_repo
        .get_by_id(db, org_id)
        .await?
        .ok_or_else(|| crate::utils::AppError::NotFound("Organization not found".to_string()))?;

    let tier = if let Some(ref billing_account_id) = org.billing_account_id {
        billing_repo
            .get_by_id(db, billing_account_id)
            .await?
            .map(|ba| Tier::from_str_value(&ba.tier).unwrap_or(Tier::Free))
            .unwrap_or(Tier::Free)
    } else {
        Tier::Free
    };

    // Apply tier-based gating
    let (mut start, end) = time_range.calculate_timestamps();
    let now = crate::models::analytics::now_timestamp();
    let gating_result = apply_analytics_gating(tier, start, end, now);
    start = gating_result.adjusted_start;

    // If gated, return empty data
    if gating_result.gated {
        return Ok(LinkAnalyticsResult {
            link,
            total_clicks: 0,
            clicks_over_time: vec![],
            referrers: vec![],
            countries: vec![],
            user_agents: vec![],
            gated: true,
            gated_reason: gating_result.reason,
        });
    }

    // Fetch analytics data
    let total_clicks = analytics_repo
        .get_link_total_clicks_in_range(db, link_id, org_id, start, end)
        .await?;

    let clicks_over_time = analytics_repo
        .get_link_clicks_over_time(db, link_id, org_id, start, end)
        .await?;

    let referrers = analytics_repo
        .get_link_top_referrers(db, link_id, org_id, start, end, 10)
        .await?;

    let countries = analytics_repo
        .get_link_top_countries(db, link_id, org_id, start, end, 10)
        .await?;

    let user_agents = analytics_repo
        .get_link_top_user_agents(db, link_id, org_id, start, end, 20)
        .await?;

    Ok(LinkAnalyticsResult {
        link,
        total_clicks,
        clicks_over_time,
        referrers,
        countries,
        user_agents,
        gated: false,
        gated_reason: None,
    })
}

/// Link analytics result.
#[derive(Debug)]
#[allow(dead_code)]
pub struct LinkAnalyticsResult {
    pub link: crate::models::Link,
    pub total_clicks: i64,
    pub clicks_over_time: Vec<crate::models::analytics::DailyClicks>,
    pub referrers: Vec<crate::models::analytics::ReferrerCount>,
    pub countries: Vec<crate::models::analytics::CountryCount>,
    pub user_agents: Vec<crate::models::analytics::UserAgentCount>,
    pub gated: bool,
    pub gated_reason: Option<String>,
}

/// Get organization-level analytics.
///
/// Returns aggregate click analytics for the entire organization with tier-based gating.
#[allow(dead_code)]
pub async fn get_org_analytics(
    db: &worker::d1::D1Database,
    org_id: &str,
    time_range: crate::models::TimeRange,
) -> Result<OrgAnalyticsResult, crate::utils::AppError> {
    use crate::models::Tier;
    use crate::repositories::{AnalyticsRepository, BillingRepository, OrgRepository};

    let org_repo = OrgRepository::new();
    let billing_repo = BillingRepository::new();
    let analytics_repo = AnalyticsRepository::new();

    // Resolve tier from billing account
    let org = org_repo
        .get_by_id(db, org_id)
        .await?
        .ok_or_else(|| crate::utils::AppError::NotFound("Organization not found".to_string()))?;

    let tier = if let Some(ref billing_account_id) = org.billing_account_id {
        billing_repo
            .get_by_id(db, billing_account_id)
            .await?
            .map(|ba| Tier::from_str_value(&ba.tier).unwrap_or(Tier::Free))
            .unwrap_or(Tier::Free)
    } else {
        Tier::Free
    };

    // Apply tier-based gating
    let (mut start, end) = time_range.calculate_timestamps();
    let now = crate::models::analytics::now_timestamp();
    let gating_result = apply_analytics_gating(tier, start, end, now);
    start = gating_result.adjusted_start;

    // Fetch org-level analytics
    let total_clicks = analytics_repo
        .get_org_total_clicks_in_range(db, org_id, start, end)
        .await?;

    let unique_links = analytics_repo
        .get_org_unique_links_clicked(db, org_id, start, end)
        .await?;

    let clicks_over_time = analytics_repo
        .get_org_clicks_over_time(db, org_id, start, end)
        .await?;

    let top_links = analytics_repo
        .get_org_top_links(db, org_id, start, end, 10)
        .await?;

    let referrers = analytics_repo
        .get_org_top_referrers(db, org_id, start, end, 10)
        .await?;

    let countries = analytics_repo
        .get_org_top_countries(db, org_id, start, end, 10)
        .await?;

    let user_agents = analytics_repo
        .get_org_top_user_agents(db, org_id, start, end, 20)
        .await?;

    Ok(OrgAnalyticsResult {
        total_clicks,
        unique_links,
        clicks_over_time,
        top_links,
        referrers,
        countries,
        user_agents,
        gated: gating_result.gated,
        gated_reason: gating_result.reason,
    })
}

/// Organization analytics result.
#[derive(Debug)]
#[allow(dead_code)]
pub struct OrgAnalyticsResult {
    pub total_clicks: i64,
    pub unique_links: i64,
    pub clicks_over_time: Vec<crate::models::analytics::DailyClicks>,
    pub top_links: Vec<crate::models::analytics::TopLinkCount>,
    pub referrers: Vec<crate::models::analytics::ReferrerCount>,
    pub countries: Vec<crate::models::analytics::CountryCount>,
    pub user_agents: Vec<crate::models::analytics::UserAgentCount>,
    pub gated: bool,
    pub gated_reason: Option<String>,
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
