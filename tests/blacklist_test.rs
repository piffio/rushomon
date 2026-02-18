use reqwest::StatusCode;

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
