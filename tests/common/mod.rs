// Allow dead_code warnings - these functions are used across different test files
// but Rust's test compilation model treats each test file as a separate crate
#![allow(dead_code)]

use reqwest::{Client, Response};
use serde_json::{Value, json};

pub const BASE_URL: &str = "http://localhost:8787";

/// Helper to create a test HTTP client that doesn't follow redirects (unauthenticated)
pub fn test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

/// Get the test JWT from environment variable
/// Panics if TEST_JWT is not set - run scripts/run-integration-tests.sh first
pub fn get_test_jwt() -> String {
    std::env::var("TEST_JWT").expect("TEST_JWT not set. Run: ./scripts/run-integration-tests.sh")
}

/// Get the user ID from the test JWT
/// The mock OAuth server generates user IDs starting at 1000, so the first test user is "1000"
pub fn get_test_user_id() -> String {
    "1000".to_string()
}

// Test user ID - matches the ID created during test setup
pub const TEST_USER_ID: &str = "1000";

/// Create an authenticated test client using JWT from environment
pub fn authenticated_client() -> Client {
    let jwt = get_test_jwt();
    let cookie = format!("rushomon_session={}", jwt);

    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .default_headers(reqwest::header::HeaderMap::from_iter([(
            reqwest::header::COOKIE,
            cookie.parse().unwrap(),
        )]))
        .build()
        .unwrap()
}

/// Helper to create a test link (authenticated) and return the response
pub async fn create_test_link(url: &str, title: Option<&str>) -> Response {
    let client = authenticated_client();
    let mut body = json!({"destination_url": url});

    if let Some(t) = title {
        body["title"] = json!(t);
    }

    client
        .post(format!("{}/api/links", BASE_URL))
        .json(&body)
        .send()
        .await
        .expect("Failed to create test link")
}

/// Helper to extract short_code from create response
pub async fn create_link_and_get_code(url: &str) -> String {
    let response = create_test_link(url, None).await;
    let link: Value = response.json().await.unwrap();
    link["short_code"].as_str().unwrap().to_string()
}

/// Helper to create a test link and return error response (for testing blocked URLs)
pub async fn create_test_link_expect_error(url: &str, title: Option<&str>) -> String {
    let client = authenticated_client();
    let mut body = json!({"destination_url": url});

    if let Some(t) = title {
        body["title"] = json!(t);
    }

    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&body)
        .send()
        .await
        .expect("Failed to create test link");

    let status = response.status();
    let text = response.text().await.unwrap();

    format!("Status: {}, Response: {}", status, text)
}

/// Get the primary org ID for the test user.
/// Uses GET /api/orgs and returns the first org where the user is an owner.
/// This is more reliable than GET /api/auth/me which reads from KV and can
/// return a stale org_id if switch-org was called by a parallel test.
pub async fn get_primary_test_org_id() -> String {
    let client = authenticated_client();
    let response = client
        .get(format!("{}/api/orgs", BASE_URL))
        .send()
        .await
        .expect("Failed to call /api/orgs");

    let body: Value = response
        .json()
        .await
        .expect("Failed to parse /api/orgs response");
    let orgs = body["orgs"].as_array().expect("orgs should be an array");

    // Return the first org where the user is owner — this is the primary/initial org
    orgs.iter()
        .find(|o| o["role"].as_str() == Some("owner"))
        .and_then(|o| o["id"].as_str())
        .expect("Test user should have at least one owned org")
        .to_string()
}

/// Generate a unique short code for testing
/// Uses timestamp to avoid collisions between test runs
pub fn unique_short_code(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}{}", prefix, timestamp % 100000)
}
