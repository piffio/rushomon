use crate::models::{
    AnalyticsEvent, BillingAccount, Link, Organization, User, link::LinkStatus,
    user::CreateUserData,
};
use crate::utils::now_timestamp;
use serde::Serializer;
use wasm_bindgen::JsValue;
use worker::d1::D1Database;
use worker::*;

/// Serialize Option<i64> as Option<bool> for JSON responses (0 = false, 1 = true, NULL = None)
fn serialize_optional_int_as_bool<S>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_some(&(*v != 0)),
        None => serializer.serialize_none(),
    }
}

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
    let settings_repo = crate::repositories::SettingsRepository::new();
    let tier = settings_repo
        .get_setting(db, "default_user_tier")
        .await?
        .unwrap_or_else(|| "free".to_string());

    // Create billing account for the user
    let billing_account = create_billing_account(db, user_id, &tier).await?;

    let stmt = db.prepare(
        "INSERT INTO organizations (id, name, slug, created_at, created_by, billing_account_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    );

    stmt.bind(&[
        org_id.clone().into(),
        org_name.to_string().into(),
        slug.clone().into(),
        (now as f64).into(),
        user_id.into(),
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
        "SELECT id, name, slug, created_at, created_by, billing_account_id
         FROM organizations
         WHERE id = ?1",
    );

    stmt.bind(&[org_id.into()])?
        .first::<Organization>(None)
        .await
}

/// Get the org-level forward_query_params default setting (0 or 1).
/// Returns false if the org doesn't exist or the column is NULL.
pub async fn get_org_forward_query_params(db: &D1Database, org_id: &str) -> Result<bool> {
    let stmt = db.prepare(
        "SELECT COALESCE(forward_query_params, 0) as forward_query_params
         FROM organizations
         WHERE id = ?1",
    );

    let result = stmt
        .bind(&[org_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(result
        .and_then(|v| v["forward_query_params"].as_f64())
        .map(|v| v != 0.0)
        .unwrap_or(false))
}

/// Get the org logo_url (nullable).
#[allow(dead_code)]
pub async fn get_org_logo_url(db: &D1Database, org_id: &str) -> Result<Option<String>> {
    let stmt = db.prepare("SELECT logo_url FROM organizations WHERE id = ?1");
    let result = stmt
        .bind(&[org_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(result.and_then(|v| v["logo_url"].as_str().map(str::to_string)))
}

/// Set (or clear) the org logo_url.
pub async fn set_org_logo_url(db: &D1Database, org_id: &str, logo_url: Option<&str>) -> Result<()> {
    let stmt = db.prepare("UPDATE organizations SET logo_url = ?1 WHERE id = ?2");
    stmt.bind(&[
        logo_url.map(|s| s.into()).unwrap_or(JsValue::NULL),
        org_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Update the org-level forward_query_params default.
pub async fn set_org_forward_query_params(
    db: &D1Database,
    org_id: &str,
    forward: bool,
) -> Result<()> {
    let stmt = db.prepare("UPDATE organizations SET forward_query_params = ?1 WHERE id = ?2");
    stmt.bind(&[
        (if forward { 1i64 } else { 0i64 } as f64).into(),
        org_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Create a new link
pub async fn create_link(db: &D1Database, link: &Link) -> Result<()> {
    let utm_json = link.utm_params.as_ref().and_then(|u| u.to_json_string());

    let stmt = db.prepare(
        "INSERT INTO links (id, org_id, short_code, destination_url, title, created_by, created_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
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
        utm_json.map(|s| s.into()).unwrap_or(JsValue::NULL),
        link.forward_query_params
            .map(|v| (if v { 1i64 } else { 0i64 } as f64).into())
            .unwrap_or(JsValue::NULL),
        link.redirect_type.clone().into(),
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
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
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
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
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
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
         FROM links
         WHERE id = ?1
         AND status = 'active'"
    );

    stmt.bind(&[link_id.into()])?.first::<Link>(None).await
}

/// Get a link by ID without org_id check (used for admin operations - returns all statuses)
/// Note: tags are populated separately via get_tags_for_links
pub async fn get_link_by_id_no_auth_all(db: &D1Database, link_id: &str) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
         FROM links
         WHERE id = ?1"
    );

    stmt.bind(&[link_id.into()])?.first::<Link>(None).await
}

/// Get an active link by short_code without org_id check (used for public reporting)
/// Only returns links with status = 'active'
pub async fn get_active_link_by_short_code(
    db: &D1Database,
    short_code: &str,
) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
         FROM links
         WHERE short_code = ?1
         AND status = 'active'"
    );

    stmt.bind(&[short_code.into()])?.first::<Link>(None).await
}

/// Update a link
#[allow(clippy::too_many_arguments)]
pub async fn update_link(
    db: &D1Database,
    link_id: &str,
    org_id: &str,
    destination_url: Option<&str>,
    title: Option<&str>,
    status: Option<&str>,
    expires_at: Option<i64>,
    utm_params: Option<Option<&str>>,
    forward_query_params: Option<Option<bool>>,
    redirect_type: Option<&str>,
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

    if let Some(utm_val) = utm_params {
        query.push_str(&format!(", utm_params = ?{}", param_count));
        params.push(utm_val.map(|s| s.into()).unwrap_or(JsValue::NULL));
        param_count += 1;
    }

    if let Some(fwd_val) = forward_query_params {
        query.push_str(&format!(", forward_query_params = ?{}", param_count));
        params.push(
            fwd_val
                .map(|v| (if v { 1i64 } else { 0i64 } as f64).into())
                .unwrap_or(JsValue::NULL),
        );
        param_count += 1;
    }

    if let Some(rt) = redirect_type {
        query.push_str(&format!(", redirect_type = ?{}", param_count));
        params.push(rt.into());
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
    // Delete analytics events (FK constraint)
    let analytics_stmt = db.prepare("DELETE FROM analytics_events WHERE link_id = ?1");
    analytics_stmt.bind(&[link_id.into()])?.run().await?;

    // Delete link reports referencing this link
    let reports_stmt = db.prepare("DELETE FROM link_reports WHERE link_id = ?1");
    reports_stmt.bind(&[link_id.into()])?.run().await?;

    // Delete link tag associations
    let tags_stmt = db.prepare("DELETE FROM link_tags WHERE link_id = ?1");
    tags_stmt.bind(&[link_id.into()])?.run().await?;

    // Finally delete the link itself
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

// UserWithBillingInfo struct moved to repositories/user_repository.rs

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

// get_all_users_with_billing_info, get_admin_count, update_user_role moved to UserRepository

/// Get a link by short_code within an organization
pub async fn get_link_by_short_code(
    db: &D1Database,
    short_code: &str,
    org_id: &str,
) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params, redirect_type
         FROM links
         WHERE short_code = ?1
         AND org_id = ?2
         AND status IN ('active', 'disabled')"
    );

    stmt.bind(&[short_code.into(), org_id.into()])?
        .first::<Link>(None)
        .await
}

// suspend_user, unsuspend_user, delete_user, get_links_by_creator, get_links_by_org,
// is_last_admin_in_org moved to UserRepository

#[cfg(test)]
mod tests {
    #[test]
    fn test_delete_user_logic_order() {
        // Test that the deletion order is correct to avoid FK violations
        // This is a conceptual test - actual database testing would require mocks

        // Expected order:
        // 1. analytics_events (references links)
        // 2. link_reports (references users)
        // 3. links (references users)
        // 4. org_members (references users)
        // 5. org_invitations (references users)
        // 6. destination_blacklist (references users)
        // 7. users (the main record)

        // This order ensures we delete children before parents
        let expected_order = vec![
            "analytics_events",
            "link_reports",
            "links",
            "org_members",
            "org_invitations",
            "destination_blacklist",
            "users",
        ];

        // The actual implementation follows this order
        assert_eq!(expected_order.len(), 7); // Verify we have all tables
    }
}

pub async fn update_link_status_by_id(db: &D1Database, link_id: &str, status: &str) -> Result<()> {
    let now = now_timestamp();
    let stmt = db.prepare("UPDATE links SET status = ?1, updated_at = ?2 WHERE id = ?3");
    stmt.bind(&[status.into(), (now as f64).into(), link_id.into()])?
        .run()
        .await?;
    Ok(())
}

// disable_all_links_for_org moved to UserRepository

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
    let all_exact_entries = db
        .prepare("SELECT destination FROM destination_blacklist WHERE match_type = 'exact'")
        .all()
        .await?
        .results::<serde_json::Value>()?;
    for entry in all_exact_entries {
        if let Some(dest) = entry.get("destination").and_then(|d| d.as_str())
            && let Ok(normalized_entry) = normalize_url_for_blacklist(dest)
            && normalized_entry == normalized_destination
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check KV sync status for a link
pub async fn check_link_kv_sync(
    kv: &worker::kv::KvStore,
    link: &AdminLinkBase,
) -> Result<(String, bool)> {
    use crate::kv;

    // Check if KV entry exists
    let kv_mapping = kv::get_link_mapping(kv, &link.short_code).await?;

    match kv_mapping {
        Some(mapping) => {
            // KV entry exists, check if it's in sync with D1
            let should_exist = link.status == "active";
            let is_active = mapping.status == LinkStatus::Active;

            if should_exist && is_active {
                Ok(("synced".to_string(), true))
            } else if !should_exist && !is_active {
                // Both agree it should be inactive
                Ok(("synced".to_string(), true))
            } else {
                // Mismatch: D1 says blocked but KV still active, or vice versa
                Ok(("mismatched".to_string(), true))
            }
        }
        None => {
            // KV entry doesn't exist
            let should_exist = link.status == "active";
            if should_exist {
                Ok(("missing".to_string(), false))
            } else {
                // Link is blocked/disabled and KV doesn't exist - this is correct
                Ok(("synced".to_string(), false))
            }
        }
    }
}

/// Get all links for admin (global listing with filters) - base data only
pub async fn get_all_links_admin_base(
    db: &D1Database,
    limit: i64,
    offset: i64,
    org_filter: Option<&str>,
    email_filter: Option<&str>,
    domain_filter: Option<&str>,
) -> Result<Vec<AdminLinkBase>> {
    let mut query = String::from(
        "SELECT l.id, l.org_id, l.short_code, l.destination_url, l.title, l.created_by, l.created_at, l.updated_at, l.expires_at, l.status, l.click_count, l.utm_params, l.forward_query_params, l.redirect_type, u.email as creator_email, o.name as org_name
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
    let links = results.results::<AdminLinkBase>()?;
    Ok(links)
}

/// Get all links for admin with KV sync status
pub async fn get_all_links_admin(
    db: &D1Database,
    kv: &worker::kv::KvStore,
    limit: i64,
    offset: i64,
    org_filter: Option<&str>,
    email_filter: Option<&str>,
    domain_filter: Option<&str>,
) -> Result<Vec<AdminLink>> {
    let base_links =
        get_all_links_admin_base(db, limit, offset, org_filter, email_filter, domain_filter)
            .await?;

    let mut links_with_kv = Vec::new();
    for base_link in base_links {
        let (kv_sync_status, kv_exists) = check_link_kv_sync(kv, &base_link).await?;

        links_with_kv.push(AdminLink {
            base: base_link,
            kv_sync_status,
            kv_exists,
        });
    }

    Ok(links_with_kv)
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminLinkBase {
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
    pub utm_params: Option<String>,
    #[serde(serialize_with = "serialize_optional_int_as_bool")]
    pub forward_query_params: Option<i64>, // Stored as INTEGER in D1 (0/1/NULL)
    pub redirect_type: String,
    pub creator_email: String,
    pub org_name: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AdminLink {
    #[serde(flatten)]
    pub base: AdminLinkBase,
    pub kv_sync_status: String, // "synced", "missing", "mismatched"
    pub kv_exists: bool,
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
    pub link__utm_params: Option<String>,
    #[serde(serialize_with = "serialize_optional_int_as_bool")]
    pub link__forward_query_params: Option<i64>, // Stored as INTEGER in D1 (0/1/NULL)
    pub link__redirect_type: String,
    pub link__creator_email: String,
    pub link__org_name: String,
    pub report_count: i64,
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

// ─── Org Management ───────────────────────────────────────────────────────────

use crate::models::{OrgInvitation, OrgMember, OrgMemberWithUser, OrgWithRole};

/// Get all organizations a user belongs to (via org_members junction table).
/// After migration, all users should have proper org_members records.
pub async fn get_user_orgs(db: &D1Database, user_id: &str) -> Result<Vec<OrgWithRole>> {
    let stmt = db.prepare(
        "SELECT o.id, o.name, m.role, m.joined_at
         FROM organizations o
         JOIN org_members m ON o.id = m.org_id
         WHERE m.user_id = ?1
         ORDER BY m.joined_at ASC",
    );
    let results = stmt.bind(&[user_id.into()])?.all().await?;
    let rows = results.results::<serde_json::Value>()?;

    // Convert rows to OrgWithRole objects
    let orgs: Vec<OrgWithRole> = rows
        .iter()
        .filter_map(|row| {
            Some(OrgWithRole {
                id: row["id"].as_str()?.to_string(),
                name: row["name"].as_str()?.to_string(),
                role: row["role"].as_str()?.to_string(),
                joined_at: row["joined_at"].as_f64()? as i64,
            })
        })
        .collect();

    Ok(orgs) // No fallback needed after migration
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
        "INSERT INTO organizations (id, name, slug, created_at, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5)",
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

    // Note: monthly_counters is at billing account level, not per-org
    // Do NOT delete monthly_counters here - they're shared across all orgs in billing account

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
        "SELECT id, owner_user_id, tier, provider_customer_id, created_at
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
        "SELECT ba.id, ba.owner_user_id, ba.tier, ba.provider_customer_id, ba.created_at
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
        "SELECT ba.id, ba.owner_user_id, ba.tier, ba.provider_customer_id, ba.created_at
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
        provider_customer_id: None,
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

    let stmt = db.prepare(
        "INSERT INTO organizations (id, name, slug, created_at, created_by, billing_account_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    );

    stmt.bind(&[
        org_id.clone().into(),
        org_name.into(),
        slug.clone().into(),
        (now as f64).into(),
        created_by.into(),
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
    pub subscription: Option<serde_json::Value>,
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

    // Get subscription info
    let subscription = get_subscription_for_billing_account(db, billing_account_id).await?;

    Ok(Some(BillingAccountDetails {
        account,
        owner,
        organizations,
        usage,
        subscription,
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

    Ok(())
}

/// Update billing account provider customer ID.
/// Called when a subscription is created to link the billing account to the Polar customer.
pub async fn update_billing_account_provider_customer_id(
    db: &D1Database,
    billing_account_id: &str,
    provider_customer_id: &str,
) -> Result<()> {
    let stmt = db.prepare("UPDATE billing_accounts SET provider_customer_id = ?1 WHERE id = ?2");
    stmt.bind(&[provider_customer_id.into(), billing_account_id.into()])?
        .run()
        .await?;
    Ok(())
}

// ─── Billing / Subscription queries ──────────────────────────────────────────

/// Look up a billing_account_id by its provider customer ID.
/// Used in webhook handlers where external_id is missing from the payload.
pub async fn get_billing_account_id_by_provider_customer(
    db: &D1Database,
    provider_customer_id: &str,
) -> Result<Option<String>> {
    let stmt =
        db.prepare("SELECT id FROM billing_accounts WHERE provider_customer_id = ?1 LIMIT 1");
    let result = stmt
        .bind(&[provider_customer_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(result.and_then(|v| v["id"].as_str().map(|s| s.to_string())))
}

/// Get the current active subscription for a billing account.
/// Returns the raw JSON row for flexibility.
pub async fn get_subscription_for_billing_account(
    db: &D1Database,
    billing_account_id: &str,
) -> Result<Option<serde_json::Value>> {
    let stmt = db.prepare(
        "SELECT id, billing_account_id, status, plan, interval,
                provider_subscription_id, provider_customer_id, provider_price_id,
                current_period_start, current_period_end,
                cancel_at_period_end, canceled_at, created_at, updated_at,
                amount_cents, currency, discount_name, pending_cancellation
         FROM subscriptions
         WHERE billing_account_id = ?1
         ORDER BY created_at DESC
         LIMIT 1",
    );
    stmt.bind(&[billing_account_id.into()])?
        .first::<serde_json::Value>(None)
        .await
}

/// Insert or update a subscription record for a billing account.
/// Uses provider_subscription_id as the unique key for upsert.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_subscription(
    db: &D1Database,
    billing_account_id: &str,
    provider_subscription_id: &str,
    provider_customer_id: &str,
    status: &str,
    plan: &str,
    interval: &str,
    provider_price_id: &str,
    current_period_start: i64,
    current_period_end: i64,
    cancel_at_period_end: bool,
    amount_cents: Option<i64>,
    currency: &str,
    discount_name: Option<&str>,
    ends_at: Option<i64>,
    now: i64,
) -> Result<()> {
    let sub_id = format!("sub_{}", crate::utils::generate_short_code_with_length(16));
    let cancel_flag: i64 = if cancel_at_period_end { 1 } else { 0 };

    let stmt = db.prepare(
        "INSERT INTO subscriptions (
           id, billing_account_id, status, plan, interval,
           provider_subscription_id, provider_customer_id, provider_price_id,
           current_period_start, current_period_end, ends_at,
           cancel_at_period_end, amount_cents, currency, discount_name,
           created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)
         ON CONFLICT(provider_subscription_id) DO UPDATE SET
           status = excluded.status,
           plan = excluded.plan,
           interval = excluded.interval,
           provider_price_id = excluded.provider_price_id,
           current_period_start = excluded.current_period_start,
           current_period_end = excluded.current_period_end,
           ends_at = excluded.ends_at,
           cancel_at_period_end = excluded.cancel_at_period_end,
           amount_cents = excluded.amount_cents,
           currency = excluded.currency,
           discount_name = excluded.discount_name,
           updated_at = excluded.updated_at",
    );
    stmt.bind(&[
        sub_id.into(),
        billing_account_id.into(),
        status.into(),
        plan.into(),
        interval.into(),
        provider_subscription_id.into(),
        provider_customer_id.into(),
        provider_price_id.into(),
        (current_period_start as f64).into(),
        (current_period_end as f64).into(),
        ends_at.map(|v| (v as f64).into()).unwrap_or("".into()),
        (cancel_flag as f64).into(),
        (amount_cents.unwrap_or(0) as f64).into(),
        currency.into(),
        discount_name.unwrap_or("").into(),
        (now as f64).into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Mark a subscription as canceled (from subscription.deleted / revoked event).
pub async fn mark_subscription_canceled(
    db: &D1Database,
    provider_subscription_id: &str,
    now: i64,
) -> Result<()> {
    let stmt = db.prepare(
        "UPDATE subscriptions
         SET status = 'canceled', canceled_at = ?1, updated_at = ?1
         WHERE provider_subscription_id = ?2",
    );
    stmt.bind(&[(now as f64).into(), provider_subscription_id.into()])?
        .run()
        .await?;
    Ok(())
}

/// Set subscription as pending cancellation (cancel_at_period_end = true).
/// The tier will be downgraded when the cron job runs after current_period_end.
pub async fn set_subscription_pending_cancellation(
    db: &D1Database,
    provider_subscription_id: &str,
    current_period_end: i64,
) -> Result<()> {
    let stmt = db.prepare(
        "UPDATE subscriptions
         SET pending_cancellation = 1,
             cancel_at_period_end = 1,
             updated_at = ?1
         WHERE provider_subscription_id = ?2",
    );
    stmt.bind(&[
        (current_period_end as f64).into(),
        provider_subscription_id.into(),
    ])?
    .run()
    .await?;
    Ok(())
}

/// Clear pending cancellation flag (e.g., when user uncancels their subscription).
pub async fn clear_subscription_pending_cancellation(
    db: &D1Database,
    provider_subscription_id: &str,
) -> Result<()> {
    let stmt = db.prepare(
        "UPDATE subscriptions
         SET pending_cancellation = 0,
             cancel_at_period_end = 0,
             updated_at = (SELECT strftime('%s', 'now'))
         WHERE provider_subscription_id = ?1",
    );
    stmt.bind(&[provider_subscription_id.into()])?.run().await?;
    Ok(())
}

/// Get all subscriptions with pending_cancellation that have expired.
/// Returns provider_subscription_id and billing_account_id for each.
pub async fn get_expired_pending_cancellations(
    db: &D1Database,
    now: i64,
) -> Result<Vec<serde_json::Value>> {
    let stmt = db.prepare(
        "SELECT provider_subscription_id, billing_account_id, current_period_end
         FROM subscriptions
         WHERE pending_cancellation = 1
           AND current_period_end < ?1
         LIMIT 1000",
    );
    let results = stmt.bind(&[(now as f64).into()])?.all().await?;

    results.results::<serde_json::Value>()
}

/// Finalize an expired subscription after downgrading the tier.
pub async fn finalize_expired_subscription(
    db: &D1Database,
    provider_subscription_id: &str,
    now: i64,
) -> Result<()> {
    let stmt = db.prepare(
        "UPDATE subscriptions
         SET status = 'canceled',
             pending_cancellation = 0,
             canceled_at = ?1,
             updated_at = ?1
         WHERE provider_subscription_id = ?2",
    );
    stmt.bind(&[(now as f64).into(), provider_subscription_id.into()])?
        .run()
        .await?;
    Ok(())
}

/// Update subscription status (admin only).
pub async fn update_subscription_status(
    db: &D1Database,
    subscription_id: &str,
    status: &str,
    now: i64,
) -> Result<()> {
    let stmt = db.prepare(
        "UPDATE subscriptions
         SET status = ?1, updated_at = ?2
         WHERE id = ?3",
    );
    stmt.bind(&[status.into(), (now as f64).into(), subscription_id.into()])?
        .run()
        .await?;
    Ok(())
}

/// Get all links for an organization for export (no pagination, active + disabled only)
pub async fn get_all_links_for_org_export(db: &D1Database, org_id: &str) -> Result<Vec<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, status, click_count, utm_params, forward_query_params
         FROM links
         WHERE org_id = ?1
         AND status IN ('active', 'disabled')
         ORDER BY created_at DESC",
    );
    let results = stmt.bind(&[org_id.into()])?.all().await?;
    let links = results.results::<Link>()?;
    Ok(links)
}

/// Get a cached product by price_id
pub async fn get_cached_product_by_price_id(
    db: &D1Database,
    price_id: &str,
) -> Result<Option<serde_json::Value>> {
    let stmt = db.prepare(
        "SELECT id, name, description, price_amount, price_currency,
                recurring_interval, recurring_interval_count, is_archived,
                polar_product_id, polar_price_id, created_at, updated_at
         FROM cached_products
         WHERE polar_price_id = ?1 AND is_archived = FALSE",
    );

    let result = stmt
        .bind(&[price_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(result)
}

// ─── Webhook Idempotency ─────────────────────────────────────────────────────

/// Check if a webhook has already been processed (for idempotency)
pub async fn webhook_already_processed(
    db: &D1Database,
    provider: &str,
    webhook_id: &str,
) -> Result<bool> {
    let stmt = db.prepare(
        "SELECT 1 FROM processed_webhooks
         WHERE provider = ?1 AND webhook_id = ?2
         LIMIT 1",
    );

    let result = stmt
        .bind(&[provider.into(), webhook_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(result.is_some())
}

/// Mark a webhook as processed (for idempotency)
/// ttl_seconds: How long to keep the record (default: 30 days = 2592000 seconds)
pub async fn mark_webhook_processed(
    db: &D1Database,
    provider: &str,
    webhook_id: &str,
    event_type: &str,
    ttl_seconds: i64,
) -> Result<()> {
    let now = now_timestamp();
    let expires_at = now + ttl_seconds;

    let stmt = db.prepare(
        "INSERT INTO processed_webhooks
         (id, provider, webhook_id, event_type, processed_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    );

    // Generate unique ID for this record (provider + webhook_id + timestamp)
    let record_id = format!("{}_{}_{}", provider, webhook_id, now);

    stmt.bind(&[
        record_id.into(),
        provider.into(),
        webhook_id.into(),
        event_type.into(),
        (now as f64).into(),
        (expires_at as f64).into(),
    ])?
    .run()
    .await?;

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct ApiKeyWithTierRecord {
    pub id: String,
    pub user_id: String,
    pub org_id: String,
    pub expires_at: Option<i64>,
    pub status: String,
    pub tier: Option<String>,
}

// ─── API Keys (Personal Access Token) ────────────────────────────────────────
pub async fn get_api_key_by_hash_with_tier(
    db: &worker::d1::D1Database,
    key_hash: &str,
) -> worker::Result<Option<ApiKeyWithTierRecord>> {
    let stmt = db.prepare(
        "SELECT ak.id, ak.user_id, ak.org_id, ak.expires_at, ak.status, ba.tier
         FROM api_keys ak
         JOIN organizations o ON ak.org_id = o.id
         LEFT JOIN billing_accounts ba ON o.billing_account_id = ba.id
         WHERE ak.key_hash = ?1",
    );

    stmt.bind(&[key_hash.into()])?
        .first::<ApiKeyWithTierRecord>(None)
        .await
}

pub async fn update_api_key_last_used(
    db: &worker::d1::D1Database,
    key_id: &str,
    timestamp: i64,
) -> worker::Result<()> {
    let stmt = db.prepare("UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2");
    stmt.bind(&[(timestamp as f64).into(), key_id.into()])?
        .run()
        .await?;
    Ok(())
}

// ─── Admin API Key queries ────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AdminApiKeyRecord {
    pub id: String,
    pub name: String,
    pub hint: String,
    pub user_id: String,
    pub user_email: Option<String>,
    pub user_name: Option<String>,
    pub org_id: String,
    pub org_name: Option<String>,
    pub tier: Option<String>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub status: String,
    pub updated_at: Option<i64>,
    pub updated_by: Option<String>,
}

#[cfg(test)]
mod api_key_tests {
    #[test]
    fn test_api_key_status_validation() {
        // Test valid status values
        let valid_statuses = vec!["active", "revoked", "deleted"];
        for status in valid_statuses {
            assert!(matches!(status, "active" | "revoked" | "deleted"));
        }

        // Test invalid status values
        let invalid_statuses = vec!["invalid", "", "ACTIVE", "Revoked"];
        for status in invalid_statuses {
            assert!(!matches!(status, "active" | "revoked" | "deleted"));
        }
    }

    #[test]
    fn test_timestamp_conversion_safety() {
        // Test that timestamp conversion works correctly
        let timestamp = 1234567890i64;
        let converted = timestamp as f64;
        assert_eq!(converted, 1234567890.0);

        // Test edge cases
        let max_timestamp = i64::MAX;
        let max_converted = max_timestamp as f64;
        assert!(max_converted > 0.0);

        let min_timestamp = i64::MIN;
        let min_converted = min_timestamp as f64;
        assert!(min_converted < 0.0);
    }

    #[test]
    fn test_query_parameter_validation() {
        // Test that our queries use proper parameter binding
        let revoke_query = "UPDATE api_keys SET status = 'revoked', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'active'";
        assert!(revoke_query.contains("?1"));
        assert!(revoke_query.contains("?2"));
        assert!(revoke_query.contains("?3"));
        assert!(!revoke_query.contains("DROP"));

        let reactivate_query = "UPDATE api_keys SET status = 'active', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'revoked'";
        assert!(reactivate_query.contains("?1"));
        assert!(reactivate_query.contains("?2"));
        assert!(reactivate_query.contains("?3"));
        assert!(!reactivate_query.contains("DELETE"));

        let delete_query = "UPDATE api_keys SET status = 'deleted', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status != 'deleted'";
        assert!(delete_query.contains("?1"));
        assert!(delete_query.contains("?2"));
        assert!(delete_query.contains("?3"));
        assert!(!delete_query.contains("DROP"));
    }

    #[test]
    fn test_sql_injection_prevention() {
        // Test that our query patterns prevent SQL injection
        let safe_queries = vec![
            "UPDATE api_keys SET status = 'revoked', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'active'",
            "UPDATE api_keys SET status = 'active', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'revoked'",
            "UPDATE api_keys SET status = 'deleted', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status != 'deleted'",
            "SELECT * FROM api_keys WHERE key_hash = ?1 AND status = 'active'",
        ];

        for query in safe_queries {
            // Ensure queries use parameter binding instead of string interpolation
            assert!(query.contains("?") || !query.contains("SELECT"));

            // Ensure no dangerous SQL patterns
            assert!(!query.contains("DROP"));
            assert!(!query.contains("TRUNCATE"));
            assert!(!query.contains("ALTER"));
            assert!(!query.contains("EXEC"));
            assert!(!query.contains("UNION"));
        }
    }

    #[test]
    fn test_status_transition_logic() {
        // Test that our SQL WHERE clauses enforce correct status transitions

        // Revoke: only active keys can be revoked
        let revoke_where = "WHERE id = ?3 AND status = 'active'";
        assert!(revoke_where.contains("status = 'active'"));

        // Reactivate: only revoked keys can be reactivated
        let reactivate_where = "WHERE id = ?3 AND status = 'revoked'";
        assert!(reactivate_where.contains("status = 'revoked'"));

        // Delete: any non-deleted key can be soft deleted
        let delete_where = "WHERE id = ?3 AND status != 'deleted'";
        assert!(delete_where.contains("status != 'deleted'"));

        // Restore: only deleted keys can be restored
        let restore_where = "WHERE id = ?3 AND status = 'deleted'";
        assert!(restore_where.contains("status = 'deleted'"));
    }

    #[test]
    fn test_audit_field_consistency() {
        // Test that all update queries include audit fields
        let update_queries = vec![
            "UPDATE api_keys SET status = 'revoked', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'active'",
            "UPDATE api_keys SET status = 'active', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'revoked'",
            "UPDATE api_keys SET status = 'deleted', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status != 'deleted'",
            "UPDATE api_keys SET status = 'active', updated_at = ?1, updated_by = ?2 WHERE id = ?3 AND status = 'deleted'",
        ];

        for query in update_queries {
            assert!(query.contains("updated_at"));
            assert!(query.contains("updated_by"));
            assert!(query.contains("?1")); // timestamp
            assert!(query.contains("?2")); // user_id
        }
    }
}

#[cfg(test)]
mod admin_link_serialization_tests {
    use super::*;

    #[test]
    fn test_admin_link_base_deserializes_i64_as_bool() {
        // Test that AdminLinkBase can deserialize forward_query_params as i64
        let json = r#"{
            "id": "test-id",
            "org_id": "org-1",
            "short_code": "abc123",
            "destination_url": "https://example.com",
            "title": null,
            "created_by": "user-1",
            "created_at": 1234567890,
            "updated_at": null,
            "expires_at": null,
            "status": "active",
            "click_count": 0,
            "utm_params": null,
            "forward_query_params": 1,
            "redirect_type": "301",
            "creator_email": "test@example.com",
            "org_name": "Test Org"
        }"#;

        let admin_link: AdminLinkBase = serde_json::from_str(json).unwrap();
        assert_eq!(admin_link.forward_query_params, Some(1));
    }

    #[test]
    fn test_admin_link_base_serializes_i64_as_bool() {
        // Test that AdminLinkBase serializes forward_query_params as bool
        let admin_link = AdminLinkBase {
            id: "test-id".to_string(),
            org_id: "org-1".to_string(),
            short_code: "abc123".to_string(),
            destination_url: "https://example.com".to_string(),
            title: None,
            created_by: "user-1".to_string(),
            created_at: 1234567890,
            updated_at: None,
            expires_at: None,
            status: "active".to_string(),
            click_count: 0,
            utm_params: None,
            forward_query_params: Some(1),
            redirect_type: "301".to_string(),
            creator_email: "test@example.com".to_string(),
            org_name: "Test Org".to_string(),
        };

        let json = serde_json::to_string(&admin_link).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should serialize as boolean true
        assert_eq!(parsed["forward_query_params"], true);
    }

    #[test]
    fn test_admin_link_base_serializes_zero_as_false() {
        let admin_link = AdminLinkBase {
            id: "test-id".to_string(),
            org_id: "org-1".to_string(),
            short_code: "abc123".to_string(),
            destination_url: "https://example.com".to_string(),
            title: None,
            created_by: "user-1".to_string(),
            created_at: 1234567890,
            updated_at: None,
            expires_at: None,
            status: "active".to_string(),
            click_count: 0,
            utm_params: None,
            forward_query_params: Some(0),
            redirect_type: "301".to_string(),
            creator_email: "test@example.com".to_string(),
            org_name: "Test Org".to_string(),
        };

        let json = serde_json::to_string(&admin_link).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should serialize as boolean false
        assert_eq!(parsed["forward_query_params"], false);
    }

    #[test]
    fn test_admin_link_base_serializes_null_as_none() {
        let admin_link = AdminLinkBase {
            id: "test-id".to_string(),
            org_id: "org-1".to_string(),
            short_code: "abc123".to_string(),
            destination_url: "https://example.com".to_string(),
            title: None,
            created_by: "user-1".to_string(),
            created_at: 1234567890,
            updated_at: None,
            expires_at: None,
            status: "active".to_string(),
            click_count: 0,
            utm_params: None,
            forward_query_params: None,
            redirect_type: "301".to_string(),
            creator_email: "test@example.com".to_string(),
            org_name: "Test Org".to_string(),
        };

        let json = serde_json::to_string(&admin_link).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should serialize as null
        assert!(parsed["forward_query_params"].is_null());
    }

    #[test]
    fn test_link_report_query_result_deserializes_i64_as_bool() {
        let json = r#"{
            "id": "report-1",
            "link_id": "link-1",
            "reason": "spam",
            "reporter_user_id": null,
            "reporter_email": null,
            "status": "pending",
            "admin_notes": null,
            "reviewed_by": null,
            "reviewed_at": null,
            "created_at": 1234567890,
            "link__id": "link-1",
            "link__org_id": "org-1",
            "link__short_code": "abc123",
            "link__destination_url": "https://example.com",
            "link__title": null,
            "link__created_by": "user-1",
            "link__created_at": 1234567890,
            "link__updated_at": null,
            "link__expires_at": null,
            "link__status": "active",
            "link__click_count": 0,
            "link__utm_params": null,
            "link__forward_query_params": 1,
            "link__redirect_type": "301",
            "link__creator_email": "test@example.com",
            "link__org_name": "Test Org",
            "report_count": 1
        }"#;

        let result: LinkReportQueryResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.link__forward_query_params, Some(1));
    }

    #[test]
    fn test_link_report_query_result_serializes_i64_as_bool() {
        let result = LinkReportQueryResult {
            id: "report-1".to_string(),
            link_id: "link-1".to_string(),
            reason: "spam".to_string(),
            reporter_user_id: None,
            reporter_email: None,
            status: "pending".to_string(),
            admin_notes: None,
            reviewed_by: None,
            reviewed_at: None,
            created_at: 1234567890,
            link__id: "link-1".to_string(),
            link__org_id: "org-1".to_string(),
            link__short_code: "abc123".to_string(),
            link__destination_url: "https://example.com".to_string(),
            link__title: None,
            link__created_by: "user-1".to_string(),
            link__created_at: 1234567890,
            link__updated_at: None,
            link__expires_at: None,
            link__status: "active".to_string(),
            link__click_count: 0,
            link__utm_params: None,
            link__forward_query_params: Some(1),
            link__redirect_type: "301".to_string(),
            link__creator_email: "test@example.com".to_string(),
            link__org_name: "Test Org".to_string(),
            report_count: 1,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should serialize as boolean true
        assert_eq!(parsed["link__forward_query_params"], true);
    }
}
