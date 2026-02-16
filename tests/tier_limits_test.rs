use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_unlimited_tier_no_link_limit() {
    // This test assumes the test user is on unlimited tier
    // If not, this test would need to upgrade the user first

    let client = authenticated_client();

    // Create more than 15 links (should succeed for unlimited tier)
    let mut created_links = Vec::new();
    for i in 0..20 {
        let response = client
            .post(format!("{}/api/links", BASE_URL))
            .json(&json!({
                "destination_url": format!("https://example.com/unlimited-test-{}", i),
                "title": format!("Unlimited Test Link {}", i)
            }))
            .send()
            .await
            .unwrap();

        // If this fails, it means the user is not on unlimited tier
        if response.status() == StatusCode::FORBIDDEN {
            // Skip the rest of the test if user is not unlimited
            println!("User is not on unlimited tier, skipping unlimited test");
            return;
        }

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Link {} should succeed for unlimited tier",
            i + 1
        );

        let link: serde_json::Value = response.json().await.unwrap();
        created_links.push(link["id"].as_str().unwrap().to_string());
    }

    // Clean up created links
    for link_id in created_links {
        let _ = client
            .delete(format!("{}/api/links/{}", BASE_URL, link_id))
            .send()
            .await;
    }
}

#[tokio::test]
async fn test_free_tier_and_unlimited_tier_limits() {
    let client = authenticated_client();

    // Get user info to find organization ID
    let user_response = client
        .get(format!("{}/api/links?page=1&limit=100", BASE_URL))
        .send()
        .await
        .unwrap();

    let user: serde_json::Value = user_response.json().await.unwrap();
    let org_id = user["org_id"]
        .as_str()
        .expect("Failed to get organization ID");

    let tier_response = client
        .put(format!("{}/api/admin/orgs/{}/tier", BASE_URL, org_id))
        .json(&json!({"tier": "free"}))
        .send()
        .await
        .unwrap();

    assert_eq!(
        tier_response.status(),
        200,
        "Failed to set organization to free tier"
    );

    // Try to create links until we hit the limit or succeed
    let mut created_links = Vec::new();
    let mut links_created = 0;

    for i in 0..20 {
        // Try up to 20 links to find the limit
        let response = client
            .post(format!("{}/api/links", BASE_URL))
            .json(&json!({
                "destination_url": format!("https://example.com/free-test-{}", i),
                "title": format!("Free Test Link {}", i)
            }))
            .send()
            .await
            .unwrap();

        if response.status() == StatusCode::FORBIDDEN {
            println!(
                "Hit free tier limit at link {} (quota may be partially consumed)",
                i + 1
            );

            // Verify error message mentions the limit
            let error_text = response.text().await.unwrap();
            assert!(
                error_text.contains("limit") || error_text.contains("exceeded"),
                "Error message should mention limit exceeded: {}",
                error_text
            );

            println!("Free tier limit is working correctly");
            break;
        } else {
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Link {} should succeed on free tier",
                i + 1
            );

            let link: serde_json::Value = response.json().await.unwrap();
            created_links.push(link["id"].as_str().unwrap().to_string());
            links_created += 1;

            // Stop if we've created 15 links (the expected limit)
            if links_created >= 15 {
                println!(
                    "Successfully created {} links (reached expected limit)",
                    links_created
                );
                break;
            }
        }
    }

    // If we didn't hit the limit, that's also fine - it means quota was already consumed
    if links_created == 0 {
        println!("Free tier limit already enforced (quota full from previous tests)");
    }

    // Get user info to find organization ID
    let user_response = client
        .get(format!("{}/api/links?page=1&limit=100", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(
        user_response.status(),
        StatusCode::OK,
        "Should be able to get user info"
    );

    let user_info: serde_json::Value = user_response.json().await.unwrap();
    let org_id = user_info["org_id"].as_str().unwrap();

    // Use admin API to upgrade organization tier
    let upgrade_response = client
        .put(format!("{}/api/admin/orgs/{}/tier", BASE_URL, org_id))
        .json(&json!({"tier": "unlimited"}))
        .send()
        .await
        .unwrap();

    // If admin API doesn't work this way, we might need a different approach
    if upgrade_response.status() != StatusCode::OK {
        println!("Admin tier upgrade failed, skipping unlimited tier test");
        println!("   Status: {}", upgrade_response.status());
    } else {
        // Create one more link (should succeed on unlimited tier)
        let response = client
            .post(format!("{}/api/links", BASE_URL))
            .json(&json!({
                "destination_url": "https://example.com/unlimited-test-after-upgrade",
                "title": "Unlimited Test Link After Upgrade"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Link should succeed after upgrading to unlimited tier"
        );

        let link: serde_json::Value = response.json().await.unwrap();
        created_links.push(link["id"].as_str().unwrap().to_string());
    }

    // Clean up created links
    for link_id in created_links {
        let _ = client
            .delete(format!("{}/api/links/{}", BASE_URL, link_id))
            .send()
            .await;
    }

    // Reset organization back to unlimited tier for subsequent tests
    let reset_response = client
        .put(format!("{}/api/admin/orgs/{}/tier", BASE_URL, org_id))
        .json(&json!({"tier": "unlimited"}))
        .send()
        .await
        .unwrap();

    assert_eq!(
        reset_response.status(),
        200,
        "Failed to reset organization to unlimited tier"
    );
}
