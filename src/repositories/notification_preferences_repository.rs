/// Notification Preferences Repository
///
/// Data access layer for per-user email notification opt-in/out flags.
/// A missing row in `notification_preferences` is treated as fully opted-in
/// (all notifications enabled by default).
use crate::utils::now_timestamp;
use serde::{Deserialize, Serialize};
use worker::Result;
use worker::d1::D1Database;

/// User notification preferences.
/// All fields default to `true` (opted in) if no row exists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub email_monthly_stats: bool,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            email_monthly_stats: true,
        }
    }
}

/// A user record enriched with the data needed to send notifications.
#[derive(Debug, Clone)]
pub struct UserForNotification {
    pub user_id: String,
    pub email: String,
    pub name: Option<String>,
}

pub struct NotificationPreferencesRepository;

impl NotificationPreferencesRepository {
    pub fn new() -> Self {
        Self
    }

    /// Get notification preferences for a user.
    /// Returns defaults (all enabled) if no row exists.
    pub async fn get_by_user_id(
        &self,
        db: &D1Database,
        user_id: &str,
    ) -> Result<NotificationPreferences> {
        let result = db
            .prepare(
                "SELECT email_monthly_stats
                 FROM notification_preferences
                 WHERE user_id = ?1",
            )
            .bind(&[user_id.into()])?
            .first::<serde_json::Value>(None)
            .await?;

        match result {
            Some(row) => {
                let email_monthly_stats = row["email_monthly_stats"]
                    .as_f64()
                    .map(|v| v != 0.0)
                    .unwrap_or(true);
                Ok(NotificationPreferences {
                    email_monthly_stats,
                })
            }
            // No row → return defaults (all opted in)
            None => Ok(NotificationPreferences::default()),
        }
    }

    /// Upsert notification preferences for a user.
    pub async fn upsert(
        &self,
        db: &D1Database,
        user_id: &str,
        prefs: &NotificationPreferences,
    ) -> Result<NotificationPreferences> {
        let now = now_timestamp();
        db.prepare(
            "INSERT INTO notification_preferences (user_id, email_monthly_stats, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(user_id) DO UPDATE SET
                 email_monthly_stats = excluded.email_monthly_stats,
                 updated_at = excluded.updated_at",
        )
        .bind(&[
            user_id.into(),
            (if prefs.email_monthly_stats {
                1_f64
            } else {
                0_f64
            })
            .into(),
            (now as f64).into(),
        ])?
        .run()
        .await?;

        Ok(prefs.clone())
    }

    /// Return all non-suspended users who have opted into monthly stats emails.
    ///
    /// Users with no row in `notification_preferences` are included because a
    /// missing row means "default = opted in" (LEFT JOIN + COALESCE).
    pub async fn get_monthly_stats_recipients(
        &self,
        db: &D1Database,
    ) -> Result<Vec<UserForNotification>> {
        let rows = db
            .prepare(
                "SELECT u.id AS user_id, u.email, u.name
                 FROM users u
                 LEFT JOIN notification_preferences np ON np.user_id = u.id
                 WHERE u.suspended_at IS NULL
                   AND COALESCE(np.email_monthly_stats, 1) = 1",
            )
            .all()
            .await?
            .results::<serde_json::Value>()?;

        let users = rows
            .into_iter()
            .filter_map(|row| {
                let user_id = row["user_id"].as_str()?.to_string();
                let email = row["email"].as_str()?.to_string();
                let name = row["name"].as_str().map(|s| s.to_string());
                Some(UserForNotification {
                    user_id,
                    email,
                    name,
                })
            })
            .collect();

        Ok(users)
    }
}
