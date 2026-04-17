/// URL utilities
use worker::*;

/// Extract a query parameter from a query string
pub fn extract_query_param(query: &str, name: &str) -> Result<String> {
    query
        .split('&')
        .find_map(|pair| {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 && parts[0] == name {
                // URL-decode the parameter value
                let decoded = urlencoding::decode(parts[1]).ok()?;
                Some(decoded.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| Error::RustError(format!("Missing {} parameter", name)))
}
