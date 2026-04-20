// Services layer - Business logic and orchestration
//
// Services contain business rules, validation, and coordination logic.
// They orchestrate repositories to implement use cases.
//
// Dependencies allowed:
// - repositories/
// - models/
// - utils/
//
// Dependencies NOT allowed:
// - api/ (would create circular dependency)
// - Direct DB access (use repositories instead)

// Add service modules here as they are created:
pub mod analytics_service;
pub mod api_key_service;
pub mod auth_service;
pub mod billing_service;
pub mod blacklist_service;
pub mod link_service;
pub mod oauth_service;
pub mod org_service;
pub mod product_service;
pub mod report_service;
pub mod settings_service;
pub mod subscription_service;
pub mod tag_service;
pub use api_key_service::ApiKeyService;
pub use auth_service::AuthService;
pub use billing_service::BillingService;
pub use blacklist_service::BlacklistService;
pub use link_service::LinkService;
pub use oauth_service::OAuthService;
pub use org_service::OrgService;
pub use product_service::ProductService;
pub use report_service::ReportService;
pub use settings_service::SettingsService;
pub use subscription_service::SubscriptionService;
pub use tag_service::TagService;
