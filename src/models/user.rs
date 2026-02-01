use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub oauth_provider: String,
    pub oauth_id: String,
    pub org_id: String,
    pub role: String,
    pub created_at: i64,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserData {
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub oauth_provider: String,
    pub oauth_id: String,
}
