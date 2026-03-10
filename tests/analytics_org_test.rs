use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_get_org_analytics_requires_auth() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/analytics/org", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_org_analytics_empty() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/analytics/org?days=7", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    assert!(body["total_clicks"].is_number(), "expected total_clicks");
    assert!(
        body["unique_links_clicked"].is_number(),
        "expected unique_links_clicked"
    );
    assert!(
        body["clicks_over_time"].is_array(),
        "expected clicks_over_time array"
    );
    assert!(body["top_links"].is_array(), "expected top_links array");
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
async fn test_get_org_analytics_default_days() {
    let client = authenticated_client();

    // No ?days= param — should default to 7 and still return valid response
    let response = client
        .get(format!("{}/api/analytics/org", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["total_clicks"].is_number());
}

#[tokio::test]
async fn test_get_org_analytics_with_clicks() {
    let client = authenticated_client();
    let redirect_client = test_client();

    // Create a link
    let create_response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/org-analytics-test",
            "title": "Org Analytics Test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);

    let created_link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = created_link["id"].as_str().unwrap().to_string();
    let short_code = created_link["short_code"].as_str().unwrap().to_string();

    // Generate 3 clicks with different referrers
    for i in 0..3 {
        let response = redirect_client
            .get(format!("{}/{}", BASE_URL, short_code))
            .header("User-Agent", format!("Mozilla/5.0 OrgAnalyticsBot/{}", i))
            .header("Referer", "https://twitter.com")
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

    // Wait for deferred analytics to complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Fetch org analytics
    let response = client
        .get(format!("{}/api/analytics/org?days=7", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    let total = body["total_clicks"].as_i64().unwrap();
    assert!(
        total >= 3,
        "Expected at least 3 org-level clicks, got {}",
        total
    );

    let unique_links = body["unique_links_clicked"].as_i64().unwrap();
    assert!(unique_links >= 1, "Expected at least 1 unique link clicked");

    // Top links should contain our link
    let top_links = body["top_links"].as_array().unwrap();
    assert!(!top_links.is_empty(), "Expected top_links to be non-empty");
    let found = top_links
        .iter()
        .any(|l| l["link_id"].as_str() == Some(&link_id));
    assert!(found, "Expected created link to appear in top_links");

    // Top referrers should contain twitter.com
    let top_referrers = body["top_referrers"].as_array().unwrap();
    let has_twitter = top_referrers.iter().any(|r| {
        r["referrer"]
            .as_str()
            .map(|s| s.contains("twitter.com"))
            .unwrap_or(false)
    });
    assert!(has_twitter, "Expected twitter.com in top referrers");
}

#[tokio::test]
async fn test_get_org_analytics_custom_range() {
    let client = authenticated_client();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let start = now - 7 * 24 * 60 * 60;

    let response = client
        .get(format!(
            "{}/api/analytics/org?start={}&end={}",
            BASE_URL, start, now
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["total_clicks"].is_number());
    assert!(body["clicks_over_time"].is_array());
}

#[tokio::test]
async fn test_get_org_analytics_top_links_structure() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/analytics/org?days=7", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    let top_links = body["top_links"].as_array().unwrap();

    // Verify structure of each top link entry
    for link in top_links {
        assert!(link["link_id"].is_string(), "link_id should be a string");
        assert!(
            link["short_code"].is_string(),
            "short_code should be a string"
        );
        assert!(link["count"].is_number(), "count should be a number");
        // title can be null or string
        assert!(
            link["title"].is_null() || link["title"].is_string(),
            "title should be null or string"
        );
    }
}
