/// Cryptographic utilities for security-critical operations
/// Constant-time string comparison to prevent timing attacks
///
/// This function compares two strings in constant time to prevent timing attacks.
/// Traditional string comparison (==) can leak information about where strings differ
/// based on how long the comparison takes.
///
/// Use this for comparing:
/// - Session IDs
/// - User IDs
/// - OAuth states
/// - API keys
/// - Any other security-critical identifiers
///
/// # Security
///
/// This implementation:
/// 1. Immediately returns false if lengths differ (length is not secret)
/// 2. XORs all bytes and accumulates the result
/// 3. Always processes all bytes regardless of differences found
/// 4. Returns true only if all bytes matched (result == 0)
///
/// # Example
///
/// ```rust
/// use crate::utils::crypto::secure_compare;
///
/// let user_session_id = "abc123";
/// let claimed_session_id = "abc123";
///
/// if secure_compare(user_session_id, claimed_session_id) {
///     // Session IDs match - authenticated
/// } else {
///     // Session IDs don't match - unauthorized
/// }
/// ```
pub fn secure_compare(a: &str, b: &str) -> bool {
    // Length comparison is not secret - timing attacks on length are acceptable
    // This prevents unnecessary byte-by-byte comparison when lengths differ
    if a.len() != b.len() {
        return false;
    }

    // Constant-time byte comparison using XOR accumulation
    // The compiler should not optimize this loop away since result is used
    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }

    // Return true only if all bytes matched (no differences found)
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_compare_equal() {
        assert!(secure_compare("abc123", "abc123"));
        assert!(secure_compare("", ""));
        assert!(secure_compare("session-id-12345678", "session-id-12345678"));
    }

    #[test]
    fn test_secure_compare_not_equal() {
        assert!(!secure_compare("abc123", "abc124"));
        assert!(!secure_compare("abc123", "abc12"));
        assert!(!secure_compare("", "a"));
        assert!(!secure_compare("user1", "user2"));
    }

    #[test]
    fn test_secure_compare_different_lengths() {
        assert!(!secure_compare("short", "longer"));
        assert!(!secure_compare("abc", "abcd"));
        assert!(!secure_compare("abcdef", "abc"));
    }

    #[test]
    fn test_secure_compare_unicode() {
        assert!(secure_compare("hello🌟", "hello🌟"));
        assert!(!secure_compare("hello🌟", "hello⭐"));
    }

    #[test]
    fn test_secure_compare_special_chars() {
        assert!(secure_compare("a!@#$%^&*()", "a!@#$%^&*()"));
        assert!(!secure_compare("a!@#$%^&*()", "b!@#$%^&*()"));
    }
}
