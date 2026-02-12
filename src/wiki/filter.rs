use super::Page;

/// Filter pages based on include/exclude lists of slugs.
pub fn filter_pages(pages: Vec<Page>, include: &Option<Vec<String>>, exclude: &Option<Vec<String>>) -> Vec<Page> {
    pages
        .into_iter()
        .filter(|page| {
            // If include list is specified, page must match
            if let Some(include_slugs) = include {
                if !include_slugs.iter().any(|s| slug_matches(&page.slug, s)) {
                    return false;
                }
            }

            // If exclude list is specified, page must not match
            if let Some(exclude_slugs) = exclude {
                if exclude_slugs.iter().any(|s| slug_matches(&page.slug, s)) {
                    return false;
                }
            }

            true
        })
        .collect()
}

/// Check if a page slug matches a filter pattern.
/// Supports exact match and prefix match (e.g., "3" matches "3-architecture").
fn slug_matches(slug: &str, pattern: &str) -> bool {
    if slug == pattern {
        return true;
    }

    // Prefix match: pattern "3" matches slug "3-architecture"
    if slug.starts_with(pattern) {
        let rest = &slug[pattern.len()..];
        if rest.starts_with('-') || rest.starts_with('.') {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_page(slug: &str) -> Page {
        Page {
            slug: slug.to_string(),
            title: slug.to_string(),
            depth: 0,
            content: None,
            error: None,
        }
    }

    #[test]
    fn test_no_filters() {
        let pages = vec![make_page("a"), make_page("b")];
        let result = filter_pages(pages, &None, &None);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_include_filter() {
        let pages = vec![make_page("1-overview"), make_page("2-setup"), make_page("3-arch")];
        let include = Some(vec!["1-overview".to_string(), "3-arch".to_string()]);
        let result = filter_pages(pages, &include, &None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].slug, "1-overview");
        assert_eq!(result[1].slug, "3-arch");
    }

    #[test]
    fn test_exclude_filter() {
        let pages = vec![make_page("1-overview"), make_page("2-setup"), make_page("3-arch")];
        let exclude = Some(vec!["2-setup".to_string()]);
        let result = filter_pages(pages, &None, &exclude);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].slug, "1-overview");
        assert_eq!(result[1].slug, "3-arch");
    }

    #[test]
    fn test_slug_prefix_match() {
        assert!(slug_matches("3-architecture", "3"));
        assert!(slug_matches("3.1-data-pipeline", "3.1"));
        assert!(!slug_matches("3-architecture", "31"));
        assert!(!slug_matches("30-other", "3"));
    }

    #[test]
    fn test_slug_exact_match() {
        assert!(slug_matches("1-overview", "1-overview"));
        assert!(!slug_matches("1-overview", "1-over"));
    }

    #[test]
    fn test_include_and_exclude_combined() {
        let pages = vec![
            make_page("1-overview"),
            make_page("2-setup"),
            make_page("3-arch"),
            make_page("3.1-data"),
        ];
        let include = Some(vec![
            "1-overview".to_string(),
            "3-arch".to_string(),
            "3.1-data".to_string(),
        ]);
        let exclude = Some(vec!["3-arch".to_string()]);
        let result = filter_pages(pages, &include, &exclude);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].slug, "1-overview");
        assert_eq!(result[1].slug, "3.1-data");
    }
}
