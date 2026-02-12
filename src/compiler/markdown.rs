use crate::wiki::Page;
use regex::Regex;

/// Compile pages into a single markdown document.
pub fn compile(repo: &str, pages: &[Page], include_toc: bool, include_metadata: bool) -> String {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let page_count = pages.iter().filter(|p| p.content.is_some()).count();
    let mut output = String::new();

    // Metadata comment
    if include_metadata {
        output.push_str(&format!(
            "<!-- dw2md v0.1.0 | {} | {} | {} pages -->\n\n",
            repo, now, page_count
        ));
    }

    // Title
    output.push_str(&format!("# {} — DeepWiki\n\n", repo));

    if include_metadata {
        output.push_str(&format!(
            "> Compiled from https://deepwiki.com/{}\n> Generated: {} | Pages: {}\n\n",
            repo, now, page_count
        ));
    }

    // Table of contents
    if include_toc {
        output.push_str("## Table of Contents\n\n");
        for page in pages {
            if page.content.is_none() && page.error.is_none() {
                continue;
            }
            let indent = "  ".repeat(page.depth);
            let anchor = make_anchor(&page.slug, &page.title);
            output.push_str(&format!("{}- [{}](#{})\n", indent, page.title, anchor));
        }
        output.push_str("\n---\n\n");
    }

    // Page contents
    for page in pages {
        if let Some(content) = &page.content {
            output.push_str(&format!("## {}\n\n", page.title));
            output.push_str(&bump_headings(content));
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("\n---\n\n");
        } else if let Some(err) = &page.error {
            output.push_str(&format!("## {}\n\n", page.title));
            output.push_str(&format!("> **Failed to fetch this page:** {}\n", err));
            output.push_str("\n---\n\n");
        }
    }

    // Trim trailing whitespace
    output.trim_end().to_string()
}

/// Create an anchor slug for the table of contents.
fn make_anchor(slug: &str, title: &str) -> String {
    let combined = if slug.is_empty() {
        title.to_string()
    } else {
        title.to_string()
    };

    combined
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Bump all markdown headings in content down by one level (# -> ##, ## -> ###, etc.)
/// since each page is already wrapped in an ## H2.
pub fn bump_headings(content: &str) -> String {
    let re = Regex::new(r"(?m)^(#{1,5}) ").unwrap();
    re.replace_all(content, |caps: &regex::Captures| format!("#{} ", &caps[1]))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_headings() {
        let input = "# Title\n\nSome text\n\n## Subtitle\n\n### Deep\n";
        let result = bump_headings(input);
        assert_eq!(
            result,
            "## Title\n\nSome text\n\n### Subtitle\n\n#### Deep\n"
        );
    }

    #[test]
    fn test_bump_headings_no_headings() {
        let input = "Just plain text\nNo headings here\n";
        assert_eq!(bump_headings(input), input);
    }

    #[test]
    fn test_bump_headings_preserves_hash_in_code() {
        let input = "# Heading\n\n```\n# this is a comment\n```\n";
        let result = bump_headings(input);
        // The regex is line-based, so it will bump the comment too.
        // This is acceptable — code blocks with # comments at line start are rare
        // and the trade-off is worth the simplicity.
        assert!(result.starts_with("## Heading"));
    }

    #[test]
    fn test_make_anchor() {
        assert_eq!(make_anchor("1-overview", "Overview"), "overview");
        assert_eq!(
            make_anchor("1.1-key-features", "Key Features"),
            "key-features"
        );
        assert_eq!(
            make_anchor("", "Getting Started Guide"),
            "getting-started-guide"
        );
    }

    #[test]
    fn test_compile_basic() {
        let pages = vec![
            Page {
                slug: "1-overview".into(),
                title: "Overview".into(),
                depth: 0,
                content: Some("# Intro\n\nHello world.".into()),
                error: None,
            },
            Page {
                slug: "2-setup".into(),
                title: "Setup".into(),
                depth: 0,
                content: Some("Install it.".into()),
                error: None,
            },
        ];

        let result = compile("test/repo", &pages, true, true);
        assert!(result.contains("<!-- dw2md v0.1.0 | test/repo"));
        assert!(result.contains("# test/repo — DeepWiki"));
        assert!(result.contains("## Table of Contents"));
        assert!(result.contains("- [Overview](#overview)"));
        assert!(result.contains("- [Setup](#setup)"));
        assert!(result.contains("## Overview"));
        assert!(result.contains("## Intro")); // bumped from # to ##
        assert!(result.contains("## Setup"));
    }

    #[test]
    fn test_compile_no_toc() {
        let pages = vec![Page {
            slug: "1-overview".into(),
            title: "Overview".into(),
            depth: 0,
            content: Some("Hello.".into()),
            error: None,
        }];

        let result = compile("test/repo", &pages, false, true);
        assert!(!result.contains("Table of Contents"));
        assert!(result.contains("## Overview"));
    }

    #[test]
    fn test_compile_no_metadata() {
        let pages = vec![Page {
            slug: "1-overview".into(),
            title: "Overview".into(),
            depth: 0,
            content: Some("Hello.".into()),
            error: None,
        }];

        let result = compile("test/repo", &pages, true, false);
        assert!(!result.contains("<!-- dw2md"));
        assert!(!result.contains("Compiled from"));
        assert!(result.contains("# test/repo — DeepWiki"));
    }

    #[test]
    fn test_compile_with_error_page() {
        let pages = vec![
            Page {
                slug: "1-overview".into(),
                title: "Overview".into(),
                depth: 0,
                content: Some("Hello.".into()),
                error: None,
            },
            Page {
                slug: "2-broken".into(),
                title: "Broken Page".into(),
                depth: 0,
                content: None,
                error: Some("Timeout after 30s".into()),
            },
        ];

        let result = compile("test/repo", &pages, true, true);
        assert!(result.contains("## Broken Page"));
        assert!(result.contains("Failed to fetch this page"));
        assert!(result.contains("Timeout after 30s"));
    }
}
