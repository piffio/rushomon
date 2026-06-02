/// Custom domain management API
///
/// Endpoints for managing custom domains per organization (Pro+ feature).
/// SSL certificates are handled via Cloudflare for SaaS.
pub mod create;
pub mod delete;
pub mod list;
pub mod refresh;

pub use create::handle_create_domain;
pub use delete::handle_delete_domain;
pub use list::handle_list_domains;
pub use refresh::handle_refresh_domain;
