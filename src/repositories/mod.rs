// Repositories layer - Data access and persistence
//
// Repositories handle database queries, KV operations, and data serialization.
// They return domain models and hide persistence details from services.
//
// Dependencies allowed:
// - models/
// - db/ (legacy, will be phased out)
// - kv/
//
// Dependencies NOT allowed:
// - api/ (would create circular dependency)
// - services/ (would create circular dependency)
// - Business logic (belongs in services)

// Add repository modules here as they are created:
pub mod analytics_repository;
pub mod settings_repository;
pub mod tag_repository;
pub mod user_repository;
pub use analytics_repository::AnalyticsRepository;
pub use settings_repository::SettingsRepository;
pub use tag_repository::TagRepository;
pub use user_repository::UserRepository;
