/// Admin service - Business logic for administrative operations
///
/// Handles admin-specific business rules and validation.
/// Orchestrates UserRepository, ApiKeyRepository, BillingRepository.
use crate::repositories::{ApiKeyRepository, BillingRepository, UserRepository};
use crate::utils::AppError;
use worker::d1::D1Database;

/// Service for admin-related business logic
pub struct AdminService;

impl AdminService {
    pub fn new() -> Self {
        Self
    }

    /// List all users with billing info (paginated).
    pub async fn list_users(
        &self,
        db: &D1Database,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        let offset = (page - 1) * limit;
        let repo = UserRepository::new();
        let users = repo.list_with_billing_info(db, limit, offset).await?;
        let users_json: Vec<serde_json::Value> = users
            .into_iter()
            .map(|u| serde_json::to_value(u).unwrap())
            .collect();
        let total = repo.count(db).await?;
        Ok((users_json, total))
    }

    /// Get a single user by ID.
    pub async fn get_user(
        &self,
        db: &D1Database,
        user_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        let repo = UserRepository::new();
        match repo.get_user_by_id(db, user_id).await? {
            Some(user) => Ok(serde_json::to_value(user).unwrap()),
            None => Err(AppError::NotFound("User not found".to_string())),
        }
    }

    /// Update a user's role.
    ///
    /// Returns Err(AppError::BadRequest) if trying to modify own role or demote last admin.
    pub async fn update_user_role(
        &self,
        db: &D1Database,
        target_user_id: &str,
        new_role: &str,
        admin_user_id: &str,
    ) -> Result<(), AppError> {
        if target_user_id == admin_user_id {
            return Err(AppError::BadRequest(
                "Cannot modify your own role".to_string(),
            ));
        }

        if new_role != "admin" && new_role != "member" {
            return Err(AppError::BadRequest(
                "Role must be 'admin' or 'member'".to_string(),
            ));
        }

        let repo = UserRepository::new();
        let target_user = repo
            .get_user_by_id(db, target_user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if new_role == "member" && target_user.role == "admin" {
            let admin_count = repo.admin_count(db).await?;
            if admin_count <= 1 {
                return Err(AppError::BadRequest(
                    "Cannot demote the last admin user".to_string(),
                ));
            }
        }

        repo.update_role(db, target_user_id, new_role).await?;
        Ok(())
    }

    /// Suspend a user.
    ///
    /// Returns Err(AppError::BadRequest) if trying to suspend self or suspend last admin.
    pub async fn suspend_user(
        &self,
        db: &D1Database,
        target_user_id: &str,
        reason: &str,
        admin_user_id: &str,
    ) -> Result<serde_json::Value, AppError> {
        if target_user_id == admin_user_id {
            return Err(AppError::BadRequest("Cannot suspend yourself".to_string()));
        }

        let repo = UserRepository::new();
        let admin_count = repo.admin_count(db).await?;
        if admin_count <= 1
            && let Some(target_user) = repo.get_user_by_id(db, target_user_id).await?
            && target_user.role == "admin"
        {
            return Err(AppError::BadRequest(
                "Cannot suspend the last admin".to_string(),
            ));
        }

        let target_user = repo
            .get_user_by_id(db, target_user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let org_links = repo.get_links_by_org(db, &target_user.org_id).await?;
        repo.suspend(db, target_user_id, reason, admin_user_id)
            .await?;
        let disabled_count = repo
            .disable_all_links_for_org(db, &target_user.org_id)
            .await?;

        Ok(serde_json::json!({
            "disabled_count": disabled_count,
            "org_links": org_links
        }))
    }

    /// Unsuspend a user.
    pub async fn unsuspend_user(
        &self,
        db: &D1Database,
        target_user_id: &str,
    ) -> Result<(), AppError> {
        let repo = UserRepository::new();
        repo.unsuspend(db, target_user_id).await?;
        Ok(())
    }

    /// Delete a user and all associated data.
    ///
    /// Returns Err(AppError::BadRequest) if trying to delete self or delete last admin in org.
    pub async fn delete_user(
        &self,
        db: &D1Database,
        target_user_id: &str,
        admin_user_id: &str,
    ) -> Result<(i64, i64, i64), AppError> {
        if target_user_id == admin_user_id {
            return Err(AppError::BadRequest(
                "Cannot delete your own account".to_string(),
            ));
        }

        let repo = UserRepository::new();
        let target_user = repo
            .get_user_by_id(db, target_user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if target_user.role == "admin"
            && repo
                .is_last_admin_in_org(db, target_user_id, &target_user.org_id)
                .await?
        {
            return Err(AppError::BadRequest(
                "Cannot delete the last admin in an organization".to_string(),
            ));
        }

        let _user_links = repo.get_links_by_creator(db, target_user_id).await?;
        let (user_count, links_count, analytics_count) = repo.delete(db, target_user_id).await?;

        if user_count == 0 {
            return Err(AppError::Internal("Failed to delete user".to_string()));
        }

        Ok((
            user_count as i64,
            links_count as i64,
            analytics_count as i64,
        ))
    }

    /// List all API keys (paginated, with optional search and status filter).
    pub async fn list_api_keys(
        &self,
        db: &D1Database,
        page: i64,
        limit: i64,
        search: Option<&str>,
        status_filter: Option<&str>,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        let repo = ApiKeyRepository::new();
        let (keys, total) = repo
            .list_all(db, page, limit, search, status_filter)
            .await?;
        let keys_json: Vec<serde_json::Value> = keys
            .into_iter()
            .map(|k| serde_json::to_value(k).unwrap())
            .collect();
        Ok((keys_json, total))
    }

    /// Revoke an API key.
    pub async fn revoke_api_key(
        &self,
        db: &D1Database,
        key_id: &str,
        admin_user_id: &str,
    ) -> Result<(), AppError> {
        let repo = ApiKeyRepository::new();
        repo.revoke(db, key_id, admin_user_id).await?;
        Ok(())
    }

    /// Reactivate an API key.
    pub async fn reactivate_api_key(
        &self,
        db: &D1Database,
        key_id: &str,
        admin_user_id: &str,
    ) -> Result<(), AppError> {
        let repo = ApiKeyRepository::new();
        repo.reactivate(db, key_id, admin_user_id).await?;
        Ok(())
    }

    /// Soft-delete an API key.
    pub async fn delete_api_key(
        &self,
        db: &D1Database,
        key_id: &str,
        admin_user_id: &str,
    ) -> Result<(), AppError> {
        let repo = ApiKeyRepository::new();
        repo.delete(db, key_id, admin_user_id).await?;
        Ok(())
    }

    /// Restore a deleted API key.
    pub async fn restore_api_key(
        &self,
        db: &D1Database,
        key_id: &str,
        admin_user_id: &str,
    ) -> Result<(), AppError> {
        let repo = ApiKeyRepository::new();
        repo.restore(db, key_id, admin_user_id).await?;
        Ok(())
    }

    /// Update a billing account's tier.
    ///
    /// Returns Err(AppError::BadRequest) if tier is invalid.
    pub async fn update_billing_account_tier(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        new_tier: &str,
    ) -> Result<(), AppError> {
        if !matches!(new_tier, "free" | "pro" | "business" | "unlimited") {
            return Err(AppError::BadRequest(
                "Invalid tier. Must be: free, pro, business, or unlimited".to_string(),
            ));
        }

        let repo = BillingRepository::new();
        repo.update_tier(db, billing_account_id, new_tier).await?;
        Ok(())
    }

    /// Update a subscription's status.
    ///
    /// If status is "canceled", also downgrades the billing account to "free" tier.
    pub async fn update_subscription_status(
        &self,
        db: &D1Database,
        billing_account_id: &str,
        new_status: &str,
    ) -> Result<String, AppError> {
        let repo = BillingRepository::new();
        let subscription = repo
            .get_subscription(db, billing_account_id)
            .await?
            .ok_or_else(|| AppError::NotFound("No subscription found".to_string()))?;

        let subscription_id = subscription["id"]
            .as_str()
            .ok_or_else(|| AppError::Internal("Invalid subscription data".to_string()))?
            .to_string();

        let now = crate::utils::now_timestamp();
        repo.update_subscription_status(db, &subscription_id, new_status, now)
            .await?;

        if new_status == "canceled" {
            repo.update_tier(db, billing_account_id, "free").await?;
        }

        Ok(subscription_id)
    }
}
