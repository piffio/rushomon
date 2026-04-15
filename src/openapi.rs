//! OpenAPI specification generator for Rushomon API
//!
//! This module provides the OpenAPI specification for the Rushomon URL shortener API.
//! The spec is generated using utoipa and can be exported as JSON for documentation tools.

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// Root OpenAPI spec with all API schemas
#[derive(OpenApi)]
#[openapi(
    components(
        schemas(
            // Link models
            crate::models::link::Link,
            crate::models::link::LinkStatus,
            crate::models::link::CreateLinkRequest,
            crate::models::link::UpdateLinkRequest,
            crate::models::link::UtmParams,

            // Analytics models
            crate::models::analytics::LinkAnalyticsResponse,
            crate::models::analytics::OrgAnalyticsResponse,
            crate::models::analytics::TimeRange,
            crate::models::analytics::DailyClicks,
            crate::models::analytics::ReferrerCount,
            crate::models::analytics::CountryCount,
            crate::models::analytics::UserAgentCount,
            crate::models::analytics::TopLinkCount,

            // Pagination models
            crate::models::pagination::PaginationMeta,

            // Tier models
            crate::models::tier::Tier,

            // User models
            crate::models::user::User,

            // Organization models
            crate::models::organization::Organization,
            crate::models::org_member::OrgMember,
            crate::models::org_member::OrgMemberWithUser,
            crate::models::org_member::OrgInvitation,
            crate::models::org_member::OrgWithRole,

            // Billing models
            crate::models::billing_account::BillingAccount,

            // API Keys models
            crate::api::keys::CreateApiKeyRequest,
            crate::api::keys::CreateApiKeyResponse,

            // Version response
            crate::api::version::VersionResponse,
        ),
    ),
    paths(
        // Authentication
        crate::api::auth::providers::handle_list_auth_providers,
        crate::router::handle_github_login,
        crate::router::handle_google_login,
        crate::router::handle_oauth_callback,
        crate::api::auth::session::handle_get_current_user,
        crate::api::auth::session::handle_token_refresh,
        crate::api::auth::session::handle_logout,

        // Usage
        crate::api::analytics::usage::handle_get_usage,

        // Links
        crate::router::handle_create_link,
        crate::router::handle_list_links,
        crate::router::handle_get_link,
        crate::router::handle_get_link_by_code,
        crate::api::analytics::link::handle_get_link_analytics,
        crate::router::handle_update_link,
        crate::router::handle_delete_link,
        crate::router::handle_export_links,
        crate::router::handle_import_links,

        // Analytics
        crate::api::analytics::org::handle_get_org_analytics,

        // Tags
        crate::api::tags::handle_get_org_tags,
        crate::api::tags::handle_delete_org_tag,
        crate::api::tags::handle_rename_org_tag,

        // Organizations
        crate::api::orgs::handle_list_user_orgs,
        crate::api::orgs::handle_create_org,
        crate::api::orgs::handle_switch_org,
        crate::api::orgs::handle_get_org,
        crate::api::orgs::handle_update_org,
        crate::api::orgs::handle_delete_org,
        crate::api::orgs::handle_get_org_settings,
        crate::api::orgs::handle_update_org_settings,
        crate::api::orgs::handle_remove_member,
        crate::api::orgs::handle_create_invitation,
        crate::api::orgs::handle_revoke_invitation,
        crate::api::orgs::handle_resend_invitation,
        crate::api::orgs::handle_get_invite_info,
        crate::api::orgs::handle_accept_invite,
        crate::api::orgs::handle_upload_org_logo,
        crate::api::orgs::handle_get_org_logo,
        crate::api::orgs::handle_delete_org_logo,

        // API Keys
        crate::api::keys::handle_create_api_key,
        crate::api::keys::handle_list_api_keys,
        crate::api::keys::handle_revoke_api_key,

        // Billing
        crate::api::billing::handle_get_status,
        crate::api::billing::handle_create_checkout,
        crate::api::billing::handle_portal,
        crate::api::billing::handle_webhook,

        // Reports
        crate::api::reports::create::handle_report_link,

        // Settings
        crate::api::settings::public::handle_get_public_settings,

        // System
        crate::api::version::handle_version,

        // Admin — Users
        crate::api::admin::users::handle_admin_list_users,
        crate::api::admin::users::handle_admin_get_user,
        crate::api::admin::users::handle_admin_update_user_role,
        crate::api::admin::users::handle_admin_suspend_user,
        crate::api::admin::users::handle_admin_unsuspend_user,
        crate::api::admin::users::handle_admin_delete_user,

        // Admin — Links
        crate::router::handle_admin_list_links,
        crate::router::handle_admin_update_link_status,
        crate::router::handle_admin_delete_link,
        crate::router::handle_admin_sync_link_kv,

        // Admin — Settings
        crate::api::settings::admin::handle_admin_get_settings,
        crate::api::settings::admin::handle_admin_update_setting,
        crate::router::handle_admin_reset_monthly_counter,

        // Admin — Blacklist
        crate::api::admin::blacklist::handle_admin_get_blacklist,
        crate::api::admin::blacklist::handle_admin_block_destination,
        crate::api::admin::blacklist::handle_admin_remove_blacklist,

        // Admin — Reports
        crate::api::reports::admin::handle_admin_get_reports,
        crate::api::reports::admin::handle_admin_get_report,
        crate::api::reports::admin::handle_admin_update_report,
        crate::api::reports::admin::handle_admin_get_pending_reports_count,

        // Admin — Billing accounts
        crate::router::handle_admin_list_billing_accounts,
        crate::router::handle_admin_get_billing_account,
        crate::router::handle_admin_update_billing_account_tier,
        crate::router::handle_admin_reset_billing_account_counter,
        crate::router::handle_admin_update_subscription_status,
        crate::api::billing::handle_admin_reset_billing_account,
        crate::api::billing::handle_cron_trigger_downgrade,

        // Admin — Products & Discounts
        crate::api::billing::products::handle_admin_list_discounts,
        crate::api::billing::products::handle_admin_list_products,
        crate::api::billing::products::handle_admin_sync_products,
        crate::api::billing::products::handle_admin_save_products,

        // Admin — API Keys
        crate::api::admin::api_keys::handle_admin_list_api_keys,
        crate::api::admin::api_keys::handle_admin_revoke_api_key,
        crate::api::admin::api_keys::handle_admin_reactivate_api_key,
        crate::api::admin::api_keys::handle_admin_delete_api_key,
        crate::api::admin::api_keys::handle_admin_restore_api_key,
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Authentication", description = "OAuth authentication and session management"),
        (name = "Links", description = "URL shortening and link management"),
        (name = "Analytics", description = "Link analytics and statistics"),
        (name = "Organizations", description = "Multi-tenant organization management"),
        (name = "API Keys", description = "API key management for programmatic access (Pro+ tier)"),
        (name = "Billing", description = "Subscription and billing management"),
        (name = "Settings", description = "Instance and organization settings"),
        (name = "Usage", description = "Tier usage and limit information"),
        (name = "Tags", description = "Link tag management"),
        (name = "Reports", description = "Abuse reporting"),
        (name = "System", description = "System information endpoints"),
        (name = "Admin", description = "Administrative endpoints (admin role required)"),
    ),
    info(
        title = "Rushomon URL Shortener API",
        version = env!("CARGO_PKG_VERSION"),
        description = r#"API documentation for the Rushomon URL shortener service.

## Authentication

Most endpoints require authentication using either:
- **Session cookies** (for web application users)
- **Bearer tokens** (for API keys)

### Using API Keys

1. Create an API key in the dashboard (Pro tier or higher required)
2. Include the key in the Authorization header:
   ```
   Authorization: Bearer ro_pat_...
   ```
## Response Format

All responses are JSON formatted. Error responses follow this structure:
```json
{
  "error": "Error message describing what went wrong"
}
```

## Pagination

List endpoints support pagination with `page` and `limit` query parameters:
- `page`: Page number (1-indexed, default: 1)
- `limit`: Items per page (1-100, default: 20)
"#,
        contact(name = "Rushomon Support", email = "support@rushomon.cc"),
        license(name = "AGPL-3.0", url = "https://www.gnu.org/licenses/agpl-3.0.html"),
    ),
    servers(
        (url = "https://api.rushomon.cc", description = "Production API"),
        (url = "http://localhost:8787", description = "Local development"),
    ),
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // Add security scheme
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "Bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT or API Key")
                        .build(),
                ),
            );
        }
    }
}

/// Generate the OpenAPI specification as a JSON string
pub fn generate_openapi_json() -> Result<String, serde_json::Error> {
    let openapi = ApiDoc::openapi();
    serde_json::to_string_pretty(&openapi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_openapi_json() {
        let json = generate_openapi_json();
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("\"openapi\":"));
        assert!(json_str.contains("\"info\""));
        assert!(json_str.contains("\"components\""));
    }

    #[test]
    fn test_openapi_has_security_schemes() {
        let openapi = ApiDoc::openapi();
        assert!(openapi.components.is_some());
        let components = openapi.components.unwrap();
        assert!(!components.security_schemes.is_empty());
        assert!(components.security_schemes.contains_key("Bearer"));
    }

    #[test]
    fn test_openapi_has_tags() {
        let openapi = ApiDoc::openapi();
        assert!(openapi.tags.is_some());
        assert!(!openapi.tags.unwrap().is_empty());
    }

    #[test]
    fn test_openapi_has_servers() {
        let openapi = ApiDoc::openapi();
        assert!(openapi.servers.is_some());
        assert!(!openapi.servers.unwrap().is_empty());
    }
}
