/// Query parameter parsing helpers.
///
/// # Example
///
/// ```rust
/// let params = QueryParams::from_request(&req)?;
/// let page: u32 = params.get_u32("page").unwrap_or(1);
/// let limit: u32 = params.get_u32("limit").unwrap_or(50).min(100);
/// let search: Option<String> = params.get("search");
/// ```
use std::collections::HashMap;
use worker::Request;

/// Parsed query string parameters from a request URL.
pub struct QueryParams(HashMap<String, String>);

impl QueryParams {
    /// Parse query parameters from a `Request`.
    /// Returns an empty map when there is no query string.
    pub fn from_request(req: &Request) -> worker::Result<Self> {
        let url = req.url()?;
        let map = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        Ok(Self(map))
    }

    /// Get a raw string value for a key.
    pub fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    /// Get a value parsed as `u32`. Returns `None` if the key is absent or
    /// the value cannot be parsed.
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.0.get(key)?.parse().ok()
    }

    /// Get a value parsed as `i64`. Returns `None` if the key is absent or
    /// the value cannot be parsed.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.0.get(key)?.parse().ok()
    }

    /// Returns `true` if the key is present with a truthy value (`"1"`, `"true"`, `"yes"`).
    pub fn flag(&self, key: &str) -> bool {
        matches!(
            self.0.get(key).map(|s| s.as_str()),
            Some("1") | Some("true") | Some("yes")
        )
    }
}
