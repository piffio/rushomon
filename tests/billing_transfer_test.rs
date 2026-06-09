/// Integration tests for billing account ownership transfer.
///
/// # Test setup requirements
///
/// These tests rely on the standard integration test environment provided by
/// `scripts/run-integration-tests.sh`, which creates:
///   - A primary admin user with TEST_JWT
///   - A secondary user (billing test user) with TEST_BILLING_JWT
///
/// The two users are in separate billing accounts by default. Several tests
/// invite the billing test user into the primary user's org first, then attempt
/// the transfer.
use reqwest::StatusCode;
use serde_json::{Value, json};

mod common;
use common::*;

// ─── Auth guard tests ─────────────────────────────────────────────────────────

/// POST /api/billing/transfer requires authentication.
#[tokio::test]
async fn test_initiate_transfer_requires_auth() {
    let client = test_client();
    let response = client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": "someone@example.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// DELETE /api/billing/transfer requires authentication.
#[tokio::test]
async fn test_cancel_transfer_requires_auth() {
    let client = test_client();
    let response = client
        .delete(format!("{}/api/billing/transfer", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// POST /api/billing-transfer/:token/accept requires authentication.
#[tokio::test]
async fn test_accept_transfer_requires_auth() {
    let client = test_client();
    let response = client
        .post(format!(
            "{}/api/billing-transfer/some-fake-token/accept",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// GET /api/billing-transfer/:token is public (no auth required).
#[tokio::test]
async fn test_get_transfer_info_is_public() {
    let client = test_client();
    let response = client
        .get(format!(
            "{}/api/billing-transfer/nonexistent-token",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();

    // Should get 404 (not found) rather than 401 (unauthorized) — i.e., auth not required.
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "GET /api/billing-transfer/:token should not require authentication"
    );
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ─── Validation tests ─────────────────────────────────────────────────────────

/// Initiating a transfer without to_email should fail with 400.
#[tokio::test]
async fn test_initiate_transfer_missing_email() {
    let client = authenticated_client();
    let response = client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Initiating a transfer to a non-existent email should fail with 400.
#[tokio::test]
async fn test_initiate_transfer_unknown_email() {
    let client = authenticated_client();
    let response = client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": "nobody@nonexistent-domain-xyz.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Unknown email should return 400"
    );
}

/// Initiating a transfer to oneself should fail with 400.
#[tokio::test]
async fn test_initiate_transfer_to_self_fails() {
    // Get the primary user's own email.
    let client = authenticated_client();
    let me_response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(me_response.status(), StatusCode::OK);
    let me: Value = me_response.json().await.unwrap();
    let own_email = me["email"].as_str().unwrap().to_string();

    let response = client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": own_email }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Transferring to self should return 400"
    );
}

/// A non-owner should not be able to initiate a transfer on someone else's BA.
#[tokio::test]
async fn test_initiate_transfer_non_owner_forbidden() {
    // The billing test client is NOT the owner of the primary user's billing account.
    let primary_ba_id = get_billing_test_account_id().await;

    let secondary_client = billing_test_client();
    let response = secondary_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({
            "to_email": "someone@example.com",
            "billing_account_id": primary_ba_id
        }))
        .send()
        .await
        .unwrap();

    // Either 403 (knows about the BA but not the owner) or 404 (can't even see it).
    // We just need to ensure it is NOT a success (2xx).
    assert!(
        response.status().is_client_error(),
        "Non-owner should not be able to initiate transfer, got {}",
        response.status()
    );
}

// ─── Token lookup tests ───────────────────────────────────────────────────────

/// GET /api/billing-transfer/:token with a bogus token returns 404.
#[tokio::test]
async fn test_get_transfer_info_bogus_token_returns_404() {
    let client = test_client();
    let response = client
        .get(format!(
            "{}/api/billing-transfer/00000000-0000-0000-0000-000000000000",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Accepting a bogus token returns 404.
#[tokio::test]
async fn test_accept_transfer_bogus_token_returns_404() {
    let client = authenticated_client();
    let response = client
        .post(format!(
            "{}/api/billing-transfer/00000000-0000-0000-0000-000000000000/accept",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ─── Admin force-transfer tests ───────────────────────────────────────────────

/// POST /api/admin/billing-accounts/:id/transfer requires admin.
#[tokio::test]
async fn test_admin_force_transfer_requires_admin() {
    // Billing test client is not an admin.
    let client = billing_test_client();
    let ba_id = get_billing_test_account_id().await;

    let response = client
        .post(format!(
            "{}/api/admin/billing-accounts/{}/transfer",
            BASE_URL, ba_id
        ))
        .json(&json!({ "to_user_id": "some-user-id" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// Force-transferring to a user who is not a member fails with 400.
#[tokio::test]
async fn test_admin_force_transfer_non_member_fails() {
    let admin_client = authenticated_client();

    // Get the secondary user's billing account ID.
    let ba_id = get_billing_test_account_id().await;

    // The primary user is NOT a member of the secondary user's billing account's org.
    let primary_me_response = admin_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();
    let primary_me: Value = primary_me_response.json().await.unwrap();
    let primary_user_id = primary_me["id"].as_str().unwrap().to_string();

    let response = admin_client
        .post(format!(
            "{}/api/admin/billing-accounts/{}/transfer",
            BASE_URL, ba_id
        ))
        .json(&json!({ "to_user_id": primary_user_id }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Force-transfer to non-member should return 400"
    );
}

/// Force-transferring to the current owner fails with 400.
#[tokio::test]
async fn test_admin_force_transfer_to_current_owner_fails() {
    let admin_client = authenticated_client();

    // Get the primary user's billing account ID and their user ID.
    let primary_ba_response = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();
    let primary_ba: Value = primary_ba_response.json().await.unwrap();
    let primary_ba_id = primary_ba["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();

    let me_response = admin_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();
    let me: Value = me_response.json().await.unwrap();
    let primary_user_id = me["id"].as_str().unwrap().to_string();

    let response = admin_client
        .post(format!(
            "{}/api/admin/billing-accounts/{}/transfer",
            BASE_URL, primary_ba_id
        ))
        .json(&json!({ "to_user_id": primary_user_id }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Force-transfer to current owner should return 400"
    );
}

/// Admin force-transfer requires a non-empty to_user_id.
#[tokio::test]
async fn test_admin_force_transfer_missing_user_id() {
    let admin_client = authenticated_client();

    let primary_ba_response = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap();
    let primary_ba: Value = primary_ba_response.json().await.unwrap();
    let primary_ba_id = primary_ba["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = admin_client
        .post(format!(
            "{}/api/admin/billing-accounts/{}/transfer",
            BASE_URL, primary_ba_id
        ))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─── Cancel transfer tests ────────────────────────────────────────────────────

/// Cancelling when there is no pending transfer still returns 200 (idempotent).
#[tokio::test]
async fn test_cancel_transfer_no_pending_is_ok() {
    let client = authenticated_client();
    let response = client
        .delete(format!("{}/api/billing/transfer", BASE_URL))
        .send()
        .await
        .unwrap();

    // Should succeed even if there is nothing to cancel.
    assert_eq!(response.status(), StatusCode::OK);
}
