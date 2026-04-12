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
pub mod auth_service;
pub mod settings_service;
pub mod tag_service;
pub use auth_service::AuthService;
pub use settings_service::SettingsService;
pub use tag_service::TagService;
