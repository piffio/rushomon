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
    // This test verifies that the first user on the instance has admin role
    //
    // NOTE: In local testing, users persist between runs. To properly test this:
    // Run: ./scripts/clear-test-db.sh before running integration tests
    // This clears the D1 database so the test user becomes the first user.

    let client = authenticated_client();

    let response = client
        .get(&format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let user: serde_json::Value = response.json().await.unwrap();

    // In CI/ephemeral environments, this should pass automatically
    // In local development, run ./scripts/clear-test-db.sh first
    if user["role"] != "admin" {
        println!("⚠️  Test user is not admin. Run './scripts/clear-test-db.sh' to fix.");
        println!("   This clears the local D1 database so the test user becomes first user.");
    }

    assert_eq!(user["role"], "admin");
}
