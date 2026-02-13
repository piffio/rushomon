use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_get_link_analytics_requires_auth() {
    let client = test_client();

    let response = client
        .get(&format!("{}/api/links/fake-id/analytics", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_link_analytics_not_found() {
    let client = authenticated_client();

    let response = client
        .get(&format!("{}/api/links/nonexistent-id/analytics", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_link_analytics_empty() {
    let client = authenticated_client();

    // Create a fresh link (no clicks yet)
    let create_response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/analytics-empty-test",
            "title": "Analytics Empty Test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let created_link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = created_link["id"].as_str().unwrap();

    // Fetch analytics
    let response = client
        .get(&format!("{}/api/links/{}/analytics", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    // Verify response structure
    assert!(body["link"].is_object(), "expected link object");
    assert_eq!(body["link"]["id"], link_id);
    assert!(
        body["total_clicks_in_range"].is_number(),
        "expected total_clicks_in_range"
    );
    assert_eq!(body["total_clicks_in_range"], 0);
    assert!(
        body["clicks_over_time"].is_array(),
        "expected clicks_over_time array"
    );
    assert!(
        body["top_referrers"].is_array(),
        "expected top_referrers array"
    );
    assert!(
        body["top_countries"].is_array(),
        "expected top_countries array"
    );
    assert!(
        body["top_user_agents"].is_array(),
        "expected top_user_agents array"
    );
}

#[tokio::test]
async fn test_get_link_analytics_with_clicks() {
    let client = authenticated_client();
    let redirect_client = test_client();

    // Create a link
    let create_response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/analytics-clicks-test",
            "title": "Analytics Clicks Test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let created_link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = created_link["id"].as_str().unwrap();
    let short_code = created_link["short_code"].as_str().unwrap();

    // Generate clicks with different user agents and referrers
    for i in 0..3 {
        let response = redirect_client
            .get(&format!("{}/{}", BASE_URL, short_code))
            .header("User-Agent", format!("Mozilla/5.0 TestBot/{}", i))
            .header("Referer", "https://google.com")
            .send()
            .await
            .unwrap();

        let status = response.status();
        assert!(
            status == StatusCode::MOVED_PERMANENTLY || status == StatusCode::from_u16(308).unwrap(),
            "Expected redirect, got {}",
            status
        );
    }

    // Wait briefly for deferred analytics to complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Fetch analytics
    let response = client
        .get(&format!("{}/api/links/{}/analytics", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    // Should have clicks
    let total = body["total_clicks_in_range"].as_i64().unwrap();
    assert!(
        total >= 3,
        "Expected at least 3 clicks in range, got {}",
        total
    );

    // Should have referrer data
    let referrers = body["top_referrers"].as_array().unwrap();
    assert!(
        !referrers.is_empty(),
        "Expected at least one referrer entry"
    );

    // Should have user agent data
    let agents = body["top_user_agents"].as_array().unwrap();
    assert!(!agents.is_empty(), "Expected at least one user agent entry");
}

#[tokio::test]
async fn test_get_link_analytics_with_time_range() {
    let client = authenticated_client();

    // Create a link
    let create_response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/analytics-range-test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let created_link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = created_link["id"].as_str().unwrap();

    // Fetch with explicit time range (last 7 days)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let start = now - 7 * 24 * 60 * 60;

    let response = client
        .get(&format!(
            "{}/api/links/{}/analytics?start={}&end={}",
            BASE_URL, link_id, start, now
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["link"].is_object());
    assert!(body["total_clicks_in_range"].is_number());
}

#[tokio::test]
async fn test_get_link_by_short_code() {
    let client = authenticated_client();

    // Create a link with a known short code
    let code = unique_short_code("an");

    let create_response = client
        .post(&format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/by-code-test",
            "short_code": code,
            "title": "By Code Test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    // Look up by short code
    let response = client
        .get(&format!("{}/api/links/by-code/{}", BASE_URL, code))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let link: serde_json::Value = response.json().await.unwrap();
    assert_eq!(link["short_code"], code);
    assert_eq!(link["title"], "By Code Test");
}

#[tokio::test]
async fn test_get_link_by_short_code_not_found() {
    let client = authenticated_client();

    let response = client
        .get(&format!("{}/api/links/by-code/nonexistent123", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
