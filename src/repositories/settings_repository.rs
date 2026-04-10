/// Settings repository - Data access for system settings
///
/// Handles all database operations related to system settings.
use std::collections::HashMap;
use worker::d1::D1Database;
use worker::*;

/// Repository for settings operations
#[derive(Default)]
pub struct SettingsRepository;

impl SettingsRepository {
    /// Create a new settings repository instance
    pub fn new() -> Self {
        Self
    }

    /// Get a setting value by key
    pub async fn get_setting(&self, db: &D1Database, key: &str) -> Result<Option<String>> {
        let stmt = db.prepare("SELECT value FROM settings WHERE key = ?1");
        let result = stmt
            .bind(&[key.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        match result {
            Some(val) => Ok(val["value"].as_str().map(|s| s.to_string())),
            None => Ok(None),
        }
    }

    /// Get all settings as a HashMap
    pub async fn get_all_settings(&self, db: &D1Database) -> Result<HashMap<String, String>> {
        let stmt = db.prepare("SELECT key, value FROM settings");
        let results = stmt.all().await?;
        let rows = results.results::<serde_json::Value>()?;

        let mut settings = HashMap::new();
        for row in rows {
            if let (Some(key), Some(value)) = (row["key"].as_str(), row["value"].as_str()) {
                settings.insert(key.to_string(), value.to_string());
            }
        }
        Ok(settings)
    }

    /// Set a setting value (upsert)
    pub async fn set_setting(&self, db: &D1Database, key: &str, value: &str) -> Result<()> {
        let now = crate::utils::now_timestamp();
        let stmt = db.prepare(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3",
        );

        stmt.bind(&[key.into(), value.into(), (now as f64).into()])?
            .run()
            .await?;

        Ok(())
    }
}
