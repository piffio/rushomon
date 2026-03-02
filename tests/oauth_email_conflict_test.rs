use reqwest;

mod common;

#[tokio::test]
async fn test_oauth_callback_endpoint_exists() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Test that the OAuth callback endpoint exists and handles requests
    let response = client
        .get(&format!(
            "{}/api/auth/callback?code=test&state=test",
            base_url
        ))
        .send()
        .await
        .expect("Failed to call OAuth callback");

    // Should either succeed, redirect, return error, or unauthorized
    // The exact behavior depends on the mock OAuth server setup and state
    let status = response.status();
    assert!(
        status.is_success() || status == 302 || status == 400 || status == 401 || status == 500,
        "Unexpected status code: {}",
        status
    );
}

#[tokio::test]
async fn test_oauth_provider_compatibility() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Test that different OAuth providers are available
    let response = client
        .get(&format!("{}/api/auth/providers", base_url))
        .send()
        .await
        .expect("Failed to get auth providers");

    assert_eq!(response.status(), 200);

    let providers: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse providers response");
    let providers_array = providers["providers"]
        .as_array()
        .expect("Providers should be an array");

    // Should have at least GitHub and Google available
    assert!(providers_array.len() >= 2);

    let provider_names: Vec<&str> = providers_array
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();

    assert!(provider_names.contains(&"github"));
    assert!(provider_names.contains(&"google"));
}

#[tokio::test]
async fn test_oauth_me_endpoint_works() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Test that the /api/auth/me endpoint works with our test JWT
    let jwt = common::get_test_jwt();

    let response = client
        .get(&format!("{}/api/auth/me", base_url))
        .header("Authorization", &format!("Bearer {}", jwt))
        .send()
        .await
        .expect("Failed to call auth/me endpoint");

    assert_eq!(response.status(), 200);

    let user_info: serde_json::Value = response.json().await.expect("Failed to parse user info");

    // Should have user information
    assert!(user_info["id"].as_str().is_some());
    assert!(user_info["email"].as_str().is_some());
    assert!(user_info["oauth_provider"].as_str().is_some());
}

#[tokio::test]
async fn test_oauth_logout_endpoint_works() {
    let client = reqwest::Client::new();
    let base_url =
        std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Test that the logout endpoint works
    let response = client
        .post(&format!("{}/api/auth/logout", base_url))
        .send()
        .await
        .expect("Failed to call logout endpoint");

    // Should succeed, redirect, or return appropriate status
    let status = response.status();
    assert!(
        status.is_success() || status == 302 || status == 401,
        "Unexpected logout status: {}",
        status
    );
}
