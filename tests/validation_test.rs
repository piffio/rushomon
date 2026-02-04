use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_reject_javascript_url() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "javascript:alert(1)"
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reject_file_url() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "file:///etc/passwd"
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reject_data_uri() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "data:text/html,<script>alert(1)</script>"
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reject_malformed_url() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "not a url"
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_accept_valid_http_url() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "http://example.com/page"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_accept_valid_https_url() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/page?foo=bar"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_reject_short_code_too_short() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": "abc"  // 3 chars, minimum is 4
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reject_short_code_too_long() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": "abcdefghijk"  // 11 chars, maximum is 10
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reject_short_code_with_special_chars() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": "test-code"  // Hyphen not allowed
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reject_short_code_with_underscore() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": "test_code"  // Underscore not allowed
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reject_reserved_word_api() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": "api"  // Reserved word
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reject_reserved_word_auth() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": "auth"  // Reserved word
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_reserved_word_case_insensitive() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": "API"  // Reserved word (uppercase)
        }))
        .send()
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected 4xx or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_accept_valid_alphanumeric_code() {
    let client = authenticated_client();

    // Use unique code to avoid collisions between test runs
    let valid_code = unique_short_code("test");

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": valid_code
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
