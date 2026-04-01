use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrgMember {
    #[schema(example = "org-123")]
    pub org_id: String,
    #[schema(example = "user-456")]
    pub user_id: String,
    #[schema(example = "admin")]
    pub role: String,
    #[schema(example = 1609459200)]
    pub joined_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrgMemberWithUser {
    #[schema(example = "user-456")]
    pub user_id: String,
    #[schema(example = "user@example.com")]
    pub email: String,
    #[schema(example = "John Doe")]
    pub name: Option<String>,
    #[schema(example = "https://avatars.githubusercontent.com/u/123456")]
    pub avatar_url: Option<String>,
    #[schema(example = "admin")]
    pub role: String,
    #[schema(example = 1609459200)]
    pub joined_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrgInvitation {
    #[schema(example = "inv-123")]
    pub id: String,
    #[schema(example = "org-456")]
    pub org_id: String,
    #[schema(example = "user-789")]
    pub invited_by: String,
    #[schema(example = "newmember@example.com")]
    pub email: String,
    #[schema(example = "member")]
    pub role: String,
    #[schema(example = 1609459200)]
    pub created_at: i64,
    #[schema(example = 1612137600)]
    pub expires_at: i64,
    pub accepted_at: Option<i64>,
}

/// An organization with the current user's membership role attached
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrgWithRole {
    #[schema(example = "org-123")]
    pub id: String,
    #[schema(example = "My Organization")]
    pub name: String,
    #[schema(example = "owner")]
    pub role: String,
    #[schema(example = 1609459200)]
    pub joined_at: i64,
}
