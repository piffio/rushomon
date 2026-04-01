use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Pagination metadata for paginated API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    /// Current page number (1-indexed)
    #[schema(example = 1)]
    pub page: i64,
    /// Number of items per page
    #[schema(example = 20)]
    pub limit: i64,
    /// Total number of items across all pages
    #[schema(example = 150)]
    pub total: i64,
    /// Total number of pages
    #[schema(example = 8)]
    pub total_pages: i64,
    /// Whether there is a next page
    #[schema(example = true)]
    pub has_next: bool,
    /// Whether there is a previous page
    #[schema(example = false)]
    pub has_prev: bool,
}

impl PaginationMeta {
    /// Create pagination metadata from page, limit, and total
    pub fn new(page: i64, limit: i64, total: i64) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            (total + limit - 1) / limit // Ceiling division
        };

        Self {
            page,
            limit,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T: ToSchema> {
    /// The data items for the current page
    pub data: Vec<T>,
    /// Pagination metadata
    pub pagination: PaginationMeta,
    /// Optional stats (e.g., dashboard statistics)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<serde_json::Value>,
}

impl<T: ToSchema> PaginatedResponse<T> {
    /// Create a new paginated response with stats
    pub fn with_stats(data: Vec<T>, pagination: PaginationMeta, stats: serde_json::Value) -> Self {
        Self {
            data,
            pagination,
            stats: Some(stats),
        }
    }
}
