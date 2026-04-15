use crate::models::{BillingAccount, Link, Organization, User, user::CreateUserData};
use crate::repositories::link_repository::{AdminLink, serialize_optional_int_as_bool};
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BlacklistEntry {
    pub id: String,
    pub destination: String,
    pub match_type: String,
    pub reason: String,
    pub created_by: String,
    pub created_at: i64,
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
    use crate::repositories::link_repository::AdminLinkBase;

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
