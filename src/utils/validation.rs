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
/// - 4-10 characters long
/// - Alphanumeric only (a-z, A-Z, 0-9)
/// - Not in reserved list
pub fn validate_short_code(code: &str) -> Result<(), String> {
    // Check length
    if code.len() < 4 || code.len() > 10 {
        return Err("Short code must be 4-10 characters long".to_string());
    }

    // Check alphanumeric
    if !code.chars().all(|c| c.is_alphanumeric()) {
        return Err("Short code must contain only letters and numbers".to_string());
    }

    // Check reserved words
    if RESERVED_CODES.contains(&code.to_lowercase().as_str()) {
        return Err(format!("Short code '{}' is reserved", code));
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
        assert!(validate_short_code("abcd").is_ok()); // 4 chars - minimum
        assert!(validate_short_code("abcdefghij").is_ok()); // 10 chars - maximum
    }

    #[test]
    fn test_validate_short_code_accepts_only_letters() {
        assert!(validate_short_code("abcd").is_ok());
        assert!(validate_short_code("ABCD").is_ok());
    }

    #[test]
    fn test_validate_short_code_accepts_only_numbers() {
        assert!(validate_short_code("1234").is_ok());
        assert!(validate_short_code("567890").is_ok());
    }

    #[test]
    fn test_validate_short_code_rejects_too_short() {
        assert!(validate_short_code("abc").is_err()); // 3 chars
        assert!(validate_short_code("ab").is_err());
        assert!(validate_short_code("a").is_err());
    }

    #[test]
    fn test_validate_short_code_rejects_too_long() {
        assert!(validate_short_code("abcdefghijk").is_err()); // 11 chars
        assert!(validate_short_code("verylongcode123").is_err());
    }

    #[test]
    fn test_validate_short_code_rejects_special_chars() {
        assert!(validate_short_code("abc-123").is_err()); // hyphen
        assert!(validate_short_code("abc_123").is_err()); // underscore
        assert!(validate_short_code("abc.123").is_err()); // period
        assert!(validate_short_code("abc 123").is_err()); // space
        assert!(validate_short_code("abc@123").is_err()); // special char
        assert!(validate_short_code("abc/123").is_err()); // slash
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
}
