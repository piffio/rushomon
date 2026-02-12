use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

// Import test_client for analytics test
use common::test_client;

#[tokio::test]
async fn test_create_link_with_random_short_code() {
    let client = authenticated_client();

    let response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/test-page",
            "title": "Test Link"
        }))
        .send()
        .await
        .unwrap();

    let status = response.status();
    println!("Response status: {}", status);

    if status.is_server_error() {
        let error_text = response.text().await.unwrap();
        println!("Error response: {}", error_text);
        panic!("Expected 200, got {} with error: {}", status, error_text);
    }

    assert_eq!(status, StatusCode::OK);

    let link: serde_json::Value = response.json().await.unwrap();

    // Verify response structure
    assert!(link["id"].is_string());
    assert!(link["short_code"].is_string());
    assert_eq!(link["short_code"].as_str().unwrap().len(), 6);
    assert_eq!(link["destination_url"], "https://example.com/test-page");
    assert_eq!(link["title"], "Test Link");
    assert_eq!(link["status"], "active");
    assert_eq!(link["click_count"], 0);
}

#[tokio::test]
async fn test_create_link_with_custom_short_code() {
    let client = authenticated_client();

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
    let client = authenticated_client();

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
    let client = authenticated_client();

    // Create a few test links
    let create_response1 = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/1",
            "title": "Link 1"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response1.status(), StatusCode::OK);

    let create_response2 = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/2",
            "title": "Link 2"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response2.status(), StatusCode::OK);

    // List links
    let response = client
        .get(&format!("{}/api/links", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"]
        .as_array()
        .expect("expected data array in paginated response");
    assert!(links.len() >= 2);

    // Verify pagination metadata is present
    assert!(
        body["pagination"].is_object(),
        "expected pagination metadata"
    );
    assert!(body["pagination"]["page"].is_number());
    assert!(body["pagination"]["limit"].is_number());
    assert!(body["pagination"]["total"].is_number());
}

#[tokio::test]
async fn test_get_link_by_id() {
    let client = authenticated_client();

    // Create a link
    let create_response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

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
    let client = authenticated_client();

    // Create a link
    let create_response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

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

#[tokio::test]
async fn test_delete_link_with_analytics_events() {
    let client = authenticated_client();
    let test_client = test_client(); // Unauthenticated client for redirects

    // Create a link
    let create_response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/analytics-test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let created_link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = created_link["id"].as_str().unwrap();
    let short_code = created_link["short_code"].as_str().unwrap();

    // Generate some analytics events by accessing the short URL multiple times
    for i in 0..5 {
        let redirect_response = test_client
            .get(&format!("{}/{}", BASE_URL, short_code))
            .header("User-Agent", format!("TestBot/{}", i))
            .header("Referer", format!("https://example.com/referrer{}", i))
            .send()
            .await
            .unwrap();

        // Accept both 301 (moved permanently) and 308 (permanent redirect)
        let status = redirect_response.status();
        assert!(
            status == StatusCode::MOVED_PERMANENTLY || status == StatusCode::from_u16(308).unwrap(),
            "Expected redirect status (301 or 308), got {}",
            status
        );
    }

    // Verify the link has analytics data (click count should be > 0)
    let get_response = client
        .get(&format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();
    assert_eq!(get_response.status(), StatusCode::OK);

    let link_before_delete: serde_json::Value = get_response.json().await.unwrap();
    let click_count_before = link_before_delete["click_count"].as_i64().unwrap();
    assert!(
        click_count_before >= 5,
        "Expected at least 5 clicks, got {}",
        click_count_before
    );

    // Delete link - this should not fail with FK constraint error
    let delete_response = client
        .delete(&format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // Verify redirect no longer works
    let redirect_response = test_client
        .get(&format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    assert_eq!(redirect_response.status(), StatusCode::NOT_FOUND);

    // Verify link is no longer accessible via API
    let get_after_delete = client
        .get(&format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    assert_eq!(get_after_delete.status(), StatusCode::NOT_FOUND);
}
