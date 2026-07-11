use reqwest::StatusCode;
use serde_json::json;

mod common;
use common::*;

use rushomon::utils::short_code::MAX_SHORT_CODE_LENGTH;

#[tokio::test]
async fn test_settings_requires_auth() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/admin/settings", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_get_settings_returns_defaults() {
    let client = authenticated_client();

    let response = client
        .get(format!("{}/api/admin/settings", BASE_URL))
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

    // signups_enabled should exist and default to "true"
    assert!(body["signups_enabled"].is_string());
    // Value should be either "true" or "false" (may have been changed by other tests)
    let value = body["signups_enabled"].as_str().unwrap();
    assert!(value == "true" || value == "false");
}

#[tokio::test]
async fn test_update_setting_signups_enabled() {
    let client = authenticated_client();

    // First get current value
    let get_response = client
        .get(format!("{}/api/admin/settings", BASE_URL))
        .send()
        .await
        .unwrap();

    if get_response.status() == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    let current: serde_json::Value = get_response.json().await.unwrap();
    let original_value = current["signups_enabled"]
        .as_str()
        .unwrap_or("true")
        .to_string();

    // Toggle the value
    let new_value = if original_value == "true" {
        "false"
    } else {
        "true"
    };

    let response = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "signups_enabled", "value": new_value }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["signups_enabled"].as_str().unwrap(), new_value);

    // Restore original value
    let restore_response = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "signups_enabled", "value": original_value }))
        .send()
        .await
        .unwrap();

    assert_eq!(restore_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_setting_invalid_key() {
    let client = authenticated_client();

    let response = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "nonexistent_setting", "value": "true" }))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_setting_invalid_value() {
    let client = authenticated_client();

    let response = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "signups_enabled", "value": "maybe" }))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_setting_missing_fields() {
    let client = authenticated_client();

    // Missing value
    let response = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "signups_enabled" }))
        .send()
        .await
        .unwrap();

    let status = response.status();

    if status == StatusCode::FORBIDDEN {
        println!("Test user is not an admin - skipping test");
        return;
    }

    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Missing key
    let response = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "value": "true" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_update_settings_requires_auth() {
    let client = test_client();

    let response = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "signups_enabled", "value": "false" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ── GET /api/settings (public endpoint) ──────────────────────────────────────

#[tokio::test]
async fn test_public_settings_accessible_without_auth() {
    // This endpoint has no authentication requirement — anyone can call it.
    let client = test_client();

    let response = client
        .get(format!("{}/api/settings", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "public settings must be accessible without authentication"
    );
}

#[tokio::test]
async fn test_public_settings_contains_email_notifications_enabled() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/settings", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    assert!(
        body.get("email_notifications_enabled").is_some(),
        "public settings must include email_notifications_enabled, got: {:?}",
        body
    );
    assert!(
        body["email_notifications_enabled"].is_boolean(),
        "email_notifications_enabled must be a boolean, got: {:?}",
        body["email_notifications_enabled"]
    );
    // In the test environment MAILGUN_API_KEY and MAILGUN_DOMAIN are not set,
    // so is_mailgun_configured() returns false.
    assert_eq!(
        body["email_notifications_enabled"].as_bool().unwrap(),
        false,
        "email_notifications_enabled should be false when Mailgun is not configured"
    );
}

#[tokio::test]
async fn test_public_settings_contains_required_fields() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/settings", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    // All fields that the frontend depends on must be present.
    let required_fields = [
        "founder_pricing_active",
        "email_notifications_enabled",
        "active_discount_amount_pro_monthly",
        "active_discount_amount_pro_annual",
        "active_discount_amount_business_monthly",
        "active_discount_amount_business_annual",
        "min_random_code_length",
        "min_custom_code_length",
        "system_min_code_length",
    ];

    for field in required_fields {
        assert!(
            body.get(field).is_some(),
            "public settings must include '{field}', got: {:?}",
            body
        );
    }
}

#[tokio::test]
async fn test_public_settings_discount_amounts_are_numbers() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/settings", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    let amount_fields = [
        "active_discount_amount_pro_monthly",
        "active_discount_amount_pro_annual",
        "active_discount_amount_business_monthly",
        "active_discount_amount_business_annual",
    ];

    for field in amount_fields {
        assert!(
            body[field].is_number(),
            "'{field}' must be a number, got: {:?}",
            body[field]
        );
    }
}

#[tokio::test]
async fn test_public_settings_founder_pricing_is_boolean() {
    let client = test_client();

    let response = client
        .get(format!("{}/api/settings", BASE_URL))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.unwrap();

    assert!(
        body["founder_pricing_active"].is_boolean(),
        "founder_pricing_active must be a boolean, got: {:?}",
        body["founder_pricing_active"]
    );
}

#[tokio::test]
async fn test_update_min_random_code_length_validation() {
    let client = authenticated_client();

    // 1. Test valid value (lower bound) - Assuming 3 is a valid system minimum
    let res = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "min_random_code_length", "value": "3" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. Test valid value (upper bound: 100)
    let res = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(
            &json!({ "key": "min_random_code_length", "value": MAX_SHORT_CODE_LENGTH.to_string() }),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. Test invalid value (too short)
    let res = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "min_random_code_length", "value": "0" })) // Assuming 0 is below system_min
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 4. Test invalid value (too long: 101)
    let res = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "min_random_code_length", "value": (MAX_SHORT_CODE_LENGTH + 1).to_string() }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Restore the default so other tests are unaffected
    let res = client
        .put(format!("{}/api/admin/settings", BASE_URL))
        .json(&json!({ "key": "min_random_code_length", "value": "6" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
