/// Tag API handlers
mod delete;
mod list;
mod rename;

// Re-export handlers and utoipa path helpers
#[allow(unused_imports)]
pub use delete::__path_handle_delete_org_tag;
pub use delete::handle_delete_org_tag;
#[allow(unused_imports)]
pub use list::__path_handle_get_org_tags;
pub use list::handle_get_org_tags;
#[allow(unused_imports)]
pub use rename::__path_handle_rename_org_tag;
pub use rename::handle_rename_org_tag;
