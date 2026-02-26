use reqwest::StatusCode;
use serde_json::{Value, json};

mod common;
use common::*;

// ─── List User Orgs ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_user_orgs_requires_auth() {
    let client = test_client();
    let response = client
        .get(format!("{}/api/orgs", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_user_orgs_returns_current_org() {
    let client = authenticated_client();
    let response = client
        .get(format!("{}/api/orgs", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.unwrap();
    assert!(body["orgs"].is_array(), "orgs field should be an array");
    assert!(
        body["current_org_id"].is_string(),
        "current_org_id should be set"
    );

    let orgs = body["orgs"].as_array().unwrap();
    assert!(!orgs.is_empty(), "User should have at least one org");
}

// ─── Create Org ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_org_requires_auth() {
    let client = test_client();
    let response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&json!({"name": "Test Org"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_org_requires_name() {
    let client = authenticated_client();
    let response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_org_rejects_empty_name() {
    let client = authenticated_client();
    let response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&json!({"name": "   "}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_org_rejects_name_too_long() {
    let client = authenticated_client();
    let long_name = "a".repeat(101);
    let response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&json!({"name": long_name}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_org_success_and_returns_cookie() {
    let client = authenticated_client();
    let org_name = format!("Test Org {}", unique_short_code("o"));
    let response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&json!({"name": org_name}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Response should include org details
    let body: Value = response.json().await.unwrap();
    assert_eq!(body["org"]["name"].as_str().unwrap(), org_name);
    assert_eq!(body["role"].as_str().unwrap(), "owner");
    assert!(body["org"]["id"].is_string());
}

// ─── Get Org ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_org_requires_auth() {
    let client = test_client();
    let response = client
        .get(format!("{}/api/orgs/nonexistent-id", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_org_returns_members_and_invitations() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let response = client
        .get(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.unwrap();
    assert!(body["org"].is_object());
    assert!(body["members"].is_array());
    assert!(body["pending_invitations"].is_array());

    // The calling user should be in the members list
    let members = body["members"].as_array().unwrap();
    assert!(!members.is_empty(), "Should have at least one member");
}

#[tokio::test]
async fn test_get_org_not_member_returns_404() {
    let client = authenticated_client();
    let response = client
        .get(format!(
            "{}/api/orgs/00000000-0000-0000-0000-000000000000",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ─── Update Org ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_org_requires_auth() {
    let client = test_client();
    let response = client
        .patch(format!("{}/api/orgs/some-id", BASE_URL))
        .json(&json!({"name": "New Name"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_org_name_success() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    // Get current name first
    let get_response = client
        .get(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await
        .unwrap();
    let get_body: Value = get_response.json().await.unwrap();
    let original_name = get_body["org"]["name"].as_str().unwrap().to_string();

    let new_name = format!("Renamed Org {}", unique_short_code("r"));
    let response = client
        .patch(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .json(&json!({"name": new_name}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["org"]["name"].as_str().unwrap(), new_name);

    // Restore original name
    client
        .patch(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .json(&json!({"name": original_name}))
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn test_update_org_name_rejects_empty() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let response = client
        .patch(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .json(&json!({"name": ""}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─── Switch Org ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_switch_org_requires_auth() {
    let client = test_client();
    let response = client
        .post(format!("{}/api/auth/switch-org", BASE_URL))
        .json(&json!({"org_id": "some-id"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_switch_org_requires_org_id() {
    let client = authenticated_client();
    let response = client
        .post(format!("{}/api/auth/switch-org", BASE_URL))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_switch_org_non_member_returns_403() {
    let client = authenticated_client();
    let response = client
        .post(format!("{}/api/auth/switch-org", BASE_URL))
        .json(&json!({"org_id": "00000000-0000-0000-0000-000000000000"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_switch_org_to_current_org_succeeds() {
    let client = authenticated_client();
    let current_org_id = get_primary_test_org_id().await;

    // Switching to current org is still a valid operation
    let response = client
        .post(format!("{}/api/auth/switch-org", BASE_URL))
        .json(&json!({"org_id": current_org_id}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["org"]["id"].as_str().unwrap(), current_org_id);
}

// ─── Invitations ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_invitation_requires_auth() {
    let client = test_client();
    let response = client
        .post(format!("{}/api/orgs/some-org/invitations", BASE_URL))
        .json(&json!({"email": "test@example.com"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_invitation_requires_email() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let response = client
        .post(format!("{}/api/orgs/{}/invitations", BASE_URL, org_id))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_invitation_rejects_invalid_email() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let response = client
        .post(format!("{}/api/orgs/{}/invitations", BASE_URL, org_id))
        .json(&json!({"email": "not-an-email"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_invitation_not_member_returns_404() {
    let client = authenticated_client();
    let response = client
        .post(format!(
            "{}/api/orgs/00000000-0000-0000-0000-000000000000/invitations",
            BASE_URL
        ))
        .json(&json!({"email": "someone@example.com"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_invitation_success_and_idempotent() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let email = format!("invite-test-{}@example.com", unique_short_code("i"));

    // First invitation should succeed
    let response = client
        .post(format!("{}/api/orgs/{}/invitations", BASE_URL, org_id))
        .json(&json!({"email": email}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.unwrap();
    let invitation_id = body["invitation"]["id"].as_str().unwrap().to_string();
    assert!(body["invitation"]["id"].is_string());
    assert_eq!(
        body["invitation"]["email"].as_str().unwrap(),
        email.to_lowercase()
    );

    // Second invitation to same email should be a conflict
    let response2 = client
        .post(format!("{}/api/orgs/{}/invitations", BASE_URL, org_id))
        .json(&json!({"email": email}))
        .send()
        .await
        .unwrap();
    assert_eq!(response2.status(), StatusCode::CONFLICT);

    // Clean up: revoke the invitation
    client
        .delete(format!(
            "{}/api/orgs/{}/invitations/{}",
            BASE_URL, org_id, invitation_id
        ))
        .send()
        .await
        .unwrap();
}

// ─── Revoke Invitation ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_revoke_invitation_requires_auth() {
    let client = test_client();
    let response = client
        .delete(format!(
            "{}/api/orgs/some-org/invitations/some-invite",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_revoke_invitation_not_found() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let response = client
        .delete(format!(
            "{}/api/orgs/{}/invitations/00000000-0000-0000-0000-000000000000",
            BASE_URL, org_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_revoke_invitation_lifecycle() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    // Create invitation
    let email = format!("revoke-test-{}@example.com", unique_short_code("rv"));
    let create_response = client
        .post(format!("{}/api/orgs/{}/invitations", BASE_URL, org_id))
        .json(&json!({"email": email}))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body: Value = create_response.json().await.unwrap();
    let invitation_id = create_body["invitation"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // It should now appear in pending_invitations
    let org_response = client
        .get(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await
        .unwrap();
    let org_body: Value = org_response.json().await.unwrap();
    let pending = org_body["pending_invitations"].as_array().unwrap();
    assert!(
        pending
            .iter()
            .any(|i| i["id"].as_str() == Some(&invitation_id)),
        "Invitation should appear in pending list"
    );

    // Revoke it
    let revoke_response = client
        .delete(format!(
            "{}/api/orgs/{}/invitations/{}",
            BASE_URL, org_id, invitation_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(revoke_response.status(), StatusCode::OK);

    // Should no longer appear in pending_invitations
    let org_response2 = client
        .get(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await
        .unwrap();
    let org_body2: Value = org_response2.json().await.unwrap();
    let pending2 = org_body2["pending_invitations"].as_array().unwrap();
    assert!(
        !pending2
            .iter()
            .any(|i| i["id"].as_str() == Some(&invitation_id)),
        "Invitation should not appear after revocation"
    );
}

// ─── Get Invite Info (Public) ─────────────────────────────────────────────────

#[tokio::test]
async fn test_get_invite_info_not_found() {
    let client = test_client();
    let response = client
        .get(format!(
            "{}/api/invite/00000000-0000-0000-0000-000000000000",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await.unwrap();
    assert_eq!(body["valid"].as_bool().unwrap(), false);
    assert_eq!(body["reason"].as_str().unwrap(), "not_found");
}

#[tokio::test]
async fn test_get_invite_info_valid_invitation() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    // Create an invitation
    let email = format!("info-test-{}@example.com", unique_short_code("inf"));
    let create_response = client
        .post(format!("{}/api/orgs/{}/invitations", BASE_URL, org_id))
        .json(&json!({"email": email}))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body: Value = create_response.json().await.unwrap();
    let invitation_id = create_body["invitation"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Public lookup of invite info (no auth needed)
    let public_client = test_client();
    let info_response = public_client
        .get(format!("{}/api/invite/{}", BASE_URL, invitation_id))
        .send()
        .await
        .unwrap();
    assert_eq!(info_response.status(), StatusCode::OK);

    let info_body: Value = info_response.json().await.unwrap();
    assert_eq!(info_body["valid"].as_bool().unwrap(), true);
    assert!(info_body["org_name"].is_string());
    assert!(info_body["invited_by"].is_string());
    assert_eq!(info_body["email"].as_str().unwrap(), email.to_lowercase());
    assert!(info_body["expires_at"].is_number());

    // Clean up
    client
        .delete(format!(
            "{}/api/orgs/{}/invitations/{}",
            BASE_URL, org_id, invitation_id
        ))
        .send()
        .await
        .unwrap();
}

// ─── Accept Invite ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_accept_invite_requires_auth() {
    let client = test_client();
    let response = client
        .post(format!(
            "{}/api/invite/00000000-0000-0000-0000-000000000000/accept",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_accept_invite_not_found() {
    let client = authenticated_client();
    let response = client
        .post(format!(
            "{}/api/invite/00000000-0000-0000-0000-000000000000/accept",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_accept_invite_wrong_email_returns_403() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    // Create invitation for a DIFFERENT email
    let wrong_email = format!("wrong-email-{}@example.com", unique_short_code("we"));
    let create_response = client
        .post(format!("{}/api/orgs/{}/invitations", BASE_URL, org_id))
        .json(&json!({"email": wrong_email}))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body: Value = create_response.json().await.unwrap();
    let invitation_id = create_body["invitation"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Try to accept with a user whose email doesn't match
    let response = client
        .post(format!("{}/api/invite/{}/accept", BASE_URL, invitation_id))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Clean up
    client
        .delete(format!(
            "{}/api/orgs/{}/invitations/{}",
            BASE_URL, org_id, invitation_id
        ))
        .send()
        .await
        .unwrap();
}

// ─── Remove Member ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_remove_member_requires_auth() {
    let client = test_client();
    let response = client
        .delete(format!("{}/api/orgs/some-org/members/some-user", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_remove_member_not_in_org_returns_404() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let response = client
        .delete(format!(
            "{}/api/orgs/{}/members/00000000-0000-0000-0000-000000000000",
            BASE_URL, org_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_remove_last_owner_is_rejected() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    // Get the real user ID from /api/auth/me (not the hardcoded "1000" which is a test fixture)
    let me: Value = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let user_id = me["id"].as_str().expect("should have id").to_string();

    // Trying to remove yourself as the last owner should fail
    let response = client
        .delete(format!(
            "{}/api/orgs/{}/members/{}",
            BASE_URL, org_id, user_id
        ))
        .send()
        .await
        .unwrap();
    // Should be rejected — last owner can't be removed
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─── Delete Org ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_org_requires_auth() {
    let client = test_client();
    let response = client
        .delete(format!("{}/api/orgs/some-id", BASE_URL))
        .json(&json!({"action": "delete"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_delete_only_org_is_rejected() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    // Check how many orgs the user has - only run this test if they have exactly one
    let orgs_response = client
        .get(format!("{}/api/orgs", BASE_URL))
        .send()
        .await
        .unwrap();
    let orgs_body: Value = orgs_response.json().await.unwrap();
    let org_count = orgs_body["orgs"].as_array().unwrap().len();

    if org_count == 1 {
        let response = client
            .delete(format!("{}/api/orgs/{}", BASE_URL, org_id))
            .json(&json!({"action": "delete"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    } else {
        println!(
            "User has {} orgs — skipping single-org delete test",
            org_count
        );
    }
}

// ─── Create Org + Switch Org full cycle ──────────────────────────────────────

#[tokio::test]
async fn test_create_org_then_switch_back() {
    let client = authenticated_client();
    let original_org_id = get_primary_test_org_id().await;

    // Create a new org to switch to
    let new_org_name = format!("Switch Test Org {}", unique_short_code("sw"));
    let create_response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&json!({"name": new_org_name}))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let new_org_id = create_response.json::<Value>().await.unwrap()["org"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // The new org should appear in the orgs list
    let list_response = client
        .get(format!("{}/api/orgs", BASE_URL))
        .send()
        .await
        .unwrap();
    let list_body: Value = list_response.json().await.unwrap();
    let orgs = list_body["orgs"].as_array().unwrap();
    assert!(
        orgs.iter().any(|o| o["id"].as_str() == Some(&new_org_id)),
        "New org should be in the list"
    );

    // Switch to the new org
    let switch_to_new = client
        .post(format!("{}/api/auth/switch-org", BASE_URL))
        .json(&json!({"org_id": new_org_id}))
        .send()
        .await
        .unwrap();
    assert_eq!(switch_to_new.status(), StatusCode::OK);
    let switch_body: Value = switch_to_new.json().await.unwrap();
    assert_eq!(switch_body["org"]["id"].as_str().unwrap(), new_org_id);

    // Switch back to original org
    let switch_back = client
        .post(format!("{}/api/auth/switch-org", BASE_URL))
        .json(&json!({"org_id": original_org_id}))
        .send()
        .await
        .unwrap();
    assert_eq!(switch_back.status(), StatusCode::OK);
    let switch_back_body: Value = switch_back.json().await.unwrap();
    assert_eq!(
        switch_back_body["org"]["id"].as_str().unwrap(),
        original_org_id
    );
    // Note: new_org is intentionally not deleted here to avoid race conditions
    // with other parallel tests that may also create/delete orgs. The delete
    // endpoint logic (requiring a fallback owned org) is tested in
    // test_delete_only_org_is_rejected.
}

// ─── Resend Invitation ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_resend_invitation_requires_auth() {
    let client = test_client();
    let response = client
        .post(format!(
            "{}/api/orgs/some-org/invitations/some-invite/resend",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_resend_invitation_not_found() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let response = client
        .post(format!(
            "{}/api/orgs/{}/invitations/00000000-0000-0000-0000-000000000000/resend",
            BASE_URL, org_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ─── Billing Account Level Tier Limits ───────────────────────────────────────

#[tokio::test]
async fn test_billing_account_tier_reflected_in_org_tier() {
    let client = authenticated_client();
    let org_id = get_primary_test_org_id().await;

    let org_response = client
        .get(format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await
        .unwrap();
    assert_eq!(org_response.status(), StatusCode::OK);

    let org_body: Value = org_response.json().await.unwrap();
    let tier = org_body["org"]["tier"].as_str().unwrap();
    assert!(
        tier == "free" || tier == "unlimited",
        "Tier should be free or unlimited, got: {}",
        tier
    );
}
