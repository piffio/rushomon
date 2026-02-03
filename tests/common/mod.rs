use reqwest::{Client, Response};
use serde_json::{Value, json};

pub const BASE_URL: &str = "http://localhost:8787";

/// Helper to create a test HTTP client that doesn't follow redirects
pub fn test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none()) // Don't auto-follow redirects
        .build()
        .unwrap()
}

/// Helper to create a client that follows redirects
pub fn following_client() -> Client {
    Client::new()
}

/// Helper to create a test link and return the response
pub async fn create_test_link(url: &str, title: Option<&str>) -> Response {
    let client = test_client();
    let mut body = json!({"destination_url": url});

    if let Some(t) = title {
        body["title"] = json!(t);
    }

    client
        .post(&format!("{}/api/links", BASE_URL))
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

/// Helper to poll for a condition with timeout
/// Used for testing async operations like analytics logging
pub async fn poll_until<F, Fut>(mut condition: F, max_attempts: u32, interval_ms: u64) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..max_attempts {
        if condition().await {
            return true;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;
    }
    false
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
