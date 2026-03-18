//! Polar Webhook Integration Tests
//!
//! Tests for webhook processing including:
//! - subscription.active
//! - subscription.canceled (at period end and immediate)
//! - subscription.uncanceled
//! - subscription.updated

use reqwest::StatusCode;
use serde_json::{Value, json};

mod common;
mod fixtures;

use common::*;
use fixtures::polar_webhooks;

const TEST_EXTERNAL_ID: &str = "ba_test_billing_account";
const TEST_CUSTOMER_ID: &str = "test-customer-id";
const TEST_SUBSCRIPTION_ID: &str = "test-subscription-id";

/// Helper to get billing account ID for test user
async fn get_test_billing_account_id(client: &reqwest::Client) -> String {
    let user_response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .expect("Failed to get user info");

    let user: Value = user_response
        .json()
        .await
        .expect("Failed to parse user response");
    let user_id = user["id"].as_str().expect("User should have id");

    let billing_response = client
        .get(format!("{}/api/admin/billing-accounts", BASE_URL))
        .send()
        .await
        .expect("Failed to get billing accounts");

    let billing_data: Value = billing_response
        .json()
        .await
        .expect("Failed to parse billing response");
    billing_data["accounts"]
        .as_array()
        .expect("Billing accounts should be array")
        .iter()
        .find(|a| a["owner_user_id"].as_str() == Some(user_id))
        .and_then(|a| a["id"].as_str())
        .expect("Should find billing account for user")
        .to_string()
}

/// Helper to get subscription for billing account
async fn get_subscription_for_billing_account(
    client: &reqwest::Client,
    billing_account_id: &str,
) -> Value {
    let response = client
        .get(format!(
            "{}/api/admin/billing-accounts/{}",
            BASE_URL, billing_account_id
        ))
        .send()
        .await
        .expect("Failed to get billing account");

    let data: Value = response.json().await.expect("Failed to parse response");
    data["subscription"].clone()
}

/// Helper to get billing account details
async fn get_billing_account(client: &reqwest::Client, billing_account_id: &str) -> Value {
    let response = client
        .get(format!(
            "{}/api/admin/billing-accounts/{}",
            BASE_URL, billing_account_id
        ))
        .send()
        .await
        .expect("Failed to get billing account");

    response.json().await.expect("Failed to parse response")
}

#[tokio::test]
async fn test_webhook_subscription_active_sets_tier_and_period_dates() {
    let client = authenticated_client();
    let billing_account_id = get_test_billing_account_id(&client).await;

    // Set tier to free first
    let tier_response = client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, billing_account_id
        ))
        .json(&json!({"tier": "free"}))
        .send()
        .await
        .expect("Failed to set tier");
    assert_eq!(tier_response.status(), StatusCode::OK);

    // Send subscription.active webhook
    let payload = polar_webhooks::subscription_active(
        TEST_SUBSCRIPTION_ID,
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        "pro",
        "month",
        900,
    );

    let (status, _body) = send_webhook_event(&client, "subscription.active", payload).await;
    assert_eq!(status, StatusCode::OK, "Webhook should be accepted");

    // Verify tier was updated to pro
    let billing_account = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account["account"]["tier"].as_str().unwrap(),
        "pro",
        "Tier should be updated to pro"
    );

    // Verify subscription was created with correct period dates
    let subscription = get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert!(!subscription.is_null(), "Subscription should exist");
    assert_eq!(
        subscription["status"].as_str().unwrap(),
        "active",
        "Subscription status should be active"
    );
    assert_ne!(
        subscription["current_period_start"].as_i64().unwrap_or(0),
        0,
        "current_period_start should be set"
    );
    assert_ne!(
        subscription["current_period_end"].as_i64().unwrap_or(0),
        0,
        "current_period_end should be set"
    );
    assert_eq!(
        subscription["cancel_at_period_end"].as_i64().unwrap_or(1),
        0,
        "cancel_at_period_end should be false (0)"
    );
}

#[tokio::test]
async fn test_webhook_subscription_canceled_at_period_end_sets_pending_flag() {
    let client = authenticated_client();
    let billing_account_id = get_test_billing_account_id(&client).await;

    // First set tier to pro
    let tier_response = client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, billing_account_id
        ))
        .json(&json!({"tier": "pro"}))
        .send()
        .await
        .expect("Failed to set tier");
    assert_eq!(tier_response.status(), StatusCode::OK);

    // Send subscription.canceled webhook with cancel_at_period_end=true
    let period_end = polar_webhooks::future_iso(30); // 30 days in future
    let payload = polar_webhooks::subscription_canceled_at_period_end(
        TEST_SUBSCRIPTION_ID,
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        &period_end,
    );

    let (status, _body) = send_webhook_event(&client, "subscription.canceled", payload).await;
    assert_eq!(status, StatusCode::OK, "Webhook should be accepted");

    // Verify tier was NOT downgraded (still pro)
    let billing_account = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account["account"]["tier"].as_str().unwrap(),
        "pro",
        "Tier should NOT be downgraded yet"
    );

    // Verify subscription has pending_cancellation set
    let subscription = get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert!(!subscription.is_null(), "Subscription should exist");
    assert_eq!(
        subscription["status"].as_str().unwrap(),
        "active",
        "Subscription status should still be active"
    );
    assert_eq!(
        subscription["pending_cancellation"].as_i64().unwrap_or(0),
        1,
        "pending_cancellation should be set to 1"
    );
    assert_eq!(
        subscription["cancel_at_period_end"].as_i64().unwrap_or(0),
        1,
        "cancel_at_period_end should be set to 1"
    );

    // Verify period_end is correctly saved (not epoch 0)
    let period_end_timestamp = subscription["current_period_end"].as_i64().unwrap_or(0);
    assert_ne!(
        period_end_timestamp, 0,
        "current_period_end should be set correctly (not epoch 0)"
    );
}

#[tokio::test]
async fn test_webhook_subscription_canceled_immediate_downgrades_tier() {
    let client = authenticated_client();
    let billing_account_id = get_test_billing_account_id(&client).await;

    // First set tier to pro
    let tier_response = client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, billing_account_id
        ))
        .json(&json!({"tier": "pro"}))
        .send()
        .await
        .expect("Failed to set tier");
    assert_eq!(tier_response.status(), StatusCode::OK);

    // Send subscription.canceled webhook with immediate cancellation
    let payload = polar_webhooks::subscription_canceled_immediate(
        TEST_SUBSCRIPTION_ID,
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
    );

    let (status, _body) = send_webhook_event(&client, "subscription.canceled", payload).await;
    assert_eq!(status, StatusCode::OK, "Webhook should be accepted");

    // Verify tier was downgraded to free
    let billing_account = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account["account"]["tier"].as_str().unwrap(),
        "free",
        "Tier should be downgraded to free immediately"
    );

    // Verify subscription is canceled
    let subscription = get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert!(!subscription.is_null(), "Subscription should exist");
    assert_eq!(
        subscription["status"].as_str().unwrap(),
        "canceled",
        "Subscription status should be canceled"
    );
    assert_ne!(
        subscription["canceled_at"].as_i64().unwrap_or(0),
        0,
        "canceled_at should be set"
    );
}

#[tokio::test]
async fn test_webhook_subscription_uncanceled_clears_pending_flag() {
    let client = authenticated_client();
    let billing_account_id = get_test_billing_account_id(&client).await;

    // First set tier to pro and set up pending cancellation
    let tier_response = client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, billing_account_id
        ))
        .json(&json!({"tier": "pro"}))
        .send()
        .await
        .expect("Failed to set tier");
    assert_eq!(tier_response.status(), StatusCode::OK);

    // Send subscription.canceled webhook first
    let period_end = polar_webhooks::future_iso(30);
    let cancel_payload = polar_webhooks::subscription_canceled_at_period_end(
        TEST_SUBSCRIPTION_ID,
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        &period_end,
    );
    let (status, _body) =
        send_webhook_event(&client, "subscription.canceled", cancel_payload).await;
    assert_eq!(status, StatusCode::OK);

    // Verify pending_cancellation is set
    let subscription_before =
        get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        subscription_before["pending_cancellation"]
            .as_i64()
            .unwrap_or(0),
        1,
        "pending_cancellation should be 1 before uncancel"
    );

    // Send subscription.uncanceled webhook
    let uncancel_payload = polar_webhooks::subscription_uncanceled(
        TEST_SUBSCRIPTION_ID,
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        &period_end,
    );
    let (status, _body) =
        send_webhook_event(&client, "subscription.uncanceled", uncancel_payload).await;
    assert_eq!(status, StatusCode::OK, "Webhook should be accepted");

    // Verify pending_cancellation is cleared
    let subscription_after =
        get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert!(!subscription_after.is_null(), "Subscription should exist");
    assert_eq!(
        subscription_after["pending_cancellation"]
            .as_i64()
            .unwrap_or(1),
        0,
        "pending_cancellation should be cleared to 0"
    );
    assert_eq!(
        subscription_after["cancel_at_period_end"]
            .as_i64()
            .unwrap_or(1),
        0,
        "cancel_at_period_end should be cleared to 0"
    );

    // Verify tier is still pro
    let billing_account = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account["account"]["tier"].as_str().unwrap(),
        "pro",
        "Tier should remain pro"
    );
}

#[tokio::test]
async fn test_webhook_parses_snake_case_fields_correctly() {
    let client = authenticated_client();
    let billing_account_id = get_test_billing_account_id(&client).await;

    // Set tier to pro
    let tier_response = client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, billing_account_id
        ))
        .json(&json!({"tier": "pro"}))
        .send()
        .await
        .expect("Failed to set tier");
    assert_eq!(tier_response.status(), StatusCode::OK);

    // Send subscription.updated with snake_case fields
    let period_end = polar_webhooks::future_iso(30);
    let payload = polar_webhooks::subscription_updated(
        TEST_SUBSCRIPTION_ID,
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        "active",
        true, // cancel_at_period_end
        &period_end,
        "pro",
        "month",
    );

    let (status, _body) = send_webhook_event(&client, "subscription.updated", payload).await;
    assert_eq!(status, StatusCode::OK, "Webhook should be accepted");

    // Verify snake_case fields were parsed correctly
    let subscription = get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert!(!subscription.is_null(), "Subscription should exist");

    // Verify cancel_at_period_end was parsed (not None/default)
    assert_eq!(
        subscription["cancel_at_period_end"].as_i64().unwrap_or(0),
        1,
        "cancel_at_period_end should be 1 (true)"
    );

    // Verify current_period_end was parsed (not epoch 0)
    let period_end_timestamp = subscription["current_period_end"].as_i64().unwrap_or(0);
    assert_ne!(
        period_end_timestamp, 0,
        "current_period_end should be set correctly (not epoch 0)"
    );

    // Verify pending_cancellation was set
    assert_eq!(
        subscription["pending_cancellation"].as_i64().unwrap_or(0),
        1,
        "pending_cancellation should be set when cancel_at_period_end is true"
    );
}
