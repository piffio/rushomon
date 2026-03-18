//! Polar Integration Flow Tests
//!
//! End-to-end tests for complete subscription lifecycle flows:
//! - Cancel at period end then cron downgrades
//! - Cancel then uncancel retains tier
//! - Plan changes through subscription lifecycle

use reqwest::StatusCode;
use serde_json::{Value, json};

mod common;
mod fixtures;

use common::*;
use fixtures::polar_webhooks;

const TEST_EXTERNAL_ID: &str = "ba_test_flow_billing";
const TEST_CUSTOMER_ID: &str = "test-flow-customer";

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
async fn test_full_flow_cancel_at_period_end_then_cron_downgrades() {
    let client = authenticated_client();
    let billing_account_id = get_test_billing_account_id(&client).await;

    // Step 1: Start with pro tier
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

    // Verify initial state
    let billing_account = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account["account"]["tier"].as_str().unwrap(),
        "pro",
        "Initial tier should be pro"
    );

    // Step 2: Cancel at period end
    let period_end = polar_webhooks::future_iso(30);
    let cancel_payload = polar_webhooks::subscription_canceled_at_period_end(
        &format!("test-sub-flow1-{}", billing_account_id),
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        &period_end,
    );

    let (status, _body) =
        send_webhook_event(&client, "subscription.canceled", cancel_payload).await;
    assert_eq!(status, StatusCode::OK);

    // Step 3: Verify tier NOT downgraded yet (pending cancellation)
    let billing_account_after_cancel = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account_after_cancel["account"]["tier"]
            .as_str()
            .unwrap(),
        "pro",
        "Tier should remain pro after cancel at period end"
    );

    let subscription = get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        subscription["pending_cancellation"].as_i64().unwrap_or(0),
        1,
        "pending_cancellation should be 1"
    );
    assert_eq!(
        subscription["status"].as_str().unwrap(),
        "active",
        "Status should still be active"
    );

    // Step 4: Simulate cron running after period_end
    // Send webhook with period_end in the past to simulate cron finding expired subscription
    let period_end_past = polar_webhooks::past_iso(1);
    let updated_payload = polar_webhooks::subscription_updated(
        &format!("test-sub-flow1-{}", billing_account_id),
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        "canceled", // Status changed to canceled
        false,
        &period_end_past,
        "pro",
        "month",
    );

    let (status, _body) =
        send_webhook_event(&client, "subscription.updated", updated_payload).await;
    assert_eq!(status, StatusCode::OK);

    // Step 5: Verify tier downgraded after "cron" runs
    let billing_account_after_cron = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account_after_cron["account"]["tier"]
            .as_str()
            .unwrap(),
        "free",
        "Tier should be downgraded to free after period ends"
    );
}

#[tokio::test]
async fn test_full_flow_cancel_then_uncancel_retains_tier() {
    let client = authenticated_client();
    let billing_account_id = get_test_billing_account_id(&client).await;

    // Step 1: Start with pro tier
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

    // Step 2: Cancel at period end
    let period_end = polar_webhooks::future_iso(30);
    let cancel_payload = polar_webhooks::subscription_canceled_at_period_end(
        &format!("test-sub-flow2-{}", billing_account_id),
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        &period_end,
    );

    let (status, _body) =
        send_webhook_event(&client, "subscription.canceled", cancel_payload).await;
    assert_eq!(status, StatusCode::OK);

    // Verify pending_cancellation is set
    let subscription_after_cancel =
        get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        subscription_after_cancel["pending_cancellation"]
            .as_i64()
            .unwrap_or(0),
        1,
        "pending_cancellation should be 1 after cancel"
    );

    // Step 3: Uncancel the subscription
    let uncancel_payload = polar_webhooks::subscription_uncanceled(
        &format!("test-sub-flow2-{}", billing_account_id),
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        &period_end,
    );

    let (status, _body) =
        send_webhook_event(&client, "subscription.uncanceled", uncancel_payload).await;
    assert_eq!(status, StatusCode::OK);

    // Step 4: Verify pending_cancellation cleared and tier retained
    let subscription_after_uncancel =
        get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        subscription_after_uncancel["pending_cancellation"]
            .as_i64()
            .unwrap_or(1),
        0,
        "pending_cancellation should be 0 after uncancel"
    );
    assert_eq!(
        subscription_after_uncancel["cancel_at_period_end"]
            .as_i64()
            .unwrap_or(1),
        0,
        "cancel_at_period_end should be 0 after uncancel"
    );

    let billing_account = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account["account"]["tier"].as_str().unwrap(),
        "pro",
        "Tier should remain pro after uncancel"
    );
}

#[tokio::test]
async fn test_full_flow_pro_to_business_to_canceled() {
    let client = authenticated_client();
    let billing_account_id = get_test_billing_account_id(&client).await;

    // Step 1: Start with pro tier
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

    // Step 2: Upgrade to business via subscription.updated
    let period_end = polar_webhooks::future_iso(30);
    let upgrade_payload = polar_webhooks::subscription_updated(
        &format!("test-sub-flow3-{}", billing_account_id),
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        "active",
        false,
        &period_end,
        "business",
        "month",
    );

    let (status, _body) =
        send_webhook_event(&client, "subscription.updated", upgrade_payload).await;
    assert_eq!(status, StatusCode::OK);

    // Verify tier upgraded to business
    let billing_account_after_upgrade = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account_after_upgrade["account"]["tier"]
            .as_str()
            .unwrap(),
        "business",
        "Tier should be upgraded to business"
    );

    // Step 3: Cancel at period end
    let cancel_payload = polar_webhooks::subscription_canceled_at_period_end(
        &format!("test-sub-flow3-{}", billing_account_id),
        TEST_CUSTOMER_ID,
        TEST_EXTERNAL_ID,
        &period_end,
    );

    let (status, _body) =
        send_webhook_event(&client, "subscription.canceled", cancel_payload).await;
    assert_eq!(status, StatusCode::OK);

    // Verify tier NOT downgraded yet (still business, pending cancellation)
    let billing_account_after_cancel = get_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        billing_account_after_cancel["account"]["tier"]
            .as_str()
            .unwrap(),
        "business",
        "Tier should remain business after cancel at period end"
    );

    let subscription = get_subscription_for_billing_account(&client, &billing_account_id).await;
    assert_eq!(
        subscription["pending_cancellation"].as_i64().unwrap_or(0),
        1,
        "pending_cancellation should be 1"
    );
}
