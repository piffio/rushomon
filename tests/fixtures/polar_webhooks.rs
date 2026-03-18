//! Pre-built Polar webhook payloads for testing
//!
//! These fixtures match the actual payload structure from Polar webhooks
//! using snake_case field names as Polar sends them.

use serde_json::{Value, json};

/// Get the current Unix timestamp as ISO 8601 string
fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Convert to ISO 8601 format
    let datetime = chrono::DateTime::from_timestamp(secs as i64, 0)
        .unwrap()
        .to_rfc3339();
    datetime
}

/// Get timestamp for a future date (for period_end)
pub fn future_iso(days_from_now: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let future_secs = secs as i64 + (days_from_now * 24 * 60 * 60);
    let datetime = chrono::DateTime::from_timestamp(future_secs, 0)
        .unwrap()
        .to_rfc3339();
    datetime
}

/// Get timestamp for a past date
pub fn past_iso(days_ago: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let past_secs = secs as i64 - (days_ago * 24 * 60 * 60);
    let datetime = chrono::DateTime::from_timestamp(past_secs, 0)
        .unwrap()
        .to_rfc3339();
    datetime
}

/// Customer object for webhook payloads
fn customer_object(external_id: &str) -> Value {
    json!({
        "id": "test-customer-id",
        "email": "test@example.com",
        "name": "Test User",
        "external_id": external_id,
        "type": "individual",
        "avatar_url": null,
        "billing_address": {
            "country": "US",
            "city": null,
            "line1": null,
            "line2": null,
            "postal_code": null,
            "state": null
        },
        "created_at": now_iso(),
        "modified_at": now_iso(),
        "deleted_at": null,
        "email_verified": true,
        "locale": "en-US",
        "metadata": {},
        "organization_id": "test-org-id",
        "tax_id": null
    })
}

/// Discount object for webhook payloads
fn discount_object() -> Value {
    json!({
        "id": "test-discount-id",
        "name": "Test Discount",
        "code": null,
        "type": "percentage",
        "amount": 10,
        "currency": "usd",
        "duration": "once",
        "created_at": now_iso(),
        "modified_at": null,
        "ends_at": null,
        "starts_at": null,
        "max_redemptions": null,
        "redemptions_count": 0,
        "metadata": {},
        "organization_id": "test-org-id"
    })
}

/// Price object for webhook payloads
fn price_object() -> Value {
    json!({
        "id": "test-price-id",
        "type": "recurring",
        "recurring_interval": "month",
        "price_amount": 900,
        "price_currency": "usd",
        "amount_type": "fixed",
        "product_id": "test-product-id",
        "source": "catalog",
        "is_archived": false,
        "created_at": now_iso(),
        "modified_at": null
    })
}

/// Product object for webhook payloads
fn product_object(recurring_interval: &str) -> Value {
    json!({
        "id": "test-product-id",
        "name": "Test Product",
        "description": null,
        "recurring_interval": recurring_interval,
        "recurring_interval_count": 1,
        "is_recurring": true,
        "is_archived": false,
        "visibility": "public",
        "created_at": now_iso(),
        "modified_at": now_iso(),
        "metadata": {},
        "organization_id": "test-org-id",
        "prices": [price_object()],
        "benefits": [],
        "medias": [],
        "attached_custom_fields": []
    })
}

/// Create subscription.active webhook payload
pub fn subscription_active(
    subscription_id: &str,
    customer_id: &str,
    external_id: &str,
    plan: &str,
    interval: &str,
    amount_cents: i64,
) -> Value {
    json!({
        "id": subscription_id,
        "customer_id": customer_id,
        "product_id": "test-product-id",
        "price_id": "test-price-id",
        "status": "active",
        "amount": amount_cents,
        "currency": "usd",
        "recurring_interval": interval,
        "recurring_interval_count": 1,
        "current_period_start": now_iso(),
        "current_period_end": future_iso(30),
        "cancel_at_period_end": false,
        "canceled_at": null,
        "started_at": now_iso(),
        "ends_at": null,
        "ended_at": null,
        "trial_start": null,
        "trial_end": null,
        "discount_id": null,
        "checkout_id": "test-checkout-id",
        "metadata": {
            "billing_account_id": external_id
        },
        "custom_field_data": {},
        "customer_cancellation_reason": null,
        "customer_cancellation_comment": null,
        "seats": null,
        "created_at": now_iso(),
        "modified_at": now_iso(),
        "customer": customer_object(external_id),
        "product": product_object(interval),
        "discount": null,
        "prices": [price_object()],
        "price": price_object(),
        "meters": [],
        "user": {
            "id": customer_id,
            "email": "test@example.com",
            "public_name": "Test User"
        }
    })
}

/// Create subscription.canceled webhook payload at period end
pub fn subscription_canceled_at_period_end(
    subscription_id: &str,
    customer_id: &str,
    external_id: &str,
    period_end: &str,
) -> Value {
    json!({
        "id": subscription_id,
        "customer_id": customer_id,
        "product_id": "test-product-id",
        "price_id": "test-price-id",
        "status": "active",
        "amount": 900,
        "currency": "usd",
        "recurring_interval": "month",
        "recurring_interval_count": 1,
        "current_period_start": now_iso(),
        "current_period_end": period_end,
        "cancel_at_period_end": true,
        "canceled_at": now_iso(),
        "started_at": now_iso(),
        "ends_at": period_end,
        "ended_at": null,
        "trial_start": null,
        "trial_end": null,
        "discount_id": null,
        "checkout_id": "test-checkout-id",
        "metadata": {
            "billing_account_id": external_id
        },
        "custom_field_data": {},
        "customer_cancellation_reason": "other",
        "customer_cancellation_comment": null,
        "seats": null,
        "created_at": now_iso(),
        "modified_at": now_iso(),
        "customer": customer_object(external_id),
        "product": product_object("month"),
        "discount": null,
        "prices": [price_object()],
        "price": price_object(),
        "meters": [],
        "user": {
            "id": customer_id,
            "email": "test@example.com",
            "public_name": "Test User"
        }
    })
}

/// Create subscription.canceled webhook payload for immediate cancellation
pub fn subscription_canceled_immediate(
    subscription_id: &str,
    customer_id: &str,
    external_id: &str,
) -> Value {
    json!({
        "id": subscription_id,
        "customer_id": customer_id,
        "product_id": "test-product-id",
        "price_id": "test-price-id",
        "status": "canceled",
        "amount": 900,
        "currency": "usd",
        "recurring_interval": "month",
        "recurring_interval_count": 1,
        "current_period_start": past_iso(30),
        "current_period_end": past_iso(1),
        "cancel_at_period_end": false,
        "canceled_at": now_iso(),
        "started_at": past_iso(30),
        "ends_at": now_iso(),
        "ended_at": now_iso(),
        "trial_start": null,
        "trial_end": null,
        "discount_id": null,
        "checkout_id": "test-checkout-id",
        "metadata": {
            "billing_account_id": external_id
        },
        "custom_field_data": {},
        "customer_cancellation_reason": "other",
        "customer_cancellation_comment": null,
        "seats": null,
        "created_at": past_iso(30),
        "modified_at": now_iso(),
        "customer": customer_object(external_id),
        "product": product_object("month"),
        "discount": null,
        "prices": [price_object()],
        "price": price_object(),
        "meters": [],
        "user": {
            "id": customer_id,
            "email": "test@example.com",
            "public_name": "Test User"
        }
    })
}

/// Create subscription.uncanceled webhook payload
pub fn subscription_uncanceled(
    subscription_id: &str,
    customer_id: &str,
    external_id: &str,
    period_end: &str,
) -> Value {
    json!({
        "id": subscription_id,
        "customer_id": customer_id,
        "product_id": "test-product-id",
        "price_id": "test-price-id",
        "status": "active",
        "amount": 900,
        "currency": "usd",
        "recurring_interval": "month",
        "recurring_interval_count": 1,
        "current_period_start": now_iso(),
        "current_period_end": period_end,
        "cancel_at_period_end": false,
        "canceled_at": null,
        "started_at": now_iso(),
        "ends_at": null,
        "ended_at": null,
        "trial_start": null,
        "trial_end": null,
        "discount_id": null,
        "checkout_id": "test-checkout-id",
        "metadata": {
            "billing_account_id": external_id
        },
        "custom_field_data": {},
        "customer_cancellation_reason": null,
        "customer_cancellation_comment": null,
        "seats": null,
        "created_at": now_iso(),
        "modified_at": now_iso(),
        "customer": customer_object(external_id),
        "product": product_object("month"),
        "discount": null,
        "prices": [price_object()],
        "price": price_object(),
        "meters": [],
        "user": {
            "id": customer_id,
            "email": "test@example.com",
            "public_name": "Test User"
        }
    })
}

/// Create subscription.updated webhook payload
pub fn subscription_updated(
    subscription_id: &str,
    customer_id: &str,
    external_id: &str,
    status: &str,
    cancel_at_period_end: bool,
    period_end: &str,
    plan: &str,
    interval: &str,
) -> Value {
    json!({
        "id": subscription_id,
        "customer_id": customer_id,
        "product_id": "test-product-id",
        "price_id": "test-price-id",
        "status": status,
        "amount": 900,
        "currency": "usd",
        "recurring_interval": interval,
        "recurring_interval_count": 1,
        "current_period_start": now_iso(),
        "current_period_end": period_end,
        "cancel_at_period_end": cancel_at_period_end,
        "canceled_at": null,
        "started_at": now_iso(),
        "ends_at": if cancel_at_period_end { Some(period_end) } else { None },
        "ended_at": null,
        "trial_start": null,
        "trial_end": null,
        "discount_id": null,
        "checkout_id": "test-checkout-id",
        "metadata": {
            "billing_account_id": external_id
        },
        "custom_field_data": {},
        "customer_cancellation_reason": null,
        "customer_cancellation_comment": null,
        "seats": null,
        "created_at": now_iso(),
        "modified_at": now_iso(),
        "customer": customer_object(external_id),
        "product": product_object(interval),
        "discount": null,
        "prices": [price_object()],
        "price": price_object(),
        "meters": [],
        "user": {
            "id": customer_id,
            "email": "test@example.com",
            "public_name": "Test User"
        }
    })
}
