mod common;
use common::*;
use reqwest::{Client, StatusCode};

#[tokio::test]
async fn test_free_tier_tag_limit() {
    let client: Client = create_test_client();

    // Create a new org to ensure clean state
    let org_response: reqwest::Response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&serde_json::json!({
            "name": "Tag Test Org"
        }))
        .send()
        .await
        .expect("Failed to create test org");

    assert_eq!(org_response.status(), StatusCode::OK);
    let org_data: serde_json::Value = org_response.json().await.unwrap();
    let org_id = org_data["id"]
        .as_str()
        .expect("Org response should have id field");

    // Create 5 links with unique tags (should succeed)
    for i in 1..=5 {
        let tag_name = format!("tag{}", i);
        let response: reqwest::Response = client
            .post(format!("{}/api/links", BASE_URL))
            .json(&serde_json::json!({
                "destination_url": format!("https://example{}.com", i),
                "title": format!("Test Link {}", i),
                "tags": [tag_name]
            }))
            .send()
            .await
            .expect("Failed to create link");

        assert_eq!(response.status(), StatusCode::OK);
    }

    // Try to create a 6th link with a new tag (should fail)
    let response: reqwest::Response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&serde_json::json!({
            "destination_url": "https://example6.com",
            "title": "Test Link 6",
            "tags": ["tag6"]
        }))
        .send()
        .await
        .expect("Failed to create link");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let error_text = response.text().await.unwrap();
    assert!(error_text.contains("tag limit") || error_text.contains("reached your tag limit"));

    // But we can still create links with existing tags (should succeed)
    let response: reqwest::Response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&serde_json::json!({
            "destination_url": "https://example7.com",
            "title": "Test Link 7",
            "tags": ["tag1"] // Reusing existing tag
        }))
        .send()
        .await
        .expect("Failed to create link");

    assert_eq!(response.status(), StatusCode::OK);

    // Clean up
    let _ = client
        .delete(&format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_tag_management_endpoints() {
    let client: Client = create_test_client();

    // Create a test org
    let org_response: reqwest::Response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&serde_json::json!({
            "name": "Tag Management Test Org"
        }))
        .send()
        .await
        .expect("Failed to create test org");

    assert_eq!(org_response.status(), StatusCode::OK);
    let org_data: serde_json::Value = org_response.json().await.unwrap();
    let org_id = org_data["id"]
        .as_str()
        .expect("Org response should have id field");

    // Create a link with tags
    let response: reqwest::Response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&serde_json::json!({
            "destination_url": "https://example.com",
            "title": "Test Link",
            "tags": ["original-tag", "another-tag"]
        }))
        .send()
        .await
        .expect("Failed to create link");

    assert_eq!(response.status(), StatusCode::OK);

    // Get tags list
    let response: reqwest::Response = client
        .get(format!("{}/api/tags", BASE_URL))
        .send()
        .await
        .expect("Failed to get tags");

    assert_eq!(response.status(), StatusCode::OK);
    let tags: serde_json::Value = response.json().await.unwrap();
    assert_eq!(tags.as_array().unwrap().len(), 2);

    // Rename a tag
    let response: reqwest::Response = client
        .patch(format!("{}/api/tags/original-tag", BASE_URL))
        .json(&serde_json::json!({
            "new_name": "renamed-tag"
        }))
        .send()
        .await
        .expect("Failed to rename tag");

    assert_eq!(response.status(), StatusCode::OK);
    let updated_tags: serde_json::Value = response.json().await.unwrap();
    assert_eq!(updated_tags.as_array().unwrap().len(), 2);

    // Verify the tag was renamed
    let tag_names: Vec<String> = updated_tags
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert!(tag_names.contains(&"renamed-tag".to_string()));
    assert!(!tag_names.contains(&"original-tag".to_string()));

    // Delete a tag
    let response: reqwest::Response = client
        .delete(format!("{}/api/tags/another-tag", BASE_URL))
        .send()
        .await
        .expect("Failed to delete tag");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Verify the tag was deleted
    let response: reqwest::Response = client
        .get(format!("{}/api/tags", BASE_URL))
        .send()
        .await
        .expect("Failed to get tags");

    let tags: serde_json::Value = response.json().await.unwrap();
    assert_eq!(tags.as_array().unwrap().len(), 1);
    assert_eq!(tags[0]["name"].as_str().unwrap(), "renamed-tag");

    // Clean up
    let _ = client
        .delete(&format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_tag_usage_endpoint() {
    let client: Client = create_test_client();

    // Create a test org
    let org_response: reqwest::Response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&serde_json::json!({
            "name": "Tag Usage Test Org"
        }))
        .send()
        .await
        .expect("Failed to create test org");

    assert_eq!(org_response.status(), StatusCode::OK);
    let org_data: serde_json::Value = org_response.json().await.unwrap();
    let org_id = org_data["id"]
        .as_str()
        .expect("Org response should have id field");

    // Create links with tags
    for i in 1..=3 {
        let response: reqwest::Response = client
            .post(format!("{}/api/links", BASE_URL))
            .json(&serde_json::json!({
                "destination_url": format!("https://example{}.com", i),
                "title": format!("Test Link {}", i),
                "tags": ["shared-tag", format!("unique-tag{}", i)]
            }))
            .send()
            .await
            .expect("Failed to create link");

        assert_eq!(response.status(), StatusCode::OK);
    }

    // Check usage endpoint includes tag info
    let response: reqwest::Response = client
        .get(format!("{}/api/usage", BASE_URL))
        .send()
        .await
        .expect("Failed to get usage");

    assert_eq!(response.status(), StatusCode::OK);
    let usage: serde_json::Value = response.json().await.unwrap();

    // Verify tag limit is present
    assert!(usage["limits"]["max_tags"].is_number());
    assert_eq!(usage["limits"]["max_tags"].as_i64().unwrap(), 5); // Free tier

    // Verify tag count is present and correct
    assert!(usage["usage"]["tags_count"].is_number());
    let tags_count = usage["usage"]["tags_count"].as_i64().unwrap();
    assert_eq!(tags_count, 4); // shared-tag + unique-tag1 + unique-tag2 + unique-tag3

    // Clean up
    let _ = client
        .delete(&format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await;
}

#[tokio::test]
async fn test_tag_rename_merge() {
    let client: Client = create_test_client();

    // Create a test org
    let org_response: reqwest::Response = client
        .post(format!("{}/api/orgs", BASE_URL))
        .json(&serde_json::json!({
            "name": "Tag Merge Test Org"
        }))
        .send()
        .await
        .expect("Failed to create test org");

    assert_eq!(org_response.status(), StatusCode::OK);
    let org_data: serde_json::Value = org_response.json().await.unwrap();
    let org_id = org_data["id"]
        .as_str()
        .expect("Org response should have id field");

    // Create two links with different tags
    let response: reqwest::Response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&serde_json::json!({
            "destination_url": "https://example1.com",
            "title": "Test Link 1",
            "tags": ["tag-a"]
        }))
        .send()
        .await
        .expect("Failed to create link");

    assert_eq!(response.status(), StatusCode::OK);

    let response: reqwest::Response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&serde_json::json!({
            "destination_url": "https://example2.com",
            "title": "Test Link 2",
            "tags": ["tag-b"]
        }))
        .send()
        .await
        .expect("Failed to create link");

    assert_eq!(response.status(), StatusCode::OK);

    // Rename tag-a to tag-b (should merge them)
    let response: reqwest::Response = client
        .patch(format!("{}/api/tags/tag-a", BASE_URL))
        .json(&serde_json::json!({
            "new_name": "tag-b"
        }))
        .send()
        .await
        .expect("Failed to rename tag");

    assert_eq!(response.status(), StatusCode::OK);

    // Verify both links now have tag-b
    let response: reqwest::Response = client
        .get(format!("{}/api/tags", BASE_URL))
        .send()
        .await
        .expect("Failed to get tags");

    let tags: serde_json::Value = response.json().await.unwrap();
    assert_eq!(tags.as_array().unwrap().len(), 1);
    assert_eq!(tags[0]["name"].as_str().unwrap(), "tag-b");
    assert_eq!(tags[0]["count"].as_i64().unwrap(), 2); // Both links now have tag-b

    // Clean up
    let _ = client
        .delete(&format!("{}/api/orgs/{}", BASE_URL, org_id))
        .send()
        .await;
}
