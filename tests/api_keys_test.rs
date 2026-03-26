use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

#[tokio::test]
async fn test_api_key_lifecycle() {
    // 1. Setup clients
    let auth_client = authenticated_client(); // Simulates the browser dashboard
    let server_client = test_client(); // Simulates a 3rd-party unauthenticated server

    // 2. Create the API Key via the Dashboard
    let create_res = auth_client
        .post(format!("{}/api/settings/api-keys", BASE_URL))
        .json(&json!({
            "name": "Integration Test Key",
            "expires_in_days": 30
        }))
        .send()
        .await
        .expect("Failed to execute creation request");

    assert_eq!(create_res.status(), StatusCode::OK);
    let key_data: serde_json::Value = create_res.json().await.unwrap();

    let raw_token = key_data["raw_token"].as_str().unwrap().to_string();
    let key_id = key_data["id"].as_str().unwrap().to_string();

    // Ensure it's using the standard identifiable prefix
    assert!(raw_token.starts_with("ro_pat_"));

    // 3. Verify the token grants programmatic access (The Middleware Test)
    // First, find out the real ID of our authenticated browser user
    let whoami_res = auth_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .send()
        .await
        .expect("Failed to get browser user info");

    let browser_user_data: serde_json::Value = whoami_res.json().await.unwrap();
    let actual_user_id = browser_user_data["id"].as_str().unwrap();

    // Now verify the API Key grants access as that SAME user
    let auth_me_res = server_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .header("Authorization", format!("Bearer {}", raw_token))
        .send()
        .await
        .unwrap();

    assert_eq!(auth_me_res.status(), StatusCode::OK);
    let me_data: serde_json::Value = auth_me_res.json().await.unwrap();

    // Check against the dynamic ID instead of TEST_USER_ID
    assert_eq!(me_data["id"].as_str().unwrap(), actual_user_id);

    // 4. Verify the raw token is NEVER returned in the list endpoint
    let list_res = auth_client
        .get(format!("{}/api/settings/api-keys", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(list_res.status(), StatusCode::OK);
    let list_data = list_res.json::<Vec<serde_json::Value>>().await.unwrap();

    let found_key = list_data
        .iter()
        .find(|k| k["id"].as_str().unwrap() == key_id);
    assert!(found_key.is_some());
    // CRITICAL SECURITY CHECK: Ensure `raw_token` is missing from the list response
    assert!(found_key.unwrap().get("raw_token").is_none());
    assert!(found_key.unwrap().get("hint").is_some());

    // 5. Revoke the API Key
    let revoke_res = auth_client
        .delete(format!("{}/api/settings/api-keys/{}", BASE_URL, key_id))
        .send()
        .await
        .unwrap();

    assert_eq!(revoke_res.status(), StatusCode::NO_CONTENT);

    // 6. Verify the revoked token is immediately rejected
    let fail_res = server_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .header("Authorization", format!("Bearer {}", raw_token))
        .send()
        .await
        .unwrap();

    assert_eq!(fail_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_api_key_rejected() {
    let server_client = test_client();

    let fail_res = server_client
        .get(format!("{}/api/auth/me", BASE_URL))
        .header(
            "Authorization",
            "Bearer ro_pat_totallyfakeandinvalidkey123456",
        )
        .send()
        .await
        .unwrap();

    assert_eq!(fail_res.status(), StatusCode::UNAUTHORIZED);
}
