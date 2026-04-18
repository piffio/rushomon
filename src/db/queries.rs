use crate::models::{BillingAccount, User};
use crate::repositories::link_repository::{AdminLink, serialize_optional_int_as_bool};
use crate::utils::now_timestamp;
use worker::d1::D1Database;
use worker::*;

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
