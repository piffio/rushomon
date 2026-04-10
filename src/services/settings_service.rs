/// Settings service - Business logic for system settings
///
/// Handles setting validation, business rules, and orchestrates the settings repository.
use crate::models::Tier;
use crate::repositories::SettingsRepository;
use crate::utils::AppError;
use std::collections::HashMap;
use worker::d1::D1Database;
use worker::*;

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

    /// Get public settings for frontend consumption
    pub async fn get_public_settings(&self, db: &D1Database) -> Result<serde_json::Value> {
        let settings = self.repository.get_all_settings(db).await?;

        // Get founder pricing status from settings
        let founder_pricing_active = settings
            .get("founder_pricing_active")
            .map(|v| v == "true")
            .unwrap_or(false);

        // Helper to parse setting as i64
        let get_setting_i64 = |key: &str| -> i64 {
            settings
                .get(key)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0)
        };

        Ok(serde_json::json!({
            "founder_pricing_active": founder_pricing_active,
            "active_discount_amount_pro_monthly": get_setting_i64("active_discount_amount_pro_monthly"),
            "active_discount_amount_pro_annual": get_setting_i64("active_discount_amount_pro_annual"),
            "active_discount_amount_business_monthly": get_setting_i64("active_discount_amount_business_monthly"),
            "active_discount_amount_business_annual": get_setting_i64("active_discount_amount_business_annual"),
        }))
    }
}
