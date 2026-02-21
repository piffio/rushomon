use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_list_links_requires_auth() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/links", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_links_basic() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/links?page=1&limit=10", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["data"].is_array());
    assert!(body["pagination"].is_object());
}

#[tokio::test]
async fn test_list_links_with_search() {
    let client = authenticated_client();

    // Create a link with a unique title for searching
    let unique_title = format!("SearchTest-{}", unique_short_code("st"));
    let create_response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/search-test",
            "title": unique_title,
            "short_code": unique_short_code("srch")
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let created_link: serde_json::Value = create_response.json().await.unwrap();
    let short_code = created_link["short_code"].as_str().unwrap();

    // Search by title
    let response = client
        .get(format!(
            "{}/api/links?search={}",
            BASE_URL,
            urlencoding::encode(&unique_title)
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"].as_array().unwrap();
    assert!(
        !links.is_empty(),
        "Should find at least one link matching the title"
    );

    // Search by short_code
    let response = client
        .get(format!("{}/api/links?search={}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"].as_array().unwrap();
    assert!(!links.is_empty(), "Should find the link by short_code");

    // Search by destination URL
    let response = client
        .get(format!("{}/api/links?search=search-test", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"].as_array().unwrap();
    assert!(!links.is_empty(), "Should find the link by destination URL");

    // Search for non-existent term
    let response = client
        .get(format!("{}/api/links?search=nonexistentxyz123", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"].as_array().unwrap();
    assert!(
        links.is_empty(),
        "Should return empty array for non-matching search"
    );
}

#[tokio::test]
async fn test_list_links_with_status_filter() {
    let client = authenticated_client();

    // Create an active link
    let active_response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/active-test",
            "title": "Active Test Link"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(active_response.status(), StatusCode::OK);

    // Get all links
    let response = client
        .get(format!("{}/api/links?limit=100", BASE_URL))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = response.json().await.unwrap();
    let total_all = body["pagination"]["total"].as_i64().unwrap();

    // Filter by active status
    let response = client
        .get(format!("{}/api/links?status=active", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"].as_array().unwrap();
    let total_active = body["pagination"]["total"].as_i64().unwrap();

    // All returned links should be active
    for link in links {
        assert_eq!(link["status"].as_str().unwrap(), "active");
    }

    // Active count should be <= total count
    assert!(
        total_active <= total_all,
        "Active count should not exceed total"
    );
}

#[tokio::test]
async fn test_list_links_with_sort() {
    let client = authenticated_client();

    // Get links sorted by clicks
    let response = client
        .get(format!("{}/api/links?sort=clicks", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"].as_array().unwrap();

    // Verify clicks are in descending order
    if links.len() >= 2 {
        let clicks: Vec<i64> = links
            .iter()
            .map(|l| l["click_count"].as_i64().unwrap_or(0))
            .collect();
        for i in 0..clicks.len() - 1 {
            assert!(
                clicks[i] >= clicks[i + 1],
                "Links should be sorted by clicks in descending order"
            );
        }
    }

    // Get links sorted by title
    let response = client
        .get(format!("{}/api/links?sort=title", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Get links sorted by code (short_code)
    let response = client
        .get(format!("{}/api/links?sort=code", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_links_combined_filters() {
    let client = authenticated_client();

    // Create a link with specific title
    let unique_title = format!("CombinedTest-{}", unique_short_code("ct"));
    let create_response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/combined-test",
            "title": unique_title
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    // Test combined search + status filter
    let response = client
        .get(format!(
            "{}/api/links?search={}&status=active&sort=created",
            BASE_URL,
            urlencoding::encode(&unique_title)
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"].as_array().unwrap();

    // Should find the link with all filters applied
    assert!(!links.is_empty());
    assert_eq!(links[0]["status"].as_str().unwrap(), "active");

    // Pagination should reflect filtered count
    let pagination = body["pagination"].as_object().unwrap();
    assert!(pagination["total"].as_i64().unwrap() >= 1);
}

#[tokio::test]
async fn test_list_links_pagination_with_filters() {
    let client = authenticated_client();

    // Get first page with small limit
    let response = client
        .get(format!(
            "{}/api/links?page=1&limit=5&sort=created",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let pagination = body["pagination"].as_object().unwrap();

    assert_eq!(pagination["page"].as_i64().unwrap(), 1);
    assert_eq!(pagination["limit"].as_i64().unwrap(), 5);

    // If there are more pages, test pagination works with filters
    if pagination["total_pages"].as_i64().unwrap() > 1 {
        let response = client
            .get(format!(
                "{}/api/links?page=2&limit=5&status=active",
                BASE_URL
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value = response.json().await.unwrap();
        let pagination = body["pagination"].as_object().unwrap();
        assert_eq!(pagination["page"].as_i64().unwrap(), 2);
    }
}

#[tokio::test]
async fn test_list_links_empty_search_param() {
    let client = authenticated_client();

    // Empty search param should behave same as no search
    let response = client
        .get(format!("{}/api/links?search=", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_links_invalid_sort_fallback() {
    let client = authenticated_client();

    // Invalid sort parameter should fall back to default (created)
    let response = client
        .get(format!("{}/api/links?sort=invalid", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    // Should still return results with default sorting
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn test_list_links_case_insensitive_search() {
    let client = authenticated_client();

    // Create a link with mixed case title
    let create_response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/case-test",
            "title": "MiXeDCaseTitle"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    // Search with different case - SQLite LIKE is case-insensitive by default
    let response = client
        .get(format!("{}/api/links?search=mixedcase", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let links = body["data"].as_array().unwrap();
    assert!(!links.is_empty(), "Search should be case-insensitive");
}
