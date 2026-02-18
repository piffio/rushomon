use reqwest::StatusCode;
use uuid;

mod common;
use common::*;

#[tokio::test]
async fn test_url_blocking_with_normalization() {
    let auth_client = authenticated_client();

    // Block a URL without trailing slash
    let block_response = auth_client
        .post(format!("{}/api/admin/blacklist", BASE_URL))
        .json(&serde_json::json!({
            "destination": "http://example.com",
            "match_type": "exact",
            "reason": "Test blocking with normalization"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(block_response.status(), StatusCode::OK);

    // Try to create a link with trailing slash - should be blocked
    let create_response = create_test_link_expect_error("http://example.com/", None).await;
    assert!(
        create_response.contains("blocked")
            || create_response.contains("Destination URL is blocked")
    );

    // Try to create a link with query parameters - should NOT be blocked (different content)
    let create_response2 =
        create_test_link_expect_error("http://example.com?param=value", None).await;
    println!("Query param test response: {}", create_response2);
    println!("DEBUG: Testing if 'http://example.com?param=value' is blocked");
    // Query parameters point to potentially different content, so they should NOT be blocked by exact match
    assert!(
        !create_response2.contains("blocked")
            && !create_response2.contains("Destination URL is blocked")
    );

    // Try to create a link with www prefix - should be blocked
    let create_response3 = create_test_link_expect_error("http://www.example.com", None).await;
    assert!(
        create_response3.contains("blocked")
            || create_response3.contains("Destination URL is blocked")
    );

    // Try to create a link with default port - should be blocked
    let create_response4 = create_test_link_expect_error("http://example.com:80", None).await;
    assert!(
        create_response4.contains("blocked")
            || create_response4.contains("Destination URL is blocked")
    );

    // Clean up - remove from blacklist
    let blacklist_entries: serde_json::Value = auth_client
        .get(format!("{}/api/admin/blacklist", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    if let Some(entries) = blacklist_entries.as_array() {
        for entry in entries {
            if entry["destination"].as_str() == Some("http://example.com") {
                let delete_response = auth_client
                    .delete(format!(
                        "{}/api/admin/blacklist/{}",
                        BASE_URL,
                        entry["id"].as_str().unwrap()
                    ))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(delete_response.status(), StatusCode::OK);
            }
        }
    }
}

#[tokio::test]
async fn test_url_blocking_domain_match_still_works() {
    let auth_client = authenticated_client();

    // Block a domain
    let block_response = auth_client
        .post(format!("{}/api/admin/blacklist", BASE_URL))
        .json(&serde_json::json!({
            "destination": "example.com",
            "match_type": "domain",
            "reason": "Test domain blocking"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(block_response.status(), StatusCode::OK);

    // Try to create a link with subdomain - should be blocked
    let create_response = create_test_link_expect_error("http://subdomain.example.com", None).await;
    assert!(
        create_response.contains("blocked")
            || create_response.contains("Destination URL is blocked")
    );

    // Clean up - remove from blacklist
    let blacklist_entries: serde_json::Value = auth_client
        .get(format!("{}/api/admin/blacklist", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    if let Some(entries) = blacklist_entries.as_array() {
        for entry in entries {
            if entry["destination"].as_str() == Some("example.com") {
                let delete_response = auth_client
                    .delete(format!(
                        "{}/api/admin/blacklist/{}",
                        BASE_URL,
                        entry["id"].as_str().unwrap()
                    ))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(delete_response.status(), StatusCode::OK);
            }
        }
    }
}

#[tokio::test]
async fn test_prevent_duplicate_blacklist() {
    let auth_client = authenticated_client();

    // Generate a unique URL for this test
    let unique_url = format!(
        "http://duplicate-test-{}.com",
        uuid::Uuid::new_v4().to_string()
    );

    // The URL will be normalized to add a trailing slash, so we need to use the normalized version for comparison
    let normalized_url = format!("{}/", unique_url);

    // Block a URL
    let block_response = auth_client
        .post(format!("{}/api/admin/blacklist", BASE_URL))
        .json(&serde_json::json!({
            "destination": unique_url,
            "match_type": "exact",
            "reason": "First block"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(block_response.status(), StatusCode::OK);
    let first_response: serde_json::Value = block_response.json().await.unwrap();
    assert_eq!(first_response["success"], true);

    // Try to block the same URL again
    let duplicate_response = auth_client
        .post(format!("{}/api/admin/blacklist", BASE_URL))
        .json(&serde_json::json!({
            "destination": unique_url,
            "match_type": "exact",
            "reason": "Duplicate block"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(duplicate_response.status(), StatusCode::OK);
    let duplicate_result: serde_json::Value = duplicate_response.json().await.unwrap();
    assert_eq!(duplicate_result["success"], false);
    assert_eq!(duplicate_result["already_blocked"], true);
    assert_eq!(
        duplicate_result["message"],
        "Destination is already blocked"
    );

    // Verify only one entry exists in the blacklist
    let blacklist_entries: serde_json::Value = auth_client
        .get(format!("{}/api/admin/blacklist", BASE_URL))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    if let Some(entries) = blacklist_entries.as_array() {
        let duplicate_entries = entries
            .iter()
            .filter(|e| e["destination"].as_str() == Some(&normalized_url))
            .count();
        assert_eq!(duplicate_entries, 1, "Should only have one blacklist entry");
    }

    // Clean up
    if let Some(entries) = blacklist_entries.as_array() {
        for entry in entries {
            if entry["destination"].as_str() == Some(&normalized_url) {
                let delete_response = auth_client
                    .delete(format!(
                        "{}/api/admin/blacklist/{}",
                        BASE_URL,
                        entry["id"].as_str().unwrap()
                    ))
                    .send()
                    .await
                    .unwrap();
                assert_eq!(delete_response.status(), StatusCode::OK);
            }
        }
    }
}
