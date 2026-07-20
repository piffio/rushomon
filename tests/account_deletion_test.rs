use reqwest;
use serde_json::json;

mod common;

#[tokio::test]
async fn test_delete_account_requires_auth() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    let response = client
        .post(&format!("{}/api/auth/delete-account", base_url))
        .json(&json!({"confirmation": "DELETE"}))
        .send()
        .await
        .expect("Failed to call delete-account endpoint");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_delete_account_requires_confirmation() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    let token = common::get_test_jwt();

    let response = client
        .post(&format!("{}/api/auth/delete-account", base_url))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&json!({"confirmation": "WRONG"}))
        .send()
        .await
        .expect("Failed to call delete-account endpoint");

    assert_eq!(response.status(), 400);

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse response body");
    assert!(
        body["message"]
            .as_str()
            .unwrap_or("")
            .contains("confirmation")
    );
}

#[tokio::test]
async fn test_deletion_status_no_pending() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    let token = common::get_test_jwt();

    let response = client
        .get(&format!("{}/api/auth/deletion-status", base_url))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to call deletion-status endpoint");

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse response body");
    assert_eq!(body["pending"], false);
    assert!(body["scheduled_deletion_at"].is_null());
    assert!(body["days_remaining"].is_null());
}

#[tokio::test]
async fn test_cancel_deletion_without_pending() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    let token = common::get_test_jwt();

    let response = client
        .post(&format!("{}/api/auth/cancel-deletion", base_url))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to call cancel-deletion endpoint");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_delete_account_allows_solo_free_org() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    let token = common::get_test_jwt();

    // The test admin user is the sole owner of their free-tier org.
    // With the updated logic, solo free-tier org owners are allowed to delete
    // their account (the org will be auto-deleted during hard deletion).
    let response = client
        .post(&format!("{}/api/auth/delete-account", base_url))
        .header("Authorization", &format!("Bearer {}", token))
        .json(&json!({"confirmation": "DELETE"}))
        .send()
        .await
        .expect("Failed to call delete-account endpoint");

    // Should return 200 — deletion scheduled successfully
    assert_eq!(
        response.status(),
        200,
        "Solo free-tier org owner should be allowed to delete account"
    );

    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse response body");
    assert!(
        body["scheduled_deletion_at"].as_i64().is_some(),
        "Response should include scheduled_deletion_at"
    );
    assert_eq!(
        body["grace_period_seconds"].as_i64(),
        Some(7 * 24 * 60 * 60),
        "Grace period should be 7 days"
    );

    // Cancel the deletion so we don't leave the user in a pending state
    let _ = client
        .post(&format!("{}/api/auth/cancel-deletion", base_url))
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
}

#[tokio::test]
async fn test_deletion_status_requires_auth() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    let response = client
        .get(&format!("{}/api/auth/deletion-status", base_url))
        .send()
        .await
        .expect("Failed to call deletion-status endpoint");

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_cancel_deletion_requires_auth() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    let response = client
        .post(&format!("{}/api/auth/cancel-deletion", base_url))
        .send()
        .await
        .expect("Failed to call cancel-deletion endpoint");

    assert_eq!(response.status(), 401);
}
