use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Status values for a custom domain
pub const STATUS_PENDING: &str = "pending";
pub const STATUS_ACTIVE: &str = "active";
#[allow(dead_code)]
pub const STATUS_FAILED: &str = "failed";

/// A custom domain registered to an organization.
///
/// SSL certificates and domain verification are managed via Cloudflare for SaaS.
/// Once a domain is `active`, redirects on that hostname use KV key `{hostname}:{short_code}`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomDomain {
    #[schema(example = "cd_abc123")]
    pub id: String,
    #[schema(example = "org-123456")]
    pub org_id: String,
    #[schema(example = "go.mybrand.com")]
    pub hostname: String,
    /// "pending" | "active" | "failed"
    #[schema(example = "pending")]
    pub status: String,
    /// Cloudflare for SaaS custom hostname record ID
    pub cf_hostname_id: Option<String>,
    #[schema(example = 1609459200)]
    pub created_at: i64,
    pub verified_at: Option<i64>,
}

impl CustomDomain {
    pub fn generate_id() -> String {
        format!("cd_{}", crate::utils::generate_short_code_with_length(16))
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.status == STATUS_ACTIVE
    }
}

/// DNS instructions returned to the user after adding a custom domain
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DnsInstructions {
    /// The CNAME target the user should point their subdomain to
    #[schema(example = "rush.mn")]
    pub cname_target: String,
    /// TXT record name for domain ownership verification (from CF for SaaS)
    pub txt_name: Option<String>,
    /// TXT record value for domain ownership verification (from CF for SaaS)
    pub txt_value: Option<String>,
    /// Whether the user needs to add a CNAME record
    pub needs_cname: bool,
    /// Whether the user needs to add a TXT record (for CF verification)
    pub needs_txt: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_domain_is_active() {
        let mut d = CustomDomain {
            id: "cd_test".to_string(),
            org_id: "org-1".to_string(),
            hostname: "go.example.com".to_string(),
            status: STATUS_PENDING.to_string(),
            cf_hostname_id: None,
            created_at: 0,
            verified_at: None,
        };
        assert!(!d.is_active());
        d.status = STATUS_ACTIVE.to_string();
        assert!(d.is_active());
    }

    #[test]
    fn test_generate_id_prefix() {
        let id = CustomDomain::generate_id();
        assert!(id.starts_with("cd_"));
        assert!(id.len() > 3);
    }
}
