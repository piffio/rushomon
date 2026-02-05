use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub created_at: i64,
    pub created_by: String,
}

impl Organization {
    pub fn validate_slug(slug: &str) -> bool {
        // URL-safe: alphanumeric and hyphens only, 3-50 chars
        if slug.len() < 3 || slug.len() > 50 {
            return false;
        }
        slug.chars().all(|c| c.is_alphanumeric() || c == '-')
    }
}

// Reserved for future organization creation API
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateOrganizationData {
    pub name: String,
    pub slug: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_slug_accepts_valid_slugs() {
        assert!(Organization::validate_slug("my-org"));
        assert!(Organization::validate_slug("test123"));
        assert!(Organization::validate_slug("org-name-123"));
    }

    #[test]
    fn test_validate_slug_accepts_minimum_length() {
        assert!(Organization::validate_slug("abc")); // 3 chars - minimum
    }

    #[test]
    fn test_validate_slug_accepts_maximum_length() {
        let slug_50_chars = "a".repeat(50);
        assert!(Organization::validate_slug(&slug_50_chars));
    }

    #[test]
    fn test_validate_slug_accepts_only_lowercase() {
        assert!(Organization::validate_slug("myorg"));
    }

    #[test]
    fn test_validate_slug_accepts_only_uppercase() {
        assert!(Organization::validate_slug("MYORG"));
    }

    #[test]
    fn test_validate_slug_accepts_mixed_case() {
        assert!(Organization::validate_slug("MyOrg"));
        assert!(Organization::validate_slug("mYoRg"));
    }

    #[test]
    fn test_validate_slug_accepts_numbers() {
        assert!(Organization::validate_slug("org123"));
        assert!(Organization::validate_slug("123org"));
    }

    #[test]
    fn test_validate_slug_rejects_too_short() {
        assert!(!Organization::validate_slug("ab")); // 2 chars
        assert!(!Organization::validate_slug("a"));
        assert!(!Organization::validate_slug(""));
    }

    #[test]
    fn test_validate_slug_rejects_too_long() {
        let slug_51_chars = "a".repeat(51);
        assert!(!Organization::validate_slug(&slug_51_chars));
    }

    #[test]
    fn test_validate_slug_rejects_special_chars() {
        assert!(!Organization::validate_slug("my_org")); // underscore
        assert!(!Organization::validate_slug("my.org")); // period
        assert!(!Organization::validate_slug("my org")); // space
        assert!(!Organization::validate_slug("my@org")); // special char
        assert!(!Organization::validate_slug("my/org")); // slash
        assert!(!Organization::validate_slug("my\\org")); // backslash
    }

    #[test]
    fn test_validate_slug_accepts_hyphens() {
        assert!(Organization::validate_slug("my-org")); // hyphen is allowed
        assert!(Organization::validate_slug("my-org-name"));
        assert!(Organization::validate_slug("org-123-test"));
    }

    #[test]
    fn test_validate_slug_only_alphanumeric_and_hyphens() {
        assert!(Organization::validate_slug("test-org-123"));
        assert!(Organization::validate_slug("ABC-DEF-789"));
    }

    #[test]
    fn test_validate_slug_multiple_hyphens() {
        assert!(Organization::validate_slug("my-org-name"));
        assert!(Organization::validate_slug("a-b-c-d"));
    }
}
