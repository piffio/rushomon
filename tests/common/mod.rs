// Allow dead_code warnings - these functions are used across different test files
// but Rust's test compilation model treats each test file as a separate crate
#![allow(dead_code)]

use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};

pub const BASE_URL: &str = "http://localhost:8787";

/// Helper to create a test HTTP client that doesn't follow redirects (unauthenticated)
pub fn test_client() -> Client {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

/// Helper to create an authenticated test client (convenience wrapper)
pub fn create_test_client() -> Client {
    authenticated_client()
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

/// Get admin JWT token for testing admin functionality
/// Uses the existing test JWT from the integration test environment
pub async fn get_admin_jwt(_client: &Client, _base_url: &str) -> String {
    // For integration tests, we use the existing TEST_JWT
    // The test environment sets up an admin user automatically
    get_test_jwt()
}

/// Get regular user JWT token for testing user functionality
/// Uses the existing test JWT (integration tests use admin user for simplicity)
pub async fn get_user_jwt(_client: &Client, _base_url: &str) -> String {
    // For integration tests, we use the same TEST_JWT
    // In a real scenario, you'd create a separate user via OAuth
    get_test_jwt()
}

/// Get test Polar webhook secret from environment
/// Panics if POLAR_WEBHOOK_SECRET is not set
pub fn get_test_webhook_secret() -> String {
    std::env::var("POLAR_WEBHOOK_SECRET")
        .expect("POLAR_WEBHOOK_SECRET not set. Run: ./scripts/run-integration-tests.sh")
}

/// Generate HMAC-SHA256 signature for webhook payload
/// Returns signature in format: "v1,<base64_encoded_signature>"
pub fn sign_webhook_payload(payload: &str, secret: &str, timestamp: i64) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // The signed content is: "<webhook-id>.<webhook-timestamp>.<body>"
    let webhook_id = "test_webhook_id";
    let to_sign = format!("{}.{}.{}", webhook_id, timestamp, payload);

    let secret_bytes = secret.strip_prefix("whsec_").unwrap_or(secret).as_bytes();
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret_bytes).expect("HMAC can take key of any size");
    mac.update(to_sign.as_bytes());
    let result = mac.finalize();
    let signature = BASE64.encode(result.into_bytes());

    format!("v1={}", signature)
}

/// Get current Unix timestamp for webhook
pub fn get_test_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Create a test subscription by sending webhook events
/// Returns the subscription ID created
pub async fn create_test_subscription(client: &Client, plan: &str, interval: &str) -> String {
    // For integration tests, we use the webhook to create subscriptions
    // In a real scenario, this would happen through Polar checkout flow
    // For now, we'll use admin API to set up the subscription state

    // Get billing account ID for test user
    let user_response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .expect("Failed to get user info");

    let user: Value = user_response
        .json()
        .await
        .expect("Failed to parse user response");
    let user_id = user["id"].as_str().expect("User should have id");

    // Get billing accounts
    let billing_response = client
        .get(format!("{}/api/admin/billing-accounts", BASE_URL))
        .send()
        .await
        .expect("Failed to get billing accounts");

    let billing_data: Value = billing_response
        .json()
        .await
        .expect("Failed to parse billing response");
    let billing_account_id = billing_data["accounts"]
        .as_array()
        .expect("Billing accounts should be array")
        .iter()
        .find(|a| a["owner_user_id"].as_str() == Some(user_id))
        .and_then(|a| a["id"].as_str())
        .expect("Should find billing account for user");

    // Set tier based on plan
    let tier = match plan {
        "pro" => "pro",
        "business" => "business",
        _ => "free",
    };

    let tier_response = client
        .put(format!(
            "{}/api/admin/billing-accounts/{}/tier",
            BASE_URL, billing_account_id
        ))
        .json(&json!({"tier": tier}))
        .send()
        .await
        .expect("Failed to set tier");

    assert_eq!(tier_response.status(), StatusCode::OK);

    billing_account_id.to_string()
}

/// Send a webhook event to the test server
/// Returns the response status and body
pub async fn send_webhook_event(
    client: &Client,
    event_type: &str,
    data: Value,
) -> (StatusCode, Value) {
    let secret = get_test_webhook_secret();
    let timestamp = get_test_timestamp();

    let payload = json!({
        "type": event_type,
        "data": data,
        "timestamp": format!("{}-00-00T00:00:00Z", timestamp)
    });

    let payload_str = payload.to_string();
    let signature = sign_webhook_payload(&payload_str, &secret, timestamp);

    let response = client
        .post(format!("{}/api/billing/webhook", BASE_URL))
        .header("webhook-signature", signature)
        .header("webhook-id", "test_webhook_id")
        .header("webhook-timestamp", timestamp.to_string())
        .json(&payload)
        .send()
        .await
        .expect("Failed to send webhook");

    let status = response.status();
    let body: Value = response.json().await.unwrap_or(json!({}));

    (status, body)
}
