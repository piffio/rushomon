use crate::models::{AnalyticsEvent, Link, Organization, User, user::CreateUserData};
use worker::d1::D1Database;
use worker::*;

/// Create or update a user from OAuth data
pub async fn create_or_update_user(
    db: &D1Database,
    data: CreateUserData,
    org_id: &str,
) -> Result<User> {
    let user_id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

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
        })
    } else {
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
                "admin".into(), // First user in org is admin
                now.into(),
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
            role: "admin".to_string(),
            created_at: now,
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

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let stmt = db.prepare(
        "INSERT INTO organizations (id, name, slug, created_at, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    );

    stmt.bind(&[
        org_id.clone().into(),
        org_name.into(),
        slug.clone().into(),
        now.into(),
        user_id.into(),
    ])?
    .run()
    .await?;

    Ok(Organization {
        id: org_id,
        name: org_name.to_string(),
        slug,
        created_at: now,
        created_by: user_id.to_string(),
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
        "SELECT id, name, slug, created_at, created_by
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
        "INSERT INTO links (id, org_id, short_code, destination_url, title, created_by, created_at, expires_at, is_active, click_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
    );

    stmt.bind(&[
        link.id.clone().into(),
        link.org_id.clone().into(),
        link.short_code.clone().into(),
        link.destination_url.clone().into(),
        link.title.clone().into(),
        link.created_by.clone().into(),
        link.created_at.into(),
        link.expires_at.into(),
        if link.is_active { 1 } else { 0 }.into(),
        link.click_count.into(),
    ])?
    .run()
    .await?;

    Ok(())
}

/// Get links for an organization (paginated)
pub async fn get_links_by_org(
    db: &D1Database,
    org_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, is_active, click_count
         FROM links
         WHERE org_id = ?1
         ORDER BY created_at DESC
         LIMIT ?2 OFFSET ?3"
    );

    let results = stmt
        .bind(&[org_id.into(), limit.into(), offset.into()])?
        .all()
        .await?;

    results.results::<Link>()
}

/// Get a link by ID
pub async fn get_link_by_id(db: &D1Database, link_id: &str, org_id: &str) -> Result<Option<Link>> {
    let stmt = db.prepare(
        "SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, is_active, click_count
         FROM links
         WHERE id = ?1 AND org_id = ?2"
    );

    stmt.bind(&[link_id.into(), org_id.into()])?
        .first::<Link>(None)
        .await
}

/// Update a link
pub async fn update_link(
    db: &D1Database,
    link_id: &str,
    destination_url: Option<&str>,
    title: Option<&str>,
    is_active: Option<bool>,
    expires_at: Option<i64>,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Build dynamic update query
    let mut query = String::from("UPDATE links SET updated_at = ?1");
    let mut params: Vec<worker::wasm_bindgen::JsValue> = vec![now.into()];
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

    if let Some(active) = is_active {
        query.push_str(&format!(", is_active = ?{}", param_count));
        params.push(if active { 1 } else { 0 }.into());
        param_count += 1;
    }

    if expires_at.is_some() {
        query.push_str(&format!(", expires_at = ?{}", param_count));
        params.push(expires_at.into());
        param_count += 1;
    }

    query.push_str(&format!(" WHERE id = ?{}", param_count));
    params.push(link_id.into());

    let stmt = db.prepare(&query);
    stmt.bind(&params)?.run().await?;

    Ok(())
}

/// Soft delete a link (set is_active = false)
pub async fn soft_delete_link(db: &D1Database, link_id: &str, org_id: &str) -> Result<()> {
    let stmt = db.prepare("UPDATE links SET is_active = 0 WHERE id = ?1 AND org_id = ?2");

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
        event.timestamp.into(),
        event.referrer.clone().into(),
        event.user_agent.clone().into(),
        event.country.clone().into(),
        event.city.clone().into(),
    ])?
    .run()
    .await?;

    Ok(())
}
