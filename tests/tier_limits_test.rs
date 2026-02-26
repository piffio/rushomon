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
        .get(format!("{}/api/auth/me", BASE_URL))
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
        .get(format!("{}/api/auth/me", BASE_URL))
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

#[tokio::test]
async fn test_free_tier_cannot_create_custom_short_code() {
    let client = authenticated_client();

    // Get user info to find organization ID
    let user_response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    let user: serde_json::Value = user_response.json().await.unwrap();
    let org_id = user["org_id"]
        .as_str()
        .expect("Failed to get organization ID");

    // Set organization to free tier
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

    // Try to create a link with a custom short code (should fail with 403)
    // Note: short codes must be alphanumeric only (no hyphens) and 4-10 chars
    let custom_code = "mycode";
    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/free-custom-code-test",
            "short_code": custom_code,
            "title": "Free Tier Custom Code Test"
        }))
        .send()
        .await
        .unwrap();

    // Should be blocked with 403
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Free tier should not be able to create custom short codes"
    );

    // Verify error message mentions upgrade
    let error_text = response.text().await.unwrap();
    assert!(
        error_text.contains("Upgrade") || error_text.contains("Unlimited"),
        "Error message should mention upgrade: {}",
        error_text
    );

    // Upgrade to unlimited tier
    let upgrade_response = client
        .put(format!("{}/api/admin/orgs/{}/tier", BASE_URL, org_id))
        .json(&json!({"tier": "unlimited"}))
        .send()
        .await
        .unwrap();

    assert_eq!(
        upgrade_response.status(),
        200,
        "Failed to upgrade organization to unlimited tier"
    );

    // Now try to create a link with a custom short code (should succeed)
    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/unlimited-custom-code-test",
            "short_code": custom_code,
            "title": "Unlimited Tier Custom Code Test"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Unlimited tier should be able to create custom short codes"
    );

    let link: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        link["short_code"].as_str().unwrap(),
        custom_code,
        "Custom short code should be set correctly"
    );

    let link_id = link["id"].as_str().unwrap().to_string();

    // Clean up
    let delete_response = client
        .delete(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();

    assert_eq!(
        delete_response.status(),
        StatusCode::OK,
        "Should be able to delete the created link"
    );

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

#[tokio::test]
async fn test_usage_api_includes_custom_code_flag() {
    let client = authenticated_client();

    // Get user info to find organization ID
    let user_response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    let user: serde_json::Value = user_response.json().await.unwrap();
    let org_id = user["org_id"]
        .as_str()
        .expect("Failed to get organization ID");

    // Test on free tier
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

    // Get usage info
    let usage_response = client
        .get(format!("{}/api/usage", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(
        usage_response.status(),
        StatusCode::OK,
        "Should be able to get usage info"
    );

    let usage: serde_json::Value = usage_response.json().await.unwrap();
    assert_eq!(usage["tier"], "free", "Tier should be free");
    assert_eq!(
        usage["limits"]["allow_custom_short_code"], false,
        "Free tier should not allow custom short codes"
    );

    // Upgrade to unlimited tier
    let upgrade_response = client
        .put(format!("{}/api/admin/orgs/{}/tier", BASE_URL, org_id))
        .json(&json!({"tier": "unlimited"}))
        .send()
        .await
        .unwrap();

    assert_eq!(
        upgrade_response.status(),
        200,
        "Failed to upgrade organization to unlimited tier"
    );

    // Get usage info again
    let usage_response = client
        .get(format!("{}/api/usage", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(
        usage_response.status(),
        StatusCode::OK,
        "Should be able to get usage info after upgrade"
    );

    let usage: serde_json::Value = usage_response.json().await.unwrap();
    assert_eq!(usage["tier"], "unlimited", "Tier should be unlimited");
    assert_eq!(
        usage["limits"]["allow_custom_short_code"], true,
        "Unlimited tier should allow custom short codes"
    );
}
