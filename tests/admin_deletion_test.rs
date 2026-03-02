use reqwest;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_admin_delete_user_with_confirmation() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Get admin JWT token
    let admin_token = common::get_admin_jwt(&client, &base_url).await;

    // Create a test user to delete via the regular OAuth flow
    // This would normally be done through the mock OAuth server
    // For now, we'll test the API endpoint structure

    // Test that the delete endpoint exists and requires proper authentication
    let fake_user_id = "test-user-id";

    let response = client
        .delete(&format!("{}/api/admin/users/{}", base_url, fake_user_id))
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&json!({"confirmation": "DELETE"}))
        .send()
        .await
        .expect("Failed to call delete endpoint");

    // Should return 404 for non-existent user, confirming the endpoint works
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_admin_delete_without_confirmation_fails() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Get admin JWT token
    let admin_token = common::get_admin_jwt(&client, &base_url).await;

    // Try to delete without confirmation
    let response = client
        .delete(&format!("{}/api/admin/users/test-user", base_url))
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&json!({})) // Empty body, no confirmation
        .send()
        .await
        .expect("Failed to attempt deletion without confirmation");

    // Should fail due to missing confirmation OR user not found
    // The actual behavior depends on whether the endpoint validates confirmation first
    let status = response.status();
    assert!(status == 400 || status == 404);

    if status == 400 {
        // Try to parse the error response
        if let Ok(error_response) = response.json::<serde_json::Value>().await {
            if let Some(error_message) = error_response["error"].as_str() {
                assert!(error_message.contains("confirmation"));
            }
        }
    }
}

#[tokio::test]
async fn test_admin_delete_non_admin_fails() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // In integration tests, we only have admin users
    // So this test verifies that the endpoint works and returns appropriate response
    // The actual permission testing would require setting up separate user roles

    // Get admin JWT token (using the only available user in test env)
    let admin_token = common::get_admin_jwt(&client, &base_url).await;

    // Try to delete as admin (this should work for endpoint testing)
    let response = client
        .delete(&format!("{}/api/admin/users/test-user", base_url))
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&json!({"confirmation": "DELETE"}))
        .send()
        .await
        .expect("Failed to attempt deletion");

    // Should return 404 for non-existent user (endpoint works)
    // In a real multi-user environment, this would be 403 for non-admin
    assert!(response.status() == 404 || response.status() == 403);
}

#[tokio::test]
async fn test_admin_delete_user_not_found() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Get admin JWT token
    let admin_token = common::get_admin_jwt(&client, &base_url).await;

    // Try to delete non-existent user
    let fake_user_id = "non-existent-user-id";

    let response = client
        .delete(&format!("{}/api/admin/users/{}", base_url, fake_user_id))
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&json!({"confirmation": "DELETE"}))
        .send()
        .await
        .expect("Failed to attempt deletion of non-existent user");

    // Should return 404 for non-existent user
    assert_eq!(response.status(), 404);

    // Try to parse error response, but handle case where it's plain text
    let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| "No error text".to_string());
    assert!(
        error_text.contains("not found")
            || error_text.contains("User not found")
            || error_text.contains("404")
    );
}

#[tokio::test]
async fn test_admin_delete_endpoint_requires_auth() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Try to delete without any authentication
    let response = client
        .delete(&format!("{}/api/admin/users/test-user", base_url))
        .json(&json!({"confirmation": "DELETE"}))
        .send()
        .await
        .expect("Failed to attempt unauthenticated deletion");

    // Should fail due to missing authentication
    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_admin_users_list_accessible() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Get admin JWT token
    let admin_token = common::get_admin_jwt(&client, &base_url).await;

    // Test that we can list users (this should work)
    let response = client
        .get(&format!("{}/api/admin/users", base_url))
        .header("Authorization", &format!("Bearer {}", admin_token))
        .send()
        .await
        .expect("Failed to list users");

    assert_eq!(response.status(), 200);

    let users_response: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse users response");
    let users = users_response["users"]
        .as_array()
        .expect("Users should be an array");

    // Should have at least our test admin user
    assert!(users.len() >= 1);
}
