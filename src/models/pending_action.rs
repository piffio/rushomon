use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Represents a pending action that requires out-of-band confirmation (e.g. email link).
///
/// The `id` doubles as the magic-link token.
/// `subject_id` is an indexed pointer whose meaning is determined by `action_type`:
///   - `billing_account_transfer` → billing_account_id
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingAction {
    /// UUID — also the token embedded in the confirmation link.
    pub id: String,
    /// Discriminator for the action type (e.g. "billing_account_transfer").
    pub action_type: String,
    /// Indexed subject reference (meaning depends on action_type).
    pub subject_id: String,
    /// User ID of the person who initiated the action.
    pub initiated_by: String,
    /// Email address the confirmation was sent to.
    pub to_email: String,
    /// JSON payload with action-specific data.
    pub payload: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub accepted_at: Option<i64>,
    pub cancelled_at: Option<i64>,
}

impl PendingAction {
    /// Returns true if the action is still open (not expired, accepted, or cancelled).
    #[allow(dead_code)]
    pub fn is_pending(&self, now: i64) -> bool {
        self.accepted_at.is_none() && self.cancelled_at.is_none() && self.expires_at > now
    }
}

/// Known action type discriminators.
pub mod action_type {
    pub const BILLING_ACCOUNT_TRANSFER: &str = "billing_account_transfer";
}

/// Typed payload for a billing account ownership transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingTransferPayload {
    /// The billing account being transferred.
    pub billing_account_id: String,
    /// User ID of the current owner (person initiating the transfer).
    pub from_user_id: String,
}
