use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_admin_list_users_requires_auth() {
    let client = test_client();

    let response = client
        .get(&format!("{}/api/admin/users", BASE_URL))
        .send()
        .await
        .unwrap();

    // Should return 401 without authentication
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_list_users_returns_users() {
    let client = authenticated_client();

    let response = client
        .get(&format!("{}/api/admin/users?page=1&limit=50", BASE_URL))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(status, StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    // Verify response structure
    assert!(body["users"].is_array());
    assert!(body["total"].is_number());
    assert!(body["page"].is_number());
    assert!(body["limit"].is_number());

    // There should be at least one user (the test user)
    let users = body["users"].as_array().unwrap();
    assert!(!users.is_empty());

    // Verify user structure
    let first_user = &users[0];
    assert!(first_user["id"].is_string());
    assert!(first_user["email"].is_string());
    assert!(first_user["role"].is_string());
    let role = first_user["role"].as_str().unwrap();
    assert!(role == "admin" || role == "member");
}

#[tokio::test]
async fn test_admin_get_user_not_found() {
    let client = authenticated_client();

    let response = client
        .get(&format!("{}/api/admin/users/nonexistent-user-id", BASE_URL))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_update_user_invalid_role() {
    let client = authenticated_client();

    let response = client
        .put(&format!("{}/api/admin/users/some-user-id", BASE_URL))
        .json(&json!({ "role": "superadmin" }))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    // Should reject invalid role values
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_update_user_missing_role() {
    let client = authenticated_client();

    let response = client
        .put(&format!("{}/api/admin/users/some-user-id", BASE_URL))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    // Should reject missing role field
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_cannot_demote_self() {
    let client = authenticated_client();

    // First, get current user to find our own ID
    let me_response = client
        .get(&format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    if me_response.status() != StatusCode::OK {
        println!("Could not get current user - skipping test");
        return;
    }

    let me: serde_json::Value = me_response.json().await.unwrap();
    let my_id = me["id"].as_str().unwrap();

    // Only run if we're admin
    if me["role"].as_str().unwrap() != "admin" {
        println!("Test user is not an admin - skipping test");
        return;
    }

    // Try to demote ourselves
    let response = client
        .put(&format!("{}/api/admin/users/{}", BASE_URL, my_id))
        .json(&json!({ "role": "member" }))
        .send()
        .await
        .unwrap();

    // Should be rejected
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.text().await.unwrap();
    assert!(body.contains("Cannot demote yourself"));
}

#[tokio::test]
async fn test_first_user_is_admin() {
    let client = authenticated_client();

    // Get current user info
    let response = client
        .get(&format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let user: serde_json::Value = response.json().await.unwrap();
    assert_eq!(user["role"], "admin");
}

#[tokio::test]
async fn test_admin_reset_counter_requires_auth() {
    let client = test_client(); // Unauthenticated client

    let response = client
        .post(&format!(
            "{}/api/admin/orgs/test-org-id/reset-counter",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();

    // Should return 401 without authentication
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_reset_counter_admin_access() {
    let client = authenticated_client(); // First user is admin

    // Get current user info to get org_id
    let user_response = client
        .get(&format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    let user: serde_json::Value = user_response.json().await.unwrap();
    let org_id = user["org_id"]
        .as_str()
        .expect("Failed to get organization ID");

    // Reset counter as admin user
    let response = client
        .post(&format!(
            "{}/api/admin/orgs/{}/reset-counter",
            BASE_URL, org_id
        ))
        .send()
        .await
        .unwrap();

    // Should return 200 for admin user
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap_or(false));
    assert_eq!(body["message"], "Monthly counter reset successfully");
}
