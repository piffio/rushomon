/// Auth Service
///
/// Business logic for session management and user authentication.
/// Sessions are stored in KV via auth::session; user records are in D1 via UserRepository.
use crate::models::user::User;
use crate::repositories::UserRepository;
use worker::d1::D1Database;
use worker::{KvStore, Result};

pub struct AuthService {
    user_repo: UserRepository,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            user_repo: UserRepository::new(),
        }
    }

    /// Get user by ID, returning None if not found
    pub async fn get_user_by_id(&self, db: &D1Database, user_id: &str) -> Result<Option<User>> {
        self.user_repo.get_user_by_id(db, user_id).await
    }

    /// Logout: delete the KV session
    pub async fn logout(&self, kv: &KvStore, session_id: &str) -> Result<()> {
        crate::auth::session::delete_session(kv, session_id).await
    }
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}
