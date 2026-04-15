/// Organization API handlers
///
/// Module structure:
/// - `list`: List user orgs, switch org
/// - `crud`: Create, get, update, delete orgs
/// - `members`: Member management
/// - `invitations`: Invite flow
/// - `settings`: Org-level settings
/// - `logo`: Logo upload/get/delete
pub mod crud;
pub mod invitations;
pub mod list;
pub mod logo;
pub mod members;
pub mod settings;

// Re-export all public handlers for router registration
pub use crud::{handle_create_org, handle_delete_org, handle_get_org, handle_update_org};
pub use invitations::{
    handle_accept_invite, handle_create_invitation, handle_get_invite_info,
    handle_resend_invitation, handle_revoke_invitation,
};
pub use list::{handle_list_user_orgs, handle_switch_org};
pub use logo::{handle_delete_org_logo, handle_get_org_logo, handle_upload_org_logo};
pub use members::handle_remove_member;
pub use settings::{handle_get_org_settings, handle_update_org_settings};
