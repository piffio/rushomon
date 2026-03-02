/// Parameters for creating a checkout session
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreateCheckoutSessionParams {
    pub billing_account_id: String,
    pub customer_id: Option<String>,
    pub price_id: String,
    pub success_url: String,
    pub cancel_url: String,
    pub coupon_id: Option<String>,
    pub client_reference_id: String,
}

/// Result of creating a Checkout session
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CheckoutSession {
    pub id: String,
    pub url: String,
    pub customer_id: Option<String>,
}
