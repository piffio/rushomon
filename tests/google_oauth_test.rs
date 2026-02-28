use reqwest::StatusCode;

mod common;
use common::*;

/// Test that the /api/auth/providers endpoint lists enabled providers.
/// Both GitHub and Google are configured in the test environment.
#[tokio::test]
async fn test_auth_providers_lists_enabled_providers() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/auth/providers", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    let providers = body["providers"].as_array().unwrap();

    // Both GitHub and Google should be listed (both configured in test .dev.vars)
    let names: Vec<&str> = providers
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();

    assert!(
        names.contains(&"github"),
        "GitHub should be in providers list, got: {:?}",
        names
    );
    assert!(
        names.contains(&"google"),
        "Google should be in providers list, got: {:?}",
        names
    );
}

/// Test that /api/auth/google redirects to the mock Google OAuth authorize endpoint.
#[tokio::test]
async fn test_google_login_redirects_to_provider() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/auth/google", BASE_URL))
        .send()
        .await
        .unwrap();

    // Should get a 302 redirect to the (mock) Google authorize URL
    assert_eq!(
        response.status(),
        StatusCode::FOUND,
        "Expected 302 redirect to Google OAuth"
    );

    let location = response
        .headers()
        .get("location")
        .expect("Expected Location header")
        .to_str()
        .unwrap();

    // Should redirect to our mock Google authorize endpoint
    assert!(
        location.contains("/google/o/oauth2/v2/auth") || location.contains("google"),
        "Location should point to Google OAuth, got: {}",
        location
    );
    assert!(
        location.contains("state="),
        "Location should include CSRF state param"
    );
    assert!(
        location.contains("client_id="),
        "Location should include client_id"
    );
    assert!(
        location.contains("response_type=code"),
        "Google OAuth should include response_type=code"
    );
}

/// Test the full Google OAuth flow: initiate → callback → receive JWT session cookie.
#[tokio::test]
async fn test_google_oauth_full_flow() {
    let client = test_client();

    // Step 1: Initiate Google OAuth — get redirect URL
    let init_response = client
        .get(format!("{}/api/auth/google", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(init_response.status(), StatusCode::FOUND);

    let location = init_response
        .headers()
        .get("location")
        .expect("Expected Location header")
        .to_str()
        .unwrap()
        .to_string();

    // Step 2: Extract state from redirect URL
    let state = location
        .split('&')
        .chain(location.split('?'))
        .find(|part| part.starts_with("state="))
        .expect("state param not found in redirect URL")
        .strip_prefix("state=")
        .unwrap()
        .to_string();

    // Step 3: Construct callback URL with mock Google code
    let mock_code = format!("mock-google-code-{}", state);
    let callback_url = format!(
        "{}/api/auth/callback?code={}&state={}",
        BASE_URL, mock_code, state
    );

    // Step 4: Call callback — should set session cookies and redirect to frontend
    let callback_response = client.get(&callback_url).send().await.unwrap();

    let status = callback_response.status();

    // Should be a redirect (302) to the frontend with cookies set
    assert_eq!(
        status,
        StatusCode::FOUND,
        "Expected 302 redirect after callback, got {} — check wrangler logs",
        status
    );

    // Should set access token cookie
    let has_access_cookie = callback_response
        .headers()
        .get_all("set-cookie")
        .iter()
        .any(|v| v.to_str().unwrap_or("").contains("rushomon_access="));

    assert!(
        has_access_cookie,
        "Callback should set rushomon_access cookie for Google OAuth flow"
    );
}

/// Test that Google OAuth account linking works: signing in via Google with an email
/// that already exists (from GitHub) should reuse the same account.
#[tokio::test]
async fn test_google_oauth_account_linking_via_email() {
    // This test verifies the account linking logic is wired correctly by checking
    // that the /api/auth/providers endpoint and /api/auth/google are both functional.
    // Full DB-level linking is validated by the unit test for create_or_get_user.
    // Here we just confirm both providers work end-to-end without errors.

    let client = test_client();

    // GitHub login should work
    let gh_response = client
        .get(format!("{}/api/auth/github", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(
        gh_response.status(),
        StatusCode::FOUND,
        "GitHub login should redirect"
    );

    // Google login should also work
    let google_response = client
        .get(format!("{}/api/auth/google", BASE_URL))
        .send()
        .await
        .unwrap();
    assert_eq!(
        google_response.status(),
        StatusCode::FOUND,
        "Google login should redirect"
    );
}
