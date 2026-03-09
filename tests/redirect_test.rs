use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_redirect_with_301() {
    let short_code = create_link_and_get_code("https://example.com/destination").await;
    let client = test_client(); // Doesn't follow redirects

    let response = client
        .get(format!("{}/{}", BASE_URL, short_code))
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
async fn test_nonexistent_short_code_redirects_to_404_page() {
    let client = test_client();

    let response = client
        .get(format!("{}/nonexistent999", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND); // 302
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        location.ends_with("/404"),
        "Expected redirect to /404, got: {}",
        location
    );
}

#[tokio::test]
async fn test_redirect_increments_click_count() {
    let auth_client = authenticated_client();
    let public_client = test_client();

    // Create link (authenticated)
    let create_response = create_test_link("https://example.com", None).await;
    let link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = link["id"].as_str().unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Initial click count should be 0
    assert_eq!(link["click_count"], 0);

    // Access the short link (public, unauthenticated)
    let redirect_response = public_client
        .get(format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    // Verify we got the redirect
    assert_eq!(
        redirect_response.status(),
        reqwest::StatusCode::MOVED_PERMANENTLY
    );

    // Get link and check click count (authenticated)
    let get_response = auth_client
        .get(format!("{}/api/links/{}", BASE_URL, link_id))
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
async fn test_disabled_link_redirects_to_404_page() {
    let auth_client = authenticated_client();
    let public_client = test_client();

    // Create a link
    let create_response = create_test_link("https://example.com", None).await;
    let link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = link["id"].as_str().unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Verify link works initially and check initial click count
    let initial_response = public_client
        .get(format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();
    assert_eq!(
        initial_response.status(),
        reqwest::StatusCode::MOVED_PERMANENTLY
    );
    assert_eq!(
        initial_response.headers().get("location").unwrap(),
        "https://example.com/"
    );

    // Check click count after initial access
    let initial_count_response = auth_client
        .get(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();
    let initial_link: serde_json::Value = initial_count_response.json().await.unwrap();
    let initial_click_count = initial_link["click_count"].as_i64().unwrap_or(0);

    // Disable the link using regular user endpoint
    let disable_response = auth_client
        .put(format!("{}/api/links/{}", BASE_URL, link_id))
        .json(&serde_json::json!({"status": "disabled"}))
        .send()
        .await
        .unwrap();

    assert_eq!(disable_response.status(), StatusCode::OK);

    // Try to access the disabled link (public)
    let response = public_client
        .get(format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    // Should redirect to 404
    assert_eq!(response.status(), StatusCode::FOUND); // 302 redirect to 404
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        location.ends_with("/404"),
        "Expected redirect to /404 for disabled link, got: {}",
        location
    );

    // Verify click count didn't increment beyond initial access
    let get_response = auth_client
        .get(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    let updated_link: serde_json::Value = get_response.json().await.unwrap();
    let final_click_count = updated_link["click_count"].as_i64().unwrap_or(0);
    assert_eq!(
        final_click_count, initial_click_count,
        "Click count should remain {} for disabled link, got {}",
        initial_click_count, final_click_count
    );

    // Clean up - restore original status
    let restore_response = auth_client
        .put(format!("{}/api/links/{}", BASE_URL, link_id))
        .json(&serde_json::json!({"status": "active"}))
        .send()
        .await
        .unwrap();
    assert_eq!(restore_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_inactive_link_redirects_to_404_page() {
    let auth_client = authenticated_client();
    let public_client = test_client();

    // Create and then delete (soft delete) a link
    let create_response = create_test_link("https://example.com", None).await;
    let link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = link["id"].as_str().unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Delete the link (soft delete, authenticated)
    let _ = auth_client
        .delete(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    // Try to access the short link (public)
    let response = public_client
        .get(format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND); // 302
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        location.ends_with("/404"),
        "Expected redirect to /404, got: {}",
        location
    );
}

#[tokio::test]
async fn test_root_redirects_to_frontend() {
    let client = test_client();

    let response = client.get(format!("{}/", BASE_URL)).send().await.unwrap();

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY); // 301
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        !location.is_empty(),
        "Expected Location header to contain frontend URL"
    );
}

#[tokio::test]
async fn test_admin_update_link_status_updates_kv() {
    let auth_client = authenticated_client();
    let public_client = test_client();

    // Create a link
    let create_response = create_test_link("https://example.com", None).await;
    let link: serde_json::Value = create_response.json().await.unwrap();
    let link_id = link["id"].as_str().unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Verify link works initially
    let initial_response = public_client
        .get(format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();
    assert_eq!(
        initial_response.status(),
        reqwest::StatusCode::MOVED_PERMANENTLY
    );

    // Disable the link using admin endpoint
    let disable_response = auth_client
        .put(format!("{}/api/admin/links/{}", BASE_URL, link_id))
        .json(&serde_json::json!({"status": "disabled"}))
        .send()
        .await
        .unwrap();

    assert_eq!(disable_response.status(), StatusCode::OK);

    // Try to access the disabled link (public)
    let response = public_client
        .get(format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    // Should redirect to 404
    assert_eq!(response.status(), StatusCode::FOUND); // 302 redirect to 404
    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        location.ends_with("/404"),
        "Expected redirect to /404 for disabled link, got: {}",
        location
    );

    // Clean up - restore original status
    let restore_response = auth_client
        .put(format!("{}/api/admin/links/{}", BASE_URL, link_id))
        .json(&serde_json::json!({"status": "active"}))
        .send()
        .await
        .unwrap();
    assert_eq!(restore_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_redirect_appends_utm_params() {
    let auth_client = authenticated_client();
    let public_client = test_client();

    // Create link with UTM params
    let create_response = auth_client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/page",
            "utm_params": {
                "utm_source": "newsletter",
                "utm_medium": "email",
                "utm_campaign": "spring_sale"
            }
        }))
        .send()
        .await
        .unwrap();

    let status = create_response.status();
    if status == StatusCode::FORBIDDEN {
        println!("User not on Pro tier, skipping UTM test");
        return;
    }
    assert_eq!(status, StatusCode::OK);

    let link: serde_json::Value = create_response.json().await.unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Follow the redirect and check Location header
    let response = public_client
        .get(format!("{}/{}", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);

    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();

    assert!(
        location.contains("utm_source=newsletter"),
        "Expected utm_source in redirect URL, got: {}",
        location
    );
    assert!(
        location.contains("utm_medium=email"),
        "Expected utm_medium in redirect URL, got: {}",
        location
    );
    assert!(
        location.contains("utm_campaign=spring_sale"),
        "Expected utm_campaign in redirect URL, got: {}",
        location
    );
    assert!(
        location.starts_with("https://example.com/page"),
        "Expected destination base URL, got: {}",
        location
    );

    // Clean up
    let link_id = link["id"].as_str().unwrap();
    let _ = auth_client
        .delete(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_redirect_forwards_visitor_query_params() {
    let auth_client = authenticated_client();
    let public_client = test_client();

    // Create link with forward_query_params enabled
    let create_response = auth_client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/landing",
            "forward_query_params": true
        }))
        .send()
        .await
        .unwrap();

    let status = create_response.status();
    if status == StatusCode::FORBIDDEN {
        println!("User not on Pro tier, skipping forwarding test");
        return;
    }
    assert_eq!(status, StatusCode::OK);

    let link: serde_json::Value = create_response.json().await.unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Access short link with visitor query params
    let response = public_client
        .get(format!(
            "{}/{}?ref=twitter&campaign=launch",
            BASE_URL, short_code
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);

    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();

    assert!(
        location.contains("ref=twitter"),
        "Expected ref param forwarded, got: {}",
        location
    );
    assert!(
        location.contains("campaign=launch"),
        "Expected campaign param forwarded, got: {}",
        location
    );
    assert!(
        location.starts_with("https://example.com/landing"),
        "Expected destination base URL, got: {}",
        location
    );

    // Clean up
    let link_id = link["id"].as_str().unwrap();
    let _ = auth_client
        .delete(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_redirect_visitor_params_override_utm() {
    let auth_client = authenticated_client();
    let public_client = test_client();

    // Create link with both UTM params and forwarding enabled
    let create_response = auth_client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/page",
            "utm_params": {
                "utm_source": "newsletter",
                "utm_medium": "email"
            },
            "forward_query_params": true
        }))
        .send()
        .await
        .unwrap();

    let status = create_response.status();
    if status == StatusCode::FORBIDDEN {
        println!("User not on Pro tier, skipping UTM override test");
        return;
    }
    assert_eq!(status, StatusCode::OK);

    let link: serde_json::Value = create_response.json().await.unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Visitor provides utm_source — should override the static value
    let response = public_client
        .get(format!("{}/{}?utm_source=twitter", BASE_URL, short_code))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);

    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();

    // Visitor's utm_source=twitter should win over static newsletter
    assert!(
        location.contains("utm_source=twitter"),
        "Expected visitor utm_source to override static, got: {}",
        location
    );
    // Static utm_medium should still be present (no conflict)
    assert!(
        location.contains("utm_medium=email"),
        "Expected static utm_medium to be present, got: {}",
        location
    );

    // Clean up
    let link_id = link["id"].as_str().unwrap();
    let _ = auth_client
        .delete(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_redirect_no_forwarding_strips_visitor_params() {
    let auth_client = authenticated_client();
    let public_client = test_client();

    // Create link WITHOUT forward_query_params
    let create_response = auth_client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/clean",
            "forward_query_params": false
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    let link: serde_json::Value = create_response.json().await.unwrap();
    let short_code = link["short_code"].as_str().unwrap();

    // Access short link with visitor query params
    let response = public_client
        .get(format!(
            "{}/{}?ref=should_not_forward",
            BASE_URL, short_code
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);

    let location = response
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();

    assert!(
        !location.contains("ref=should_not_forward"),
        "Visitor params should NOT be forwarded when disabled, got: {}",
        location
    );

    // Clean up
    let link_id = link["id"].as_str().unwrap();
    let _ = auth_client
        .delete(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await;
}
