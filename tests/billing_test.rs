use reqwest::StatusCode;

mod common;
use common::*;

#[tokio::test]
async fn test_billing_status_requires_auth() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_billing_status_authenticated() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(
        body.get("tier").is_some(),
        "Response should contain 'tier' field"
    );
    assert!(
        body.get("provider_customer_id").is_some(),
        "Response should contain 'provider_customer_id' field"
    );
    assert!(
        body.get("cancel_at_period_end").is_some(),
        "Response should contain 'cancel_at_period_end' field"
    );

    let tier = body["tier"].as_str().unwrap_or("");
    assert!(
        ["free", "pro", "business", "unlimited"].contains(&tier),
        "Tier should be one of: free, pro, business, unlimited. Got: {}",
        tier
    );
}

#[tokio::test]
async fn test_billing_checkout_requires_auth() {
    let client = test_client();

    let response = client
        .post(format!("{}/api/billing/checkout", BASE_URL))
        .json(&serde_json::json!({
            "price_id": "price_test_pro_monthly",
            "billing_interval": "monthly"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_billing_checkout_without_polar_returns_error() {
    let client = authenticated_client();

    let response = client
        .post(format!("{}/api/billing/checkout", BASE_URL))
        .json(&serde_json::json!({
            "price_id": "price_test_pro_monthly",
            "billing_interval": "monthly"
        }))
        .send()
        .await
        .unwrap();

    // Without Polar configured, should return 503 (billing not configured)
    // or 400 (invalid price ID). Either is acceptable in the test environment.
    assert!(
        response.status() == StatusCode::SERVICE_UNAVAILABLE
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "Expected 400/503/500 when Polar is not configured, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_billing_portal_requires_auth() {
    let client = test_client();

    let response = client
        .post(format!("{}/api/billing/portal", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_billing_portal_without_polar_returns_error() {
    let client = authenticated_client();

    let response = client
        .post(format!("{}/api/billing/portal", BASE_URL))
        .send()
        .await
        .unwrap();

    // Without Polar configured, or without a provider_customer_id, should return an error
    assert!(
        response.status() == StatusCode::SERVICE_UNAVAILABLE
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "Expected 400/404/503/500 when Polar is not configured, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_billing_subscription_update_requires_secret() {
    let client = test_client();

    let response = client
        .post(format!("{}/api/billing/subscription-update", BASE_URL))
        .json(&serde_json::json!({"event_type": "subscription_activated"}))
        .send()
        .await
        .unwrap();

    // Without the X-Internal-Secret header the endpoint should reject
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "subscription-update without secret should be rejected, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_billing_no_active_subscription_by_default() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    // The test user is the first user (admin) so may have 'unlimited' tier.
    // In all cases, a fresh test user should have no active subscription.
    assert!(
        body["subscription_id"].is_null(),
        "Test user should have no subscription ID by default"
    );
    assert!(
        body["subscription_status"].is_null(),
        "Test user should have no subscription status by default"
    );
    let cancel = body["cancel_at_period_end"].as_bool().unwrap_or(true);
    assert!(
        !cancel,
        "Default user should not have cancel_at_period_end set"
    );
}
