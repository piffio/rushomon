use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_create_link_with_random_short_code() {
    let response = create_test_link("https://example.com/test-page", Some("Test Link")).await;

    assert_eq!(response.status(), StatusCode::OK);

    let link: serde_json::Value = response.json().await.unwrap();

    // Verify response structure
    assert!(link["id"].is_string());
    assert!(link["short_code"].is_string());
    assert_eq!(link["short_code"].as_str().unwrap().len(), 6);
    assert_eq!(link["destination_url"], "https://example.com/test-page");
    assert_eq!(link["title"], "Test Link");
    assert_eq!(link["is_active"], true);
    assert_eq!(link["click_count"], 0);
}

#[tokio::test]
async fn test_create_link_with_custom_short_code() {
    let client = test_client();

    // Generate unique short code for this test run to avoid collisions
    let custom_code = unique_short_code("gh");

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://github.com",
            "short_code": custom_code,
            "title": "GitHub"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let link: serde_json::Value = response.json().await.unwrap();
    assert_eq!(link["short_code"], custom_code);
}

#[tokio::test]
async fn test_create_duplicate_short_code_fails() {
    let client = test_client();

    // Generate unique code for this test run
    let unique_code = unique_short_code("dup");

    // Create first link
    let _ = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com",
            "short_code": unique_code
        }))
        .send()
        .await
        .unwrap();

    // Try to create with same short code (should fail)
    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://other.com",
            "short_code": unique_code
        }))
        .send()
        .await
        .unwrap();

    // Should return either 409 Conflict or 500 (current implementation)
    assert!(
        response.status() == StatusCode::CONFLICT || response.status().is_server_error(),
        "Expected 409 or 5xx, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_list_links() {
    let client = test_client();

    // Create a few test links
    create_test_link("https://example.com/1", Some("Link 1")).await;
    create_test_link("https://example.com/2", Some("Link 2")).await;

    // List links
    let response = client
        .get(&format!("{}/api/links", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let links: Vec<serde_json::Value> = response.json().await.unwrap();
    assert!(links.len() >= 2);
}

#[tokio::test]
async fn test_get_link_by_id() {
    let client = test_client();

    // Create a link
    let create_response = create_test_link("https://example.com", None).await;
    let created_link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = created_link["id"].as_str().unwrap();

    // Get link by ID
    let response = client
        .get(&format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let link: serde_json::Value = response.json().await.unwrap();
    assert_eq!(link["id"], link_id);
}

#[tokio::test]
async fn test_delete_link() {
    let client = test_client();

    // Create a link
    let create_response = create_test_link("https://example.com", None).await;
    let created_link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = created_link["id"].as_str().unwrap();
    let short_code = created_link["short_code"].as_str().unwrap();

    // Delete link
    let delete_response = client
        .delete(&format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // Verify redirect no longer works
    let redirect_response = client
        .get(&format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    assert_eq!(redirect_response.status(), StatusCode::NOT_FOUND); // 404
}
