// Allow dead_code warnings - these functions are used across different test files
// but Rust's test compilation model treats each test file as a separate crate
#![allow(dead_code)]

use reqwest::{Client, Response, StatusCode};
use serde_json::{Value, json};

/// The webhook secret injected into .dev.vars by run-integration-tests.sh
pub const POLAR_WEBHOOK_SECRET: &str = "test-polar-webhook-secret";

/// The price ID used to seed a fake cached_product row for webhook tests.
pub const TEST_PRICE_ID: &str = "price-test-pro-monthly";

/// The Polar product ID used for the test product row.
pub const TEST_POLAR_PRODUCT_ID: &str = "prod-test-pro-monthly";

/// Compute a Standard Webhooks HMAC-SHA256 signature string (`v1,<base64>`).
///
/// This mirrors the production `verify_polar_webhook_signature` signing algorithm
/// so integration tests can generate valid signatures without importing wasm-only crates.
pub fn sign_webhook_payload(body: &str, webhook_id: &str, timestamp: &str, secret: &str) -> String {
    let raw_secret = secret.strip_prefix("whsec_").unwrap_or(secret);
    let to_sign = format!("{}.{}.{}", webhook_id, timestamp, body);

    // Pure-Rust HMAC-SHA256 using only std — avoids pulling in extra deps in tests.
    // We replicate the algorithm using the RFC 2104 definition manually.
    let key = raw_secret.as_bytes();
    let msg = to_sign.as_bytes();

    // SHA-256 block size is 64 bytes
    const BLOCK_SIZE: usize = 64;

    // Pad/hash the key to block size
    let mut k = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        // Hash the key with SHA-256 if it's too long (unlikely for our test secrets)
        let hashed = sha256(key);
        k[..32].copy_from_slice(&hashed);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    // ipad and opad
    let mut ipad = [0u8; BLOCK_SIZE];
    let mut opad = [0u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] = k[i] ^ 0x36;
        opad[i] = k[i] ^ 0x5c;
    }

    // Inner hash: SHA256(ipad || message)
    let mut inner = Vec::with_capacity(BLOCK_SIZE + msg.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(msg);
    let inner_hash = sha256(&inner);

    // Outer hash: SHA256(opad || inner_hash)
    let mut outer = Vec::with_capacity(BLOCK_SIZE + 32);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash);
    let result = sha256(&outer);

    let b64 = base64_encode(&result);
    format!("v1,{}", b64)
}

/// Minimal pure-Rust SHA-256 implementation for test use only.
/// Uses the standard SHA-256 constants and message schedule.
fn sha256(data: &[u8]) -> [u8; 32] {
    // SHA-256 initial hash values (first 32 bits of fractional parts of square roots of first 8 primes)
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    // SHA-256 round constants (first 32 bits of fractional parts of cube roots of first 64 primes)
    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    // Pre-processing: adding padding bits
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0x00);
    }
    // Append original length in bits as 64-bit big-endian
    for i in (0..8).rev() {
        msg.push(((bit_len >> (i * 8)) & 0xff) as u8);
    }

    // Process each 512-bit chunk
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut result = [0u8; 32];
    for (i, &word) in h.iter().enumerate() {
        let bytes = word.to_be_bytes();
        result[i * 4..i * 4 + 4].copy_from_slice(&bytes);
    }
    result
}

/// Base64 encode bytes (standard alphabet, no line breaks).
fn base64_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() {
            bytes[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            bytes[i + 2] as u32
        } else {
            0
        };
        result.push(CHARS[((b0 >> 2) & 0x3f) as usize] as char);
        result.push(CHARS[(((b0 << 4) | (b1 >> 4)) & 0x3f) as usize] as char);
        result.push(if i + 1 < bytes.len() {
            CHARS[(((b1 << 2) | (b2 >> 6)) & 0x3f) as usize] as char
        } else {
            '='
        });
        result.push(if i + 2 < bytes.len() {
            CHARS[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
        i += 3;
    }
    result
}

/// Current Unix timestamp as a string (for webhook headers).
pub fn webhook_timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

/// Generate a unique webhook message ID for each test call.
pub fn webhook_message_id(label: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("msg-test-{}-{}", label, ts)
}

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

/// Get the billing test JWT from environment variable.
/// This is a second, non-admin user created during billing test setup.
/// Panics if TEST_BILLING_JWT is not set — run scripts/run-integration-tests.sh first.
pub fn get_billing_test_jwt() -> String {
    std::env::var("TEST_BILLING_JWT")
        .expect("TEST_BILLING_JWT not set. Run: ./scripts/run-integration-tests.sh")
}

/// Create an authenticated HTTP client for the dedicated billing test user.
/// This user is separate from the primary admin test user to avoid polluting shared state.
pub fn billing_test_client() -> Client {
    let jwt = get_billing_test_jwt();
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

/// Get the billing account ID for the billing test user via GET /api/billing/status.
/// Creates the billing account if it doesn't exist yet (lazy creation).
pub async fn get_billing_test_account_id() -> String {
    let client = billing_test_client();
    let response = client
        .get(format!("{}/api/billing/status", BASE_URL))
        .send()
        .await
        .expect("Failed to call /api/billing/status");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "billing/status should return 200 for billing test user"
    );

    let body: Value = response
        .json()
        .await
        .expect("Failed to parse billing/status");
    body["billing_account_id"]
        .as_str()
        .expect("billing_account_id should be present in billing/status response")
        .to_string()
}

/// Send a Polar webhook event to the running wrangler dev server with a valid signature.
/// Returns the HTTP response.
pub async fn send_signed_webhook(payload: &Value) -> reqwest::Response {
    let body_str = serde_json::to_string(payload).unwrap();
    let id = webhook_message_id("wh");
    let ts = webhook_timestamp_now();
    let sig = sign_webhook_payload(&body_str, &id, &ts, POLAR_WEBHOOK_SECRET);

    let client = test_client();
    client
        .post(format!("{}/api/billing/webhook", BASE_URL))
        .header("webhook-id", &id)
        .header("webhook-timestamp", &ts)
        .header("webhook-signature", &sig)
        .header("content-type", "application/json")
        .body(body_str)
        .send()
        .await
        .expect("Failed to send webhook")
}

/// Build a subscription.active webhook payload for the billing test user.
pub fn make_subscription_active_payload(
    billing_account_id: &str,
    polar_customer_id: &str,
    polar_subscription_id: &str,
    price_id: &str,
    current_period_end_iso: &str,
) -> Value {
    json!({
        "type": "subscription.active",
        "data": {
            "id": polar_subscription_id,
            "customer_id": polar_customer_id,
            "customer": {
                "external_id": billing_account_id
            },
            "prices": [{ "id": price_id }],
            "recurringInterval": "month",
            "status": "active",
            "cancel_at_period_end": false,
            "current_period_start": "2025-01-01T00:00:00Z",
            "current_period_end": current_period_end_iso,
            "ends_at": null,
            "amount": 999,
            "currency": "usd",
            "discount": null
        }
    })
}

/// Build a subscription.canceled webhook payload.
pub fn make_subscription_canceled_payload(
    billing_account_id: &str,
    polar_customer_id: &str,
    polar_subscription_id: &str,
    cancel_at_period_end: bool,
    current_period_end_iso: &str,
) -> Value {
    let status = if cancel_at_period_end {
        "active"
    } else {
        "canceled"
    };
    json!({
        "type": "subscription.canceled",
        "data": {
            "id": polar_subscription_id,
            "customer_id": polar_customer_id,
            "customer": {
                "external_id": billing_account_id
            },
            "status": status,
            "cancel_at_period_end": cancel_at_period_end,
            "current_period_start": "2025-01-01T00:00:00Z",
            "current_period_end": current_period_end_iso,
            "ends_at": null
        }
    })
}

/// Build a subscription.updated payload with cancel_at_period_end=true.
/// Used to simulate the Polar "cancel at period end" flow where the subscription
/// remains active until `current_period_end`, then the cron job downgrades.
pub fn make_subscription_updated_cancel_at_period_end_payload(
    billing_account_id: &str,
    polar_customer_id: &str,
    polar_subscription_id: &str,
    price_id: &str,
    current_period_end_iso: &str,
) -> Value {
    json!({
        "type": "subscription.updated",
        "data": {
            "id": polar_subscription_id,
            "customer_id": polar_customer_id,
            "customer": {
                "external_id": billing_account_id
            },
            "status": "active",
            "cancel_at_period_end": true,
            "current_period_start": "2025-01-01T00:00:00Z",
            "current_period_end": current_period_end_iso,
            "ends_at": null,
            "prices": [{"id": price_id}],
            "recurringInterval": "month",
            "amount": 999,
            "currency": "usd",
            "discount": null
        }
    })
}

/// Build a subscription.revoked webhook payload (always immediate downgrade).
pub fn make_subscription_revoked_payload(
    billing_account_id: &str,
    polar_customer_id: &str,
    polar_subscription_id: &str,
) -> Value {
    json!({
        "type": "subscription.revoked",
        "data": {
            "id": polar_subscription_id,
            "customer_id": polar_customer_id,
            "customer": {
                "external_id": billing_account_id
            },
            "status": "canceled",
            "cancel_at_period_end": false,
            "current_period_start": "2025-01-01T00:00:00Z",
            "current_period_end": "2025-02-01T00:00:00Z",
            "ends_at": null
        }
    })
}
