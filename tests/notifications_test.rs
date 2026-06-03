use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

// ── GET /api/notifications/preferences ───────────────────────────────────────

#[tokio::test]
async fn test_get_preferences_requires_auth() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/notifications/preferences", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_preferences_returns_defaults_when_no_row_exists() {
    // A fresh authenticated user who has never touched their preferences
    // should get all flags as true (the "opt-in by default" invariant).
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/notifications/preferences", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    assert!(
        body["email_monthly_stats"].is_boolean(),
        "email_monthly_stats must be a boolean, got: {:?}",
        body
    );
    // Default: opted in (true). The user has not yet called PATCH so no row
    // exists; the repository must return the default value.
    assert_eq!(
        body["email_monthly_stats"].as_bool().unwrap(),
        true,
        "fresh user should default to opted-in"
    );
}

#[tokio::test]
async fn test_get_preferences_response_shape() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/notifications/preferences", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    // The response must be an object (not an array, not null, not a scalar).
    assert!(
        body.is_object(),
        "preferences response must be a JSON object, got: {:?}",
        body
    );

    // email_monthly_stats must always be present.
    assert!(
        body.get("email_monthly_stats").is_some(),
        "email_monthly_stats key must be present in response"
    );
}

// ── PATCH /api/notifications/preferences ─────────────────────────────────────

#[tokio::test]
async fn test_patch_preferences_requires_auth() {
    let client = test_client();

    let response = client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .json(&json!({ "email_monthly_stats": false }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_patch_preferences_invalid_json_returns_400() {
    let client = authenticated_client();

    let response = client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .header("Content-Type", "application/json")
        .body("this is not json")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_patch_preferences_opt_out_and_back_in() {
    let client = authenticated_client();

    // Step 1 — opt out
    let opt_out = client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .json(&json!({ "email_monthly_stats": false }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        opt_out.status(),
        StatusCode::OK,
        "PATCH opt-out should return 200"
    );

    let after_opt_out: serde_json::Value = opt_out.json().await.unwrap();
    assert_eq!(
        after_opt_out["email_monthly_stats"].as_bool().unwrap(),
        false,
        "email_monthly_stats should be false after opt-out"
    );

    // Step 2 — verify GET reflects the persisted value
    let get_response = client
        .get(format!("{}/api/notifications/preferences", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
    let get_body: serde_json::Value = get_response.json().await.unwrap();
    assert_eq!(
        get_body["email_monthly_stats"].as_bool().unwrap(),
        false,
        "GET after opt-out should still return false"
    );

    // Step 3 — opt back in (restore state for other tests)
    let opt_in = client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .json(&json!({ "email_monthly_stats": true }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        opt_in.status(),
        StatusCode::OK,
        "PATCH opt-in should return 200"
    );

    let after_opt_in: serde_json::Value = opt_in.json().await.unwrap();
    assert_eq!(
        after_opt_in["email_monthly_stats"].as_bool().unwrap(),
        true,
        "email_monthly_stats should be true after opt-in"
    );
}

#[tokio::test]
async fn test_patch_preferences_returns_updated_preferences() {
    let client = authenticated_client();

    // PATCH must return the full, updated preferences object (not just the
    // field that was patched).
    let response = client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .json(&json!({ "email_monthly_stats": false }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    assert!(
        body.is_object(),
        "PATCH response must be a JSON object, got: {:?}",
        body
    );
    assert!(
        body.get("email_monthly_stats").is_some(),
        "PATCH response must include email_monthly_stats"
    );

    // Restore state
    client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .json(&json!({ "email_monthly_stats": true }))
        .send()
        .await
        .unwrap();
}

#[tokio::test]
async fn test_patch_preferences_empty_body_preserves_existing_value() {
    // An empty JSON object `{}` is a valid PATCH — it should be a no-op
    // (all fields default to None in the request struct, so existing values
    // are kept unchanged).
    let client = authenticated_client();

    // First, set a known state.
    client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .json(&json!({ "email_monthly_stats": false }))
        .send()
        .await
        .unwrap();

    // PATCH with empty body — must not reset to defaults.
    let response = client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(
        body["email_monthly_stats"].as_bool().unwrap(),
        false,
        "empty PATCH must not reset email_monthly_stats"
    );

    // Restore state
    client
        .patch(format!("{}/api/notifications/preferences", BASE_URL))
        .json(&json!({ "email_monthly_stats": true }))
        .send()
        .await
        .unwrap();
}

// ── POST /api/admin/cron/trigger-monthly-stats ────────────────────────────────

#[tokio::test]
async fn test_cron_trigger_requires_auth() {
    let client = test_client();

    let response = client
        .post(format!("{}/api/admin/cron/trigger-monthly-stats", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_cron_trigger_requires_admin_role() {
    // The billing test user is a regular (non-admin) member created by the
    // test harness. They should get 403 from this endpoint.
    let client = billing_test_client();

    let response = client
        .post(format!("{}/api/admin/cron/trigger-monthly-stats", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "non-admin user must receive 403 from cron trigger endpoint"
    );
}

#[tokio::test]
async fn test_cron_trigger_admin_returns_sent_errors_shape() {
    let client = authenticated_client();

    let response = client
        .post(format!("{}/api/admin/cron/trigger-monthly-stats", BASE_URL))
        .send()
        .await
        .unwrap();

    let status = response.status();

    // The admin test user may or may not be an admin depending on the
    // environment; if not admin, skip rather than hard-fail.
    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping cron trigger response shape test");
        return;
    }

    assert_eq!(
        status,
        StatusCode::OK,
        "admin cron trigger should return 200"
    );

    let body: serde_json::Value = response.json().await.unwrap();

    assert!(
        body.is_object(),
        "cron trigger response must be a JSON object, got: {:?}",
        body
    );
    assert!(
        body["sent"].is_number(),
        "response must contain numeric 'sent' field, got: {:?}",
        body
    );
    assert!(
        body["errors"].is_number(),
        "response must contain numeric 'errors' field, got: {:?}",
        body
    );

    // In the test environment Mailgun is not configured, so the job exits
    // early and returns (0, 0). This is the expected fast-path.
    assert_eq!(
        body["sent"].as_i64().unwrap(),
        0,
        "sent count should be 0 when Mailgun is not configured in test env"
    );
    assert_eq!(
        body["errors"].as_i64().unwrap(),
        0,
        "error count should be 0 when Mailgun is not configured in test env"
    );
}
