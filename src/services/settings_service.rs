/// Settings service - Business logic for system settings
///
/// Handles setting validation, business rules, and orchestrates the settings repository.
use crate::models::Tier;
use crate::repositories::SettingsRepository;
use crate::utils::AppError;
use crate::utils::short_code::{
    DEFAULT_MIN_CUSTOM_CODE_LENGTH, DEFAULT_MIN_RANDOM_CODE_LENGTH, DEFAULT_SYSTEM_MIN_CODE_LENGTH,
    MAX_SHORT_CODE_LENGTH,
};
use std::collections::HashMap;
use worker::d1::D1Database;
use worker::*;

/// Code length settings with effective limits applied
///
/// `system_min_length` is a self-healing high-watermark: when random code
/// generation detects namespace exhaustion at a given length, it raises this
/// value so the whole application scales up together.
pub struct CodeLengthSettings {
    pub min_random_length: usize,
    pub system_min_length: usize,
    pub effective_custom_min: usize,
}

/// Service for settings operations
#[derive(Default)]
pub struct SettingsService {
    repository: SettingsRepository,
}

impl SettingsService {
    /// Create a new settings service instance
    pub fn new() -> Self {
        Self {
            repository: SettingsRepository::new(),
        }
    }

    /// Get all settings as a HashMap
    pub async fn get_all_settings(&self, db: &D1Database) -> Result<HashMap<String, String>> {
        self.repository.get_all_settings(db).await
    }

    /// Get a single setting value by key
    #[allow(dead_code)]
    pub async fn get_setting(&self, db: &D1Database, key: &str) -> Result<Option<String>> {
        self.repository.get_setting(db, key).await
    }

    /// Validate and update a setting
    /// Returns the updated settings map on success
    pub async fn update_setting(
        &self,
        db: &D1Database,
        key: &str,
        value: &str,
    ) -> Result<HashMap<String, String>, AppError> {
        // Validate known settings
        match key {
            "signups_enabled" => {
                if value != "true" && value != "false" {
                    return Err(AppError::BadRequest(
                        "Invalid value for 'signups_enabled'. Must be 'true' or 'false'"
                            .to_string(),
                    ));
                }
            }
            "default_user_tier" => {
                if Tier::from_str_value(value).is_none() {
                    return Err(AppError::BadRequest(
                        "Invalid value for 'default_user_tier'. Must be 'free' or 'unlimited'"
                            .to_string(),
                    ));
                }
            }
            "founder_pricing_active" => {
                if value != "true" && value != "false" {
                    return Err(AppError::BadRequest(
                        "Invalid value for 'founder_pricing_active'. Must be 'true' or 'false'"
                            .to_string(),
                    ));
                }
            }
            "min_random_code_length" | "min_custom_code_length" => {
                let val = value.parse::<usize>().unwrap_or(0);
                // The system watermark is a hard floor: namespaces below it are
                // already exhausted, so admins cannot configure lengths under it
                let system_min = self
                    .repository
                    .get_setting(db, "system_min_code_length")
                    .await?
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(DEFAULT_SYSTEM_MIN_CODE_LENGTH);

                if val < system_min || val > MAX_SHORT_CODE_LENGTH {
                    return Err(AppError::BadRequest(format!(
                        "The namespace for lengths under {} is exhausted. Value must be between {} and {}.",
                        system_min, system_min, MAX_SHORT_CODE_LENGTH
                    )));
                }
            }
            "active_discount_pro_monthly"
            | "active_discount_pro_annual"
            | "active_discount_business_monthly"
            | "active_discount_business_annual"
            | "active_discount_amount_pro_monthly"
            | "active_discount_amount_pro_annual"
            | "active_discount_amount_business_monthly"
            | "active_discount_amount_business_annual"
            | "product_pro_monthly_id"
            | "product_pro_annual_id"
            | "product_business_monthly_id"
            | "product_business_annual_id" => {
                // Any string is valid (discount UUID / product UUID / amount in cents, or empty string to clear)
            }
            _ => {
                return Err(AppError::BadRequest(format!("Unknown setting: {}", key)));
            }
        }

        // Update the setting
        self.repository.set_setting(db, key, value).await?;

        // Return updated settings
        let settings = self.repository.get_all_settings(db).await?;
        Ok(settings)
    }

    /// Fetch all code length related settings in a single query and calculate
    /// the effective limits (the configured minimums, raised to the
    /// `system_min_code_length` high-watermark where applicable)
    pub async fn get_code_length_settings(&self, db: &D1Database) -> Result<CodeLengthSettings> {
        let settings = self.repository.get_all_settings(db).await?;

        let get_setting_usize = |key: &str, default: usize| -> usize {
            settings
                .get(key)
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(default)
        };

        let min_random =
            get_setting_usize("min_random_code_length", DEFAULT_MIN_RANDOM_CODE_LENGTH);
        let min_custom =
            get_setting_usize("min_custom_code_length", DEFAULT_MIN_CUSTOM_CODE_LENGTH);
        let system_min =
            get_setting_usize("system_min_code_length", DEFAULT_SYSTEM_MIN_CODE_LENGTH);

        Ok(CodeLengthSettings {
            min_random_length: min_random,
            system_min_length: system_min,
            effective_custom_min: min_custom.max(system_min),
        })
    }

    /// Get public settings for frontend consumption
    pub async fn get_public_settings(
        &self,
        db: &D1Database,
        env: &worker::Env,
    ) -> Result<serde_json::Value> {
        let settings = self.repository.get_all_settings(db).await?;

        // Get founder pricing status from settings
        let founder_pricing_active = settings
            .get("founder_pricing_active")
            .map(|v| v == "true")
            .unwrap_or(false);

        // Helper to parse setting as i64 (with default)
        let get_setting_i64 = |key: &str, default: i64| -> i64 {
            settings
                .get(key)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(default)
        };

        let raw_min_random = get_setting_i64(
            "min_random_code_length",
            DEFAULT_MIN_RANDOM_CODE_LENGTH as i64,
        );
        let raw_min_custom = get_setting_i64(
            "min_custom_code_length",
            DEFAULT_MIN_CUSTOM_CODE_LENGTH as i64,
        );
        let system_min = get_setting_i64(
            "system_min_code_length",
            DEFAULT_SYSTEM_MIN_CODE_LENGTH as i64,
        );

        let effective_min_random = raw_min_random.max(system_min);
        let effective_min_custom = raw_min_custom.max(system_min);

        // Email notification toggles are only shown when Mailgun is configured
        let email_notifications_enabled = crate::utils::is_mailgun_configured(env);

        Ok(serde_json::json!({
            "founder_pricing_active": founder_pricing_active,
            "min_random_code_length": effective_min_random,
            "min_custom_code_length": effective_min_custom,
            "system_min_code_length": system_min,
            "active_discount_amount_pro_monthly": get_setting_i64("active_discount_amount_pro_monthly", 0),
            "active_discount_amount_pro_annual": get_setting_i64("active_discount_amount_pro_annual", 0),
            "active_discount_amount_business_monthly": get_setting_i64("active_discount_amount_business_monthly", 0),
            "active_discount_amount_business_annual": get_setting_i64("active_discount_amount_business_annual", 0),
            "email_notifications_enabled": email_notifications_enabled,
        }))
    }
}
