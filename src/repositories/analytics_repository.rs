/// Analytics Repository
///
/// Data access layer for analytics queries (link-level and org-level).
use crate::models::analytics::{
    CountryCount, DailyClicks, ReferrerCount, TopLinkCount, UserAgentCount,
};
use worker::Result;
use worker::d1::D1Database;

pub struct AnalyticsRepository;

impl AnalyticsRepository {
    pub fn new() -> Self {
        Self
    }

    // ── Link-level queries ───────────────────────────────────────────────────

    /// Get total click count for a link within a time range
    pub async fn get_link_total_clicks_in_range(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
        start: i64,
        end: i64,
    ) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT COUNT(*) as count
             FROM analytics_events
             WHERE link_id = ?1 AND org_id = ?2 AND timestamp >= ?3 AND timestamp <= ?4",
        );

        let result = stmt
            .bind(&[
                link_id.into(),
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
            ])?
            .first::<serde_json::Value>(None)
            .await?;

        match result {
            Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
            None => Ok(0),
        }
    }

    /// Get clicks over time for a link, grouped by day
    pub async fn get_link_clicks_over_time(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<DailyClicks>> {
        let stmt = db.prepare(
            "SELECT date(timestamp, 'unixepoch') as date, COUNT(*) as count
             FROM analytics_events
             WHERE link_id = ?1 AND org_id = ?2 AND timestamp >= ?3 AND timestamp <= ?4
             GROUP BY date
             ORDER BY date ASC",
        );

        let results = stmt
            .bind(&[
                link_id.into(),
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
            ])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let clicks = rows
            .iter()
            .filter_map(|row| {
                let date = row["date"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(DailyClicks { date, count })
            })
            .collect();

        Ok(clicks)
    }

    /// Get top referrers for a link
    pub async fn get_link_top_referrers(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
        start: i64,
        end: i64,
        limit: i64,
    ) -> Result<Vec<ReferrerCount>> {
        let stmt = db.prepare(
            "SELECT COALESCE(referrer, 'Direct / Unknown') as referrer, COUNT(*) as count
             FROM analytics_events
             WHERE link_id = ?1 AND org_id = ?2 AND timestamp >= ?3 AND timestamp <= ?4
             GROUP BY referrer
             ORDER BY count DESC
             LIMIT ?5",
        );

        let results = stmt
            .bind(&[
                link_id.into(),
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
                (limit as f64).into(),
            ])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let referrers = rows
            .iter()
            .filter_map(|row| {
                let referrer = row["referrer"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(ReferrerCount { referrer, count })
            })
            .collect();

        Ok(referrers)
    }

    /// Get top countries for a link
    pub async fn get_link_top_countries(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
        start: i64,
        end: i64,
        limit: i64,
    ) -> Result<Vec<CountryCount>> {
        let stmt = db.prepare(
            "SELECT COALESCE(country, 'Unknown') as country, COUNT(*) as count
             FROM analytics_events
             WHERE link_id = ?1 AND org_id = ?2 AND timestamp >= ?3 AND timestamp <= ?4
             GROUP BY country
             ORDER BY count DESC
             LIMIT ?5",
        );

        let results = stmt
            .bind(&[
                link_id.into(),
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
                (limit as f64).into(),
            ])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let countries = rows
            .iter()
            .filter_map(|row| {
                let country = row["country"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(CountryCount { country, count })
            })
            .collect();

        Ok(countries)
    }

    /// Get top user agents for a link (raw strings, parsed client-side)
    pub async fn get_link_top_user_agents(
        &self,
        db: &D1Database,
        link_id: &str,
        org_id: &str,
        start: i64,
        end: i64,
        limit: i64,
    ) -> Result<Vec<UserAgentCount>> {
        let stmt = db.prepare(
            "SELECT COALESCE(user_agent, 'Unknown') as user_agent, COUNT(*) as count
             FROM analytics_events
             WHERE link_id = ?1 AND org_id = ?2 AND timestamp >= ?3 AND timestamp <= ?4
             GROUP BY user_agent
             ORDER BY count DESC
             LIMIT ?5",
        );

        let results = stmt
            .bind(&[
                link_id.into(),
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
                (limit as f64).into(),
            ])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let agents = rows
            .iter()
            .filter_map(|row| {
                let user_agent = row["user_agent"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(UserAgentCount { user_agent, count })
            })
            .collect();

        Ok(agents)
    }

    // ── Org-level queries ────────────────────────────────────────────────────

    /// Get total click count for an org within a time range
    pub async fn get_org_total_clicks_in_range(
        &self,
        db: &D1Database,
        org_id: &str,
        start: i64,
        end: i64,
    ) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT COUNT(*) as count
             FROM analytics_events
             WHERE org_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3",
        );

        let result = stmt
            .bind(&[org_id.into(), (start as f64).into(), (end as f64).into()])?
            .first::<serde_json::Value>(None)
            .await?;

        match result {
            Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
            None => Ok(0),
        }
    }

    /// Get unique link count clicked in an org within a time range
    pub async fn get_org_unique_links_clicked(
        &self,
        db: &D1Database,
        org_id: &str,
        start: i64,
        end: i64,
    ) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT COUNT(DISTINCT link_id) as count
             FROM analytics_events
             WHERE org_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3",
        );

        let result = stmt
            .bind(&[org_id.into(), (start as f64).into(), (end as f64).into()])?
            .first::<serde_json::Value>(None)
            .await?;

        match result {
            Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
            None => Ok(0),
        }
    }

    /// Get clicks over time for an org, grouped by day
    pub async fn get_org_clicks_over_time(
        &self,
        db: &D1Database,
        org_id: &str,
        start: i64,
        end: i64,
    ) -> Result<Vec<DailyClicks>> {
        let stmt = db.prepare(
            "SELECT date(timestamp, 'unixepoch') as date, COUNT(*) as count
             FROM analytics_events
             WHERE org_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3
             GROUP BY date
             ORDER BY date ASC",
        );

        let results = stmt
            .bind(&[org_id.into(), (start as f64).into(), (end as f64).into()])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let clicks = rows
            .iter()
            .filter_map(|row| {
                let date = row["date"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(DailyClicks { date, count })
            })
            .collect();

        Ok(clicks)
    }

    /// Get top links by click count in an org within a time range
    pub async fn get_org_top_links(
        &self,
        db: &D1Database,
        org_id: &str,
        start: i64,
        end: i64,
        limit: i64,
    ) -> Result<Vec<TopLinkCount>> {
        let stmt = db.prepare(
            "SELECT ae.link_id, l.short_code, l.title, COUNT(*) as count
             FROM analytics_events ae
             JOIN links l ON ae.link_id = l.id
             WHERE ae.org_id = ?1 AND ae.timestamp >= ?2 AND ae.timestamp <= ?3
             GROUP BY ae.link_id
             ORDER BY count DESC
             LIMIT ?4",
        );

        let results = stmt
            .bind(&[
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
                (limit as f64).into(),
            ])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let links = rows
            .iter()
            .filter_map(|row| {
                let link_id = row["link_id"].as_str()?.to_string();
                let short_code = row["short_code"].as_str()?.to_string();
                let title = row["title"].as_str().map(|s| s.to_string());
                let count = row["count"].as_f64()? as i64;
                Some(TopLinkCount {
                    link_id,
                    short_code,
                    title,
                    count,
                })
            })
            .collect();

        Ok(links)
    }

    /// Get top referrers for an org
    pub async fn get_org_top_referrers(
        &self,
        db: &D1Database,
        org_id: &str,
        start: i64,
        end: i64,
        limit: i64,
    ) -> Result<Vec<ReferrerCount>> {
        let stmt = db.prepare(
            "SELECT COALESCE(referrer, 'Direct / Unknown') as referrer, COUNT(*) as count
             FROM analytics_events
             WHERE org_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3
             GROUP BY referrer
             ORDER BY count DESC
             LIMIT ?4",
        );

        let results = stmt
            .bind(&[
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
                (limit as f64).into(),
            ])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let referrers = rows
            .iter()
            .filter_map(|row| {
                let referrer = row["referrer"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(ReferrerCount { referrer, count })
            })
            .collect();

        Ok(referrers)
    }

    /// Get top countries for an org
    pub async fn get_org_top_countries(
        &self,
        db: &D1Database,
        org_id: &str,
        start: i64,
        end: i64,
        limit: i64,
    ) -> Result<Vec<CountryCount>> {
        let stmt = db.prepare(
            "SELECT COALESCE(country, 'Unknown') as country, COUNT(*) as count
             FROM analytics_events
             WHERE org_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3
             GROUP BY country
             ORDER BY count DESC
             LIMIT ?4",
        );

        let results = stmt
            .bind(&[
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
                (limit as f64).into(),
            ])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let countries = rows
            .iter()
            .filter_map(|row| {
                let country = row["country"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(CountryCount { country, count })
            })
            .collect();

        Ok(countries)
    }

    /// Get top user agents for an org (raw strings, parsed client-side)
    pub async fn get_org_top_user_agents(
        &self,
        db: &D1Database,
        org_id: &str,
        start: i64,
        end: i64,
        limit: i64,
    ) -> Result<Vec<UserAgentCount>> {
        let stmt = db.prepare(
            "SELECT COALESCE(user_agent, 'Unknown') as user_agent, COUNT(*) as count
             FROM analytics_events
             WHERE org_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3
             GROUP BY user_agent
             ORDER BY count DESC
             LIMIT ?4",
        );

        let results = stmt
            .bind(&[
                org_id.into(),
                (start as f64).into(),
                (end as f64).into(),
                (limit as f64).into(),
            ])?
            .all()
            .await?;

        let rows = results.results::<serde_json::Value>()?;
        let agents = rows
            .iter()
            .filter_map(|row| {
                let user_agent = row["user_agent"].as_str()?.to_string();
                let count = row["count"].as_f64()? as i64;
                Some(UserAgentCount { user_agent, count })
            })
            .collect();

        Ok(agents)
    }

    // ── Usage queries ────────────────────────────────────────────────────────

    /// Get monthly counter for billing account
    pub async fn get_monthly_counter_for_billing_account(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        year_month: &str,
    ) -> Result<i64> {
        let stmt = db.prepare(
            "SELECT links_created
             FROM monthly_counters
             WHERE billing_account_id = ?1 AND year_month = ?2",
        );

        let result = stmt
            .bind(&[billing_account_id.into(), year_month.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        match result {
            Some(val) => Ok(val["links_created"].as_f64().unwrap_or(0.0) as i64),
            None => Ok(0),
        }
    }
}

impl Default for AnalyticsRepository {
    fn default() -> Self {
        Self::new()
    }
}
