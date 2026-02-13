use crate::models::Link;
use serde::{Deserialize, Serialize};

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
}
