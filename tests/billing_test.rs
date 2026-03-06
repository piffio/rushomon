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
async fn test_billing_webhook_rejects_missing_signature() {
    let client = test_client();

    let response = client
        .post(format!("{}/api/billing/webhook", BASE_URL))
        .json(&serde_json::json!({"type": "subscription.active", "data": {}}))
        .send()
        .await
        .unwrap();

    // Without the webhook-signature header the endpoint must reject with 401 or 503
    // (503 when POLAR_WEBHOOK_SECRET is not configured in the test environment)
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "Webhook without signature should be rejected, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_billing_webhook_rejects_invalid_signature() {
    let client = test_client();

    let response = client
        .post(format!("{}/api/billing/webhook", BASE_URL))
        .header(
            "webhook-signature",
            "sha256=deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        )
        .json(&serde_json::json!({"type": "subscription.active", "data": {}}))
        .send()
        .await
        .unwrap();

    // With a bogus signature the endpoint must reject with 401 or 503
    // (503 when POLAR_WEBHOOK_SECRET is not configured in the test environment)
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "Webhook with invalid signature should be rejected, got: {}",
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

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Portal endpoint should require authentication"
    );
}

#[tokio::test]
async fn test_billing_portal_without_subscription_returns_error() {
    let client = authenticated_client();

    let response = client
        .post(format!("{}/api/billing/portal", BASE_URL))
        .send()
        .await
        .unwrap();

    // Authenticated but no subscription/customer_id – expect 400 or 503
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::SERVICE_UNAVAILABLE,
        "Portal without subscription should return 400 or 503, got: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_billing_subscription_update_endpoint_removed() {
    let client = test_client();

    let response = client
        .post(format!("{}/api/billing/subscription-update", BASE_URL))
        .json(&serde_json::json!({"event_type": "subscription_activated"}))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Old subscription-update endpoint should be gone (404)"
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
