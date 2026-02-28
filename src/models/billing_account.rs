use crate::models::tier::Tier;
use serde::{Deserialize, Serialize};

/// Billing Account represents the payment entity that owns one or more organizations.
/// Tier limits and quotas are enforced at the billing account level, not per-organization.
///
/// This prevents abuse where users could create multiple orgs to multiply their quotas.
/// For example:
/// - Business tier ($29) = 10k links/month across ALL orgs in the billing account
/// - Not 10k per org (which would be 30k for 3 orgs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingAccount {
    pub id: String,
    pub owner_user_id: String,
    pub tier: String,
    pub stripe_customer_id: Option<String>,
    pub created_at: i64,
}

impl BillingAccount {
    /// Get the parsed Tier enum from the tier string
    #[allow(dead_code)]
    pub fn get_tier(&self) -> Tier {
        Tier::from_str_value(&self.tier).unwrap_or(Tier::Free)
    }

    /// Check if this billing account is on a paid tier
    #[allow(dead_code)]
    pub fn is_paid(&self) -> bool {
        matches!(
            self.get_tier(),
            Tier::Pro | Tier::Business | Tier::Unlimited
        )
    }

    /// Generate a new billing account ID
    pub fn generate_id() -> String {
        format!("ba_{}", crate::utils::generate_short_code_with_length(16))
    }
}
