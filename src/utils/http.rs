/// HTTP utilities for request handling
use worker::*;

/// Extract client IP from Cloudflare headers
pub fn get_client_ip(req: &Request) -> String {
    if let Ok(Some(ip)) = req.headers().get("CF-Connecting-IP") {
        return ip;
    }
    // Fallback to X-Forwarded-For
    if let Ok(Some(forwarded)) = req.headers().get("X-Forwarded-For")
        && let Some(ip) = forwarded.split(',').next()
    {
        // Take first IP in the list
        return ip.trim().to_string();
    }
    // Fallback to X-Real-IP
    if let Ok(Some(ip)) = req.headers().get("X-Real-IP") {
        return ip;
    }
    // Last resort: use a placeholder (should never happen with Cloudflare)
    "unknown".to_string()
}

/// Hash an IP address for logging to avoid storing raw IPs
pub fn hash_ip(ip: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    ip.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
