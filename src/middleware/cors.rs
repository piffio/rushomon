/// CORS and security header middleware
use worker::{Env, Headers, Response, Result};

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
pub fn add_security_headers(mut response: Response, is_https: bool, env: &Env) -> Response {
    let headers = response.headers_mut();

    // Get API domain for connect-src - needed when frontend and API are on different domains
    let api_domain = env.var("DOMAIN").map(|d| d.to_string()).unwrap_or_default();
    let connect_src = if api_domain.is_empty() {
        "connect-src 'self';".to_string()
    } else {
        format!("connect-src 'self' https://{};", api_domain)
    };

    // Content Security Policy - Defense in depth against XSS
    // - default-src 'self': Only load resources from same origin by default
    // - script-src 'self': Only execute scripts from same origin (no inline scripts)
    // - style-src 'self' 'unsafe-inline': Allow same-origin styles + inline (needed for SvelteKit)
    // - img-src 'self' data: https:: Allow images from same origin, data URIs, and HTTPS
    // - font-src 'self': Only load fonts from same origin
    // - connect-src 'self' [api_domain]: Allow API calls to same origin and configured API domain
    // - frame-ancestors 'none': Prevent embedding in iframes (redundant with X-Frame-Options)
    // - base-uri 'self': Prevent base tag injection
    // - form-action 'self': Restrict form submissions to same origin
    // - upgrade-insecure-requests: Automatically upgrade HTTP to HTTPS
    let csp = if is_https {
        &format!(
            "default-src 'self'; \
         script-src 'self' 'unsafe-inline'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; \
         font-src 'self' data:; \
         {} \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         form-action 'self'; \
         upgrade-insecure-requests",
            connect_src
        )
    } else {
        // Development mode (no upgrade-insecure-requests for localhost)
        &format!(
            "default-src 'self'; \
         script-src 'self' 'unsafe-inline'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: https:; \
         font-src 'self' data:; \
         {} \
         frame-ancestors 'none'; \
         base-uri 'self'; \
         form-action 'self'",
            connect_src
        )
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

/// Compute the filtered and augmented header list for a rebuilt asset response.
///
/// Pure function (no wasm I/O) so it can be unit-tested on native targets.
/// Rules applied to the incoming `(name, value)` pairs:
/// - Drop `content-encoding` — Cloudflare re-applies compression on egress.
/// - Drop `content-length` — body length changes after we strip encoding.
/// - For `/_app/immutable/*` paths, drop `cache-control` from the source and
///   inject a long-lived immutable directive instead.
pub fn build_asset_headers(source: &[(String, String)], path: &str) -> Vec<(String, String)> {
    let is_immutable = path.starts_with("/_app/immutable/");
    let mut out: Vec<(String, String)> = source
        .iter()
        .filter(|(k, _)| {
            let lk = k.to_lowercase();
            if lk == "content-encoding" || lk == "content-length" {
                return false;
            }
            if lk == "cache-control" && is_immutable {
                return false;
            }
            true
        })
        .cloned()
        .collect();
    if is_immutable {
        out.push((
            "Cache-Control".to_string(),
            "public, max-age=31536000, immutable".to_string(),
        ));
    }
    out
}

/// Rebuild an ASSETS-binding response with a fresh mutable Headers object
///
/// Responses from `Fetcher::fetch()` have an immutable Headers guard per the Fetch API spec,
/// so `headers_mut().set()` is a silent no-op on those responses. This function copies all
/// headers into a new mutable Headers, rebuilds the response body, and — for content-hashed
/// assets under `/_app/immutable/` — replaces the cache-control with a long immutable TTL.
/// `content-encoding` and `content-length` are dropped so Cloudflare re-applies correct
/// encoding on egress after we modify the headers.
pub async fn rebuild_asset_response(mut resp: Response, path: &str) -> Result<Response> {
    let status = resp.status_code();
    let source: Vec<(String, String)> = resp.headers().entries().collect();
    let headers = Headers::new();
    for (k, v) in build_asset_headers(&source, path) {
        let _ = headers.set(&k, &v);
    }
    let bytes = resp.bytes().await.unwrap_or_default();
    Ok(Response::from_bytes(bytes)?
        .with_headers(headers)
        .with_status(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    fn find<'a>(headers: &'a [(String, String)], key: &str) -> Option<&'a str> {
        headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == key.to_lowercase())
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn test_build_asset_headers_drops_content_encoding() {
        let source = vec![
            h("content-type", "text/html"),
            h("content-encoding", "br"),
            h("content-length", "1234"),
        ];
        let out = build_asset_headers(&source, "/index.html");
        assert!(find(&out, "content-encoding").is_none());
        assert!(find(&out, "content-length").is_none());
        assert_eq!(find(&out, "content-type"), Some("text/html"));
    }

    #[test]
    fn test_build_asset_headers_immutable_path_sets_long_cache() {
        let source = vec![
            h("content-type", "application/javascript"),
            h("cache-control", "public, max-age=0, must-revalidate"),
        ];
        let out = build_asset_headers(&source, "/_app/immutable/chunks/foo.js");
        assert_eq!(
            find(&out, "cache-control"),
            Some("public, max-age=31536000, immutable")
        );
    }

    #[test]
    fn test_build_asset_headers_non_immutable_path_keeps_original_cache() {
        let source = vec![
            h("content-type", "text/html"),
            h("cache-control", "public, max-age=0, must-revalidate"),
        ];
        let out = build_asset_headers(&source, "/fallback.html");
        assert_eq!(
            find(&out, "cache-control"),
            Some("public, max-age=0, must-revalidate")
        );
    }

    #[test]
    fn test_build_asset_headers_immutable_only_one_cache_control() {
        let source = vec![h("cache-control", "public, max-age=0, must-revalidate")];
        let out = build_asset_headers(&source, "/_app/immutable/entry/start.js");
        let cc_values: Vec<&str> = out
            .iter()
            .filter(|(k, _)| k.to_lowercase() == "cache-control")
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(cc_values.len(), 1);
        assert_eq!(cc_values[0], "public, max-age=31536000, immutable");
    }

    #[test]
    fn test_build_asset_headers_preserves_other_headers() {
        let source = vec![
            h("content-type", "text/css"),
            h("etag", "\"abc123\""),
            h("vary", "Accept-Encoding"),
        ];
        let out = build_asset_headers(&source, "/_app/immutable/assets/style.css");
        assert_eq!(find(&out, "content-type"), Some("text/css"));
        assert_eq!(find(&out, "etag"), Some("\"abc123\""));
        assert_eq!(find(&out, "vary"), Some("Accept-Encoding"));
    }

    #[test]
    fn test_build_asset_headers_case_insensitive_drops() {
        let source = vec![
            h("Content-Encoding", "gzip"),
            h("Content-Length", "999"),
            h("content-type", "image/png"),
        ];
        let out = build_asset_headers(&source, "/favicon.ico");
        assert!(find(&out, "content-encoding").is_none());
        assert!(find(&out, "content-length").is_none());
        assert_eq!(find(&out, "content-type"), Some("image/png"));
    }
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
