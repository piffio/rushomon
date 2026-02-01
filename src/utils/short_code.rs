use rand::Rng;

const BASE62_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const SHORT_CODE_LENGTH: usize = 6;

/// Generate a random 6-character base62 short code
/// Character set: 0-9, A-Z, a-z (62 chars)
/// Combinations: 62^6 = 56,800,235,584 (56.8 billion)
/// Collision probability: < 0.01% at 1M links
pub fn generate_short_code() -> String {
    let mut rng = rand::rng();
    (0..SHORT_CODE_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..BASE62_CHARS.len());
            BASE62_CHARS[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_short_code_returns_correct_length() {
        let code = generate_short_code();
        assert_eq!(code.len(), SHORT_CODE_LENGTH);
    }

    #[test]
    fn test_generate_short_code_only_alphanumeric() {
        let code = generate_short_code();
        assert!(code.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_short_code_uses_base62_charset() {
        let code = generate_short_code();
        for c in code.chars() {
            assert!(
                c.is_ascii_digit() || c.is_ascii_uppercase() || c.is_ascii_lowercase(),
                "Character {} not in base62 charset", c
            );
        }
    }

    #[test]
    fn test_generate_short_code_uniqueness() {
        // Generate 100 codes and ensure they're all different
        let mut codes = std::collections::HashSet::new();
        for _ in 0..100 {
            let code = generate_short_code();
            codes.insert(code);
        }
        // Very high probability all 100 are unique with 56.8B combinations
        assert_eq!(codes.len(), 100);
    }

    #[test]
    fn test_generate_short_code_not_empty() {
        let code = generate_short_code();
        assert!(!code.is_empty());
    }

    #[test]
    fn test_generate_short_code_multiple_calls_different() {
        let code1 = generate_short_code();
        let code2 = generate_short_code();
        let code3 = generate_short_code();
        // While not guaranteed, extremely unlikely to be equal
        assert_ne!(code1, code2);
        assert_ne!(code2, code3);
        assert_ne!(code1, code3);
    }
}
