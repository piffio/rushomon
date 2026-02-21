/// Rate limiting middleware for protecting endpoints from abuse
///
/// Uses Cloudflare KV for distributed rate limit tracking across edge locations.
/// Implements a sliding window rate limit algorithm.
///
/// **IMPORTANT**: KV-based rate limiting is now disabled by default in favor of
/// Cloudflare's built-in rate limiting rules. Set ENABLE_KV_RATE_LIMITING=true
/// to re-enable KV-based rate limiting for specific use cases.
///
/// # Current Implementation Status
///
/// Rate limiting is currently applied to:
/// - ✅ Public redirects (GET /{short_code}): 300/min per IP
/// - ✅ Link creation (POST /api/links): 100/hour per user
/// - ✅ OAuth endpoints (GET /api/auth/github, GET /api/auth/callback): 20/15min per IP
/// - ✅ Token refresh (POST /api/auth/refresh): 30/hour per session
/// - ✅ Auth check (GET /api/auth/me): 100/min per session
///
/// TODO: Apply rate limiting to remaining endpoints:
/// - Link listing (GET /api/links): 200/hour per user
/// - Admin endpoints (GET /api/admin/users): 50/hour per admin
///
/// See SECURITY.md for complete rate limiting roadmap.
use serde::{Deserialize, Serialize};
use worker::kv::KvStore;

/// Rate limit tracking data stored in KV
#[derive(Debug, Serialize, Deserialize)]
struct RateLimitData {
    /// Number of requests made in current window
    count: u32,
    /// Timestamp when the current window started (seconds)
    window_start: u64,
}

/// Rate limit configuration for different endpoint types
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests allowed in the time window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
}

impl RateLimitConfig {
    /// OAuth endpoints: 20 attempts per 15 minutes per IP
    /// Increased from 5 to allow for legitimate retries and testing
    pub fn oauth() -> Self {
        Self {
            max_requests: 20,
            window_seconds: 900, // 15 minutes
        }
    }

    /// Token refresh: 30 attempts per hour per session
    /// Increased from 10 to handle active sessions and multiple tabs
    pub fn token_refresh() -> Self {
        Self {
            max_requests: 30,
            window_seconds: 3600, // 1 hour
        }
    }

    /// Auth check endpoint (/api/auth/me): 100 per minute per session
    /// Increased from 30 to handle frequent polling from frontend
    pub fn auth_check() -> Self {
        Self {
            max_requests: 100,
            window_seconds: 60, // 1 minute
        }
    }

    /// Link creation: 100 per hour
    /// Increased from 20 to support power users and bulk operations
    pub fn link_creation() -> Self {
        Self {
            max_requests: 100,
            window_seconds: 3600, // 1 hour
        }
    }

    /// Link listing: 200 per hour
    /// Increased from 100 to handle frequent dashboard refreshes
    #[allow(dead_code)] // TODO: Apply to link listing endpoint
    pub fn link_listing() -> Self {
        Self {
            max_requests: 200,
            window_seconds: 3600, // 1 hour
        }
    }

    /// Public redirects: 300 per minute per IP
    /// Increased from 100 to handle legitimate high-traffic scenarios
    pub fn redirect() -> Self {
        Self {
            max_requests: 300,
            window_seconds: 60, // 1 minute
        }
    }
}

/// Rate limit error
#[derive(Debug)]
pub enum RateLimitError {
    /// Rate limit exceeded
    Exceeded {
        /// How many seconds until the rate limit resets
        retry_after: u64,
    },
    /// Internal error checking rate limit
    #[allow(dead_code)] // Error message used in Display impl
    Internal(String),
}

impl RateLimitError {
    pub fn to_error_response(&self) -> String {
        match self {
            RateLimitError::Exceeded { retry_after } => {
                format!("Rate limit exceeded. Try again in {} seconds.", retry_after)
            }
            RateLimitError::Internal(_) => "Failed to check rate limit".to_string(),
        }
    }

    pub fn retry_after(&self) -> Option<u64> {
        match self {
            RateLimitError::Exceeded { retry_after } => Some(*retry_after),
            RateLimitError::Internal(_) => None,
        }
    }
}

/// Rate limiter implementation
pub struct RateLimiter;

impl RateLimiter {
    /// Check if a request should be rate limited
    ///
    /// # Arguments
    ///
    /// * `kv` - KV store for tracking rate limits
    /// * `key` - Unique identifier for rate limit (e.g., "ratelimit:oauth:{ip}")
    /// * `config` - Rate limit configuration
    /// * `kv_rate_limiting_enabled` - Whether KV rate limiting is enabled (from environment)
    ///
    /// # Returns
    ///
    /// Ok(()) if request is allowed, Err(RateLimitError) if rate limit exceeded
    pub async fn check(
        kv: &KvStore,
        key: &str,
        config: &RateLimitConfig,
        kv_rate_limiting_enabled: bool,
    ) -> std::result::Result<(), RateLimitError> {
        // Check if KV-based rate limiting is disabled
        if !kv_rate_limiting_enabled {
            // KV rate limiting is disabled, allow all requests
            return Ok(());
        }

        let now = Self::current_timestamp();

        // Get existing rate limit data
        let existing_data = match kv.get(key).text().await {
            Ok(Some(data)) => {
                // Parse existing data (corrupted data treated as new)
                serde_json::from_str::<RateLimitData>(&data).ok()
            }
            Ok(None) => None,
            Err(_) => {
                return Err(RateLimitError::Internal(
                    "Failed to read rate limit data".to_string(),
                ));
            }
        };

        // Calculate new rate limit state
        let (new_count, window_start) = match existing_data {
            Some(data) => {
                // Check if we're still in the same window
                if now - data.window_start < config.window_seconds {
                    // Same window, increment count
                    (data.count + 1, data.window_start)
                } else {
                    // New window, reset count
                    (1, now)
                }
            }
            None => {
                // First request, start new window
                (1, now)
            }
        };

        // Check if rate limit exceeded
        if new_count > config.max_requests {
            let retry_after = config.window_seconds - (now - window_start);
            return Err(RateLimitError::Exceeded { retry_after });
        }

        // Update rate limit data in KV
        let new_data = RateLimitData {
            count: new_count,
            window_start,
        };

        let value = serde_json::to_string(&new_data)
            .map_err(|e| RateLimitError::Internal(format!("Failed to serialize: {}", e)))?;

        // Store with TTL equal to window duration
        if kv
            .put(key, value)
            .map_err(|e| RateLimitError::Internal(format!("Failed to put: {}", e)))?
            .expiration_ttl(config.window_seconds)
            .execute()
            .await
            .is_err()
        {
            return Err(RateLimitError::Internal("Failed to store".to_string()));
        }

        Ok(())
    }

    /// Get current timestamp in seconds
    fn current_timestamp() -> u64 {
        (js_sys::Date::now() / 1000.0) as u64
    }

    /// Generate rate limit key for IP-based limiting
    pub fn ip_key(prefix: &str, ip: &str) -> String {
        format!("ratelimit:{}:{}", prefix, ip)
    }

    /// Generate rate limit key for user-based limiting
    pub fn user_key(prefix: &str, user_id: &str) -> String {
        format!("ratelimit:{}:user:{}", prefix, user_id)
    }

    /// Generate rate limit key for session-based limiting
    pub fn session_key(prefix: &str, session_id: &str) -> String {
        format!("ratelimit:{}:session:{}", prefix, session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config() {
        let oauth = RateLimitConfig::oauth();
        assert_eq!(oauth.max_requests, 20);
        assert_eq!(oauth.window_seconds, 900); // 15 minutes

        let refresh = RateLimitConfig::token_refresh();
        assert_eq!(refresh.max_requests, 30);
        assert_eq!(refresh.window_seconds, 3600);
    }

    #[test]
    fn test_rate_limit_keys() {
        assert_eq!(
            RateLimiter::ip_key("oauth", "1.2.3.4"),
            "ratelimit:oauth:1.2.3.4"
        );
        assert_eq!(
            RateLimiter::user_key("links", "user123"),
            "ratelimit:links:user:user123"
        );
        assert_eq!(
            RateLimiter::session_key("refresh", "sess456"),
            "ratelimit:refresh:session:sess456"
        );
    }

    #[test]
    fn test_rate_limit_error_messages() {
        let exceeded = RateLimitError::Exceeded { retry_after: 120 };
        assert_eq!(
            exceeded.to_error_response(),
            "Rate limit exceeded. Try again in 120 seconds."
        );
        assert_eq!(exceeded.retry_after(), Some(120));

        let internal = RateLimitError::Internal("Test error".to_string());
        assert_eq!(internal.to_error_response(), "Failed to check rate limit");
        assert_eq!(internal.retry_after(), None);
    }
}
