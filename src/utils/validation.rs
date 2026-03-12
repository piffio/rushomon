use url::Url;

/// Reserved short codes that cannot be used (prevent conflicts with routes)
const RESERVED_CODES: &[&str] = &[
    "api",
    "auth",
    "login",
    "logout",
    "dashboard",
    "admin",
    "static",
    "assets",
    "docs",
    "about",
    "help",
    "terms",
    "privacy",
    // Brand and navigation codes
    "rushomon",
    "download",
    "app",
    "go",
    "get",
    "blog",
    "pricing",
    "settings",
    "support",
    "contact",
    "link",
    "links",
    "signup",
    "register",
    "news",
    "team",
    "careers",
    "home",
    "index",
];

/// Validate a destination URL
/// Must be http or https scheme
pub fn validate_url(url_str: &str) -> Result<String, String> {
    match Url::parse(url_str) {
        Ok(url) => {
            let scheme = url.scheme();
            if scheme != "http" && scheme != "https" {
                return Err(format!(
                    "Invalid URL scheme: {}. Only http and https are allowed",
                    scheme
                ));
            }
            Ok(url.to_string())
        }
        Err(e) => Err(format!("Invalid URL: {}", e)),
    }
}

/// Validate a custom short code
/// Rules:
/// - 3-100 characters long
/// - Alphanumeric, hyphens, and forward slashes (a-z, A-Z, 0-9, -, /)
/// - Cannot start or end with hyphen or forward slash
/// - Cannot contain consecutive forward slashes
/// - Max 3 segments separated by slashes
/// - Each segment 1-50 characters
/// - No segment can be a reserved word
pub fn validate_short_code(code: &str) -> Result<(), String> {
    // Check length
    if code.len() < 3 || code.len() > 100 {
        return Err("Short code must be 3-100 characters long".to_string());
    }

    // Check charset: alphanumeric, hyphens, and forward slashes only
    if !code
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '/')
    {
        return Err(
            "Short code must contain only letters, numbers, hyphens, and forward slashes"
                .to_string(),
        );
    }

    // No leading or trailing hyphens or slashes
    if code.starts_with('-') || code.ends_with('-') {
        return Err("Short code cannot start or end with a hyphen".to_string());
    }
    if code.starts_with('/') || code.ends_with('/') {
        return Err("Short code cannot start or end with a forward slash".to_string());
    }

    // No consecutive forward slashes
    if code.contains("//") {
        return Err("Short code cannot contain consecutive forward slashes".to_string());
    }

    // Segment validation
    let segments: Vec<&str> = code.split('/').collect();
    if segments.len() > 3 {
        return Err("Short code can have at most 3 segments separated by slashes".to_string());
    }

    for segment in segments {
        if segment.is_empty() || segment.len() > 50 {
            return Err("Each segment must be 1-50 characters long".to_string());
        }
        if segment.starts_with('-') || segment.ends_with('-') {
            return Err("Segment cannot start or end with a hyphen".to_string());
        }
        if RESERVED_CODES.contains(&segment.to_lowercase().as_str()) {
            return Err(format!("Segment '{}' is reserved", segment));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // URL Validation Tests
    #[test]
    fn test_validate_url_accepts_https() {
        assert!(validate_url("https://example.com").is_ok());
    }

    #[test]
    fn test_validate_url_accepts_http() {
        assert!(validate_url("http://example.com").is_ok());
    }

    #[test]
    fn test_validate_url_accepts_with_path() {
        assert!(validate_url("https://example.com/path/to/page").is_ok());
    }

    #[test]
    fn test_validate_url_accepts_with_query() {
        assert!(validate_url("https://example.com?foo=bar&baz=qux").is_ok());
    }

    #[test]
    fn test_validate_url_accepts_with_fragment() {
        assert!(validate_url("https://example.com/page#section").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_javascript() {
        assert!(validate_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn test_validate_url_rejects_file() {
        assert!(validate_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn test_validate_url_rejects_data() {
        assert!(validate_url("data:text/html,<script>alert(1)</script>").is_err());
    }

    #[test]
    fn test_validate_url_rejects_ftp() {
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_validate_url_rejects_malformed() {
        assert!(validate_url("not a url").is_err());
        assert!(validate_url("").is_err());
        assert!(validate_url("htp://example.com").is_err()); // typo in scheme
    }

    // Short Code Validation Tests
    #[test]
    fn test_validate_short_code_accepts_alphanumeric() {
        assert!(validate_short_code("abc123").is_ok());
        assert!(validate_short_code("ABC123").is_ok());
        assert!(validate_short_code("aBc123").is_ok());
    }

    #[test]
    fn test_validate_short_code_accepts_valid_length() {
        assert!(validate_short_code("abc").is_ok()); // 3 chars - minimum
        // 100 chars - maximum, split into valid segments
        assert!(validate_short_code(&format!("{}/{}", "a".repeat(49), "b".repeat(49))).is_ok());
    }

    #[test]
    fn test_validate_short_code_accepts_only_letters() {
        assert!(validate_short_code("abc").is_ok());
        assert!(validate_short_code("ABCD").is_ok());
    }

    #[test]
    fn test_validate_short_code_accepts_only_numbers() {
        assert!(validate_short_code("1234").is_ok());
        assert!(validate_short_code("567890").is_ok());
    }

    #[test]
    fn test_validate_short_code_rejects_too_short() {
        assert!(validate_short_code("ab").is_err()); // 2 chars
        assert!(validate_short_code("a").is_err());
        assert!(validate_short_code("").is_err());
    }

    #[test]
    fn test_validate_short_code_rejects_too_long() {
        assert!(validate_short_code(&"a".repeat(101)).is_err()); // 101 chars
        assert!(validate_short_code(&"a".repeat(200)).is_err());
    }

    #[test]
    fn test_validate_short_code_rejects_special_chars() {
        assert!(validate_short_code("abc_123").is_err()); // underscore
        assert!(validate_short_code("abc.123").is_err()); // period
        assert!(validate_short_code("abc 123").is_err()); // space
        assert!(validate_short_code("abc@123").is_err()); // special char
    }

    #[test]
    fn test_validate_short_code_accepts_hyphens() {
        assert!(validate_short_code("abc-123").is_ok());
        assert!(validate_short_code("my-link").is_ok());
        assert!(validate_short_code("ciccio-veEIoZ6").is_ok());
        assert!(validate_short_code("a-b-c").is_ok());
    }

    #[test]
    fn test_validate_short_code_rejects_leading_trailing_hyphen() {
        assert!(validate_short_code("-abc").is_err()); // leading hyphen
        assert!(validate_short_code("abc-").is_err()); // trailing hyphen
        assert!(validate_short_code("-abc-").is_err()); // both
    }

    #[test]
    fn test_validate_short_code_rejects_reserved_words() {
        assert!(validate_short_code("api").is_err());
        assert!(validate_short_code("API").is_err()); // case insensitive
        assert!(validate_short_code("auth").is_err());
        assert!(validate_short_code("login").is_err());
        assert!(validate_short_code("logout").is_err());
        assert!(validate_short_code("dashboard").is_err());
        assert!(validate_short_code("admin").is_err());
        assert!(validate_short_code("static").is_err());
        assert!(validate_short_code("assets").is_err());
        assert!(validate_short_code("docs").is_err());
        assert!(validate_short_code("about").is_err());
        assert!(validate_short_code("help").is_err());
    }

    #[test]
    fn test_validate_short_code_reserved_case_insensitive() {
        // Test that reserved words are blocked regardless of case
        assert!(validate_short_code("Api").is_err());
        assert!(validate_short_code("aDmIn").is_err());
        assert!(validate_short_code("LOGIN").is_err());
    }

    #[test]
    fn test_validate_short_code_accepts_slashes() {
        assert!(validate_short_code("abc/123").is_ok());
        assert!(validate_short_code("company/promo").is_ok());
        assert!(validate_short_code("acme/corp/2024").is_ok());
        assert!(validate_short_code("my-link/promo-code").is_ok());
    }

    #[test]
    fn test_validate_short_code_rejects_leading_trailing_slash() {
        assert!(validate_short_code("/abc").is_err()); // leading slash
        assert!(validate_short_code("abc/").is_err()); // trailing slash
        assert!(validate_short_code("/abc/").is_err()); // both
    }

    #[test]
    fn test_validate_short_code_rejects_consecutive_slashes() {
        assert!(validate_short_code("abc//123").is_err()); // consecutive slashes
        assert!(validate_short_code("a//b//c").is_err()); // multiple consecutive
    }

    #[test]
    fn test_validate_short_code_rejects_too_many_segments() {
        assert!(validate_short_code("a/b/c/d").is_err()); // 4 segments
        assert!(validate_short_code("one/two/three/four").is_err()); // 4 segments with names
    }

    #[test]
    fn test_validate_short_code_rejects_invalid_segments() {
        assert!(validate_short_code("a//b").is_err()); // empty segment
        assert!(validate_short_code("abc/").is_err()); // empty trailing segment
        assert!(validate_short_code("/abc").is_err()); // empty leading segment
    }

    #[test]
    fn test_validate_short_code_rejects_segment_too_long() {
        let long_segment = "a".repeat(51); // 51 chars
        assert!(validate_short_code(&format!("{}/short", long_segment)).is_err());
        assert!(validate_short_code(&format!("short/{}", long_segment)).is_err());
    }

    #[test]
    fn test_validate_short_code_rejects_segment_with_leading_trailing_hyphen() {
        assert!(validate_short_code("-abc/123").is_err()); // leading hyphen in segment
        assert!(validate_short_code("abc-/123").is_err()); // trailing hyphen in segment
        assert!(validate_short_code("123/-abc").is_err()); // leading hyphen in second segment
        assert!(validate_short_code("123/abc-").is_err()); // trailing hyphen in second segment
    }

    #[test]
    fn test_validate_short_code_rejects_reserved_segments() {
        assert!(validate_short_code("api/promo").is_err()); // api is reserved
        assert!(validate_short_code("company/auth").is_err()); // auth is reserved
        assert!(validate_short_code("dashboard/2024").is_err()); // dashboard is reserved
        assert!(validate_short_code("promo/admin").is_err()); // admin is reserved
        assert!(validate_short_code("my-api/2024").is_ok()); // my-api is not reserved
    }

    #[test]
    fn test_validate_short_code_accepts_valid_segments_with_reserved_words() {
        // Should accept when reserved words are part of longer segments
        assert!(validate_short_code("apiary/promo").is_ok());
        assert!(validate_short_code("authenticate/2024").is_ok());
        assert!(validate_short_code("dashboarding/abc").is_ok());
    }
}
