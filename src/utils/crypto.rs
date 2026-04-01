/// Cryptographic utilities for security-critical operations
///
/// Verifies a Polar webhook signature (HMAC-SHA256).
///
/// Standard Webhooks signs: `<webhook-id>.<webhook-timestamp>.<body>`
/// The secret is base64-encoded (optionally prefixed with `whsec_`).
/// The `webhook-signature` header contains space-separated `v1,<base64>` signatures.
pub fn verify_polar_webhook_signature(
    body: &[u8],
    webhook_id: &str,
    webhook_timestamp: &str,
    signature_header: &str,
    secret: &str,
) -> Result<bool, String> {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    // Polar uses the secret as raw bytes, not base64-decoded
    // (despite Standard Webhooks spec saying secrets are base64-encoded)
    let secret_bytes = secret
        .strip_prefix("whsec_")
        .unwrap_or(secret)
        .as_bytes()
        .to_vec();

    // The signed content is: "<webhook-id>.<webhook-timestamp>.<body>"
    // Note: webhook-timestamp is already a unix integer string, used as-is
    let body_str = String::from_utf8_lossy(body);
    let to_sign = format!("{}.{}.{}", webhook_id, webhook_timestamp, body_str);

    let mut mac = Hmac::<Sha256>::new_from_slice(&secret_bytes)
        .map_err(|e| format!("Invalid HMAC key: {}", e))?;
    mac.update(to_sign.as_bytes());
    let computed_bytes = mac.finalize().into_bytes();

    // Encode our computed signature to base64 to compare as strings
    // (matches the reference JS implementation exactly)
    let computed_b64 = encode_base64(&computed_bytes);

    // The signature header is a space-separated list of "v1,<base64>" signatures
    use subtle::ConstantTimeEq;
    for sig_entry in signature_header.split(' ') {
        let sig_b64 = if let Some(b) = sig_entry.strip_prefix("v1,") {
            b
        } else {
            continue; // skip unknown versions (e.g. v1a for asymmetric)
        };

        // Compare base64 strings in constant time
        let matches: bool = computed_b64.as_bytes().ct_eq(sig_b64.as_bytes()).into();
        if matches {
            return Ok(true);
        }
    }

    Ok(false)
}

fn encode_base64(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() {
            bytes[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            bytes[i + 2] as u32
        } else {
            0
        };
        result.push(CHARS[((b0 >> 2) & 0x3f) as usize] as char);
        result.push(CHARS[(((b0 << 4) | (b1 >> 4)) & 0x3f) as usize] as char);
        result.push(if i + 1 < bytes.len() {
            CHARS[(((b1 << 2) | (b2 >> 6)) & 0x3f) as usize] as char
        } else {
            '='
        });
        result.push(if i + 2 < bytes.len() {
            CHARS[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
        i += 3;
    }
    result
}

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

    // ─── verify_polar_webhook_signature tests ────────────────────────────────

    /// Compute a valid Standard Webhooks HMAC-SHA256 signature for use in tests.
    /// Mirrors the production signing algorithm exactly.
    fn compute_test_signature(
        body: &[u8],
        webhook_id: &str,
        timestamp: &str,
        secret: &str,
    ) -> String {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;

        let secret_bytes = secret
            .strip_prefix("whsec_")
            .unwrap_or(secret)
            .as_bytes()
            .to_vec();

        let body_str = String::from_utf8_lossy(body);
        let to_sign = format!("{}.{}.{}", webhook_id, timestamp, body_str);

        let mut mac = Hmac::<Sha256>::new_from_slice(&secret_bytes).unwrap();
        mac.update(to_sign.as_bytes());
        let computed_bytes = mac.finalize().into_bytes();
        let b64 = encode_base64(&computed_bytes);
        format!("v1,{}", b64)
    }

    #[test]
    fn test_webhook_signature_valid() {
        let body = b"{\"type\":\"subscription.active\",\"data\":{}}";
        let id = "msg-test-001";
        let ts = "1700000000";
        let secret = "test-polar-webhook-secret";

        let sig = compute_test_signature(body, id, ts, secret);
        let result = verify_polar_webhook_signature(body, id, ts, &sig, secret);
        assert_eq!(result, Ok(true), "Valid signature should be accepted");
    }

    #[test]
    fn test_webhook_signature_wrong_secret() {
        let body = b"{\"type\":\"subscription.active\",\"data\":{}}";
        let id = "msg-test-002";
        let ts = "1700000000";

        let sig = compute_test_signature(body, id, ts, "correct-secret");
        let result = verify_polar_webhook_signature(body, id, ts, &sig, "wrong-secret");
        assert_eq!(
            result,
            Ok(false),
            "Signature computed with different secret should be rejected"
        );
    }

    #[test]
    fn test_webhook_signature_tampered_body() {
        let original_body = b"{\"type\":\"subscription.active\",\"data\":{}}";
        let tampered_body = b"{\"type\":\"subscription.active\",\"data\":{\"injected\":true}}";
        let id = "msg-test-003";
        let ts = "1700000000";
        let secret = "test-polar-webhook-secret";

        let sig = compute_test_signature(original_body, id, ts, secret);
        let result = verify_polar_webhook_signature(tampered_body, id, ts, &sig, secret);
        assert_eq!(
            result,
            Ok(false),
            "Signature for original body should not validate tampered body"
        );
    }

    #[test]
    fn test_webhook_signature_multiple_sigs_in_header_one_valid() {
        let body = b"{\"type\":\"subscription.active\",\"data\":{}}";
        let id = "msg-test-004";
        let ts = "1700000000";
        let secret = "test-polar-webhook-secret";

        let valid_sig = compute_test_signature(body, id, ts, secret);
        let header = format!("v1,invalidsignaturehere {}", valid_sig);
        let result = verify_polar_webhook_signature(body, id, ts, &header, secret);
        assert_eq!(
            result,
            Ok(true),
            "At least one valid signature in the list should be accepted"
        );
    }

    #[test]
    fn test_webhook_signature_multiple_sigs_all_invalid() {
        let body = b"{\"type\":\"subscription.active\",\"data\":{}}";
        let id = "msg-test-005";
        let ts = "1700000000";
        let secret = "test-polar-webhook-secret";

        let header = "v1,invalidsig1 v1,invalidsig2";
        let result = verify_polar_webhook_signature(body, id, ts, header, secret);
        assert_eq!(
            result,
            Ok(false),
            "All invalid signatures should be rejected"
        );
    }

    #[test]
    fn test_webhook_signature_ignores_unknown_version() {
        let body = b"{\"type\":\"subscription.active\",\"data\":{}}";
        let id = "msg-test-006";
        let ts = "1700000000";
        let secret = "test-polar-webhook-secret";

        let valid_sig = compute_test_signature(body, id, ts, secret);
        // Prepend an unknown version entry before the valid v1 entry
        let header = format!("v2,someasymmetrickey {}", valid_sig);
        let result = verify_polar_webhook_signature(body, id, ts, &header, secret);
        assert_eq!(
            result,
            Ok(true),
            "Unknown version prefix should be ignored; valid v1 entry accepted"
        );
    }

    #[test]
    fn test_webhook_signature_whsec_prefix_stripped() {
        let body = b"{\"type\":\"subscription.active\",\"data\":{}}";
        let id = "msg-test-007";
        let ts = "1700000000";
        let raw_secret = "test-polar-webhook-secret";
        let prefixed_secret = format!("whsec_{}", raw_secret);

        // Sign with the raw secret (no prefix) — should match verification with whsec_ prefix
        let sig = compute_test_signature(body, id, ts, raw_secret);
        let result = verify_polar_webhook_signature(body, id, ts, &sig, &prefixed_secret);
        assert_eq!(
            result,
            Ok(true),
            "whsec_ prefix should be stripped before HMAC computation"
        );
    }

    #[test]
    fn test_webhook_signature_empty_body() {
        let body = b"";
        let id = "msg-test-008";
        let ts = "1700000000";
        let secret = "test-polar-webhook-secret";

        let sig = compute_test_signature(body, id, ts, secret);
        let result = verify_polar_webhook_signature(body, id, ts, &sig, secret);
        assert_eq!(
            result,
            Ok(true),
            "Empty body should still produce a valid, verifiable HMAC"
        );
    }

    // ─── secure_compare tests ─────────────────────────────────────────────────

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
