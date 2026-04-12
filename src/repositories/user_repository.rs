/// User Repository
///
/// Data access layer for user records in D1.
/// Note: Session data is stored in KV and managed via auth::session.
use crate::models::user::User;
use worker::Result;
use worker::d1::D1Database;

pub struct UserRepository;

impl UserRepository {
    pub fn new() -> Self {
        Self
    }

    /// Get a user by their ID
    pub async fn get_user_by_id(&self, db: &D1Database, user_id: &str) -> Result<Option<User>> {
        let stmt = db.prepare(
            "SELECT id, email, name, avatar_url, oauth_provider, oauth_id, org_id, role, created_at,
                    suspended_at, suspension_reason, suspended_by
             FROM users
             WHERE id = ?1",
        );

        stmt.bind(&[user_id.into()])?.first::<User>(None).await
    }
}

impl Default for UserRepository {
    fn default() -> Self {
        Self::new()
    }
}
