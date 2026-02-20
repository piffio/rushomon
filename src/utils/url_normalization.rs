use url::Url;

/// Normalize a URL for blacklist comparison
/// This function applies various normalization techniques to ensure that
/// URLs that point to the same content are normalized to the same form
pub fn normalize_url_for_blacklist(input_url: &str) -> Result<String, String> {
    let mut url = Url::parse(input_url).map_err(|e| format!("Failed to parse URL: {}", e))?;

    // 1. Convert scheme and host to lowercase (RFC 3986)
    let scheme = url.scheme().to_lowercase();
    let _ = url.set_scheme(&scheme);

    if let Some(host) = url.host_str() {
        let _ = url.set_host(Some(&host.to_lowercase()));
    }

    // 2. Remove default ports (80 for http, 443 for https)
    match url.scheme() {
        "http" if url.port_or_known_default() == Some(80) => {
            let _ = url.set_port(None);
        }
        "https" if url.port_or_known_default() == Some(443) => {
            let _ = url.set_port(None);
        }
        _ => {}
    }

    // 3. Handle www prefix - normalize to non-www form
    if let Some(host) = url.host_str() {
        let host_str = host.to_string();
        if let Some(non_www_host) = host_str.strip_prefix("www.") {
            let _ = url.set_host(Some(non_www_host));
        }
    }

    // 4. Normalize path
    let mut path = url.path().to_string();

    // Remove trailing slash for non-root paths (preserves semantics)
    if path.len() > 1 && path.ends_with('/') {
        path.pop();
    }

    // Add leading slash for empty paths (when authority is present)
    if path.is_empty() && !url.authority().is_empty() {
        path = "/".to_string();
    }

    // Remove dot segments (., ..) - the url crate does this automatically
    // but we ensure it's properly normalized
    url.set_path(&path);

    // 5. Remove fragment (never seen by server)
    url.set_fragment(None);

    // 6. Sort query parameters alphabetically
    if let Some(query) = url.query()
        && !query.is_empty()
    {
        let mut params: Vec<_> = query
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                match (parts.next(), parts.next()) {
                    (Some(key), Some(value)) => Some((key, value)),
                    _ => None,
                }
            })
            .collect();

        // Sort by key, then by value
        params.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(b.1)));

        // Rebuild query string
        let normalized_query = params
            .into_iter()
            .map(|(key, value)| {
                if value.is_empty() {
                    key.to_string()
                } else {
                    format!("{}={}", key, value)
                }
            })
            .collect::<Vec<_>>()
            .join("&");

        url.set_query(Some(&normalized_query));
    }

    // 7. Remove empty query (when query is just "?")
    if url.query().is_some_and(|q| q.is_empty()) {
        url.set_query(None);
    }

    Ok(url.to_string())
}

/// Check if two URLs are effectively the same after normalization
pub fn are_urls_effectively_same(url1: &str, url2: &str) -> bool {
    match (
        normalize_url_for_blacklist(url1),
        normalize_url_for_blacklist(url2),
    ) {
        (Ok(norm1), Ok(norm2)) => norm1 == norm2,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trailing_slash_normalization() {
        let cases = vec![
            ("http://example.com", "http://example.com/"),
            ("https://example.com/path", "https://example.com/path/"),
            (
                "http://example.com/path/to/resource",
                "http://example.com/path/to/resource/",
            ),
        ];

        for (url1, url2) in cases {
            let norm1 = normalize_url_for_blacklist(url1).unwrap();
            let norm2 = normalize_url_for_blacklist(url2).unwrap();
            assert_eq!(
                norm1, norm2,
                "URLs should normalize to same: {} vs {}",
                url1, url2
            );
        }
    }

    #[test]
    fn test_www_normalization() {
        let cases = vec![
            ("http://www.example.com", "http://example.com"),
            ("https://www.example.com/path", "https://example.com/path"),
            (
                "http://www.example.com/path?param=value",
                "http://example.com/path?param=value",
            ),
        ];

        for (url1, url2) in cases {
            let norm1 = normalize_url_for_blacklist(url1).unwrap();
            let norm2 = normalize_url_for_blacklist(url2).unwrap();
            assert_eq!(
                norm1, norm2,
                "WWW URLs should normalize to same: {} vs {}",
                url1, url2
            );
        }
    }

    #[test]
    fn test_default_port_removal() {
        let cases = vec![
            ("http://example.com:80", "http://example.com"),
            ("https://example.com:443", "https://example.com"),
            ("http://example.com:80/path", "http://example.com/path"),
        ];

        for (url1, url2) in cases {
            let norm1 = normalize_url_for_blacklist(url1).unwrap();
            let norm2 = normalize_url_for_blacklist(url2).unwrap();
            assert_eq!(
                norm1, norm2,
                "Default port URLs should normalize to same: {} vs {}",
                url1, url2
            );
        }
    }

    #[test]
    fn test_case_normalization() {
        let cases = vec![
            ("HTTP://example.com", "http://example.com"),
            ("HTTPS://EXAMPLE.COM/PATH", "https://example.com/PATH"),
            ("http://EXAMPLE.COM", "http://example.com"),
        ];

        for (url1, url2) in cases {
            let norm1 = normalize_url_for_blacklist(url1).unwrap();
            let norm2 = normalize_url_for_blacklist(url2).unwrap();
            assert_eq!(
                norm1, norm2,
                "Case should be normalized: {} vs {}",
                url1, url2
            );
        }
    }

    #[test]
    fn test_query_parameter_sorting() {
        let cases = vec![
            ("http://example.com?b=2&a=1", "http://example.com?a=1&b=2"),
            (
                "http://example.com?z=last&first=1&middle=2",
                "http://example.com?first=1&middle=2&z=last",
            ),
        ];

        for (url1, url2) in cases {
            let norm1 = normalize_url_for_blacklist(url1).unwrap();
            let norm2 = normalize_url_for_blacklist(url2).unwrap();
            assert_eq!(
                norm1, norm2,
                "Query params should be sorted: {} vs {}",
                url1, url2
            );
        }
    }

    #[test]
    fn test_fragment_removal() {
        let cases = vec![
            ("http://example.com#section", "http://example.com"),
            (
                "http://example.com/path#fragment",
                "http://example.com/path",
            ),
            (
                "http://example.com?param=value#anchor",
                "http://example.com?param=value",
            ),
        ];

        for (url1, url2) in cases {
            let norm1 = normalize_url_for_blacklist(url1).unwrap();
            let norm2 = normalize_url_for_blacklist(url2).unwrap();
            assert_eq!(
                norm1, norm2,
                "Fragments should be removed: {} vs {}",
                url1, url2
            );
        }
    }

    #[test]
    fn test_complex_normalization() {
        let url1 = "HTTP://WWW.Example.COM:80/path/?b=2&a=1#section";
        let url2 = "http://example.com/path/?a=1&b=2";

        let norm1 = normalize_url_for_blacklist(url1).unwrap();
        let norm2 = normalize_url_for_blacklist(url2).unwrap();
        assert_eq!(
            norm1, norm2,
            "Complex normalization should work: {} vs {}",
            url1, url2
        );
    }

    #[test]
    fn test_are_urls_effectively_same() {
        assert!(are_urls_effectively_same(
            "http://example.com",
            "http://example.com/"
        ));
        assert!(are_urls_effectively_same(
            "http://www.example.com",
            "http://example.com"
        ));
        assert!(are_urls_effectively_same(
            "http://example.com:80",
            "http://example.com"
        ));
        assert!(are_urls_effectively_same(
            "http://example.com?a=1&b=2",
            "http://example.com?b=2&a=1"
        ));
        assert!(!are_urls_effectively_same(
            "http://example.com",
            "http://different.com"
        ));
    }
}
