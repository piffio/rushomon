/// CORS and security header middleware
use worker::{Env, Response};

/// Check if an origin is allowed for CORS
pub fn is_allowed_origin(origin: &str, env: &Env) -> bool {
    // Get allowed origins from environment variable (comma-separated)
    // Format: "https://rushomon-ui.pages.dev,http://localhost:5173,http://localhost:5174"
    if let Ok(allowed_origins) = env.var("ALLOWED_ORIGINS") {
        let allowed_str = allowed_origins.to_string();
        for allowed_origin in allowed_str.split(',') {
            if allowed_origin.trim() == origin {
                return true;
            }
        }
    }

    // Fallback: Allow local development ONLY in debug builds
    // In production (release builds), localhost is NOT allowed for security
    #[cfg(debug_assertions)]
    if origin == "http://localhost:5173" || origin == "http://localhost:5174" {
        return true;
    }

    // Check for ephemeral environment pattern from env var
    // Format: "pr-{}.rushomon-ui.pages.dev" where {} is replaced with PR number
    if let Ok(ephemeral_pattern) = env.var("EPHEMERAL_ORIGIN_PATTERN") {
        let pattern = ephemeral_pattern.to_string();

        // Check if pattern contains placeholder
        if let Some(prefix_end) = pattern.find("{}") {
            let prefix = &pattern[..prefix_end];
            let suffix = &pattern[prefix_end + 2..];

            if origin.starts_with(prefix) && origin.ends_with(suffix) {
                // Extract the value between prefix and suffix
                let value = &origin[prefix.len()..origin.len() - suffix.len()];
                // Validate it's numeric (for security - prevent arbitrary subdomains)
                return !value.is_empty() && value.chars().all(|c| c.is_numeric());
            }
        }
    }

    false
}

/// Add security headers to all responses
///
/// Security headers applied:
/// - Content-Security-Policy: Comprehensive XSS protection
/// - X-Content-Type-Options: nosniff - Prevents MIME type sniffing
/// - X-Frame-Options: DENY - Prevents clickjacking attacks
/// - X-XSS-Protection: 0 - Disables legacy XSS filter (modern CSP preferred)
/// - Strict-Transport-Security: Forces HTTPS (production only)
/// - Referrer-Policy: Controls referrer information leakage
/// - Permissions-Policy: Restricts access to browser features
pub fn add_security_headers(mut response: Response, is_https: bool) -> Response {
    let headers = response.headers_mut();

    // Content Security Policy - Defense in depth against XSS
    // - default-src 'self': Only load resources from same origin by default
    // - script-src 'self': Only execute scripts from same origin (no inline scripts)
    // - style-src 'self' 'unsafe-inline': Allow same-origin styles + inline (needed for SvelteKit)
    // - img-src 'self' data: https:: Allow images from same origin, data URIs, and HTTPS
    // - font-src 'self': Only load fonts from same origin
    // - connect-src 'self': Only allow API calls to same origin
    // - frame-ancestors 'none': Prevent embedding in iframes (redundant with X-Frame-Options)
    // - base-uri 'self': Prevent base tag injection
    // - form-action 'self': Restrict form submissions to same origin
    // - upgrade-insecure-requests: Automatically upgrade HTTP to HTTPS
    let csp = if is_https {
        "default-src 'self'; \
         script-src 'self'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; \
         font-src 'self'; \
         connect-src 'self'; \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         form-action 'self'; \
         upgrade-insecure-requests"
    } else {
        // Development mode (no upgrade-insecure-requests for localhost)
        "default-src 'self'; \
         script-src 'self'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; \
         font-src 'self'; \
         connect-src 'self'; \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         form-action 'self'"
    };
    let _ = headers.set("Content-Security-Policy", csp);

    // Prevent MIME type sniffing
    let _ = headers.set("X-Content-Type-Options", "nosniff");

    // Prevent clickjacking
    let _ = headers.set("X-Frame-Options", "DENY");

    // Disable legacy XSS filter (CSP is better)
    let _ = headers.set("X-XSS-Protection", "0");

    // Force HTTPS for all future requests (only in production)
    if is_https {
        let _ = headers.set(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        );
    }

    // Control referrer information
    let _ = headers.set("Referrer-Policy", "strict-origin-when-cross-origin");

    // Restrict dangerous browser features
    let _ = headers.set(
        "Permissions-Policy",
        "geolocation=(), microphone=(), camera=()",
    );

    response
}

/// Add CORS headers to a response based on the request origin
pub fn add_cors_headers(mut response: Response, origin: Option<String>, env: &Env) -> Response {
    if let Some(origin_value) = origin
        && is_allowed_origin(&origin_value, env)
    {
        let headers = response.headers_mut();
        let _ = headers.set("Access-Control-Allow-Origin", &origin_value);
        let _ = headers.set(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, PATCH, DELETE, OPTIONS",
        );
        let _ = headers.set(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, Cookie",
        );
        let _ = headers.set("Access-Control-Allow-Credentials", "true");
        let _ = headers.set("Access-Control-Max-Age", "86400"); // 24 hours
    }
    response
}
