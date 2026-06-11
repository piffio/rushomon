/// Tag validation and normalization utilities
use crate::utils::normalize_tag;
use worker::Result;

/// Validate and normalize a list of tags. Returns an error if any tag is invalid.
pub fn validate_and_normalize_tags(tags: &[String]) -> Result<Vec<String>> {
    if tags.len() > 20 {
        return Err(worker::Error::RustError(
            "Maximum 20 tags per link".to_string(),
        ));
    }
    let mut normalized = Vec::with_capacity(tags.len());
    for tag in tags {
        match normalize_tag(tag) {
            Some(t) => normalized.push(t),
            None => {
                return Err(worker::Error::RustError(format!(
                    "Invalid tag: '{}'. Tags must be non-empty and at most 50 characters.",
                    tag
                )));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    normalized.retain(|t| seen.insert(t.to_lowercase()));
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_and_normalize_tags_enforces_max_20() {
        let twenty_one: Vec<String> = (0..21).map(|i| format!("tag{}", i)).collect();
        let result = validate_and_normalize_tags(&twenty_one);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum 20 tags"));
    }

    #[test]
    fn test_validate_and_normalize_tags_accepts_max_20() {
        let twenty: Vec<String> = (0..20).map(|i| format!("tag{}", i)).collect();
        let result = validate_and_normalize_tags(&twenty);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 20);
    }

    #[test]
    fn test_validate_and_normalize_tags_deduplicates() {
        let tags = vec![
            "hello".to_string(),
            "Hello".to_string(),
            "HELLO".to_string(),
            "world".to_string(),
        ];
        let result = validate_and_normalize_tags(&tags).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"hello".to_string()));
        assert!(result.contains(&"world".to_string()));
    }

    #[test]
    fn test_validate_and_normalize_tags_returns_error_on_invalid() {
        let tags = vec!["valid".to_string(), "".to_string()];
        let result = validate_and_normalize_tags(&tags);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid tag"));
    }

    #[test]
    fn test_validate_and_normalize_tags_returns_error_on_too_long() {
        let long_tag = "a".repeat(51);
        let tags = vec![long_tag];
        let result = validate_and_normalize_tags(&tags);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid tag"));
    }

    #[test]
    fn test_validate_and_normalize_tags_preserves_order() {
        let tags = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let result = validate_and_normalize_tags(&tags).unwrap();
        assert_eq!(result, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_validate_and_normalize_tags_preserves_first_occurrence() {
        // When deduplicating, first occurrence should be kept
        let tags = vec![
            "CamelCase".to_string(),
            "camelcase".to_string(),
            "CAMELCASE".to_string(),
        ];
        let result = validate_and_normalize_tags(&tags).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "CamelCase"); // First occurrence preserved
    }

    #[test]
    fn test_validate_and_normalize_tags_normalizes_whitespace() {
        let tags = vec!["  hello   world  ".to_string(), "normal".to_string()];
        let result = validate_and_normalize_tags(&tags).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "hello world");
        assert_eq!(result[1], "normal");
    }

    #[test]
    fn test_validate_and_normalize_tags_empty_list() {
        let tags: Vec<String> = vec![];
        let result = validate_and_normalize_tags(&tags).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_validate_and_normalize_tags_single_tag() {
        let tags = vec!["single".to_string()];
        let result = validate_and_normalize_tags(&tags).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "single");
    }
}
