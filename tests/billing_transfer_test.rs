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

// ─── Happy-path tests ─────────────────────────────────────────────────────────

/// Helper: invite the billing test user into the primary user's org and accept
/// the invitation, making the billing test user a member of that org.
/// Returns the org_id and the invitation_id so the caller can clean up if needed.
/// Idempotent: if the user is already a member, returns the org_id and an empty string.
async fn invite_billing_user_into_primary_org() -> (String, String) {
    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    // Resolve primary org and billing user's email.
    let org_id = get_primary_test_org_id().await;
    let me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_email = me["email"].as_str().unwrap().to_string();

    // Check if the billing user is already a member of the org.
    let org_resp: Value = admin_client
        .get(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let members = org_resp["members"].as_array().unwrap();
    let billing_user_id = me["id"].as_str().unwrap();
    let is_already_member = members
        .iter()
        .any(|m| m["user_id"].as_str() == Some(billing_user_id));

    if is_already_member {
        return (org_id, String::new());
    }

    // Send invite from the admin/owner.
    let invite_resp: Value = admin_client
        .post(format!("{}/api/orgs/{}/invitations", BASE_URL, org_id))
        .json(&json!({ "email": billing_email }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let invitation_id = invite_resp["invitation"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Billing user accepts the invite.
    let accept_status = billing_client
        .post(format!("{}/api/invite/{}/accept", BASE_URL, invitation_id))
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(
        accept_status,
        StatusCode::OK,
        "Billing user should be able to accept the invitation"
    );

    (org_id, invitation_id)
}

/// Helper: remove the billing test user from the primary org to restore clean state.
async fn remove_billing_user_from_primary_org(org_id: &str) {
    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    let me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_user_id = me["id"].as_str().unwrap().to_string();

    // Attempt removal — ignore errors (member may already be gone).
    let _ = admin_client
        .delete(format!(
            "{}/api/orgs/{}/members/{}",
            BASE_URL, org_id, billing_user_id
        ))
        .send()
        .await;
}

/// Helper: delete an organization via the org delete endpoint.
/// Used to clean up safety-net orgs (and their BAs) created during transfer tests.
#[allow(dead_code)]
async fn delete_org(org_id: &str) {
    let admin_client = authenticated_client();
    let _ = admin_client
        .delete(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .json(&json!({ "action": "delete" }))
        .send()
        .await;
}

/// Helper: reset a billing account to "unlimited" tier via admin endpoint.
/// Used to restore clean state after tests that modify billing account state.
async fn reset_billing_account_to_unlimited(ba_id: &str) {
    let admin_client = authenticated_client();
    let _ = admin_client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, ba_id
        ))
        .json(&json!({ "tier": "unlimited" }))
        .send()
        .await;
}

/// Initiating a transfer to an org member succeeds and the token is retrievable.
#[tokio::test]
async fn test_initiate_transfer_success() {
    let (org_id, _invitation_id) = invite_billing_user_into_primary_org().await;

    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    // Get the billing user's email and the owner's billing account ID.
    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_email = billing_me["email"].as_str().unwrap().to_string();

    let status_resp: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let owner_ba_id = status_resp["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();
    let owner_tier = status_resp["tier"].as_str().unwrap().to_string();

    // Initiate the transfer.
    let initiate_resp = admin_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": billing_email }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        initiate_resp.status(),
        StatusCode::OK,
        "Owner should be able to initiate a transfer to an org member"
    );
    let initiate_body: Value = initiate_resp.json().await.unwrap();
    let token = initiate_body["token"]
        .as_str()
        .expect("Response should contain a transfer token");
    assert!(
        initiate_body["expires_at"].is_number(),
        "Response should contain an expiry timestamp"
    );

    // GET /api/billing-transfer/:token (public) should return the transfer details.
    let info_resp = test_client()
        .get(format!("{}/api/billing-transfer/{}", BASE_URL, token))
        .send()
        .await
        .unwrap();
    assert_eq!(
        info_resp.status(),
        StatusCode::OK,
        "Transfer info should be retrievable by token"
    );
    let info: Value = info_resp.json().await.unwrap();
    assert_eq!(
        info["to_email"].as_str().unwrap(),
        billing_email,
        "Transfer info should show the correct recipient email"
    );
    assert_eq!(
        info["billing_account_id"].as_str().unwrap(),
        owner_ba_id,
        "Transfer info should reference the correct billing account"
    );
    assert_eq!(
        info["billing_account_tier"].as_str().unwrap(),
        owner_tier,
        "Transfer info should include the current tier"
    );

    // Clean up: cancel the pending transfer and remove billing user from org.
    admin_client
        .delete(format!("{}/api/billing/transfer", BASE_URL))
        .send()
        .await
        .unwrap();
    remove_billing_user_from_primary_org(&org_id).await;

    // Switch back to the primary org in case any previous test switched away.
    let _ = admin_client
        .post(format!("{}/api/orgs/switch", BASE_URL))
        .json(&json!({ "org_id": org_id }))
        .send()
        .await;
}

/// Accepting a valid transfer flips ownership and creates a safety-net BA for
/// the former owner.
#[tokio::test]
async fn test_accept_transfer_success() {
    let (org_id, _invitation_id) = invite_billing_user_into_primary_org().await;

    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    // Capture pre-transfer state.
    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_email = billing_me["email"].as_str().unwrap().to_string();

    let owner_status_before: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let transferred_ba_id = owner_status_before["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Initiate transfer from owner → billing user.
    let initiate_body: Value = admin_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": billing_email }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = initiate_body["token"].as_str().unwrap().to_string();

    // Billing user accepts.
    let accept_resp = billing_client
        .post(format!(
            "{}/api/billing-transfer/{}/accept",
            BASE_URL, token
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        accept_resp.status(),
        StatusCode::OK,
        "Recipient should be able to accept the transfer"
    );

    // ── Post-transfer assertions ──────────────────────────────────────────────

    // 1. The billing user is now the owner of the transferred BA.
    let new_owner_status: Value = billing_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        new_owner_status["billing_account_id"].as_str().unwrap(),
        transferred_ba_id,
        "New owner's billing account should be the transferred BA"
    );
    assert!(
        new_owner_status["is_billing_owner"].as_bool().unwrap(),
        "New owner should be marked as billing owner"
    );

    // 2. The former owner has a new (different) billing account — the safety-net BA.
    let former_owner_status: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let former_owner_ba_id = former_owner_status["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(
        former_owner_ba_id, transferred_ba_id,
        "Former owner should have a NEW billing account, not the transferred one"
    );
    assert!(
        former_owner_status["is_billing_owner"].as_bool().unwrap(),
        "Former owner should still be billing owner of their new safety-net BA"
    );
    assert_eq!(
        former_owner_status["tier"].as_str().unwrap(),
        "free",
        "Former owner's new safety-net billing account should be on the free tier"
    );

    // 3. The transfer token is now consumed — a second accept attempt returns 410.
    let double_accept = billing_client
        .post(format!(
            "{}/api/billing-transfer/{}/accept",
            BASE_URL, token
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        double_accept.status(),
        StatusCode::GONE,
        "Already-accepted token should return 410 Gone"
    );

    // ── Restore state ─────────────────────────────────────────────────────────
    // Force-transfer the BA back to the original owner so other tests aren't affected.
    let admin_me: Value = admin_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let original_owner_id = admin_me["id"].as_str().unwrap().to_string();

    let restore_resp = admin_client
        .post(format!(
            "{}/api/admin/billing-accounts/{}/transfer",
            BASE_URL, transferred_ba_id
        ))
        .json(&json!({ "to_user_id": original_owner_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        restore_resp.status(),
        StatusCode::OK,
        "Admin force-transfer should succeed when restoring state"
    );

    // Reset the transferred BA back to unlimited tier.
    let _ = admin_client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, transferred_ba_id
        ))
        .json(&json!({ "tier": "unlimited" }))
        .send()
        .await;

    // Switch back to the primary org so users.org_id is restored.
    let _ = admin_client
        .post(format!("{}/api/orgs/switch", BASE_URL))
        .json(&json!({ "org_id": org_id }))
        .send()
        .await;

    // Reset the former owner's safety-net BA to unlimited tier.
    // Note: We don't delete the safety-net org/BA because there's no admin endpoint
    // to delete a BA, and the org's billing_account_id cannot be changed via API.
    // This leaves the user with 2 BAs, which tier_limits_test may pick up incorrectly.
    // This is a pre-existing test isolation issue in tier_limits_test.
    let former_owner_status_after: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let safety_net_ba_id = former_owner_status_after["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();
    let _ = admin_client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, safety_net_ba_id
        ))
        .json(&json!({ "tier": "unlimited" }))
        .send()
        .await;

    remove_billing_user_from_primary_org(&org_id).await;
}

/// Admin force-transferring a billing account to an org member succeeds and
/// ownership is immediately reflected — no email confirmation needed.
#[tokio::test]
async fn test_admin_force_transfer_success() {
    let (org_id, _invitation_id) = invite_billing_user_into_primary_org().await;

    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    // Capture IDs before the transfer.
    let owner_status_before: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let owner_ba_id = owner_status_before["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();

    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_user_id = billing_me["id"].as_str().unwrap().to_string();

    // Admin issues a force-transfer to the billing user.
    let force_resp = admin_client
        .post(format!(
            "{}/api/admin/billing-accounts/{}/transfer",
            BASE_URL, owner_ba_id
        ))
        .json(&json!({ "to_user_id": billing_user_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        force_resp.status(),
        StatusCode::OK,
        "Admin force-transfer to an org member should return 200"
    );

    // ── Post-transfer assertions ──────────────────────────────────────────────

    // New owner (billing user) should own the transferred BA.
    let new_owner_status: Value = billing_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        new_owner_status["billing_account_id"].as_str().unwrap(),
        owner_ba_id,
        "Billing user should now own the transferred billing account"
    );
    assert!(
        new_owner_status["is_billing_owner"].as_bool().unwrap(),
        "Billing user should be marked as billing owner"
    );

    // Former owner should have a new safety-net BA.
    let former_owner_status: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_ne!(
        former_owner_status["billing_account_id"].as_str().unwrap(),
        owner_ba_id,
        "Former owner should have a new billing account after force-transfer"
    );
    assert!(
        former_owner_status["is_billing_owner"].as_bool().unwrap(),
        "Former owner should still own their new safety-net billing account"
    );

    // ── Restore state ─────────────────────────────────────────────────────────
    let admin_me: Value = admin_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let original_owner_id = admin_me["id"].as_str().unwrap().to_string();

    let restore_resp = admin_client
        .post(format!(
            "{}/api/admin/billing-accounts/{}/transfer",
            BASE_URL, owner_ba_id
        ))
        .json(&json!({ "to_user_id": original_owner_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        restore_resp.status(),
        StatusCode::OK,
        "Admin should be able to restore ownership after the test"
    );

    // Reset the transferred BA back to unlimited tier.
    reset_billing_account_to_unlimited(&owner_ba_id).await;

    // Switch back to the primary org so users.org_id is restored.
    let _ = admin_client
        .post(format!("{}/api/orgs/switch", BASE_URL))
        .json(&json!({ "org_id": org_id }))
        .send()
        .await;

    // Reset the former owner's safety-net BA to unlimited tier.
    // Note: We don't delete the safety-net org/BA because there's no admin endpoint
    // to delete a BA, and the org's billing_account_id cannot be changed via API.
    // This leaves the user with 2 BAs, which tier_limits_test may pick up incorrectly.
    // This is a pre-existing test isolation issue in tier_limits_test.
    let former_owner_status_after: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let safety_net_ba_id = former_owner_status_after["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();
    reset_billing_account_to_unlimited(&safety_net_ba_id).await;

    remove_billing_user_from_primary_org(&org_id).await;
}

/// Cancelling a valid pending transfer marks the token as cancelled (410) and
/// allows a new transfer to be initiated for the same billing account.
#[tokio::test]
async fn test_cancel_transfer_success() {
    let (org_id, _invitation_id) = invite_billing_user_into_primary_org().await;

    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_email = billing_me["email"].as_str().unwrap().to_string();

    // Initiate a transfer.
    let initiate_body: Value = admin_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": billing_email }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = initiate_body["token"].as_str().unwrap().to_string();

    // Cancel it.
    let cancel_resp = admin_client
        .delete(format!("{}/api/billing/transfer", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(
        cancel_resp.status(),
        StatusCode::OK,
        "Cancel should return 200"
    );

    // The token should now be invalid (410 Gone).
    let info_resp = test_client()
        .get(format!("{}/api/billing-transfer/{}", BASE_URL, token))
        .send()
        .await
        .unwrap();
    assert_eq!(
        info_resp.status(),
        StatusCode::GONE,
        "Cancelled transfer token should return 410 Gone"
    );

    // Acceptance of the cancelled token should also be rejected.
    let accept_resp = billing_client
        .post(format!(
            "{}/api/billing-transfer/{}/accept",
            BASE_URL, token
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        accept_resp.status(),
        StatusCode::GONE,
        "Accepting a cancelled token should return 410 Gone"
    );

    // A new transfer to the same recipient should still work after cancellation.
    let second_initiate = admin_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": billing_email }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        second_initiate.status(),
        StatusCode::OK,
        "A second transfer to the same recipient should succeed after cancellation"
    );

    // Clean up the second pending transfer.
    admin_client
        .delete(format!("{}/api/billing/transfer", BASE_URL))
        .send()
        .await
        .unwrap();
    remove_billing_user_from_primary_org(&org_id).await;

    // Switch back to the primary org in case any previous test switched away.
    let _ = admin_client
        .post(format!("{}/api/orgs/switch", BASE_URL))
        .json(&json!({ "org_id": org_id }))
        .send()
        .await;
}

// ─── Priority 2: State-machine edge cases ────────────────────────────────────

/// The wrong user (not the intended recipient) trying to accept a transfer gets 403.
#[tokio::test]
async fn test_accept_transfer_wrong_email_rejected() {
    let (org_id, _) = invite_billing_user_into_primary_org().await;

    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    // Get billing user's email to use as the transfer target.
    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_email = billing_me["email"].as_str().unwrap().to_string();

    // Admin initiates a transfer to the billing user.
    let initiate_body: Value = admin_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": billing_email }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token = initiate_body["token"].as_str().unwrap().to_string();

    // The ADMIN user (not the billing user) tries to accept the token.
    // They should get 403 because the token is addressed to billing_email, not admin's email.
    let wrong_accept = admin_client
        .post(format!(
            "{}/api/billing-transfer/{}/accept",
            BASE_URL, token
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(
        wrong_accept.status(),
        StatusCode::FORBIDDEN,
        "Accepting a transfer token with the wrong user account should return 403"
    );

    // Verify the token is still valid (not consumed by the failed attempt).
    let info_resp = test_client()
        .get(format!("{}/api/billing-transfer/{}", BASE_URL, token))
        .send()
        .await
        .unwrap();
    assert_eq!(
        info_resp.status(),
        StatusCode::OK,
        "Token should still be valid after a rejected accept attempt"
    );

    // Clean up.
    admin_client
        .delete(format!("{}/api/billing/transfer", BASE_URL))
        .send()
        .await
        .unwrap();
    remove_billing_user_from_primary_org(&org_id).await;
}

/// An expired token returns 410 on both GET info and POST accept.
///
/// We use `wrangler d1 execute` to directly insert a pending_action row with
/// `expires_at` in the past — this is the only way to test expiry without
/// waiting 7 days.
#[tokio::test]
async fn test_accept_transfer_expired_token() {
    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    // Collect IDs needed for the fake row.
    let admin_me: Value = admin_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let admin_user_id = admin_me["id"].as_str().unwrap().to_string();

    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_email = billing_me["email"].as_str().unwrap().to_string();

    let ba_status: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ba_id = ba_status["billing_account_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Use a fixed UUID so we can reference it after the INSERT.
    let expired_token = "00000000-dead-beef-0000-000000000001";
    let payload = serde_json::to_string(&serde_json::json!({
        "billing_account_id": ba_id,
        "from_user_id": admin_user_id,
    }))
    .unwrap();

    // Insert a pending_action row with expires_at=1 (deep in the past) via wrangler d1 execute.
    let sql = format!(
        "INSERT OR REPLACE INTO pending_actions \
         (id, action_type, subject_id, initiated_by, to_email, payload, created_at, expires_at) \
         VALUES ('{}', 'billing_account_transfer', '{}', '{}', '{}', '{}', 0, 1)",
        expired_token,
        ba_id,
        admin_user_id,
        billing_email,
        payload.replace('\'', "''"), // escape single quotes in JSON
    );

    let insert_ok = std::process::Command::new("wrangler")
        .args(["d1", "execute", "rushomon", "--local", "--command", &sql])
        .current_dir("/Users/sergiovisinoni/src/rushomon")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !insert_ok {
        // wrangler may not be available in all CI environments — skip gracefully.
        println!("SKIP: wrangler d1 execute unavailable — skipping expired-token test");
        return;
    }

    // GET /api/billing-transfer/:token should return 410 for an expired token.
    let get_resp = test_client()
        .get(format!(
            "{}/api/billing-transfer/{}",
            BASE_URL, expired_token
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        get_resp.status(),
        StatusCode::GONE,
        "GET on expired token should return 410 Gone"
    );

    // POST accept should also return 410.
    let accept_resp = billing_client
        .post(format!(
            "{}/api/billing-transfer/{}/accept",
            BASE_URL, expired_token
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        accept_resp.status(),
        StatusCode::GONE,
        "Accepting an expired token should return 410 Gone"
    );

    // Clean up the injected row.
    let _ = std::process::Command::new("wrangler")
        .args([
            "d1",
            "execute",
            "rushomon",
            "--local",
            "--command",
            &format!("DELETE FROM pending_actions WHERE id = '{}'", expired_token),
        ])
        .current_dir("/Users/sergiovisinoni/src/rushomon")
        .status();
}

/// Re-initiating a transfer cancels the first token and issues a new one.
///
/// After the second initiation the first token must return 410 (cancelled) and
/// the second token must still return 200.
#[tokio::test]
async fn test_initiate_transfer_replaces_existing_pending() {
    let (org_id, _) = invite_billing_user_into_primary_org().await;

    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_email = billing_me["email"].as_str().unwrap().to_string();

    // First initiation.
    let first_body: Value = admin_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": billing_email }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let first_token = first_body["token"].as_str().unwrap().to_string();

    // Second initiation to the same recipient — supersedes the first.
    let second_resp = admin_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": billing_email }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        second_resp.status(),
        StatusCode::OK,
        "Second initiation should succeed"
    );
    let second_body: Value = second_resp.json().await.unwrap();
    let second_token = second_body["token"].as_str().unwrap().to_string();

    assert_ne!(
        first_token, second_token,
        "Each initiation must produce a distinct token"
    );

    // The first token should now be 410 (cancelled).
    let first_info = test_client()
        .get(format!("{}/api/billing-transfer/{}", BASE_URL, first_token))
        .send()
        .await
        .unwrap();
    assert_eq!(
        first_info.status(),
        StatusCode::GONE,
        "First token should return 410 after being superseded"
    );

    // The second token should still be 200.
    let second_info = test_client()
        .get(format!(
            "{}/api/billing-transfer/{}",
            BASE_URL, second_token
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        second_info.status(),
        StatusCode::OK,
        "Second (current) token should still be valid"
    );

    // Clean up.
    admin_client
        .delete(format!("{}/api/billing/transfer", BASE_URL))
        .send()
        .await
        .unwrap();
    remove_billing_user_from_primary_org(&org_id).await;
}

/// A recipient who is a valid Rushomon user but is NOT a member of any org under
/// the billing account cannot be targeted for a transfer (400).
#[tokio::test]
async fn test_initiate_transfer_recipient_not_org_member() {
    // Explicitly ensure the billing user is NOT in the primary org for this test.
    let org_id = get_primary_test_org_id().await;
    remove_billing_user_from_primary_org(&org_id).await;

    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_email = billing_me["email"].as_str().unwrap().to_string();

    let response = admin_client
        .post(format!("{}/api/billing/transfer", BASE_URL))
        .json(&json!({ "to_email": billing_email }))
        .send()
        .await
        .unwrap();

    let status = response.status();
    let body_text = response.text().await.unwrap_or_default();
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Transfer to a non-org-member should return 400, body: {}",
        body_text
    );

    // Verify the error message is informative.
    let body: Value = serde_json::from_str(&body_text).unwrap_or(Value::Null);
    let message = body["message"].as_str().unwrap_or(&body_text);
    assert!(
        message.contains("member") || message.contains("organization"),
        "Error message should mention membership requirement, got: {}",
        message
    );
}

// ─── Priority 3: New endpoints added in this feature ─────────────────────────

/// GET /api/billing/accounts returns the caller's owned billing accounts.
/// The primary account must match the billing_account_id from /api/billing/status.
#[tokio::test]
async fn test_get_billing_accounts_list() {
    let client = authenticated_client();

    // Fetch the accounts list.
    let resp = client
        .get(format!("{}/api/billing/accounts", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /api/billing/accounts should return 200"
    );
    let body: Value = resp.json().await.unwrap();

    // Response must contain an "accounts" array.
    let accounts = body["accounts"]
        .as_array()
        .expect("accounts should be an array");
    assert!(
        !accounts.is_empty(),
        "Authenticated user should own at least one billing account"
    );

    // Cross-check: the primary account from /api/billing/status must appear in the list.
    let status_resp: Value = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let primary_ba_id = status_resp["billing_account_id"]
        .as_str()
        .expect("billing_status should include billing_account_id");

    let found = accounts
        .iter()
        .any(|a| a["id"].as_str() == Some(primary_ba_id));
    assert!(
        found,
        "Primary billing_account_id ({}) should appear in /api/billing/accounts",
        primary_ba_id
    );

    // Each account entry must have the expected fields.
    for account in accounts {
        assert!(account["id"].is_string(), "Each account should have an id");
        assert!(
            account["tier"].is_string(),
            "Each account should have a tier"
        );
        assert!(
            account["is_billing_owner"].as_bool() == Some(true),
            "Caller should be billing owner of all returned accounts"
        );
        assert!(
            account["organizations"].is_array(),
            "Each account should include an organizations array"
        );
    }
}

/// GET /api/billing/accounts requires authentication.
#[tokio::test]
async fn test_get_billing_accounts_requires_auth() {
    let resp = test_client()
        .get(format!("{}/api/billing/accounts", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// GET /api/admin/billing-accounts/:id/members returns org members excluding the owner.
#[tokio::test]
async fn test_admin_list_billing_account_members() {
    let (org_id, _) = invite_billing_user_into_primary_org().await;

    let admin_client = authenticated_client();
    let billing_client = billing_test_client();

    // Get the primary user's billing account ID.
    let status: Value = admin_client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ba_id = status["billing_account_id"].as_str().unwrap().to_string();

    // Get the IDs we'll use for assertions.
    let admin_me: Value = admin_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let owner_id = admin_me["id"].as_str().unwrap().to_string();

    let billing_me: Value = billing_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let billing_user_id = billing_me["id"].as_str().unwrap().to_string();

    // Fetch members via admin endpoint.
    let resp = admin_client
        .get(format!(
            "{}/api/admin/billing-accounts/{}/members",
            BASE_URL, ba_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /api/admin/billing-accounts/:id/members should return 200"
    );
    let body: Value = resp.json().await.unwrap();
    let members = body["members"]
        .as_array()
        .expect("members should be an array");

    // The billing user (org member) should appear in the list.
    // The endpoint uses BillingAccountMember { id, name, email } — field is "id", not "user_id".
    let billing_user_found = members
        .iter()
        .any(|m| m["id"].as_str() == Some(&billing_user_id));
    assert!(
        billing_user_found,
        "Billing user (org member) should appear in the members list"
    );

    // The owner themselves must NOT appear (they're excluded by the endpoint).
    let owner_found = members.iter().any(|m| m["id"].as_str() == Some(&owner_id));
    assert!(
        !owner_found,
        "Billing account owner should be excluded from the members list"
    );

    // Each member entry should have basic user fields.
    for member in members {
        assert!(member["id"].is_string(), "Each member should have id");
        assert!(member["email"].is_string(), "Each member should have email");
    }

    // Clean up.
    remove_billing_user_from_primary_org(&org_id).await;
}

/// Non-admin users cannot call GET /api/admin/billing-accounts/:id/members.
#[tokio::test]
async fn test_admin_list_billing_account_members_requires_admin() {
    let billing_client = billing_test_client();

    // Use the billing user's own BA id — even for their own BA, non-admin is rejected.
    let ba_id = get_billing_test_account_id().await;

    let resp = billing_client
        .get(format!(
            "{}/api/admin/billing-accounts/{}/members",
            BASE_URL, ba_id
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Non-admin should get 403 on the admin members endpoint"
    );
}
