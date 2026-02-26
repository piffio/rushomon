use crate::models::{
    AnalyticsEvent, BillingAccount, Link, Organization, User, user::CreateUserData,
};
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

/// Generate a unique slug for an organization name
/// Always adds random 5-character suffix for consistency and collision prevention
pub async fn generate_unique_slug(_db: &D1Database, org_name: &str) -> Result<String> {
    // Generate base slug using existing logic
    let base_slug = org_name
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    let uuid_str = uuid::Uuid::new_v4().to_string().replace('-', "");
    let random_suffix = &uuid_str[..5];

    let slug = format!("{}-{}", base_slug, random_suffix);

    Ok(slug)
}

#[cfg(test)]
mod slug_tests {
    // Test slug generation logic without database dependency
    #[tokio::test]
    async fn test_slug_generation_logic() {
        // Test base slug generation
        let base_slug = "Test Organization"
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();

        assert_eq!(base_slug, "test-organization");

        // Test with special characters
        let special_slug = "John's Test Organization! @#$%"
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();

        assert_eq!(special_slug, "johns-test-organization-");

        // Test UUID-based suffix generation
        let uuid_str = uuid::Uuid::new_v4().to_string().replace('-', "");
        let suffix = &uuid_str[..5];
        assert_eq!(suffix.len(), 5);
        assert!(suffix.chars().all(|c| c.is_alphanumeric()));

        // Test full slug generation (always includes suffix)
        let full_slug = format!("{}-{}", base_slug, suffix);
        assert!(full_slug.starts_with("test-organization-"));
        assert_eq!(full_slug.len(), base_slug.len() + 1 + 5); // base + "-" + 5 chars
    }
}

/// Create a default organization for a new user
pub async fn create_default_org(
    db: &D1Database,
    user_id: &str,
    org_name: &str,
) -> Result<Organization> {
    let org_id = uuid::Uuid::new_v4().to_string();
    let slug = generate_unique_slug(db, org_name).await?;

    let now = now_timestamp();

    // Read the default tier from settings
    let tier = get_setting(db, "default_user_tier")
        .await?
        .unwrap_or_else(|| "free".to_string());

    // Create billing account for the user
    let billing_account = create_billing_account(db, user_id, &tier).await?;

    let stmt = db.prepare(
        "INSERT INTO organizations (id, name, slug, created_at, created_by, tier, billing_account_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    );

    stmt.bind(&[
        org_id.clone().into(),
        org_name.into(),
        slug.clone().into(),
        (now as f64).into(),
        user_id.into(),
        tier.clone().into(),
        billing_account.id.clone().into(),
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
        billing_account_id: Some(billing_account.id),
    })
}

/// Get user by ID
pub async fn get_user_by_id(db: &D1Database, user_id: &str) -> Result<Option<User>> {
    let stmt = db.prepare(
        "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at,
                suspended_at, suspension_reason, suspended_by
         FROM users
         WHERE id = ?1",
    );

    stmt.bind(&[user_id.into()])?.first::<User>(None).await
}

/// Get organization by ID
pub async fn get_org_by_id(db: &D1Database, org_id: &str) -> Result<Option<Organization>> {
    let stmt = db.prepare(
        "SELECT id, name, slug, created_at, created_by, tier, billing_account_id
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

/// Get links for an organization with search, filter, sort, and tag filter options
#[allow(clippy::too_many_arguments)]
pub async fn get_links_by_org_filtered(
    db: &D1Database,
    org_id: &str,
    search: Option<&str>,
    status_filter: Option<&str>,
    sort: &str,
    limit: i64,
    offset: i64,
    tags_filter: Option<&[String]>,
) -> Result<Vec<Link>> {
    let mut query = String::from(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count
         FROM links
         WHERE org_id = ?1"
    );

    let mut params: Vec<worker::wasm_bindgen::JsValue> = vec![org_id.into()];

    // Add status filter
    if let Some(status) = status_filter {
        query.push_str(&format!(" AND status = ?{}", params.len() + 1));
        params.push(status.into());
    } else {
        // Default: show both active and disabled (not blocked)
        query.push_str(" AND status IN ('active', 'disabled')");
    }

    // Add search filter (matches title, short_code, or destination_url)
    if let Some(search_term) = search {
        let term = format!("%{}%", search_term.replace('%', "\\%").replace('_', "\\_"));
        query.push_str(&format!(
            " AND (LOWER(title) LIKE LOWER(?{}) OR LOWER(short_code) LIKE LOWER(?{}) OR LOWER(destination_url) LIKE LOWER(?{}))",
            params.len() + 1,
            params.len() + 2,
            params.len() + 3
        ));
        params.push(term.clone().into());
        params.push(term.clone().into());
        params.push(term.into());
    }

    // Add tag filter (OR semantics: link must have ANY of the specified tags)
    if let Some(tags) = tags_filter {
        if tags.len() == 1 {
            // Single tag - simple EXISTS
            query.push_str(&format!(
                " AND EXISTS (SELECT 1 FROM link_tags lt WHERE lt.link_id = id AND lt.tag_name = ?{})",
                params.len() + 1
            ));
            params.push(tags[0].as_str().into());
        } else {
            // Multiple tags - use IN clause for OR semantics
            let start_param = params.len() + 1;
            let placeholders: Vec<String> = (0..tags.len())
                .map(|i| format!("?{}", start_param + i))
                .collect();
            query.push_str(&format!(
                " AND id IN (SELECT DISTINCT link_id FROM link_tags WHERE tag_name IN ({}))",
                placeholders.join(", ")
            ));
            for tag in tags {
                params.push(tag.as_str().into());
            }
        }
    }

    // Add ORDER BY based on sort parameter (whitelisted for security)
    // SECURITY: Only use pre-defined sort clauses, never interpolate user input
    let order_clause = match sort {
        "clicks" => " ORDER BY click_count DESC",
        "updated" => " ORDER BY updated_at DESC NULLS LAST",
        "title" => " ORDER BY title ASC NULLS LAST",
        "code" => " ORDER BY short_code ASC",
        _ => " ORDER BY created_at DESC", // Default: created
    };
    query.push_str(order_clause);

    // Add LIMIT and OFFSET
    query.push_str(&format!(
        " LIMIT ?{} OFFSET ?{}",
        params.len() + 1,
        params.len() + 2
    ));
    params.push((limit as f64).into());
    params.push((offset as f64).into());

    let stmt = db.prepare(&query);
    let results = stmt.bind(&params)?.all().await?;
    let links = results.results::<Link>()?;

    Ok(links)
}

/// Get total count of links for an organization with search, status, and tag filters
pub async fn get_links_count_by_org_filtered(
    db: &D1Database,
    org_id: &str,
    search: Option<&str>,
    status_filter: Option<&str>,
    tags_filter: Option<&[String]>,
) -> Result<i64> {
    let mut query = String::from("SELECT COUNT(*) as count FROM links WHERE org_id = ?1");

    let mut params: Vec<worker::wasm_bindgen::JsValue> = vec![org_id.into()];

    // Add status filter
    if let Some(status) = status_filter {
        query.push_str(&format!(" AND status = ?{}", params.len() + 1));
        params.push(status.into());
    } else {
        // Default: show both active and disabled (not blocked)
        query.push_str(" AND status IN ('active', 'disabled')");
    }

    // Add search filter
    if let Some(search_term) = search {
        let term = format!("%{}%", search_term.replace('%', "\\%").replace('_', "\\_"));
        query.push_str(&format!(
            " AND (LOWER(title) LIKE LOWER(?{}) OR LOWER(short_code) LIKE LOWER(?{}) OR LOWER(destination_url) LIKE LOWER(?{}))",
            params.len() + 1,
            params.len() + 2,
            params.len() + 3
        ));
        params.push(term.clone().into());
        params.push(term.clone().into());
        params.push(term.into());
    }

    // Add tag filter (OR semantics)
    if let Some(tags) = tags_filter {
        if tags.len() == 1 {
            // Single tag - simple EXISTS
            query.push_str(&format!(
                " AND EXISTS (SELECT 1 FROM link_tags lt WHERE lt.link_id = id AND lt.tag_name = ?{})",
                params.len() + 1
            ));
            params.push(tags[0].as_str().into());
        } else {
            // Multiple tags - use IN clause for OR semantics
            let start_param = params.len() + 1;
            let placeholders: Vec<String> = (0..tags.len())
                .map(|i| format!("?{}", start_param + i))
                .collect();
            query.push_str(&format!(
                " AND id IN (SELECT DISTINCT link_id FROM link_tags WHERE tag_name IN ({}))",
                placeholders.join(", ")
            ));
            for tag in tags {
                params.push(tag.as_str().into());
            }
        }
    }

    let stmt = db.prepare(&query);
    let result = stmt.bind(&params)?.first::<serde_json::Value>(None).await?;

    match result {
        Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
        None => Ok(0),
    }
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

/// Get a link by short_code without org_id check (used for public reporting)
pub async fn get_link_by_short_code_no_auth(
    db: &D1Database,
    short_code: &str,
) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count
         FROM links
         WHERE short_code = ?1
         AND status = 'active'"
    );

    stmt.bind(&[short_code.into()])?.first::<Link>(None).await
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
/// Extended user info for admin panel with billing account details
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UserWithBillingInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub oauth_provider: String,
    pub oauth_id: String,
    pub org_id: String,
    pub role: String,
    pub created_at: i64,
    pub suspended_at: Option<i64>,
    pub suspension_reason: Option<String>,
    pub suspended_by: Option<String>,
    pub billing_account_id: Option<String>,
    pub billing_account_tier: Option<String>,
}

#[allow(dead_code)]
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

/// Get all users with billing account information (for admin panel)
pub async fn get_all_users_with_billing_info(
    db: &D1Database,
    limit: i64,
    offset: i64,
) -> Result<Vec<UserWithBillingInfo>> {
    let stmt = db.prepare(
        "SELECT u.id, u.email, u.name, u.avatar_url, u.oauth_provider, u.oauth_id,
                u.org_id, u.role, u.created_at, u.suspended_at, u.suspension_reason, u.suspended_by,
                o.billing_account_id, ba.tier as billing_account_tier
         FROM users u
         LEFT JOIN organizations o ON u.org_id = o.id
         LEFT JOIN billing_accounts ba ON o.billing_account_id = ba.id
         ORDER BY u.created_at ASC
         LIMIT ?1 OFFSET ?2",
    );

    let results = stmt
        .bind(&[(limit as f64).into(), (offset as f64).into()])?
        .all()
        .await?;

    let rows = results.results::<serde_json::Value>()?;
    let users: Vec<UserWithBillingInfo> = rows
        .iter()
        .filter_map(|row| {
            Some(UserWithBillingInfo {
                id: row["id"].as_str()?.to_string(),
                email: row["email"].as_str()?.to_string(),
                name: row["name"].as_str().map(|s| s.to_string()),
                avatar_url: row["avatar_url"].as_str().map(|s| s.to_string()),
                oauth_provider: row["oauth_provider"].as_str()?.to_string(),
                oauth_id: row["oauth_id"].as_str()?.to_string(),
                org_id: row["org_id"].as_str()?.to_string(),
                role: row["role"].as_str()?.to_string(),
                created_at: row["created_at"].as_f64()? as i64,
                suspended_at: row["suspended_at"].as_f64().map(|v| v as i64),
                suspension_reason: row["suspension_reason"].as_str().map(|s| s.to_string()),
                suspended_by: row["suspended_by"].as_str().map(|s| s.to_string()),
                billing_account_id: row["billing_account_id"].as_str().map(|s| s.to_string()),
                billing_account_tier: row["billing_account_tier"].as_str().map(|s| s.to_string()),
            })
        })
        .collect();

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
/// DEPRECATED: Use get_monthly_counter_for_billing_account instead
#[allow(dead_code)]
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
/// DEPRECATED: Use increment_monthly_counter_for_billing_account instead
#[allow(dead_code)]
pub async fn increment_monthly_counter(
    db: &D1Database,
    org_id: &str,
    year_month: &str,
    max_links: i64,
) -> Result<bool> {
    let now = now_timestamp();

    // Atomic approach: Try to insert/update only if under limit
    // This prevents race conditions where multiple requests could bypass the limit

    // First, ensure the row exists (idempotent operation)
    let init_stmt = db.prepare(
        "INSERT INTO monthly_counters (org_id, year_month, links_created, updated_at)
         VALUES (?1, ?2, 0, ?3)
         ON CONFLICT(org_id, year_month) DO NOTHING",
    );
    init_stmt
        .bind(&[org_id.into(), year_month.into(), (now as f64).into()])?
        .run()
        .await?;

    // Now atomically increment only if below limit
    // The WHERE clause ensures we only increment if current count < max_links
    let stmt = db.prepare(
        "UPDATE monthly_counters
         SET links_created = links_created + 1, updated_at = ?1
         WHERE org_id = ?2 AND year_month = ?3 AND links_created < ?4",
    );

    let result = stmt
        .bind(&[
            (now as f64).into(),
            org_id.into(),
            year_month.into(),
            (max_links as f64).into(),
        ])?
        .run()
        .await?;

    // Check if the update actually modified a row
    // If changes == 0, it means the limit was already reached (WHERE clause prevented update)
    let changes = result.meta()?.and_then(|m| m.changes).unwrap_or(0);

    Ok(changes > 0)
}

/// Reset monthly counter for an organization to 0
/// DEPRECATED: Use reset_monthly_counter_for_billing_account instead
#[allow(dead_code)]
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

/// Check if destination is already blacklisted
pub async fn is_destination_already_blacklisted(
    db: &D1Database,
    destination: &str,
    match_type: &str,
) -> Result<bool> {
    let stmt = db.prepare(
        "SELECT 1 FROM destination_blacklist
         WHERE destination = ?1 AND match_type = ?2
         LIMIT 1",
    );
    if let Ok(Some(_)) = stmt
        .bind(&[destination.into(), match_type.into()])?
        .first::<serde_json::Value>(None)
        .await
    {
        Ok(true)
    } else {
        Ok(false)
    }
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
    use crate::utils::normalize_url_for_blacklist;

    // Normalize the destination URL for comparison
    let normalized_destination = match normalize_url_for_blacklist(destination) {
        Ok(url) => url,
        Err(_) => {
            // If URL parsing fails, fall back to exact string comparison
            destination.to_string()
        }
    };

    // First check exact match against normalized blacklist entries
    let exact_stmt = db.prepare(
        "SELECT 1 FROM destination_blacklist
         WHERE destination = ?1 AND match_type = 'exact'
         LIMIT 1",
    );
    if let Ok(Some(_)) = exact_stmt
        .bind(&[normalized_destination.clone().into()])?
        .first::<serde_json::Value>(None)
        .await
    {
        return Ok(true);
    }

    // Then check domain match (still uses original domain extraction logic)
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

    // Finally, check if any normalized blacklist entries match our normalized destination
    // This handles cases where blocked URLs have different forms but same normalized form
    let all_entries = get_all_blacklist(db).await?;
    for entry in all_entries {
        if entry.match_type == "exact"
            && let Ok(normalized_entry) = normalize_url_for_blacklist(&entry.destination)
            && normalized_entry == normalized_destination
        {
            return Ok(true);
        }
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LinkReport {
    pub id: String,
    pub link_id: String,
    pub reason: String,
    pub reporter_user_id: Option<String>,
    pub reporter_email: Option<String>,
    pub status: String,
    pub admin_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<i64>,
    pub created_at: i64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LinkReportWithLink {
    pub id: String,
    pub link_id: String,
    pub link: AdminLink,
    pub reason: String,
    pub reporter_user_id: Option<String>,
    pub reporter_email: Option<String>,
    pub status: String,
    pub admin_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<i64>,
    pub created_at: i64,
    pub report_count: i64, // For grouping
}

// Helper struct for flat query results
#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct LinkReportQueryResult {
    pub id: String,
    pub link_id: String,
    pub reason: String,
    pub reporter_user_id: Option<String>,
    pub reporter_email: Option<String>,
    pub status: String,
    pub admin_notes: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<i64>,
    pub created_at: i64,
    pub link__id: String,
    pub link__org_id: String,
    pub link__short_code: String,
    pub link__destination_url: String,
    pub link__title: Option<String>,
    pub link__created_by: String,
    pub link__created_at: i64,
    pub link__updated_at: Option<i64>,
    pub link__expires_at: Option<i64>,
    pub link__status: String,
    pub link__click_count: i64,
    pub link__creator_email: String,
    pub link__org_name: String,
    pub report_count: i64,
}

/// Create a new link abuse report
pub async fn create_link_report(
    db: &D1Database,
    link_id: &str,
    reason: &str,
    reporter_user_id: Option<&str>,
    reporter_email: Option<&str>,
) -> Result<LinkReport> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();

    let stmt = db.prepare(
        "INSERT INTO link_reports (id, link_id, reason, reporter_user_id, reporter_email, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    );

    stmt.bind(&[
        id.clone().into(),
        link_id.into(),
        reason.into(),
        reporter_user_id.map(|s| s.into()).unwrap_or(JsValue::NULL),
        reporter_email.map(|s| s.into()).unwrap_or(JsValue::NULL),
        "pending".into(),
        (now as f64).into(),
    ])?
    .run()
    .await?;

    // Return the created report
    get_link_report_by_id(db, &id).await
}

/// Get a single report by ID
pub async fn get_link_report_by_id(db: &D1Database, _report_id: &str) -> Result<LinkReport> {
    let stmt = db.prepare(
        "SELECT id, link_id, reason, reporter_user_id, reporter_email, status, 
                admin_notes, reviewed_by, reviewed_at, created_at
         FROM link_reports WHERE id = ?1",
    );

    let bound_stmt = stmt.bind(&[_report_id.into()])?;
    let result = bound_stmt.first::<LinkReport>(None).await?;

    match result {
        Some(report) => Ok(report),
        None => Err(Error::RustError("Report not found".to_string())),
    }
}

/// Get paginated list of reports with optional filtering
pub async fn get_link_reports(
    db: &D1Database,
    page: u32,
    limit: u32,
    status_filter: Option<&str>,
) -> Result<(Vec<LinkReportWithLink>, i64)> {
    let offset = (page - 1) * limit;

    // Build the base query with status filter
    let filter_param = status_filter;

    // Get total count
    let count_query = format!(
        "SELECT COUNT(*) as count
         FROM link_reports lr
         {}",
        if status_filter.is_some() {
            "WHERE lr.status = ?1"
        } else {
            ""
        }
    );

    let count_result = if let Some(filter) = filter_param {
        let stmt = db.prepare(&count_query);
        let bound_stmt = stmt.bind(&[filter.into()])?;
        bound_stmt.first::<serde_json::Value>(None).await?
    } else {
        let stmt = db.prepare(&count_query);
        stmt.first::<serde_json::Value>(None).await?
    };

    let total = count_result
        .and_then(|v| v["count"].as_f64())
        .unwrap_or(0.0) as i64;

    // Get reports with link details and report count
    let reports_query = format!(
        "SELECT
            lr.id, lr.link_id, lr.reason, lr.reporter_user_id, lr.reporter_email,
            lr.status, lr.admin_notes, lr.reviewed_by, lr.reviewed_at, lr.created_at,
            l.id as link__id, l.org_id as link__org_id, l.short_code as link__short_code,
            l.destination_url as link__destination_url, l.title as link__title,
            l.created_by as link__created_by, l.created_at as link__created_at,
            l.updated_at as link__updated_at, l.expires_at as link__expires_at,
            l.status as link__status, l.click_count as link__click_count,
            u.email as link__creator_email, o.name as link__org_name,
            COUNT(lr_sub.id) as report_count
         FROM link_reports lr
         LEFT JOIN links l ON lr.link_id = l.id
         LEFT JOIN users u ON l.created_by = u.id
         LEFT JOIN organizations o ON l.org_id = o.id
         LEFT JOIN link_reports lr_sub ON lr.link_id = lr_sub.link_id
         {}
         GROUP BY lr.id, l.id, u.id, o.id
         ORDER BY lr.created_at DESC
         LIMIT {} OFFSET {}",
        if status_filter.is_some() {
            "WHERE lr.status = ?1"
        } else {
            ""
        },
        if status_filter.is_some() { "?2" } else { "?1" },
        if status_filter.is_some() { "?3" } else { "?2" }
    );

    let results = if let Some(filter) = filter_param {
        let stmt = db.prepare(&reports_query);
        let bound_stmt =
            stmt.bind(&[filter.into(), (limit as f64).into(), (offset as f64).into()])?;
        bound_stmt.all().await?
    } else {
        let stmt = db.prepare(&reports_query);
        let bound_stmt = stmt.bind(&[(limit as f64).into(), (offset as f64).into()])?;
        bound_stmt.all().await?
    };

    let query_results: Vec<LinkReportQueryResult> = results.results()?;

    // Convert flat results to nested structure
    let reports: Vec<LinkReportWithLink> = query_results
        .into_iter()
        .map(|qr| LinkReportWithLink {
            id: qr.id,
            link_id: qr.link_id,
            link: AdminLink {
                id: qr.link__id,
                org_id: qr.link__org_id,
                short_code: qr.link__short_code,
                destination_url: qr.link__destination_url,
                title: qr.link__title,
                created_by: qr.link__created_by,
                created_at: qr.link__created_at,
                updated_at: qr.link__updated_at,
                expires_at: qr.link__expires_at,
                status: qr.link__status,
                click_count: qr.link__click_count,
                creator_email: qr.link__creator_email,
                org_name: qr.link__org_name,
            },
            reason: qr.reason,
            reporter_user_id: qr.reporter_user_id,
            reporter_email: qr.reporter_email,
            status: qr.status,
            admin_notes: qr.admin_notes,
            reviewed_by: qr.reviewed_by,
            reviewed_at: qr.reviewed_at,
            created_at: qr.created_at,
            report_count: qr.report_count,
        })
        .collect();

    Ok((reports, total))
}

/// Update report status and add admin notes
pub async fn update_link_report_status(
    db: &D1Database,
    report_id: &str,
    status: &str,
    admin_user_id: &str,
    admin_notes: Option<&str>,
) -> Result<()> {
    let now = now_timestamp();

    let stmt = db.prepare(
        "UPDATE link_reports
         SET status = ?1, admin_notes = ?2, reviewed_by = ?3, reviewed_at = ?4
         WHERE id = ?5",
    );

    stmt.bind(&[
        status.into(),
        admin_notes.map(|s| s.into()).unwrap_or(JsValue::NULL),
        admin_user_id.into(),
        (now as f64).into(),
        report_id.into(),
    ])?
    .run()
    .await?;

    Ok(())
}

/// Resolve all pending reports for a specific link
pub async fn resolve_reports_for_link(
    db: &D1Database,
    link_id: &str,
    status: &str,
    admin_notes: &str,
    admin_user_id: &str,
) -> Result<()> {
    let now = now_timestamp();

    let stmt = db.prepare(
        "UPDATE link_reports
         SET status = ?1, admin_notes = ?2, reviewed_by = ?3, reviewed_at = ?4
         WHERE link_id = ?5 AND status = 'pending'",
    );

    stmt.bind(&[
        status.into(),
        admin_notes.into(),
        admin_user_id.into(),
        (now as f64).into(),
        link_id.into(),
    ])?
    .run()
    .await?;

    Ok(())
}

/// Get count of pending reports for admin badge
pub async fn get_pending_reports_count(db: &D1Database) -> Result<i64> {
    let stmt = db.prepare("SELECT COUNT(*) as count FROM link_reports WHERE status = 'pending'");
    let result = stmt.first::<serde_json::Value>(None).await?;

    match result {
        Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
        None => Ok(0),
    }
}

/// Check for duplicate reports (same link, reason, and reporter within 24h)
pub async fn is_duplicate_report(
    db: &D1Database,
    link_id: &str,
    reason: &str,
    reporter_user_id: Option<&str>,
    reporter_email: Option<&str>,
) -> Result<bool> {
    let twenty_four_hours_ago = now_timestamp() - (24 * 60 * 60); // 24 hours ago

    let query = if reporter_user_id.is_some() {
        "SELECT COUNT(*) as count
         FROM link_reports
         WHERE link_id = ?1 AND reason = ?2 AND reporter_user_id = ?3 AND created_at > ?4"
            .to_string()
    } else {
        "SELECT COUNT(*) as count
         FROM link_reports
         WHERE link_id = ?1 AND reason = ?2 AND reporter_email = ?3 AND created_at > ?4"
            .to_string()
    };

    let stmt = db.prepare(&query);
    let reporter_id = reporter_user_id.or(reporter_email).unwrap_or("");

    let bound_stmt = stmt.bind(&[
        link_id.into(),
        reason.into(),
        reporter_id.into(),
        (twenty_four_hours_ago as f64).into(),
    ])?;

    let result = bound_stmt.first::<serde_json::Value>(None).await?;

    let count = result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64;

    Ok(count > 0)
}

// ─── Tag queries ─────────────────────────────────────────────────────────────

/// Normalize a tag name: trim whitespace, collapse internal spaces, max 50 chars.
/// Returns None if the result is empty.
pub fn normalize_tag(tag: &str) -> Option<String> {
    let normalized: String = tag.split_whitespace().collect::<Vec<&str>>().join(" ");
    if normalized.is_empty() || normalized.len() > 50 {
        None
    } else {
        Some(normalized)
    }
}

/// Validate and normalize a list of tags. Returns error string if any tag is invalid.
pub fn validate_and_normalize_tags(tags: &[String]) -> Result<Vec<String>> {
    if tags.len() > 20 {
        return Err(worker::Error::RustError(
            "Maximum 20 tags per link".to_string(),
        ));
    }
    let mut normalized = Vec::with_capacity(tags.len());
    for tag in tags {
        match normalize_tag(tag) {
            Some(t) => normalized.push(t),
            None => {
                return Err(worker::Error::RustError(format!(
                    "Invalid tag: '{}'. Tags must be non-empty and at most 50 characters.",
                    tag
                )));
            }
        }
    }
    // Deduplicate (case-insensitive)
    let mut seen = std::collections::HashSet::new();
    normalized.retain(|t| seen.insert(t.to_lowercase()));
    Ok(normalized)
}

/// Get all tags for a single link, sorted alphabetically.
pub async fn get_tags_for_link(db: &D1Database, link_id: &str) -> Result<Vec<String>> {
    let stmt =
        db.prepare("SELECT tag_name FROM link_tags WHERE link_id = ?1 ORDER BY tag_name ASC");
    let results = stmt.bind(&[link_id.into()])?.all().await?;
    let rows = results.results::<serde_json::Value>()?;
    let tags = rows
        .iter()
        .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
        .collect();
    Ok(tags)
}

/// Get tags for multiple links in a single query. Returns a map of link_id → tags.
pub async fn get_tags_for_links(
    db: &D1Database,
    link_ids: &[String],
) -> Result<std::collections::HashMap<String, Vec<String>>> {
    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    if link_ids.is_empty() {
        return Ok(map);
    }

    // Build IN clause: ?1, ?2, ...
    let placeholders: Vec<String> = (1..=link_ids.len()).map(|i| format!("?{}", i)).collect();
    let query = format!(
        "SELECT link_id, tag_name FROM link_tags WHERE link_id IN ({}) ORDER BY link_id, tag_name ASC",
        placeholders.join(", ")
    );

    let params: Vec<worker::wasm_bindgen::JsValue> =
        link_ids.iter().map(|id| id.as_str().into()).collect();

    let stmt = db.prepare(&query);
    let results = stmt.bind(&params)?.all().await?;
    let rows = results.results::<serde_json::Value>()?;

    for row in &rows {
        if let (Some(link_id), Some(tag_name)) = (row["link_id"].as_str(), row["tag_name"].as_str())
        {
            map.entry(link_id.to_string())
                .or_default()
                .push(tag_name.to_string());
        }
    }
    Ok(map)
}

/// Replace all tags for a link atomically (delete existing, insert new).
pub async fn set_tags_for_link(
    db: &D1Database,
    link_id: &str,
    org_id: &str,
    tags: &[String],
) -> Result<()> {
    // Delete existing tags
    let delete_stmt = db.prepare("DELETE FROM link_tags WHERE link_id = ?1");
    delete_stmt.bind(&[link_id.into()])?.run().await?;

    // Insert new tags
    for tag in tags {
        let insert_stmt =
            db.prepare("INSERT INTO link_tags (link_id, tag_name, org_id) VALUES (?1, ?2, ?3)");
        insert_stmt
            .bind(&[link_id.into(), tag.as_str().into(), org_id.into()])?
            .run()
            .await?;
    }
    Ok(())
}

/// Delete all tags for a link (called on link deletion).
pub async fn delete_tags_for_link(db: &D1Database, link_id: &str) -> Result<()> {
    let stmt = db.prepare("DELETE FROM link_tags WHERE link_id = ?1");
    stmt.bind(&[link_id.into()])?.run().await?;
    Ok(())
}

/// Get all tags for an org with usage counts, sorted by count desc then name asc.
pub async fn get_org_tags(db: &D1Database, org_id: &str) -> Result<Vec<OrgTag>> {
    let stmt = db.prepare(
        "SELECT tag_name, COUNT(*) as count
         FROM link_tags
         WHERE org_id = ?1
         GROUP BY tag_name
         ORDER BY count DESC, tag_name ASC",
    );
    let results = stmt.bind(&[org_id.into()])?.all().await?;
    let rows = results.results::<serde_json::Value>()?;
    let tags = rows
        .iter()
        .filter_map(|row| {
            let name = row["tag_name"].as_str()?.to_string();
            let count = row["count"].as_f64()? as i64;
            Some(OrgTag { name, count })
        })
        .collect();
    Ok(tags)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OrgTag {
    pub name: String,
    pub count: i64,
}

// ─── Org Management ───────────────────────────────────────────────────────────

use crate::models::{OrgInvitation, OrgMember, OrgMemberWithUser, OrgWithRole};

/// Get all organizations a user belongs to (via org_members junction table).
/// Falls back to the user's default org_id if no org_members rows exist yet,
/// and auto-inserts the missing membership row to self-heal.
pub async fn get_user_orgs(db: &D1Database, user_id: &str) -> Result<Vec<OrgWithRole>> {
    let stmt = db.prepare(
        "SELECT o.id, o.name, o.tier, m.role, m.joined_at
         FROM organizations o
         JOIN org_members m ON o.id = m.org_id
         WHERE m.user_id = ?1
         ORDER BY m.joined_at ASC",
    );
    let results = stmt.bind(&[user_id.into()])?.all().await?;
    let rows = results.results::<serde_json::Value>()?;
    let orgs: Vec<OrgWithRole> = rows
        .iter()
        .filter_map(|row| {
            Some(OrgWithRole {
                id: row["id"].as_str()?.to_string(),
                name: row["name"].as_str()?.to_string(),
                tier: row["tier"].as_str()?.to_string(),
                role: row["role"].as_str()?.to_string(),
                joined_at: row["joined_at"].as_f64()? as i64,
            })
        })
        .collect();

    if !orgs.is_empty() {
        return Ok(orgs);
    }

    // No org_members rows — fall back to users.org_id and auto-heal
    let user_row = db
        .prepare("SELECT org_id, created_at FROM users WHERE id = ?1")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let Some(user_row) = user_row else {
        return Ok(vec![]);
    };

    let Some(org_id) = user_row["org_id"].as_str() else {
        return Ok(vec![]);
    };
    let joined_at = user_row["created_at"].as_f64().unwrap_or(0.0) as i64;

    // Insert the missing membership row (ignore conflicts)
    db.prepare(
        "INSERT INTO org_members (org_id, user_id, role, joined_at)
         VALUES (?1, ?2, 'owner', ?3)
         ON CONFLICT(org_id, user_id) DO NOTHING",
    )
    .bind(&[org_id.into(), user_id.into(), (joined_at as f64).into()])?
    .run()
    .await?;

    // Now fetch the org details
    let org = match get_org_by_id(db, org_id).await? {
        Some(o) => o,
        None => return Ok(vec![]),
    };

    Ok(vec![OrgWithRole {
        id: org.id,
        name: org.name,
        tier: org.tier,
        role: "owner".to_string(),
        joined_at,
    }])
}

/// Get the membership record for a specific user in a specific org.
/// Falls back to users.org_id ownership check and auto-inserts the row to self-heal.
pub async fn get_org_member(
    db: &D1Database,
    org_id: &str,
    user_id: &str,
) -> Result<Option<OrgMember>> {
    let stmt = db.prepare(
        "SELECT org_id, user_id, role, joined_at
         FROM org_members
         WHERE org_id = ?1 AND user_id = ?2",
    );
    let existing = stmt
        .bind(&[org_id.into(), user_id.into()])?
        .first::<OrgMember>(None)
        .await?;

    if existing.is_some() {
        return Ok(existing);
    }

    // Not in org_members — check if org_id matches users.org_id (pre-migration user)
    let user_row = db
        .prepare("SELECT org_id, created_at FROM users WHERE id = ?1")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let Some(user_row) = user_row else {
        return Ok(None);
    };

    if user_row["org_id"].as_str() != Some(org_id) {
        return Ok(None);
    }

    let joined_at = user_row["created_at"].as_f64().unwrap_or(0.0) as i64;

    // Auto-heal: insert the missing membership row
    db.prepare(
        "INSERT INTO org_members (org_id, user_id, role, joined_at)
         VALUES (?1, ?2, 'owner', ?3)
         ON CONFLICT(org_id, user_id) DO NOTHING",
    )
    .bind(&[org_id.into(), user_id.into(), (joined_at as f64).into()])?
    .run()
    .await?;

    Ok(Some(OrgMember {
        org_id: org_id.to_string(),
        user_id: user_id.to_string(),
        role: "owner".to_string(),
        joined_at,
    }))
}

/// Get all members of an org with user details
pub async fn get_org_members(db: &D1Database, org_id: &str) -> Result<Vec<OrgMemberWithUser>> {
    let stmt = db.prepare(
        "SELECT u.id as user_id, u.email, u.name, u.avatar_url, m.role, m.joined_at
         FROM org_members m
         JOIN users u ON u.id = m.user_id
         WHERE m.org_id = ?1
         ORDER BY m.joined_at ASC",
    );
    let results = stmt.bind(&[org_id.into()])?.all().await?;
    let rows = results.results::<serde_json::Value>()?;
    let members = rows
        .iter()
        .filter_map(|row| {
            Some(OrgMemberWithUser {
                user_id: row["user_id"].as_str()?.to_string(),
                email: row["email"].as_str()?.to_string(),
                name: row["name"].as_str().map(|s| s.to_string()),
                avatar_url: row["avatar_url"].as_str().map(|s| s.to_string()),
                role: row["role"].as_str()?.to_string(),
                joined_at: row["joined_at"].as_f64()? as i64,
            })
        })
        .collect();
    Ok(members)
}

/// Add a user to an org
pub async fn add_org_member(
    db: &D1Database,
    org_id: &str,
    user_id: &str,
    role: &str,
) -> Result<()> {
    let now = now_timestamp();
    let stmt = db.prepare(
        "INSERT INTO org_members (org_id, user_id, role, joined_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(org_id, user_id) DO NOTHING",
    );
    stmt.bind(&[
        org_id.into(),
        user_id.into(),
        role.into(),
        (now as f64).into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Remove a user from an org
pub async fn remove_org_member(db: &D1Database, org_id: &str, user_id: &str) -> Result<()> {
    let stmt = db.prepare("DELETE FROM org_members WHERE org_id = ?1 AND user_id = ?2");
    stmt.bind(&[org_id.into(), user_id.into()])?.run().await?;
    Ok(())
}

/// Count owners of an org (to prevent removing the last owner)
pub async fn count_org_owners(db: &D1Database, org_id: &str) -> Result<i64> {
    let stmt = db
        .prepare("SELECT COUNT(*) as count FROM org_members WHERE org_id = ?1 AND role = 'owner'");
    let result = stmt
        .bind(&[org_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
}

/// Count all members in an organization (including owner)
/// Used for enforcing member limits
pub async fn count_org_members(db: &D1Database, org_id: &str) -> Result<i64> {
    let stmt = db.prepare("SELECT COUNT(*) as count FROM org_members WHERE org_id = ?1");
    let result = stmt
        .bind(&[org_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
}

/// Count pending (not yet accepted) invitations for an org
/// Used for enforcing member limits (pending invites count toward the limit)
pub async fn count_pending_invitations(db: &D1Database, org_id: &str) -> Result<i64> {
    let stmt = db.prepare(
        "SELECT COUNT(*) as count FROM org_invitations
         WHERE org_id = ?1 AND accepted_at IS NULL AND expires_at > ?2",
    );
    let now = now_timestamp();
    let result = stmt
        .bind(&[org_id.into(), (now as f64).into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
}

/// Count organizations where a user is an owner
/// Used for enforcing org creation limits (only owned orgs count against limit)
pub async fn count_user_owned_orgs(db: &D1Database, user_id: &str) -> Result<i64> {
    let stmt = db
        .prepare("SELECT COUNT(*) as count FROM org_members WHERE user_id = ?1 AND role = 'owner'");
    let result = stmt
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
}

/// Update an org's display name
pub async fn update_org_name(db: &D1Database, org_id: &str, name: &str) -> Result<()> {
    let stmt = db.prepare("UPDATE organizations SET name = ?1 WHERE id = ?2");
    stmt.bind(&[name.into(), org_id.into()])?.run().await?;
    Ok(())
}

/// Create a new organization (for users creating additional orgs beyond personal)
/// DEPRECATED: Use create_organization_with_billing_account instead
/// This function is kept for backward compatibility only
#[allow(dead_code)]
pub async fn create_organization(
    db: &D1Database,
    name: &str,
    created_by: &str,
) -> Result<Organization> {
    let org_id = uuid::Uuid::new_v4().to_string();
    let slug = generate_unique_slug(db, name).await?;
    let now = now_timestamp();

    let stmt = db.prepare(
        "INSERT INTO organizations (id, name, slug, created_at, created_by, tier)
         VALUES (?1, ?2, ?3, ?4, ?5, 'free')",
    );
    stmt.bind(&[
        org_id.clone().into(),
        name.into(),
        slug.clone().into(),
        (now as f64).into(),
        created_by.into(),
    ])?
    .run()
    .await?;

    Ok(Organization {
        id: org_id,
        name: name.to_string(),
        slug,
        created_at: now,
        created_by: created_by.to_string(),
        tier: "free".to_string(),
        billing_account_id: None, // Old orgs don't have billing accounts
    })
}

/// Count active links in an organization
/// Used to check if org has links before deletion and for migration slot validation
pub async fn count_org_links(db: &D1Database, org_id: &str) -> Result<i64> {
    let stmt =
        db.prepare("SELECT COUNT(*) as count FROM links WHERE org_id = ?1 AND status = 'active'");
    let result = stmt
        .bind(&[org_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64)
}

/// Get all link IDs for an organization (for KV cleanup)
pub async fn get_org_link_ids(db: &D1Database, org_id: &str) -> Result<Vec<String>> {
    let stmt = db.prepare("SELECT id FROM links WHERE org_id = ?1");
    let results = stmt.bind(&[org_id.into()])?.all().await?;

    let link_ids: Vec<String> = results
        .results::<serde_json::Value>()?
        .into_iter()
        .filter_map(|v| v["id"].as_str().map(|s| s.to_string()))
        .collect();

    Ok(link_ids)
}

/// Migrate all links from one org to another (updates both active and inactive links)
pub async fn migrate_org_links(db: &D1Database, from_org_id: &str, to_org_id: &str) -> Result<()> {
    let stmt = db.prepare("UPDATE links SET org_id = ?1 WHERE org_id = ?2");
    stmt.bind(&[to_org_id.into(), from_org_id.into()])?
        .run()
        .await?;
    Ok(())
}

/// Hard delete all links in an organization (after KV cleanup)
/// Deletes analytics events first to satisfy FK constraints
pub async fn delete_org_links(db: &D1Database, org_id: &str) -> Result<()> {
    // First delete analytics events (FK constraint: analytics_events.link_id -> links.id)
    let analytics_stmt = db.prepare("DELETE FROM analytics_events WHERE org_id = ?1");
    analytics_stmt.bind(&[org_id.into()])?.run().await?;

    // Then delete the links themselves
    let stmt = db.prepare("DELETE FROM links WHERE org_id = ?1");
    stmt.bind(&[org_id.into()])?.run().await?;

    Ok(())
}

/// Delete an organization and all related data
/// IMPORTANT: Links and analytics must be handled BEFORE calling this function
/// (either migrated or deleted via delete_org_links)
pub async fn delete_organization(db: &D1Database, org_id: &str) -> Result<()> {
    // Delete pending invitations
    let stmt = db.prepare("DELETE FROM org_invitations WHERE org_id = ?1");
    stmt.bind(&[org_id.into()])?.run().await?;

    // Delete org members
    let stmt = db.prepare("DELETE FROM org_members WHERE org_id = ?1");
    stmt.bind(&[org_id.into()])?.run().await?;

    // Delete monthly counters
    let stmt = db.prepare("DELETE FROM monthly_counters WHERE org_id = ?1");
    stmt.bind(&[org_id.into()])?.run().await?;

    // Delete the organization
    let stmt = db.prepare("DELETE FROM organizations WHERE id = ?1");
    stmt.bind(&[org_id.into()])?.run().await?;

    Ok(())
}

// ─── Invitation Queries ───────────────────────────────────────────────────────

/// Create a new org invitation (token = UUID = id)
pub async fn create_org_invitation(
    db: &D1Database,
    org_id: &str,
    invited_by: &str,
    email: &str,
    role: &str,
) -> Result<OrgInvitation> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_timestamp();
    let expires_at = now + 7 * 24 * 3600; // 7 days

    let stmt = db.prepare(
        "INSERT INTO org_invitations (id, org_id, invited_by, email, role, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    );
    stmt.bind(&[
        id.clone().into(),
        org_id.into(),
        invited_by.into(),
        email.into(),
        role.into(),
        (now as f64).into(),
        (expires_at as f64).into(),
    ])?
    .run()
    .await?;

    Ok(OrgInvitation {
        id,
        org_id: org_id.to_string(),
        invited_by: invited_by.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        created_at: now,
        expires_at,
        accepted_at: None,
    })
}

/// Look up an invitation by its token (UUID = id)
pub async fn get_invitation_by_token(
    db: &D1Database,
    token: &str,
) -> Result<Option<OrgInvitation>> {
    let stmt = db.prepare(
        "SELECT id, org_id, invited_by, email, role, created_at, expires_at, accepted_at
         FROM org_invitations
         WHERE id = ?1",
    );
    stmt.bind(&[token.into()])?
        .first::<OrgInvitation>(None)
        .await
}

/// List pending (not yet accepted, not expired) invitations for an org
pub async fn list_org_invitations(db: &D1Database, org_id: &str) -> Result<Vec<OrgInvitation>> {
    let now = now_timestamp();
    let stmt = db.prepare(
        "SELECT id, org_id, invited_by, email, role, created_at, expires_at, accepted_at
         FROM org_invitations
         WHERE org_id = ?1 AND accepted_at IS NULL AND expires_at > ?2
         ORDER BY created_at DESC",
    );
    let results = stmt
        .bind(&[org_id.into(), (now as f64).into()])?
        .all()
        .await?;
    results.results::<OrgInvitation>()
}

/// Mark an invitation as accepted
pub async fn accept_invitation(db: &D1Database, token: &str) -> Result<()> {
    let now = now_timestamp();
    let stmt = db.prepare("UPDATE org_invitations SET accepted_at = ?1 WHERE id = ?2");
    stmt.bind(&[(now as f64).into(), token.into()])?
        .run()
        .await?;
    Ok(())
}

/// Delete an invitation (revoke)
pub async fn revoke_invitation(db: &D1Database, invitation_id: &str) -> Result<()> {
    let stmt = db.prepare("DELETE FROM org_invitations WHERE id = ?1");
    stmt.bind(&[invitation_id.into()])?.run().await?;
    Ok(())
}

/// Check whether a pending (non-expired) invite for this email already exists in the org
pub async fn pending_invite_exists(db: &D1Database, org_id: &str, email: &str) -> Result<bool> {
    let now = now_timestamp();
    let stmt = db.prepare(
        "SELECT 1 FROM org_invitations
         WHERE org_id = ?1 AND email = ?2 AND accepted_at IS NULL AND expires_at > ?3
         LIMIT 1",
    );
    Ok(stmt
        .bind(&[org_id.into(), email.into(), (now as f64).into()])?
        .first::<serde_json::Value>(None)
        .await?
        .is_some())
}

/// Get user by email (used during invite acceptance to find the accepting user)
pub async fn get_user_by_email(
    db: &D1Database,
    email: &str,
) -> Result<Option<crate::models::User>> {
    let stmt = db.prepare(
        "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at,
                suspended_at, suspension_reason, suspended_by
         FROM users WHERE email = ?1",
    );
    stmt.bind(&[email.into()])?
        .first::<crate::models::User>(None)
        .await
}

// ============================================================================
// Billing Account Functions
// ============================================================================

/// Get billing account by ID
pub async fn get_billing_account(db: &D1Database, id: &str) -> Result<Option<BillingAccount>> {
    let stmt = db.prepare(
        "SELECT id, owner_user_id, tier, stripe_customer_id, created_at
         FROM billing_accounts
         WHERE id = ?1",
    );
    stmt.bind(&[id.into()])?.first::<BillingAccount>(None).await
}

/// Get billing account for an organization
pub async fn get_billing_account_for_org(
    db: &D1Database,
    org_id: &str,
) -> Result<Option<BillingAccount>> {
    let stmt = db.prepare(
        "SELECT ba.id, ba.owner_user_id, ba.tier, ba.stripe_customer_id, ba.created_at
         FROM billing_accounts ba
         INNER JOIN organizations o ON o.billing_account_id = ba.id
         WHERE o.id = ?1",
    );
    stmt.bind(&[org_id.into()])?
        .first::<BillingAccount>(None)
        .await
}

/// Get billing account for a user (their primary billing account)
/// Returns the billing account of the user's primary organization
pub async fn get_user_billing_account(
    db: &D1Database,
    user_id: &str,
) -> Result<Option<BillingAccount>> {
    let stmt = db.prepare(
        "SELECT ba.id, ba.owner_user_id, ba.tier, ba.stripe_customer_id, ba.created_at
         FROM billing_accounts ba
         INNER JOIN organizations o ON o.billing_account_id = ba.id
         INNER JOIN users u ON u.org_id = o.id
         WHERE u.id = ?1",
    );
    stmt.bind(&[user_id.into()])?
        .first::<BillingAccount>(None)
        .await
}

/// Create a billing account
pub async fn create_billing_account(
    db: &D1Database,
    owner_user_id: &str,
    tier: &str,
) -> Result<BillingAccount> {
    let id = BillingAccount::generate_id();
    let now = now_timestamp();

    let stmt = db.prepare(
        "INSERT INTO billing_accounts (id, owner_user_id, tier, created_at)
         VALUES (?1, ?2, ?3, ?4)",
    );

    stmt.bind(&[
        id.clone().into(),
        owner_user_id.into(),
        tier.into(),
        (now as f64).into(),
    ])?
    .run()
    .await?;

    Ok(BillingAccount {
        id,
        owner_user_id: owner_user_id.to_string(),
        tier: tier.to_string(),
        stripe_customer_id: None,
        created_at: now,
    })
}

/// Get monthly counter for billing account
pub async fn get_monthly_counter_for_billing_account(
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

/// Increment monthly counter for billing account
/// Returns true if increment succeeded (under limit), false if limit exceeded
pub async fn increment_monthly_counter_for_billing_account(
    db: &D1Database,
    billing_account_id: &str,
    year_month: &str,
    max_value: i64,
) -> Result<bool> {
    // Get current counter
    let current =
        get_monthly_counter_for_billing_account(db, billing_account_id, year_month).await?;

    // Check if we've hit the limit
    if current >= max_value {
        return Ok(false);
    }

    // Upsert: insert if not exists, increment if exists
    let stmt = db.prepare(
        "INSERT INTO monthly_counters (billing_account_id, year_month, links_created, updated_at)
         VALUES (?1, ?2, 1, ?3)
         ON CONFLICT(billing_account_id, year_month)
         DO UPDATE SET links_created = links_created + 1",
    );

    let now = now_timestamp();
    stmt.bind(&[
        billing_account_id.into(),
        year_month.into(),
        (now as f64).into(),
    ])?
    .run()
    .await?;

    Ok(true)
}

/// Count organizations in a billing account
pub async fn count_orgs_in_billing_account(
    db: &D1Database,
    billing_account_id: &str,
) -> Result<i64> {
    let stmt = db.prepare(
        "SELECT COUNT(*) as count
         FROM organizations
         WHERE billing_account_id = ?1",
    );

    let result = stmt
        .bind(&[billing_account_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    match result {
        Some(val) => Ok(val["count"].as_f64().unwrap_or(0.0) as i64),
        None => Ok(0),
    }
}

/// Reset monthly counter for billing account (admin only, for testing)
#[allow(dead_code)]
pub async fn reset_monthly_counter_for_billing_account(
    db: &D1Database,
    billing_account_id: &str,
    year_month: &str,
) -> Result<()> {
    let stmt = db.prepare(
        "DELETE FROM monthly_counters
         WHERE billing_account_id = ?1 AND year_month = ?2",
    );
    stmt.bind(&[billing_account_id.into(), year_month.into()])?
        .run()
        .await?;
    Ok(())
}

/// Update billing account owner
pub async fn update_billing_account_owner(
    db: &D1Database,
    billing_account_id: &str,
    new_owner_id: &str,
) -> Result<()> {
    let stmt = db.prepare("UPDATE billing_accounts SET owner_user_id = ?1 WHERE id = ?2");
    stmt.bind(&[new_owner_id.into(), billing_account_id.into()])?
        .run()
        .await?;
    Ok(())
}

/// Get billing account ID for an organization
pub async fn get_org_billing_account(db: &D1Database, org_id: &str) -> Result<Option<String>> {
    let stmt = db.prepare("SELECT billing_account_id FROM organizations WHERE id = ?1");

    let result = stmt
        .bind(&[org_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    match result {
        Some(val) => Ok(val["billing_account_id"].as_str().map(|s| s.to_string())),
        None => Ok(None),
    }
}

/// Create an organization linked to a specific billing account
pub async fn create_organization_with_billing_account(
    db: &D1Database,
    org_name: &str,
    created_by: &str,
    billing_account_id: &str,
) -> Result<Organization> {
    let org_id = uuid::Uuid::new_v4().to_string();
    let slug = generate_unique_slug(db, org_name).await?;
    let now = now_timestamp();

    // Get tier from billing account
    let billing_account = get_billing_account(db, billing_account_id)
        .await?
        .ok_or_else(|| Error::RustError("Billing account not found".to_string()))?;

    let stmt = db.prepare(
        "INSERT INTO organizations (id, name, slug, created_at, created_by, tier, billing_account_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    );

    stmt.bind(&[
        org_id.clone().into(),
        org_name.into(),
        slug.clone().into(),
        (now as f64).into(),
        created_by.into(),
        billing_account.tier.clone().into(),
        billing_account_id.into(),
    ])?
    .run()
    .await?;

    Ok(Organization {
        id: org_id,
        name: org_name.to_string(),
        slug,
        created_at: now,
        created_by: created_by.to_string(),
        tier: billing_account.tier,
        billing_account_id: Some(billing_account_id.to_string()),
    })
}

// ─── Admin Billing Account Queries ────────────────────────────────────────────

/// Response type for billing account list with stats
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BillingAccountWithStats {
    pub id: String,
    pub owner_user_id: String,
    pub owner_email: String,
    pub owner_name: Option<String>,
    pub tier: String,
    pub org_count: i64,
    pub total_members: i64,
    pub links_created_this_month: i64,
    pub created_at: i64,
}

/// Response type for org within a billing account
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OrgWithMembersCount {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub member_count: i64,
    pub link_count: i64,
    pub created_at: i64,
}

/// Response type for detailed billing account view
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BillingAccountDetails {
    pub account: BillingAccount,
    pub owner: User,
    pub organizations: Vec<OrgWithMembersCount>,
    pub usage: UsageStats,
}

/// Usage stats for billing account
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UsageStats {
    pub links_created_this_month: i64,
    pub max_links_per_month: Option<i64>,
    pub year_month: String,
}

/// List all billing accounts with statistics (admin only).
/// Returns paginated list with aggregated stats.
pub async fn list_billing_accounts_for_admin(
    db: &D1Database,
    page: i64,
    limit: i64,
    search: Option<&str>,
    tier_filter: Option<&str>,
) -> Result<(Vec<BillingAccountWithStats>, i64)> {
    let offset = (page - 1) * limit;
    let current_month = chrono::Utc::now().format("%Y-%m").to_string();

    // Build WHERE clause for filtering
    let mut where_clauses = vec![];
    let mut bind_values: Vec<JsValue> = vec![];

    if let Some(search_term) = search {
        where_clauses.push("u.email LIKE ?");
        bind_values.push(format!("%{}%", search_term).into());
    }

    if let Some(tier) = tier_filter {
        where_clauses.push("ba.tier = ?");
        bind_values.push(tier.into());
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    // Count total results
    let count_sql = format!(
        "SELECT COUNT(DISTINCT ba.id) as count
         FROM billing_accounts ba
         LEFT JOIN users u ON u.id = ba.owner_user_id
         {}",
        where_sql
    );
    let count_stmt = db.prepare(&count_sql);
    let count_result = if bind_values.is_empty() {
        count_stmt.first::<serde_json::Value>(None).await?
    } else {
        count_stmt
            .bind(&bind_values)?
            .first::<serde_json::Value>(None)
            .await?
    };
    let total = count_result
        .and_then(|v| v["count"].as_f64())
        .unwrap_or(0.0) as i64;

    // Fetch paginated results with aggregated stats
    let mut bind_values_with_limit = bind_values.clone();
    bind_values_with_limit.push(current_month.clone().into());
    bind_values_with_limit.push((limit as f64).into());
    bind_values_with_limit.push((offset as f64).into());

    let query_sql = format!(
        "SELECT ba.id, ba.owner_user_id, ba.tier, ba.created_at,
                u.email as owner_email, u.name as owner_name,
                COUNT(DISTINCT o.id) as org_count,
                COUNT(DISTINCT om.user_id) as total_members,
                COALESCE(mc.links_created, 0) as links_created_this_month
         FROM billing_accounts ba
         LEFT JOIN users u ON u.id = ba.owner_user_id
         LEFT JOIN organizations o ON o.billing_account_id = ba.id
         LEFT JOIN org_members om ON om.org_id = o.id
         LEFT JOIN monthly_counters mc ON mc.billing_account_id = ba.id AND mc.year_month = ?
         {}
         GROUP BY ba.id
         ORDER BY ba.created_at DESC
         LIMIT ? OFFSET ?",
        where_sql
    );

    let stmt = db.prepare(&query_sql);
    let results = stmt.bind(&bind_values_with_limit)?.all().await?;
    let rows = results.results::<serde_json::Value>()?;

    let accounts: Vec<BillingAccountWithStats> = rows
        .iter()
        .filter_map(|row| {
            Some(BillingAccountWithStats {
                id: row["id"].as_str()?.to_string(),
                owner_user_id: row["owner_user_id"].as_str()?.to_string(),
                owner_email: row["owner_email"].as_str()?.to_string(),
                owner_name: row["owner_name"].as_str().map(|s| s.to_string()),
                tier: row["tier"].as_str()?.to_string(),
                org_count: row["org_count"].as_f64()? as i64,
                total_members: row["total_members"].as_f64()? as i64,
                links_created_this_month: row["links_created_this_month"].as_f64()? as i64,
                created_at: row["created_at"].as_f64()? as i64,
            })
        })
        .collect();

    Ok((accounts, total))
}

/// Get detailed view of a single billing account with all orgs and users.
pub async fn get_billing_account_details(
    db: &D1Database,
    billing_account_id: &str,
) -> Result<Option<BillingAccountDetails>> {
    // Get billing account
    let account = match get_billing_account(db, billing_account_id).await? {
        Some(acc) => acc,
        None => return Ok(None),
    };

    // Get owner user
    let owner_stmt = db.prepare("SELECT * FROM users WHERE id = ?1");
    let owner = owner_stmt
        .bind(&[account.owner_user_id.clone().into()])?
        .first::<User>(None)
        .await?;

    let owner = match owner {
        Some(u) => u,
        None => return Ok(None),
    };

    // Get all organizations with member and link counts
    let orgs_stmt = db.prepare(
        "SELECT o.id, o.name, o.slug, o.created_at,
                COUNT(DISTINCT om.user_id) as member_count,
                COUNT(DISTINCT l.id) as link_count
         FROM organizations o
         LEFT JOIN org_members om ON om.org_id = o.id
         LEFT JOIN links l ON l.org_id = o.id AND l.status = 'active'
         WHERE o.billing_account_id = ?1
         GROUP BY o.id
         ORDER BY o.created_at ASC",
    );
    let orgs_result = orgs_stmt.bind(&[billing_account_id.into()])?.all().await?;
    let orgs_rows = orgs_result.results::<serde_json::Value>()?;
    let organizations: Vec<OrgWithMembersCount> = orgs_rows
        .iter()
        .filter_map(|row| {
            Some(OrgWithMembersCount {
                id: row["id"].as_str()?.to_string(),
                name: row["name"].as_str()?.to_string(),
                slug: row["slug"].as_str()?.to_string(),
                member_count: row["member_count"].as_f64()? as i64,
                link_count: row["link_count"].as_f64()? as i64,
                created_at: row["created_at"].as_f64()? as i64,
            })
        })
        .collect();

    // Get usage stats for current month
    let current_month = chrono::Utc::now().format("%Y-%m").to_string();
    let counter =
        get_monthly_counter_for_billing_account(db, billing_account_id, &current_month).await?;

    let tier =
        crate::models::Tier::from_str_value(&account.tier).unwrap_or(crate::models::Tier::Free);
    let limits = tier.limits();

    let usage = UsageStats {
        links_created_this_month: counter,
        max_links_per_month: limits.max_links_per_month,
        year_month: current_month,
    };

    Ok(Some(BillingAccountDetails {
        account,
        owner,
        organizations,
        usage,
    }))
}

/// Update billing account tier (admin only).
/// This will affect all organizations linked to this billing account.
pub async fn update_billing_account_tier(
    db: &D1Database,
    billing_account_id: &str,
    new_tier: &str,
) -> Result<()> {
    // Update billing account tier
    let stmt = db.prepare("UPDATE billing_accounts SET tier = ?1 WHERE id = ?2");
    stmt.bind(&[new_tier.into(), billing_account_id.into()])?
        .run()
        .await?;

    // Update all organizations linked to this billing account
    let org_stmt = db.prepare("UPDATE organizations SET tier = ?1 WHERE billing_account_id = ?2");
    org_stmt
        .bind(&[new_tier.into(), billing_account_id.into()])?
        .run()
        .await?;

    Ok(())
}
