use super::types::{CheckoutSession, CreateCheckoutSessionParams};
use worker::Result;

/// Abstraction layer for billing/payment operations.
/// Webhook verification is handled by the SvelteKit frontend adapter (@polar-sh/sveltekit).
pub trait BillingProvider {
    async fn create_checkout_session(
        &self,
        params: CreateCheckoutSessionParams,
    ) -> Result<CheckoutSession>;
}
