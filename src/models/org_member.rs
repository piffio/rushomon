use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMember {
    pub org_id: String,
    pub user_id: String,
    pub role: String,
    pub joined_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMemberWithUser {
    pub user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub joined_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgInvitation {
    pub id: String,
    pub org_id: String,
    pub invited_by: String,
    pub email: String,
    pub role: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub accepted_at: Option<i64>,
}

/// An organization with the current user's membership role attached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgWithRole {
    pub id: String,
    pub name: String,
    pub tier: String,
    pub role: String,
    pub joined_at: i64,
}
