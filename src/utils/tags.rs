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
