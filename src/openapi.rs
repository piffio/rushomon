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
        crate::router::handle_get_current_user,
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
        (name = "Admin", description = "Administrative endpoints (admin only)"),
        (name = "Utilities", description = "Utility endpoints like version and title fetching"),
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
