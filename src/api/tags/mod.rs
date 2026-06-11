mod analytics;
/// Tag API handlers
mod create;
mod delete;
mod list;
mod merge;
mod rename;

// Re-export handlers and utoipa path helpers
#[allow(unused_imports)]
pub use analytics::__path_handle_get_tag_analytics;
pub use analytics::handle_get_tag_analytics;

#[allow(unused_imports)]
pub use create::__path_handle_create_tag;
pub use create::handle_create_tag;

#[allow(unused_imports)]
pub use delete::__path_handle_delete_org_tag;
pub use delete::handle_delete_org_tag;

#[allow(unused_imports)]
pub use list::__path_handle_get_org_tags;
pub use list::handle_get_org_tags;

#[allow(unused_imports)]
pub use merge::__path_handle_merge_tags;
pub use merge::handle_merge_tags;

#[allow(unused_imports)]
pub use rename::__path_handle_rename_org_tag;
pub use rename::handle_rename_org_tag;
