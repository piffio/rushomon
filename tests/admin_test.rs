use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_admin_list_users_requires_auth() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/admin/users", BASE_URL))
        .send()
        .await
        .unwrap();

    // Should return 401 without authentication
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_list_users_returns_users() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/admin/users?page=1&limit=50", BASE_URL))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(status, StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    // Verify response structure
    assert!(body["users"].is_array());
    assert!(body["total"].is_number());
    assert!(body["page"].is_number());
    assert!(body["limit"].is_number());

    // There should be at least one user (the test user)
    let users = body["users"].as_array().unwrap();
    assert!(!users.is_empty());
}

#[tokio::test]
async fn test_admin_suspend_user() {
    let client = authenticated_client();

    // First, get a user to suspend (not the test user)
    let users_response = client
        .get(format!("{}/api/admin/users?page=1&limit=50", BASE_URL))
        .send()
        .await
        .unwrap();

    if users_response.status() == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    let users_body: serde_json::Value = users_response.json().await.unwrap();
    let users = users_body["users"].as_array().unwrap();

    // Find a user that's not the test user and not an admin
    let test_user_id = get_test_user_id();
    let target_user = users.iter().find(|u| {
        let user_id = u["id"].as_str().unwrap_or("");
        let role = u["role"].as_str().unwrap_or("");
        user_id != test_user_id && role == "member"
    });

    if target_user.is_none() {
        println!("No suitable user to suspend - skipping test");
        return;
    }

    let target_user = target_user.unwrap();
    let user_id = target_user["id"].as_str().unwrap();

    // Suspend the user
    let suspend_response = client
        .put(format!("{}/api/admin/users/{}/suspend", BASE_URL, user_id))
        .json(&json!({"reason": "Test suspension"}))
        .send()
        .await
        .unwrap();

    assert_eq!(suspend_response.status(), StatusCode::OK);

    let suspend_body: serde_json::Value = suspend_response.json().await.unwrap();
    assert_eq!(suspend_body["success"], true);

    // Verify user is suspended
    let user_response = client
        .get(format!("{}/api/admin/users/{}", BASE_URL, user_id))
        .send()
        .await
        .unwrap();

    let user_body: serde_json::Value = user_response.json().await.unwrap();
    assert!(user_body["suspended_at"].is_number());

    // Unsuspend the user
    let unsuspend_response = client
        .put(format!(
            "{}/api/admin/users/{}/unsuspend",
            BASE_URL, user_id
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(unsuspend_response.status(), StatusCode::OK);

    let unsuspend_body: serde_json::Value = unsuspend_response.json().await.unwrap();
    assert_eq!(unsuspend_body["success"], true);
}

#[tokio::test]
async fn test_admin_cannot_suspend_self() {
    let client = authenticated_client();

    // Get the current user ID from the API
    let me_response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    if me_response.status() != StatusCode::OK {
        println!(
            "Could not get current user - status: {}",
            me_response.status()
        );
        let text = me_response.text().await.unwrap_or_default();
        println!("Response body: {}", text);
        return;
    }

    let me: serde_json::Value = match me_response.json().await {
        Ok(json) => json,
        Err(e) => {
            println!("Failed to parse JSON: {}", e);
            return;
        }
    };

    let user_id = match me["id"].as_str() {
        Some(id) => id,
        None => {
            println!("No id field in response: {:?}", me);
            return;
        }
    };

    let response = client
        .put(format!("{}/api/admin/users/{}/suspend", BASE_URL, user_id))
        .json(&json!({"reason": "Test"}))
        .send()
        .await
        .unwrap();

    println!("Suspend response status: {}", response.status());
    let response_text = response.text().await.unwrap_or_default();
    println!("Suspend response body: {}", response_text);

    // Parse the response body for assertions
    let body: serde_json::Value = match serde_json::from_str(&response_text) {
        Ok(json) => json,
        Err(e) => {
            println!("Failed to parse response as JSON: {}", e);
            panic!("Response was not valid JSON: {}", response_text);
        }
    };
    assert!(
        body["message"]
            .as_str()
            .unwrap_or("")
            .contains("Cannot suspend yourself")
    );
}

#[tokio::test]
async fn test_admin_block_destination() {
    let client = authenticated_client();

    // Block a test destination
    let response = client
        .post(format!("{}/api/admin/blacklist", BASE_URL))
        .json(&json!({
            "destination": "https://malicious.example.com",
            "match_type": "exact",
            "reason": "Test block"
        }))
        .send()
        .await
        .unwrap();

    if response.status() == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["success"], true);

    // Get blacklist entries
    let blacklist_response = client
        .get(format!("{}/api/admin/blacklist", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(blacklist_response.status(), StatusCode::OK);

    let blacklist_body: serde_json::Value = blacklist_response.json().await.unwrap();
    let entries = blacklist_body.as_array().unwrap();

    // Find the entry we just added (check both original and normalized forms)
    let entry = entries.iter().find(|e| {
        let dest = e["destination"].as_str().unwrap_or("");
        dest == "https://malicious.example.com/" || dest == "https://malicious.example.com"
    });

    assert!(
        entry.is_some(),
        "Expected to find blacklist entry for 'https://malicious.example.com' or its normalized form"
    );

    // Remove the entry
    let entry_id = entry.unwrap()["id"].as_str().unwrap();
    let remove_response = client
        .delete(format!("{}/api/admin/blacklist/{}", BASE_URL, entry_id))
        .send()
        .await
        .unwrap();

    assert_eq!(remove_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_list_links_requires_auth() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/admin/links", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_list_links_returns_links() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/admin/links?page=1&limit=50", BASE_URL))
        .send()
        .await
        .unwrap();

    if response.status() == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    // Verify response structure
    assert!(body["links"].is_array());
    assert!(body["total"].is_number());
    assert!(body["page"].is_number());
    assert!(body["limit"].is_number());
}

#[tokio::test]
async fn test_admin_update_link_status() {
    let client = authenticated_client();

    // First, get a link to update
    let links_response = client
        .get(format!("{}/api/links?page=1&limit=1", BASE_URL))
        .send()
        .await
        .unwrap();

    if links_response.status() != StatusCode::OK {
        println!("Failed to get links - skipping test");
        return;
    }

    let links_body: serde_json::Value = links_response.json().await.unwrap();
    let links = links_body["data"].as_array().unwrap();

    if links.is_empty() {
        println!("No links to test with - skipping test");
        return;
    }

    let link_id = links[0]["id"].as_str().unwrap();
    let original_status = links[0]["status"].as_str().unwrap();

    // Update link status
    let new_status = if original_status == "active" {
        "disabled"
    } else {
        "active"
    };
    let response = client
        .put(format!("{}/api/admin/links/{}", BASE_URL, link_id))
        .json(&json!({"status": new_status}))
        .send()
        .await
        .unwrap();

    if response.status() == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["success"], true);

    // Restore original status
    let restore_response = client
        .put(format!("{}/api/admin/links/{}", BASE_URL, link_id))
        .json(&json!({"status": original_status}))
        .send()
        .await
        .unwrap();

    assert_eq!(restore_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_get_user_not_found() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/admin/users/nonexistent-user-id", BASE_URL))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_admin_update_user_invalid_role() {
    let client = authenticated_client();

    let response = client
        .put(format!("{}/api/admin/users/some-user-id", BASE_URL))
        .json(&json!({ "role": "superadmin" }))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    // Should reject invalid role values
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_update_user_missing_role() {
    let client = authenticated_client();

    let response = client
        .put(format!("{}/api/admin/users/some-user-id", BASE_URL))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    // Should reject missing role field
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_cannot_demote_self() {
    let client = authenticated_client();

    // First, get current user to find our own ID
    let me_response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    if me_response.status() != StatusCode::OK {
        println!("Could not get current user - skipping test");
        return;
    }

    let me: serde_json::Value = me_response.json().await.unwrap();
    let my_id = me["id"].as_str().unwrap();

    // Only run if we're admin
    if me["role"].as_str().unwrap() != "admin" {
        println!("Test user is not an admin - skipping test");
        return;
    }

    // Try to demote ourselves
    let response = client
        .put(format!("{}/api/admin/users/{}", BASE_URL, my_id))
        .json(&json!({ "role": "member" }))
        .send()
        .await
        .unwrap();

    // Should be rejected
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.text().await.unwrap();
    assert!(body.contains("Cannot demote yourself"));
}

#[tokio::test]
async fn test_first_user_is_admin() {
    let client = authenticated_client();

    // Get current user info
    let response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let user: serde_json::Value = response.json().await.unwrap();
    assert_eq!(user["role"], "admin");
}

#[tokio::test]
async fn test_admin_reset_counter_requires_auth() {
    let client = test_client(); // Unauthenticated client

    let response = client
        .post(format!(
            "{}/api/admin/orgs/test-org-id/reset-counter",
            BASE_URL
        ))
        .send()
        .await
        .unwrap();

    // Should return 401 without authentication
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_admin_reset_counter_admin_access() {
    let client = authenticated_client(); // First user is admin

    // Get current user info to get org_id
    let user_response = client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .unwrap();

    let user: serde_json::Value = user_response.json().await.unwrap();
    let org_id = user["org_id"]
        .as_str()
        .expect("Failed to get organization ID");

    // Reset counter as admin user
    let response = client
        .post(format!(
            "{}/api/admin/orgs/{}/reset-counter",
            BASE_URL, org_id
        ))
        .send()
        .await
        .unwrap();

    // Should return 200 for admin user
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["success"].as_bool().unwrap_or(false));
    assert_eq!(body["message"], "Monthly counter reset for billing account");
}
