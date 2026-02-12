pub mod filter;

use serde::{Deserialize, Serialize};

/// A single wiki page with its content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub slug: String,
    pub title: String,
    pub depth: usize,
    pub content: Option<String>,
    #[serde(skip)]
    pub error: Option<String>,
}

/// Parsed wiki structure (table of contents) from the MCP server.
#[derive(Debug)]
pub struct WikiStructure {
    pub pages: Vec<Page>,
}

impl WikiStructure {
    /// Parse the raw text from `read_wiki_structure`.
    ///
    /// The response is a bullet-list text like:
    /// ```text
    /// Available pages for owner/repo:
    ///
    /// - 1 Overview
    ///   - 1.1 Repository Structure
    /// - 2 Getting Started
    /// ```
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        // Try JSON first (future-proofing)
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
            return Self::from_json(&value);
        }

        Self::from_text(raw)
    }

    fn from_json(value: &serde_json::Value) -> anyhow::Result<Self> {
        let mut pages = Vec::new();

        if let Some(arr) = value.as_array() {
            for item in arr {
                Self::flatten_json_page(item, 0, &mut pages);
            }
        } else if let Some(obj) = value.as_object() {
            if let Some(arr) = obj.get("pages").and_then(|v| v.as_array()) {
                for item in arr {
                    Self::flatten_json_page(item, 0, &mut pages);
                }
            } else if let Some(arr) = obj.get("children").and_then(|v| v.as_array()) {
                for item in arr {
                    Self::flatten_json_page(item, 0, &mut pages);
                }
            }
        }

        if pages.is_empty() {
            anyhow::bail!("Failed to parse wiki structure: no pages found in JSON");
        }

        Ok(WikiStructure { pages })
    }

    fn flatten_json_page(value: &serde_json::Value, depth: usize, pages: &mut Vec<Page>) {
        let title = value
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();

        let slug = value
            .get("id")
            .or_else(|| value.get("slug"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        pages.push(Page {
            slug,
            title,
            depth,
            content: None,
            error: None,
        });

        if let Some(children) = value.get("children").and_then(|v| v.as_array()) {
            for child in children {
                Self::flatten_json_page(child, depth + 1, pages);
            }
        }
    }

    /// Parse the text-based structure listing from DeepWiki.
    ///
    /// Format:
    /// ```text
    /// Available pages for owner/repo:
    ///
    /// - 1 Overview
    ///   - 1.1 Repository Structure
    /// - 2 Getting Started
    /// ```
    fn from_text(text: &str) -> anyhow::Result<Self> {
        let mut pages = Vec::new();

        for line in text.lines() {
            // Skip empty lines and the header
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("Available pages") {
                continue;
            }

            // Lines look like "- 1 Overview" or "  - 1.1 Repository Structure"
            // Find the "- " marker
            if let Some(dash_pos) = line.find("- ") {
                let depth = line[..dash_pos].chars().filter(|c| *c == ' ').count() / 2;
                let after_dash = &line[dash_pos + 2..];

                // The title is everything after "- ", like "1 Overview" or "1.1 Repository Structure"
                let title = after_dash.trim().to_string();

                // Generate a slug from the title: "1 Overview" -> "1-overview"
                let slug = slugify(&title);

                pages.push(Page {
                    slug,
                    title,
                    depth,
                    content: None,
                    error: None,
                });
            }
        }

        if pages.is_empty() {
            anyhow::bail!("Failed to parse wiki structure: no pages found in text");
        }

        Ok(WikiStructure { pages })
    }
}

/// Split the combined content from `read_wiki_contents` into individual pages.
///
/// The content uses `# Page: <title>` as delimiters between pages.
pub fn split_pages(content: &str) -> Vec<(String, String)> {
    let mut pages = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_content = String::new();

    for line in content.lines() {
        if let Some(title) = line.strip_prefix("# Page: ") {
            // Save the previous page
            if let Some(prev_title) = current_title.take() {
                pages.push((prev_title, current_content.trim().to_string()));
                current_content.clear();
            }
            current_title = Some(title.trim().to_string());
        } else if current_title.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save the last page
    if let Some(title) = current_title {
        pages.push((title, current_content.trim().to_string()));
    }

    pages
}

/// Match split pages to the wiki structure by title.
///
/// Structure titles look like "1 Overview" or "1.1 Repository Structure",
/// while page delimiters use "# Page: Overview" or "# Page: Repository Structure".
/// We strip the leading number prefix from the structure title for matching.
pub fn merge_content(structure: &mut [Page], content_pages: &[(String, String)]) {
    for page in structure.iter_mut() {
        let stripped_title = strip_number_prefix(&page.title);

        if let Some((_, content)) = content_pages.iter().find(|(title, _)| {
            *title == page.title || *title == stripped_title
        }) {
            page.content = Some(content.clone());
        }
    }
}

/// Strip a leading number prefix like "1 ", "1.1 ", "3.2.1 " from a title.
fn strip_number_prefix(title: &str) -> String {
    let bytes = title.as_bytes();
    let mut i = 0;

    // Skip digits and dots (e.g., "1", "1.1", "3.2.1")
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }

    // Skip the space after the number
    if i > 0 && i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }

    if i > 0 && i < title.len() {
        title[i..].to_string()
    } else {
        title.to_string()
    }
}

/// Convert a title to a URL-safe slug.
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("1.1 Key Features"), "1-1-key-features");
        assert_eq!(slugify("  spaces  "), "spaces");
        assert_eq!(slugify("1 Overview"), "1-overview");
    }

    #[test]
    fn test_parse_text_structure() {
        let text = "Available pages for facebook/react:\n\
                     \n\
                     - 1 Overview\n\
                     \x20\x20- 1.1 Repository Structure\n\
                     - 2 Getting Started\n";

        let structure = WikiStructure::parse(text).unwrap();
        assert_eq!(structure.pages.len(), 3);
        assert_eq!(structure.pages[0].title, "1 Overview");
        assert_eq!(structure.pages[0].slug, "1-overview");
        assert_eq!(structure.pages[0].depth, 0);
        assert_eq!(structure.pages[1].title, "1.1 Repository Structure");
        assert_eq!(structure.pages[1].slug, "1-1-repository-structure");
        assert_eq!(structure.pages[1].depth, 1);
        assert_eq!(structure.pages[2].title, "2 Getting Started");
        assert_eq!(structure.pages[2].depth, 0);
    }

    #[test]
    fn test_split_pages() {
        let content = "# Page: Overview\n\
                        \n\
                        # Overview\n\
                        \n\
                        This is the overview.\n\
                        \n\
                        # Page: Getting Started\n\
                        \n\
                        Install the thing.\n";

        let pages = split_pages(content);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].0, "Overview");
        assert!(pages[0].1.contains("This is the overview."));
        assert_eq!(pages[1].0, "Getting Started");
        assert!(pages[1].1.contains("Install the thing."));
    }

    #[test]
    fn test_split_pages_empty() {
        let pages = split_pages("No page markers here");
        assert!(pages.is_empty());
    }

    #[test]
    fn test_merge_content() {
        let mut structure = vec![
            Page {
                slug: "1-overview".into(),
                title: "Overview".into(),
                depth: 0,
                content: None,
                error: None,
            },
            Page {
                slug: "2-setup".into(),
                title: "Setup".into(),
                depth: 0,
                content: None,
                error: None,
            },
        ];

        let content_pages = vec![
            ("Overview".to_string(), "Overview content".to_string()),
            ("Setup".to_string(), "Setup content".to_string()),
        ];

        merge_content(&mut structure, &content_pages);
        assert_eq!(structure[0].content.as_deref(), Some("Overview content"));
        assert_eq!(structure[1].content.as_deref(), Some("Setup content"));
    }

    #[test]
    fn test_merge_content_partial_match() {
        let mut structure = vec![
            Page {
                slug: "1-overview".into(),
                title: "Overview".into(),
                depth: 0,
                content: None,
                error: None,
            },
            Page {
                slug: "2-missing".into(),
                title: "Missing Page".into(),
                depth: 0,
                content: None,
                error: None,
            },
        ];

        let content_pages = vec![("Overview".to_string(), "Overview content".to_string())];

        merge_content(&mut structure, &content_pages);
        assert_eq!(structure[0].content.as_deref(), Some("Overview content"));
        assert!(structure[1].content.is_none());
    }

    #[test]
    fn test_parse_json_structure() {
        let json = r#"[
            {
                "id": "1-overview",
                "title": "Overview",
                "children": [
                    {"id": "1.1-features", "title": "Features"}
                ]
            },
            {"id": "2-getting-started", "title": "Getting Started"}
        ]"#;

        let structure = WikiStructure::parse(json).unwrap();
        assert_eq!(structure.pages.len(), 3);
        assert_eq!(structure.pages[0].slug, "1-overview");
        assert_eq!(structure.pages[0].depth, 0);
        assert_eq!(structure.pages[1].slug, "1.1-features");
        assert_eq!(structure.pages[1].depth, 1);
    }

    #[test]
    fn test_structure_titles_match_page_delimiters() {
        // The structure has titles like "1 Overview" but pages split by "# Page: Overview"
        // The title in the structure includes the numbering, page delimiter does not.
        // merge_content matches on title, so we need the structure to use the same title
        // as what appears after "# Page: ".
        //
        // In practice, the structure says "1 Overview" but the page delimiter says "# Page: Overview".
        // We handle this by stripping the leading number prefix during merge.
        let text = "Available pages for test/repo:\n\
                     \n\
                     - 1 Overview\n\
                     - 2 Setup\n";

        let structure = WikiStructure::parse(text).unwrap();
        // Title includes number: "1 Overview"
        assert_eq!(structure.pages[0].title, "1 Overview");
    }
}
