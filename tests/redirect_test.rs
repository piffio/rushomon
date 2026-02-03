use reqwest::StatusCode;

mod common;
use common::*;

#[tokio::test]
async fn test_redirect_with_301() {
    let short_code = create_link_and_get_code("https://example.com/destination").await;
    let client = test_client(); // Doesn't follow redirects

    let response = client
        .get(&format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY); // 301
    assert_eq!(
        response.headers().get("location").unwrap(),
        "https://example.com/destination"
    );
}

#[tokio::test]
async fn test_nonexistent_short_code_returns_404() {
    let client = test_client();

    let response = client
        .get(&format!("{}/nonexistent999", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_redirect_increments_click_count() {
    let client = test_client();

    // Create link
    let create_response = create_test_link("https://example.com", None).await;
    let link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = link["id"].as_str().unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Initial click count should be 0
    assert_eq!(link["click_count"], 0);

    // Access the short link (analytics are now awaited, so should complete before redirect)
    let redirect_response = client
        .get(&format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    // Verify we got the redirect
    assert_eq!(
        redirect_response.status(),
        reqwest::StatusCode::MOVED_PERMANENTLY
    );

    // Get link and check click count (should be incremented immediately now)
    let get_response = client
        .get(&format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    let updated_link: serde_json::Value = get_response.json().await.unwrap();
    let click_count = updated_link["click_count"].as_i64().unwrap_or(0);

    assert_eq!(
        click_count, 1,
        "Click count should be 1 after redirect, got {}",
        click_count
    );
}

#[tokio::test]
async fn test_inactive_link_returns_404() {
    let client = test_client();

    // Create and then delete (soft delete) a link
    let create_response = create_test_link("https://example.com", None).await;
    let link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = link["id"].as_str().unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Delete the link (soft delete)
    let _ = client
        .delete(&format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    // Try to access the short link
    let response = client
        .get(&format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
