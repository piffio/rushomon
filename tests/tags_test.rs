use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

// ─── Create link with tags ────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_link_with_tags() {
    let client = authenticated_client();
    let code = unique_short_code("tg1");

    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/tagged",
            "short_code": code,
            "tags": ["campaign-2024", "social"]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let tags = body["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);
    assert!(tags.iter().any(|t| t.as_str() == Some("campaign-2024")));
    assert!(tags.iter().any(|t| t.as_str() == Some("social")));
}

#[tokio::test]
async fn test_create_link_without_tags_returns_empty_array() {
    let client = authenticated_client();
    let code = unique_short_code("tg2");

    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/no-tags",
            "short_code": code
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let tags = body["tags"].as_array().unwrap();
    assert!(tags.is_empty());
}

// ─── Tag normalization ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tags_are_normalized_whitespace() {
    let client = authenticated_client();
    let code = unique_short_code("tg3");

    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/norm",
            "short_code": code,
            "tags": ["  hello   world  "]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let tags = body["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].as_str().unwrap(), "hello world");
}

#[tokio::test]
async fn test_duplicate_tags_are_deduplicated() {
    let client = authenticated_client();
    let code = unique_short_code("tg4");

    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/dedup",
            "short_code": code,
            "tags": ["alpha", "Alpha", "ALPHA"]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let tags = body["tags"].as_array().unwrap();
    assert_eq!(
        tags.len(),
        1,
        "Duplicate tags (case-insensitive) should be deduplicated"
    );
}

// ─── Validation errors ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_tag_too_long_returns_400() {
    let client = authenticated_client();
    let code = unique_short_code("tg5");
    let long_tag = "a".repeat(51);

    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/long-tag",
            "short_code": code,
            "tags": [long_tag]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_too_many_tags_returns_400() {
    let client = authenticated_client();
    let code = unique_short_code("tg6");
    let tags: Vec<String> = (0..21).map(|i| format!("tag{}", i)).collect();

    let response = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/too-many-tags",
            "short_code": code,
            "tags": tags
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─── Get link includes tags ───────────────────────────────────────────────────

#[tokio::test]
async fn test_get_link_includes_tags() {
    let client = authenticated_client();
    let code = unique_short_code("tg7");

    let create_resp = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/get-tags",
            "short_code": code,
            "tags": ["internal", "promo"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let created: serde_json::Value = create_resp.json().await.unwrap();
    let link_id = created["id"].as_str().unwrap();

    let get_resp = client
        .get(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let body: serde_json::Value = get_resp.json().await.unwrap();
    let tags = body["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);
    assert!(tags.iter().any(|t| t.as_str() == Some("internal")));
    assert!(tags.iter().any(|t| t.as_str() == Some("promo")));
}

// ─── Update link tags ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_link_tags_replaces_all() {
    let client = authenticated_client();
    let code = unique_short_code("tg8");

    let create_resp = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/update-tags",
            "short_code": code,
            "tags": ["old-tag"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let created: serde_json::Value = create_resp.json().await.unwrap();
    let link_id = created["id"].as_str().unwrap();

    let update_resp = client
        .put(format!("{}/api/links/{}", BASE_URL, link_id))
        .json(&json!({ "tags": ["new-tag-1", "new-tag-2"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::OK);
    let updated: serde_json::Value = update_resp.json().await.unwrap();
    let tags = updated["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);
    assert!(!tags.iter().any(|t| t.as_str() == Some("old-tag")));
    assert!(tags.iter().any(|t| t.as_str() == Some("new-tag-1")));
    assert!(tags.iter().any(|t| t.as_str() == Some("new-tag-2")));
}

#[tokio::test]
async fn test_update_link_without_tags_preserves_existing_tags() {
    let client = authenticated_client();
    let code = unique_short_code("tg9");

    let create_resp = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/preserve-tags",
            "short_code": code,
            "tags": ["keep-me"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let created: serde_json::Value = create_resp.json().await.unwrap();
    let link_id = created["id"].as_str().unwrap();

    // Update title only, no tags field
    let update_resp = client
        .put(format!("{}/api/links/{}", BASE_URL, link_id))
        .json(&json!({ "title": "Updated Title" }))
        .send()
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::OK);
    let updated: serde_json::Value = update_resp.json().await.unwrap();
    let tags = updated["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 1);
    assert!(tags.iter().any(|t| t.as_str() == Some("keep-me")));
}

#[tokio::test]
async fn test_update_link_with_empty_tags_clears_all() {
    let client = authenticated_client();
    let code = unique_short_code("tga");

    let create_resp = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/clear-tags",
            "short_code": code,
            "tags": ["remove-me"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let created: serde_json::Value = create_resp.json().await.unwrap();
    let link_id = created["id"].as_str().unwrap();

    let update_resp = client
        .put(format!("{}/api/links/{}", BASE_URL, link_id))
        .json(&json!({ "tags": [] }))
        .send()
        .await
        .unwrap();
    assert_eq!(update_resp.status(), StatusCode::OK);
    let updated: serde_json::Value = update_resp.json().await.unwrap();
    let tags = updated["tags"].as_array().unwrap();
    assert!(tags.is_empty());
}

// ─── Filter links by tags ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_filter_links_by_single_tag() {
    let client = authenticated_client();
    let unique_tag = format!("filter-{}", unique_short_code("ft"));
    let code1 = unique_short_code("tf1");
    let code2 = unique_short_code("tf2");

    // Create link with the unique tag
    let r1 = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/filter1",
            "short_code": code1,
            "tags": [unique_tag.clone()]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    // Create link without the unique tag
    let r2 = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/filter2",
            "short_code": code2
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::OK);

    // Filter by the unique tag
    let filter_resp = client
        .get(format!(
            "{}/api/links?tags={}",
            BASE_URL,
            urlencoding::encode(&unique_tag)
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(filter_resp.status(), StatusCode::OK);
    let body: serde_json::Value = filter_resp.json().await.unwrap();
    let links = body["data"].as_array().unwrap();
    assert!(!links.is_empty(), "Should find links with the tag");
    for link in links {
        let tags = link["tags"].as_array().unwrap();
        assert!(
            tags.iter().any(|t| t.as_str() == Some(unique_tag.as_str())),
            "Every returned link should have the filtered tag"
        );
    }
}

#[tokio::test]
async fn test_filter_links_by_multiple_tags_and_semantics() {
    let client = authenticated_client();
    let tag_a = format!("anda-{}", unique_short_code("ta"));
    let tag_b = format!("andb-{}", unique_short_code("tb"));
    let code_both = unique_short_code("tfa");
    let code_only_a = unique_short_code("tfb");

    // Link with both tags
    let r1 = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/both-tags",
            "short_code": code_both,
            "tags": [tag_a.clone(), tag_b.clone()]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    // Link with only tag_a
    let r2 = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/only-a",
            "short_code": code_only_a,
            "tags": [tag_a.clone()]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::OK);

    // Filter by both tags (OR semantics)
    let filter_resp = client
        .get(format!(
            "{}/api/links?tags={},{}",
            BASE_URL,
            urlencoding::encode(&tag_a),
            urlencoding::encode(&tag_b)
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(filter_resp.status(), StatusCode::OK);
    let body: serde_json::Value = filter_resp.json().await.unwrap();
    let links = body["data"].as_array().unwrap();

    // Both links should appear (OR semantics)
    let short_codes: Vec<&str> = links
        .iter()
        .filter_map(|l| l["short_code"].as_str())
        .collect();
    assert!(
        short_codes.contains(&code_both.as_str()),
        "Link with both tags should appear"
    );
    assert!(
        short_codes.contains(&code_only_a.as_str()),
        "Link with only tag_a should appear (OR semantics)"
    );
    // Verify each returned link has at least one of the tags
    for link in links {
        let tags = link["tags"].as_array().unwrap();
        let tag_names: Vec<&str> = tags.iter().filter_map(|t| t.as_str()).collect();
        assert!(
            tag_names.contains(&tag_a.as_str()) || tag_names.contains(&tag_b.as_str()),
            "OR filter: each link should have at least one of the tags"
        );
    }
}

// ─── Delete link removes tags ─────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_link_removes_tags() {
    let client = authenticated_client();
    let code = unique_short_code("tgd");
    let unique_tag = format!("del-{}", unique_short_code("dt"));

    let create_resp = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/delete-tags",
            "short_code": code,
            "tags": [unique_tag.clone()]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::OK);
    let created: serde_json::Value = create_resp.json().await.unwrap();
    let link_id = created["id"].as_str().unwrap();

    // Delete the link
    let del_resp = client
        .delete(format!("{}/api/links/{}", BASE_URL, link_id))
        .send()
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    // The tag should no longer appear in org tags (or count should be 0)
    let tags_resp = client
        .get(format!("{}/api/tags", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(tags_resp.status(), StatusCode::OK);
    let tags_body: serde_json::Value = tags_resp.json().await.unwrap();
    let tags = tags_body.as_array().unwrap();
    let found = tags
        .iter()
        .any(|t| t["name"].as_str() == Some(unique_tag.as_str()));
    assert!(
        !found,
        "Deleted link's unique tag should not appear in org tags"
    );
}

// ─── GET /api/tags ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_org_tags_requires_auth() {
    let client = test_client();
    let response = client
        .get(format!("{}/api/tags", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_org_tags_returns_counts() {
    let client = authenticated_client();
    let unique_tag = format!("cnt-{}", unique_short_code("ct"));
    let code1 = unique_short_code("tc1");
    let code2 = unique_short_code("tc2");

    // Create two links with the same unique tag
    for code in [&code1, &code2] {
        let r = client
            .post(format!("{}/api/links", BASE_URL))
            .json(&json!({
                "destination_url": "https://example.com/count-test",
                "short_code": code,
                "tags": [unique_tag.clone()]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
    }

    let tags_resp = client
        .get(format!("{}/api/tags", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(tags_resp.status(), StatusCode::OK);
    let tags_body: serde_json::Value = tags_resp.json().await.unwrap();
    let tags = tags_body.as_array().unwrap();

    let found = tags
        .iter()
        .find(|t| t["name"].as_str() == Some(unique_tag.as_str()));
    assert!(found.is_some(), "Unique tag should appear in org tags");
    let count = found.unwrap()["count"].as_i64().unwrap();
    assert_eq!(count, 2, "Tag count should be 2");
}

// ─── Tag filter combined with search ─────────────────────────────────────────

#[tokio::test]
async fn test_tag_filter_combined_with_search() {
    let client = authenticated_client();
    let unique_tag = format!("combo-{}", unique_short_code("cb"));
    let unique_title = format!("ComboTitle-{}", unique_short_code("ct"));
    let code_match = unique_short_code("cm1");
    let code_tag_only = unique_short_code("cm2");

    // Link with tag AND matching title
    let r1 = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/combo-match",
            "short_code": code_match,
            "title": unique_title.clone(),
            "tags": [unique_tag.clone()]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    // Link with tag but different title
    let r2 = client
        .post(format!("{}/api/links", BASE_URL))
        .json(&json!({
            "destination_url": "https://example.com/combo-tag-only",
            "short_code": code_tag_only,
            "title": "SomethingElse",
            "tags": [unique_tag.clone()]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::OK);

    // Filter by tag + search title
    let filter_resp = client
        .get(format!(
            "{}/api/links?tags={}&search={}",
            BASE_URL,
            urlencoding::encode(&unique_tag),
            urlencoding::encode(&unique_title)
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(filter_resp.status(), StatusCode::OK);
    let body: serde_json::Value = filter_resp.json().await.unwrap();
    let links = body["data"].as_array().unwrap();

    // Only the link matching both tag and title should appear
    assert_eq!(
        links.len(),
        1,
        "Only one link should match both tag and title"
    );
    assert_eq!(links[0]["short_code"].as_str().unwrap(), code_match);
}
