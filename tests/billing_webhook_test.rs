use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

/// Reset the billing test user to a clean free-tier state before each stateful test.
/// Uses the admin billing-account reset endpoint to wipe subscriptions and reset tier.
/// This ensures tests are idempotent across reruns and within-file parallel execution.
async fn reset_billing_test_user_state() {
    let billing_account_id = get_billing_test_account_id().await;
    let admin_client = authenticated_client();
    let resp = admin_client
        .post(format!(
            "{}/api/admin/billing-accounts/{}/reset",
            BASE_URL, billing_account_id
        ))
        .send()
        .await
        .expect("Failed to call admin billing reset endpoint");
    assert!(
        resp.status().is_success(),
        "Admin billing reset should return 2xx, got {}",
        resp.status()
    );
}

// ─── Step 3: Signature enforcement (negative paths) ──────────────────────────

#[tokio::test]
async fn test_webhook_rejects_missing_webhook_id() {
    let body = json!({"type": "subscription.active", "data": {}});
    let body_str = serde_json::to_string(&body).unwrap();
    let ts = webhook_timestamp_now();
    let sig = sign_webhook_payload(&body_str, "some-id", &ts, POLAR_WEBHOOK_SECRET);

    let client = test_client();
    let response = client
        .post(format!("{}/api/billing/webhook", BASE_URL))
        // webhook-id header intentionally omitted
        .header("webhook-timestamp", &ts)
        .header("webhook-signature", &sig)
        .header("content-type", "application/json")
        .body(body_str)
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Missing webhook-id should be rejected with 401"
    );
}

#[tokio::test]
async fn test_webhook_rejects_missing_timestamp() {
    let body = json!({"type": "subscription.active", "data": {}});
    let body_str = serde_json::to_string(&body).unwrap();
    let id = webhook_message_id("missing-ts");
    let sig = sign_webhook_payload(&body_str, &id, "1700000000", POLAR_WEBHOOK_SECRET);

    let client = test_client();
    let response = client
        .post(format!("{}/api/billing/webhook", BASE_URL))
        .header("webhook-id", &id)
        // webhook-timestamp header intentionally omitted
        .header("webhook-signature", &sig)
        .header("content-type", "application/json")
        .body(body_str)
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Missing webhook-timestamp should be rejected with 401"
    );
}

#[tokio::test]
async fn test_webhook_accepts_unknown_event_type() {
    let body = json!({"type": "some.unknown.event", "data": {}});
    let response = send_signed_webhook(&body).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Unknown event types should be acknowledged with 200 (not an error)"
    );
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["received"], json!(true));
}

// ─── Step 4: subscription.active activates the tier ─────────────────────────
// Note: the test product row (price-test-pro-monthly → Pro Monthly) is seeded by
// run-integration-tests.sh via `wrangler d1 execute` after migrations run.

#[tokio::test]
async fn test_subscription_active_upgrades_tier() {
    reset_billing_test_user_state().await;
    let billing_account_id = get_billing_test_account_id().await;
    let polar_customer_id = format!("cus-test-active-{}", webhook_message_id("c"));
    let polar_subscription_id = format!("sub-test-active-{}", webhook_message_id("s"));

    // Send subscription.active webhook
    let payload = make_subscription_active_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
        TEST_PRICE_ID,
        "2026-12-31T23:59:59Z",
    );
    let response = send_signed_webhook(&payload).await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "subscription.active should return 200"
    );
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["received"], json!(true));

    // Verify the billing status reflects the upgrade
    let client = billing_test_client();
    let status_resp = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(status_resp.status(), StatusCode::OK);
    let status: serde_json::Value = status_resp.json().await.unwrap();

    assert_eq!(
        status["tier"].as_str().unwrap_or(""),
        "pro",
        "Tier should be upgraded to 'pro' after subscription.active"
    );
    assert_eq!(
        status["cancel_at_period_end"].as_bool().unwrap_or(true),
        false,
        "cancel_at_period_end should be false for a fresh active subscription"
    );
    assert_eq!(
        status["provider_customer_id"].as_str().unwrap_or(""),
        polar_customer_id,
        "provider_customer_id should be stored from the webhook"
    );
    assert_eq!(
        status["subscription_status"].as_str().unwrap_or(""),
        "active",
        "subscription_status should be 'active'"
    );

    // Cleanup: revoke subscription so later tests are not affected
    let cleanup = make_subscription_revoked_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
    );
    let _ = send_signed_webhook(&cleanup).await;
}

// ─── Step 5: cancel-at-period-end (pending cancellation) ─────────────────────

#[tokio::test]
async fn test_subscription_canceled_at_period_end_retains_access() {
    reset_billing_test_user_state().await;
    let billing_account_id = get_billing_test_account_id().await;
    let polar_customer_id = format!("cus-test-cape-{}", webhook_message_id("c"));
    let polar_subscription_id = format!("sub-test-cape-{}", webhook_message_id("s"));

    // 1. Activate subscription first
    let activate = make_subscription_active_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
        TEST_PRICE_ID,
        "2026-12-31T23:59:59Z",
    );
    let r = send_signed_webhook(&activate).await;
    assert_eq!(
        r.status(),
        StatusCode::OK,
        "subscription.active should succeed"
    );

    // 2. Cancel at period end (subscription.canceled with cancel_at_period_end=true, status=active)
    let cancel = make_subscription_canceled_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
        true, // cancel_at_period_end = true
        "2026-12-31T23:59:59Z",
    );
    let r = send_signed_webhook(&cancel).await;
    assert_eq!(
        r.status(),
        StatusCode::OK,
        "subscription.canceled should return 200"
    );

    // 3. Verify user still has pro tier (access not revoked yet)
    let client = billing_test_client();
    let status_resp = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();
    let status: serde_json::Value = status_resp.json().await.unwrap();

    assert_eq!(
        status["tier"].as_str().unwrap_or(""),
        "pro",
        "Tier should still be 'pro' after cancel-at-period-end (access retained until period ends)"
    );
    assert_eq!(
        status["cancel_at_period_end"].as_bool().unwrap_or(false),
        true,
        "cancel_at_period_end should be true"
    );

    // Cleanup
    let cleanup = make_subscription_revoked_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
    );
    let _ = send_signed_webhook(&cleanup).await;
}

// ─── Step 6: subscription.canceled immediate revocation ──────────────────────

#[tokio::test]
async fn test_subscription_canceled_immediately_downgrades_tier() {
    reset_billing_test_user_state().await;
    let billing_account_id = get_billing_test_account_id().await;
    let polar_customer_id = format!("cus-test-imm-{}", webhook_message_id("c"));
    let polar_subscription_id = format!("sub-test-imm-{}", webhook_message_id("s"));

    // 1. Activate subscription
    let activate = make_subscription_active_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
        TEST_PRICE_ID,
        "2026-12-31T23:59:59Z",
    );
    let r = send_signed_webhook(&activate).await;
    assert_eq!(r.status(), StatusCode::OK);

    // 2. Cancel immediately (status=canceled, cancel_at_period_end=false)
    let cancel = make_subscription_canceled_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
        false, // cancel_at_period_end = false → immediate
        "2026-12-31T23:59:59Z",
    );
    let r = send_signed_webhook(&cancel).await;
    assert_eq!(
        r.status(),
        StatusCode::OK,
        "subscription.canceled should return 200"
    );

    // 3. Verify tier is immediately downgraded to free
    let client = billing_test_client();
    let status_resp = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();
    let status: serde_json::Value = status_resp.json().await.unwrap();

    assert_eq!(
        status["tier"].as_str().unwrap_or(""),
        "free",
        "Tier should be immediately downgraded to 'free' on immediate cancellation"
    );
}

// ─── Step 7: resolve_billing_account_id fallback path ────────────────────────

#[tokio::test]
async fn test_webhook_fallback_via_customer_id_when_external_id_missing() {
    reset_billing_test_user_state().await;
    let billing_account_id = get_billing_test_account_id().await;
    let polar_customer_id = format!("cus-test-fallback-{}", webhook_message_id("c"));
    let polar_subscription_id = format!("sub-test-fallback-{}", webhook_message_id("s"));

    // 1. First activate WITH external_id to record the provider_customer_id
    let activate = make_subscription_active_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
        TEST_PRICE_ID,
        "2026-12-31T23:59:59Z",
    );
    let r = send_signed_webhook(&activate).await;
    assert_eq!(
        r.status(),
        StatusCode::OK,
        "Initial activation should succeed"
    );

    // 2. Send subscription.updated WITHOUT external_id — relies on customer_id fallback
    let fallback_sub_id = format!("sub-test-fallback-upd-{}", webhook_message_id("u"));
    let updated_payload = json!({
        "type": "subscription.updated",
        "data": {
            "id": fallback_sub_id,
            "customer_id": polar_customer_id,
            "customer": {
                // external_id intentionally absent
            },
            "prices": [{ "id": TEST_PRICE_ID }],
            "recurringInterval": "month",
            "status": "active",
            "cancel_at_period_end": false,
            "current_period_start": "2025-02-01T00:00:00Z",
            "current_period_end": "2026-12-31T23:59:59Z",
            "ends_at": null,
            "amount": 999,
            "currency": "usd",
            "discount": null
        }
    });
    let r = send_signed_webhook(&updated_payload).await;
    assert_eq!(
        r.status(),
        StatusCode::OK,
        "subscription.updated without external_id should succeed via customer_id fallback"
    );

    // 3. Verify status is still correct (billing account resolved via fallback)
    let client = billing_test_client();
    let status_resp = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();
    let status: serde_json::Value = status_resp.json().await.unwrap();

    assert_eq!(
        status["tier"].as_str().unwrap_or(""),
        "pro",
        "Tier should still be 'pro' after subscription.updated via customer_id fallback"
    );

    // Cleanup
    let cleanup = make_subscription_revoked_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
    );
    let _ = send_signed_webhook(&cleanup).await;
}

// ─── Step 8: cron downgrade of expired pending cancellations ─────────────────

#[tokio::test]
async fn test_cron_trigger_requires_admin() {
    // Unauthenticated request should return 401
    let client = test_client();
    let response = client
        .post(format!("{}/api/admin/cron/trigger-downgrade", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Cron trigger should require authentication"
    );
}

#[tokio::test]
async fn test_cron_trigger_downgrade_processes_expired_pending_cancellations() {
    reset_billing_test_user_state().await;
    let billing_account_id = get_billing_test_account_id().await;
    let polar_customer_id = format!("cus-test-cron-{}", webhook_message_id("c"));
    let polar_subscription_id = format!("sub-test-cron-{}", webhook_message_id("s"));

    // 1. Activate subscription
    let activate = make_subscription_active_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
        TEST_PRICE_ID,
        "2026-12-31T23:59:59Z",
    );
    let r = send_signed_webhook(&activate).await;
    assert_eq!(r.status(), StatusCode::OK, "Activation should succeed");

    // 2. Send subscription.updated with cancel_at_period_end=true and period_end in the PAST.
    // This is how Polar signals a cancel-at-period-end: the subscription stays active
    // until period_end, then the cron job downgrades. We set period_end to the past
    // so the cron job processes it immediately.
    let cancel = make_subscription_updated_cancel_at_period_end_payload(
        &billing_account_id,
        &polar_customer_id,
        &polar_subscription_id,
        TEST_PRICE_ID,
        "2020-01-01T00:00:00Z", // past date → cron should process this
    );
    let r = send_signed_webhook(&cancel).await;
    assert_eq!(
        r.status(),
        StatusCode::OK,
        "Cancel-at-period-end webhook should succeed"
    );

    // Verify still on pro (pending cancellation, not yet processed)
    let client = billing_test_client();
    let status_resp = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();
    let status: serde_json::Value = status_resp.json().await.unwrap();
    assert_eq!(
        status["tier"].as_str().unwrap_or(""),
        "pro",
        "Should still be pro before cron runs"
    );

    // 3. Trigger the cron job via admin endpoint (uses primary admin JWT)
    let admin_client = authenticated_client();
    let cron_resp = admin_client
        .post(format!("{}/api/admin/cron/trigger-downgrade", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(
        cron_resp.status(),
        StatusCode::OK,
        "Cron trigger should return 200"
    );
    let cron_body: serde_json::Value = cron_resp.json().await.unwrap();
    assert!(
        cron_body["processed"].as_u64().unwrap_or(0) >= 1,
        "Cron trigger should have processed at least 1 expired subscription. Got: {:?}",
        cron_body
    );
    assert_eq!(
        cron_body["errors"].as_u64().unwrap_or(1),
        0,
        "Cron trigger should have 0 errors"
    );

    // 4. Verify tier is now downgraded to free
    let status_resp = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();
    let status: serde_json::Value = status_resp.json().await.unwrap();
    assert_eq!(
        status["tier"].as_str().unwrap_or(""),
        "free",
        "Tier should be downgraded to 'free' after cron processes expired pending cancellation"
    );
}
