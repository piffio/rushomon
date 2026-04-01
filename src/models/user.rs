use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct User {
    #[schema(example = "user-123456")]
    pub id: String,
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: Option<String>,
    #[schema(example = "https://avatars.githubusercontent.com/u/123456")]
    pub avatar_url: Option<String>,
    #[schema(example = "github")]
    pub oauth_provider: String,
    #[schema(example = "123456")]
    pub oauth_id: String,
    #[schema(example = "org-789")]
    pub org_id: String,
    #[schema(example = "member")]
    pub role: String,
    #[schema(example = 1609459200)]
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspension_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspended_by: Option<String>,
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
