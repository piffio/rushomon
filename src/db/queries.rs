use crate::models::{AnalyticsEvent, Link, Organization, User, user::CreateUserData};
use crate::utils::now_timestamp;
use wasm_bindgen::JsValue;
use worker::d1::D1Database;
use worker::*;

/// Get the total number of users on the instance
pub async fn get_user_count(db: &D1Database) -> Result<i64> {
    let stmt = db.prepare("SELECT COUNT(*) as count FROM users");
    let result = stmt.first::<serde_json::Value>(None).await?;

    match result {
        Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
        None => Ok(0),
    }
}

/// Create or update a user from OAuth data
pub async fn create_or_update_user(
    db: &D1Database,
    data: CreateUserData,
    org_id: &str,
) -> Result<User> {
    let user_id = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();

    // Try to find existing user by OAuth provider and ID
    let stmt = db.prepare(
        "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at
         FROM users
         WHERE oauth_provider = ?1 AND oauth_id = ?2",
    );

    let existing = stmt
        .bind(&[
            data.oauth_provider.clone().into(),
            data.oauth_id.clone().into(),
        ])?
        .first::<User>(None)
        .await?;

    if let Some(user) = existing {
        // Update existing user
        let update_stmt = db.prepare(
            "UPDATE users
             SET email = ?1, name = ?2, avatar_url = ?3
             WHERE id = ?4",
        );

        update_stmt
            .bind(&[
                data.email.clone().into(),
                data.name.clone().into(),
                data.avatar_url.clone().into(),
                user.id.clone().into(),
            ])?
            .run()
            .await?;

        Ok(User {
            id: user.id,
            email: data.email,
            name: data.name,
            avatar_url: data.avatar_url,
            oauth_provider: data.oauth_provider,
            oauth_id: data.oauth_id,
            org_id: user.org_id,
            role: user.role,
            created_at: user.created_at,
            suspended_at: user.suspended_at,
            suspension_reason: user.suspension_reason,
            suspended_by: user.suspended_by,
        })
    } else {
        // Determine role: first user on the instance gets admin, all others get member
        let user_count = get_user_count(db).await?;
        let role = if user_count == 0 { "admin" } else { "member" };

        // Create new user
        let insert_stmt = db.prepare(
            "INSERT INTO users (id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        );

        insert_stmt
            .bind(&[
                user_id.clone().into(),
                data.email.clone().into(),
                data.name.clone().into(),
                data.avatar_url.clone().into(),
                data.oauth_provider.clone().into(),
                data.oauth_id.clone().into(),
                org_id.into(),
                role.into(),
                (now as f64).into(),
            ])?
            .run()
            .await?;

        Ok(User {
            id: user_id,
            email: data.email,
            name: data.name,
            avatar_url: data.avatar_url,
            oauth_provider: data.oauth_provider,
            oauth_id: data.oauth_id,
            org_id: org_id.to_string(),
            role: role.to_string(),
            created_at: now,
            suspended_at: None,
            suspension_reason: None,
            suspended_by: None,
        })
    }
}

/// Create a default organization for a new user
pub async fn create_default_org(
    db: &D1Database,
    user_id: &str,
    org_name: &str,
) -> Result<Organization> {
    let org_id = uuid::Uuid::new_v4().to_string();
    let slug = org_name
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    let now = now_timestamp();

    // Read the default tier from settings
    let tier = get_setting(db, "default_user_tier")
        .await?
        .unwrap_or_else(|| "free".to_string());

    let stmt = db.prepare(
        "INSERT INTO organizations (id, name, slug, created_at, created_by, tier)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    );

    stmt.bind(&[
        org_id.clone().into(),
        org_name.into(),
        slug.clone().into(),
        (now as f64).into(),
        user_id.into(),
        tier.clone().into(),
    ])?
    .run()
    .await?;

    Ok(Organization {
        id: org_id,
        name: org_name.to_string(),
        slug,
        created_at: now,
        created_by: user_id.to_string(),
        tier,
    })
}

/// Get user by ID
pub async fn get_user_by_id(db: &D1Database, user_id: &str) -> Result<Option<User>> {
    let stmt = db.prepare(
        "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at
         FROM users
         WHERE id = ?1",
    );

    stmt.bind(&[user_id.into()])?.first::<User>(None).await
}

/// Get organization by ID
pub async fn get_org_by_id(db: &D1Database, org_id: &str) -> Result<Option<Organization>> {
    let stmt = db.prepare(
        "SELECT id, name, slug, created_at, created_by, tier
         FROM organizations
         WHERE id = ?1",
    );

    stmt.bind(&[org_id.into()])?
        .first::<Organization>(None)
        .await
}

/// Create a new link
pub async fn create_link(db: &D1Database, link: &Link) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO links (id, org_id, short_code, destination_url, title, created_by, created_at, expires_at, status, click_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
    );

    stmt.bind(&[
        link.id.clone().into(),
        link.org_id.clone().into(),
        link.short_code.clone().into(),
        link.destination_url.clone().into(),
        link.title
            .clone()
            .map(|t| t.into())
            .unwrap_or(JsValue::NULL),
        link.created_by.clone().into(),
        (link.created_at as f64).into(),
        link.expires_at
            .map(|t| (t as f64).into())
            .unwrap_or(JsValue::NULL),
        link.status.as_str().into(),
        (link.click_count as f64).into(),
    ])?
    .run()
    .await?;

    Ok(())
}

/// Get total count of non-deleted links for an organization
pub async fn get_links_count_by_org(db: &D1Database, org_id: &str) -> Result<i64> {
    let stmt = db.prepare(
        "SELECT COUNT(*) as count FROM links
         WHERE org_id = ?1
         AND status IN ('active', 'disabled')",
    );

    let result = stmt
        .bind(&[org_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    match result {
        Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
        None => Ok(0),
    }
}

/// Dashboard statistics for an organization
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DashboardStats {
    pub total_links: i64,
    pub active_links: i64,
    pub total_clicks: i64,
}

/// Get dashboard statistics for an organization
pub async fn get_dashboard_stats(db: &D1Database, org_id: &str) -> Result<DashboardStats> {
    let stmt = db.prepare(
        "SELECT
            COUNT(*) as total_links,
            SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) as active_links,
            SUM(click_count) as total_clicks
         FROM links
         WHERE org_id = ?1
         AND status IN ('active', 'disabled')",
    );

    let result = stmt
        .bind(&[org_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    match result {
        Some(val) => Ok(DashboardStats {
            total_links: val["total_links"].as_f64().unwrap_or(0.0) as i64,
            active_links: val["active_links"].as_f64().unwrap_or(0.0) as i64,
            total_clicks: val["total_clicks"].as_f64().unwrap_or(0.0) as i64,
        }),
        None => Ok(DashboardStats {
            total_links: 0,
            active_links: 0,
            total_clicks: 0,
        }),
    }
}

/// Get links for an organization (paginated)
pub async fn get_links_by_org(
    db: &D1Database,
    org_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count
         FROM links
         WHERE org_id = ?1
         AND status IN ('active', 'disabled')
         ORDER BY created_at DESC
         LIMIT ?2 OFFSET ?3"
    );

    // For now, let's try a simple approach that works
    // We'll fix the multiple rows issue later
    let results = stmt
        .bind(&[org_id.into(), (limit as f64).into(), (offset as f64).into()])?
        .all()
        .await?;

    // Extract results from D1Result
    let links = results.results::<Link>()?;

    Ok(links)
}

/// Get a link by ID
pub async fn get_link_by_id(db: &D1Database, link_id: &str, org_id: &str) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count
         FROM links
         WHERE id = ?1
         AND org_id = ?2
         AND status IN ('active', 'disabled')"
    );

    stmt.bind(&[link_id.into(), org_id.into()])?
        .first::<Link>(None)
        .await
}

/// Get a link by ID without org_id check (used for public redirects)
pub async fn get_link_by_id_no_auth(db: &D1Database, link_id: &str) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count
         FROM links
         WHERE id = ?1
         AND status = 'active'"
    );

    stmt.bind(&[link_id.into()])?.first::<Link>(None).await
}

/// Get a link by ID without org_id check (used for admin operations - returns all statuses)
pub async fn get_link_by_id_no_auth_all(db: &D1Database, link_id: &str) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count
         FROM links
         WHERE id = ?1"
    );

    stmt.bind(&[link_id.into()])?.first::<Link>(None).await
}

/// Update a link
pub async fn update_link(
    db: &D1Database,
    link_id: &str,
    org_id: &str,
    destination_url: Option<&str>,
    title: Option<&str>,
    status: Option<&str>,
    expires_at: Option<i64>,
) -> Result<Link> {
    let now = now_timestamp();

    // Build dynamic update query
    let mut query = String::from("UPDATE links SET updated_at = ?1");
    let mut params: Vec<worker::wasm_bindgen::JsValue> = vec![(now as f64).into()];
    let mut param_count = 2;

    if let Some(url) = destination_url {
        query.push_str(&format!(", destination_url = ?{}", param_count));
        params.push(url.into());
        param_count += 1;
    }

    if let Some(t) = title {
        query.push_str(&format!(", title = ?{}", param_count));
        params.push(t.into());
        param_count += 1;
    }

    if let Some(s) = status {
        query.push_str(&format!(", status = ?{}", param_count));
        params.push(s.into());
        param_count += 1;
    }

    if expires_at.is_some() {
        query.push_str(&format!(", expires_at = ?{}", param_count));
        params.push(expires_at.map(|t| t as f64).into());
        param_count += 1;
    }

    query.push_str(&format!(
        " WHERE id = ?{} AND org_id = ?{}",
        param_count,
        param_count + 1
    ));
    params.push(link_id.into());
    params.push(org_id.into());

    let stmt = db.prepare(&query);
    stmt.bind(&params)?.run().await?;

    // Fetch and return the updated link
    get_link_by_id(db, link_id, org_id)
        .await?
        .ok_or_else(|| worker::Error::RustError("Link not found after update".to_string()))
}

/// Hard delete a link (removes from database permanently, frees up short code)
pub async fn hard_delete_link(db: &D1Database, link_id: &str, org_id: &str) -> Result<()> {
    // First delete analytics events for this link (FK constraint)
    let analytics_stmt = db.prepare("DELETE FROM analytics_events WHERE link_id = ?1");
    analytics_stmt.bind(&[link_id.into()])?.run().await?;

    // Then delete the link itself
    let stmt = db.prepare("DELETE FROM links WHERE id = ?1 AND org_id = ?2");
    stmt.bind(&[link_id.into(), org_id.into()])?.run().await?;

    Ok(())
}

/// Increment click count for a link
pub async fn increment_click_count(db: &D1Database, link_id: &str) -> Result<()> {
    let stmt = db.prepare("UPDATE links SET click_count = click_count + 1 WHERE id = ?1");

    stmt.bind(&[link_id.into()])?.run().await?;

    Ok(())
}

/// Log an analytics event
pub async fn log_analytics_event(db: &D1Database, event: &AnalyticsEvent) -> Result<()> {
    let stmt = db.prepare(
        "INSERT INTO analytics_events (link_id, org_id, timestamp, referrer, user_agent, country, city)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    );

    stmt.bind(&[
        event.link_id.clone().into(),
        event.org_id.clone().into(),
        (event.timestamp as f64).into(),
        event
            .referrer
            .clone()
            .map(|t| t.into())
            .unwrap_or(JsValue::NULL),
        event
            .user_agent
            .clone()
            .map(|t| t.into())
            .unwrap_or(JsValue::NULL),
        event
            .country
            .clone()
            .map(|t| t.into())
            .unwrap_or(JsValue::NULL),
        event
            .city
            .clone()
            .map(|t| t.into())
            .unwrap_or(JsValue::NULL),
    ])?
    .run()
    .await?;

    Ok(())
}

/// Get all users on the instance (paginated) - for admin dashboard
pub async fn get_all_users(db: &D1Database, limit: i64, offset: i64) -> Result<Vec<User>> {
    let stmt = db.prepare(
        "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at, suspended_at, suspension_reason, suspended_by
         FROM users
         ORDER BY created_at ASC
         LIMIT ?1 OFFSET ?2",
    );

    let results = stmt
        .bind(&[(limit as f64).into(), (offset as f64).into()])?
        .all()
        .await?;

    let users = results.results::<User>()?;
    Ok(users)
}

/// Get the count of users with admin role on the instance
pub async fn get_admin_count(db: &D1Database) -> Result<i64> {
    let stmt = db.prepare("SELECT COUNT(*) as count FROM users WHERE role = 'admin'");
    let result = stmt.first::<serde_json::Value>(None).await?;

    match result {
        Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
        None => Ok(0),
    }
}

/// Update a user's instance-level role
pub async fn update_user_role(db: &D1Database, user_id: &str, new_role: &str) -> Result<()> {
    let stmt = db.prepare("UPDATE users SET role = ?1 WHERE id = ?2");

    stmt.bind(&[new_role.into(), user_id.into()])?.run().await?;

    Ok(())
}

/// Get a setting value by key
pub async fn get_setting(db: &D1Database, key: &str) -> Result<Option<String>> {
    let stmt = db.prepare("SELECT value FROM settings WHERE key = ?1");
    let result = stmt
        .bind(&[key.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    match result {
        Some(val) => Ok(val["value"].as_str().map(|s| s.to_string())),
        None => Ok(None),
    }
}

/// Set a setting value (upsert)
pub async fn set_setting(db: &D1Database, key: &str, value: &str) -> Result<()> {
    let now = now_timestamp();
    let stmt = db.prepare(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
    );

    stmt.bind(&[key.into(), value.into(), (now as f64).into()])?
        .run()
        .await?;

    Ok(())
}

/// Get a link by short_code within an organization
pub async fn get_link_by_short_code(
    db: &D1Database,
    short_code: &str,
    org_id: &str,
) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count
         FROM links
         WHERE short_code = ?1
         AND org_id = ?2
         AND status IN ('active', 'disabled')"
    );

    stmt.bind(&[short_code.into(), org_id.into()])?
        .first::<Link>(None)
        .await
}

/// Get clicks over time for a link, grouped by day
pub async fn get_link_clicks_over_time(
    db: &D1Database,
    link_id: &str,
    org_id: &str,
    start: i64,
    end: i64,
) -> Result<Vec<crate::models::analytics::DailyClicks>> {
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
            Some(crate::models::analytics::DailyClicks { date, count })
        })
        .collect();

    Ok(clicks)
}

/// Get top referrers for a link
pub async fn get_link_top_referrers(
    db: &D1Database,
    link_id: &str,
    org_id: &str,
    start: i64,
    end: i64,
    limit: i64,
) -> Result<Vec<crate::models::analytics::ReferrerCount>> {
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
            Some(crate::models::analytics::ReferrerCount { referrer, count })
        })
        .collect();

    Ok(referrers)
}

/// Get top countries for a link
pub async fn get_link_top_countries(
    db: &D1Database,
    link_id: &str,
    org_id: &str,
    start: i64,
    end: i64,
    limit: i64,
) -> Result<Vec<crate::models::analytics::CountryCount>> {
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
            Some(crate::models::analytics::CountryCount { country, count })
        })
        .collect();

    Ok(countries)
}

/// Get top user agents for a link (raw strings, parsed client-side)
pub async fn get_link_top_user_agents(
    db: &D1Database,
    link_id: &str,
    org_id: &str,
    start: i64,
    end: i64,
    limit: i64,
) -> Result<Vec<crate::models::analytics::UserAgentCount>> {
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
            Some(crate::models::analytics::UserAgentCount { user_agent, count })
        })
        .collect();

    Ok(agents)
}

/// Get total click count from analytics_events for a link within a time range
pub async fn get_link_total_clicks_in_range(
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

/// Get all settings as a list of (key, value) pairs
pub async fn get_all_settings(db: &D1Database) -> Result<Vec<(String, String)>> {
    let stmt = db.prepare("SELECT key, value FROM settings ORDER BY key");
    let results = stmt.all().await?;

    let rows = results.results::<serde_json::Value>()?;
    let settings = rows
        .iter()
        .filter_map(|row| {
            let key = row["key"].as_str()?.to_string();
            let value = row["value"].as_str()?.to_string();
            Some((key, value))
        })
        .collect();

    Ok(settings)
}

/// Get the monthly counter for an organization
pub async fn get_monthly_counter(db: &D1Database, org_id: &str, year_month: &str) -> Result<i64> {
    let stmt = db.prepare(
        "SELECT links_created FROM monthly_counters
         WHERE org_id = ?1 AND year_month = ?2",
    );

    let result = stmt
        .bind(&[org_id.into(), year_month.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    match result {
        Some(val) => Ok(val["links_created"].as_f64().unwrap_or(0.0) as i64),
        None => Ok(0),
    }
}

/// Increment monthly counter with limit check
/// Returns true if increment succeeded, false if limit reached
pub async fn increment_monthly_counter(
    db: &D1Database,
    org_id: &str,
    year_month: &str,
    max_links: i64,
) -> Result<bool> {
    let now = now_timestamp();

    // First check current count
    let current_count = get_monthly_counter(db, org_id, year_month).await?;
    if current_count >= max_links {
        return Ok(false);
    }

    // Increment the counter
    let stmt = db.prepare(
        "INSERT INTO monthly_counters (org_id, year_month, links_created, updated_at)
         VALUES (?1, ?2, 1, ?3)
         ON CONFLICT(org_id, year_month) DO UPDATE SET
           links_created = links_created + 1,
           updated_at = ?3",
    );

    stmt.bind(&[org_id.into(), year_month.into(), (now as f64).into()])?
        .run()
        .await?;

    Ok(true)
}

/// Reset monthly counter for an organization to 0
pub async fn reset_monthly_counter(db: &D1Database, org_id: &str) -> Result<()> {
    let now = now_timestamp();

    // Get current year-month
    let year_month = chrono::Utc::now().format("%Y-%m").to_string();

    // Reset the counter to 0
    let stmt = db.prepare(
        "INSERT INTO monthly_counters (org_id, year_month, links_created, updated_at)
         VALUES (?1, ?2, 0, ?3)
         ON CONFLICT(org_id, year_month) DO UPDATE SET
           links_created = 0,
           updated_at = ?3",
    );

    stmt.bind(&[org_id.into(), year_month.into(), (now as f64).into()])?
        .run()
        .await?;

    Ok(())
}

/// Backfill monthly counters for existing organizations for the current month
#[allow(dead_code)]
pub async fn backfill_monthly_counters(_db: &D1Database) -> Result<i64> {
    // This function is reserved for future use to migrate existing data
    // For now, we rely on lazy creation in increment_monthly_counter
    Ok(0)
}

/// Get all organization tiers as a map of org_id -> tier (admin only)
pub async fn get_all_org_tiers(db: &D1Database) -> Result<Vec<(String, String)>> {
    let stmt = db.prepare("SELECT id, tier FROM organizations");
    let results = stmt.all().await?;
    let rows = results.results::<serde_json::Value>()?;

    let mut tiers = Vec::new();
    for row in rows {
        if let (Some(id), Some(tier)) = (row["id"].as_str(), row["tier"].as_str()) {
            tiers.push((id.to_string(), tier.to_string()));
        }
    }
    Ok(tiers)
}

/// Update an organization's tier (admin only)
pub async fn set_org_tier(db: &D1Database, org_id: &str, tier: &str) -> Result<()> {
    let stmt = db.prepare("UPDATE organizations SET tier = ?1 WHERE id = ?2");
    stmt.bind(&[tier.into(), org_id.into()])?.run().await?;
    Ok(())
}

/// Suspend a user
pub async fn suspend_user(
    db: &D1Database,
    user_id: &str,
    reason: &str,
    suspended_by: &str,
) -> Result<()> {
    let now = now_timestamp();
    let stmt = db.prepare(
        "UPDATE users SET suspended_at = ?1, suspension_reason = ?2, suspended_by = ?3 WHERE id = ?4"
    );
    stmt.bind(&[
        (now as f64).into(),
        reason.into(),
        suspended_by.into(),
        user_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Unsuspend a user
pub async fn unsuspend_user(db: &D1Database, user_id: &str) -> Result<()> {
    let stmt = db.prepare(
        "UPDATE users SET suspended_at = NULL, suspension_reason = NULL, suspended_by = NULL WHERE id = ?1"
    );
    stmt.bind(&[user_id.into()])?.run().await?;
    Ok(())
}

/// Disable all links for a user
pub async fn disable_all_links_for_user(db: &D1Database, user_id: &str) -> Result<i64> {
    let now = now_timestamp();
    let stmt = db.prepare(
        "UPDATE links SET status = 'disabled', updated_at = ?1 WHERE created_by = ?2 AND status = 'active'"
    );
    let result = stmt
        .bind(&[(now as f64).into(), user_id.into()])?
        .run()
        .await?;
    Ok(result
        .meta()?
        .and_then(|m| m.changes)
        .map(|c| c as i64)
        .unwrap_or(0))
}

/// Enable all disabled links for a user (when user is unsuspended)
pub async fn enable_all_links_for_user(db: &D1Database, user_id: &str) -> Result<i64> {
    let now = now_timestamp();
    let stmt = db.prepare(
        "UPDATE links SET status = 'active', updated_at = ?1 WHERE created_by = ?2 AND status = 'disabled'"
    );
    let result = stmt
        .bind(&[(now as f64).into(), user_id.into()])?
        .run()
        .await?;
    Ok(result
        .meta()?
        .and_then(|m| m.changes)
        .map(|c| c as i64)
        .unwrap_or(0))
}

/// Add destination to blacklist
pub async fn add_to_blacklist(
    db: &D1Database,
    destination: &str,
    match_type: &str,
    reason: &str,
    created_by: &str,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();
    let stmt = db.prepare(
        "INSERT INTO destination_blacklist (id, destination, match_type, reason, created_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    );
    stmt.bind(&[
        id.into(),
        destination.into(),
        match_type.into(),
        reason.into(),
        created_by.into(),
        (now as f64).into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Remove destination from blacklist
pub async fn remove_from_blacklist(db: &D1Database, id: &str) -> Result<()> {
    let stmt = db.prepare("DELETE FROM destination_blacklist WHERE id = ?1");
    stmt.bind(&[id.into()])?.run().await?;
    Ok(())
}

/// Get all blacklist entries
pub async fn get_all_blacklist(db: &D1Database) -> Result<Vec<BlacklistEntry>> {
    let stmt = db.prepare(
        "SELECT id, destination, match_type, reason, created_by, created_at
         FROM destination_blacklist
         ORDER BY created_at DESC",
    );
    let results = stmt.all().await?;
    let entries = results.results::<BlacklistEntry>()?;
    Ok(entries)
}

/// Check if a destination is blacklisted (exact or domain match)
pub async fn is_destination_blacklisted(db: &D1Database, destination: &str) -> Result<bool> {
    // First check exact match
    let exact_stmt = db.prepare(
        "SELECT 1 FROM destination_blacklist
         WHERE destination = ?1 AND match_type = 'exact'
         LIMIT 1",
    );
    if let Ok(Some(_)) = exact_stmt
        .bind(&[destination.into()])?
        .first::<serde_json::Value>(None)
        .await
    {
        return Ok(true);
    }

    // Then check domain match
    let url = match url::Url::parse(destination) {
        Ok(u) => u,
        Err(_) => return Ok(false),
    };

    let domain = url.host_str().unwrap_or("");
    let domain_stmt = db.prepare(
        "SELECT 1 FROM destination_blacklist
         WHERE ?1 LIKE '%' || destination || '%' AND match_type = 'domain'
         LIMIT 1",
    );
    if let Ok(Some(_)) = domain_stmt
        .bind(&[domain.into()])?
        .first::<serde_json::Value>(None)
        .await
    {
        return Ok(true);
    }

    Ok(false)
}

/// Block all links matching a blacklist target
pub async fn block_links_matching_destination(
    db: &D1Database,
    destination: &str,
    match_type: &str,
) -> Result<i64> {
    let now = now_timestamp();
    let stmt = if match_type == "exact" {
        db.prepare(
            "UPDATE links SET status = 'blocked', updated_at = ?1
             WHERE destination_url = ?2 AND status IN ('active', 'disabled')",
        )
    } else {
        // Domain match - use LIKE with wildcards
        db.prepare(
            "UPDATE links SET status = 'blocked', updated_at = ?1
             WHERE destination_url LIKE ?2 AND status IN ('active', 'disabled')",
        )
    };

    let pattern = if match_type == "exact" {
        destination.to_string()
    } else {
        format!("%{}%", destination)
    };

    let result = stmt
        .bind(&[(now as f64).into(), pattern.into()])?
        .run()
        .await?;
    Ok(result
        .meta()?
        .and_then(|m| m.changes)
        .map(|c| c as i64)
        .unwrap_or(0))
}

/// Get all links for admin (global listing with filters)
pub async fn get_all_links_admin(
    db: &D1Database,
    limit: i64,
    offset: i64,
    org_filter: Option<&str>,
    email_filter: Option<&str>,
    domain_filter: Option<&str>,
) -> Result<Vec<AdminLink>> {
    let mut query = String::from(
        "SELECT l.id, l.org_id, l.short_code, l.destination_url, l.title, l.created_by, l.created_at, l.updated_at, l.expires_at, l.status, l.click_count, u.email as creator_email, o.name as org_name
         FROM links l
         JOIN users u ON l.created_by = u.id
         JOIN organizations o ON l.org_id = o.id
         WHERE l.status IN ('active', 'disabled', 'blocked')"
    );

    let mut params: Vec<worker::wasm_bindgen::JsValue> = vec![];
    let mut param_count = 1;

    if let Some(org_id) = org_filter {
        query.push_str(&format!(" AND l.org_id = ?{}", param_count));
        params.push(org_id.into());
        param_count += 1;
    }

    if let Some(email) = email_filter {
        query.push_str(&format!(" AND u.email LIKE ?{}", param_count));
        params.push(format!("%{}%", email).into());
        param_count += 1;
    }

    if let Some(domain) = domain_filter {
        query.push_str(&format!(" AND l.destination_url LIKE ?{}", param_count));
        params.push(format!("%{}%", domain).into());
        param_count += 1;
    }

    query.push_str(&format!(
        " ORDER BY l.created_at DESC LIMIT ?{} OFFSET ?{}",
        param_count,
        param_count + 1
    ));
    params.push((limit as f64).into());
    params.push((offset as f64).into());

    let stmt = db.prepare(&query);
    let results = stmt.bind(&params)?.all().await?;
    let links = results.results::<AdminLink>()?;
    Ok(links)
}

/// Get total count of links for admin with filters
pub async fn get_all_links_admin_count(
    db: &D1Database,
    org_filter: Option<&str>,
    email_filter: Option<&str>,
    domain_filter: Option<&str>,
) -> Result<i64> {
    let mut query = String::from(
        "SELECT COUNT(*) as count FROM links l
         JOIN users u ON l.created_by = u.id
         WHERE l.status IN ('active', 'disabled', 'blocked')",
    );

    let mut params: Vec<worker::wasm_bindgen::JsValue> = vec![];
    let mut param_count = 1;

    if let Some(org_id) = org_filter {
        query.push_str(&format!(" AND l.org_id = ?{}", param_count));
        params.push(org_id.into());
        param_count += 1;
    }

    if let Some(email) = email_filter {
        query.push_str(&format!(" AND u.email LIKE ?{}", param_count));
        params.push(format!("%{}%", email).into());
        param_count += 1;
    }

    if let Some(domain) = domain_filter {
        query.push_str(&format!(" AND l.destination_url LIKE ?{}", param_count));
        params.push(format!("%{}%", domain).into());
    }

    let stmt = db.prepare(&query);
    let result = stmt.bind(&params)?.first::<serde_json::Value>(None).await?;

    match result {
        Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
        None => Ok(0),
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BlacklistEntry {
    pub id: String,
    pub destination: String,
    pub match_type: String,
    pub reason: String,
    pub created_by: String,
    pub created_at: i64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AdminLink {
    pub id: String,
    pub org_id: String,
    pub short_code: String,
    pub destination_url: String,
    pub title: Option<String>,
    pub created_by: String,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub status: String,
    pub click_count: i64,
    pub creator_email: String,
    pub org_name: String,
}
