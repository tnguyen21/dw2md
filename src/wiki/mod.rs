pub mod filter;

use serde::{Deserialize, Serialize};

/// A single wiki page with its content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub slug: String,
    pub title: String,
    pub depth: usize,
    pub content: Option<String>,
    /// If the page fetch failed, store the error message.
    #[serde(skip)]
    pub error: Option<String>,
}

/// Parsed wiki structure (table of contents) from the MCP server.
#[derive(Debug)]
pub struct WikiStructure {
    pub pages: Vec<Page>,
}

impl WikiStructure {
    /// Parse the raw text from `read_wiki_structure` into a structured table of contents.
    ///
    /// The response is typically a JSON array of page objects with title, id/slug, and children.
    /// We flatten the tree into an ordered list with depth information.
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        // The structure response can be either JSON or a textual listing.
        // Try JSON first.
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
            return Self::from_json(&value);
        }

        // Fallback: treat as a textual listing and parse lines.
        Self::from_text(raw)
    }

    fn from_json(value: &serde_json::Value) -> anyhow::Result<Self> {
        let mut pages = Vec::new();

        if let Some(arr) = value.as_array() {
            for item in arr {
                Self::flatten_page(item, 0, &mut pages);
            }
        } else if let Some(obj) = value.as_object() {
            // Could be a single root object with a "pages" or "children" field
            if let Some(arr) = obj.get("pages").and_then(|v| v.as_array()) {
                for item in arr {
                    Self::flatten_page(item, 0, &mut pages);
                }
            } else if let Some(arr) = obj.get("children").and_then(|v| v.as_array()) {
                for item in arr {
                    Self::flatten_page(item, 0, &mut pages);
                }
            } else {
                // Single page
                Self::flatten_page(value, 0, &mut pages);
            }
        }

        if pages.is_empty() {
            anyhow::bail!("Failed to parse wiki structure: no pages found");
        }

        Ok(WikiStructure { pages })
    }

    fn flatten_page(value: &serde_json::Value, depth: usize, pages: &mut Vec<Page>) {
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
                Self::flatten_page(child, depth + 1, pages);
            }
        }
    }

    fn from_text(text: &str) -> anyhow::Result<Self> {
        let mut pages = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Count leading whitespace to determine depth
            let indent = line.len() - line.trim_start().len();
            let depth = indent / 2;

            // Try to extract a slug-like prefix (e.g., "1.2-some-title")
            let (slug, title) = if let Some(pos) = trimmed.find(' ') {
                let potential_slug = &trimmed[..pos];
                if potential_slug.contains('-')
                    || potential_slug.chars().next().map_or(false, |c| c.is_ascii_digit())
                {
                    (potential_slug.to_string(), trimmed[pos + 1..].trim().to_string())
                } else {
                    (slugify(trimmed), trimmed.to_string())
                }
            } else {
                (slugify(trimmed), trimmed.to_string())
            };

            pages.push(Page {
                slug,
                title,
                depth,
                content: None,
                error: None,
            });
        }

        if pages.is_empty() {
            anyhow::bail!("Failed to parse wiki structure: no pages found in text");
        }

        Ok(WikiStructure { pages })
    }
}

/// Convert a title to a URL-safe slug.
pub fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else {
                '-'
            }
        })
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
    }

    #[test]
    fn test_parse_json_structure() {
        let json = r#"[
            {
                "id": "1-overview",
                "title": "Overview",
                "children": [
                    {"id": "1.1-features", "title": "Features"},
                    {"id": "1.2-requirements", "title": "Requirements"}
                ]
            },
            {
                "id": "2-getting-started",
                "title": "Getting Started"
            }
        ]"#;

        let structure = WikiStructure::parse(json).unwrap();
        assert_eq!(structure.pages.len(), 4);
        assert_eq!(structure.pages[0].slug, "1-overview");
        assert_eq!(structure.pages[0].depth, 0);
        assert_eq!(structure.pages[1].slug, "1.1-features");
        assert_eq!(structure.pages[1].depth, 1);
        assert_eq!(structure.pages[3].slug, "2-getting-started");
        assert_eq!(structure.pages[3].depth, 0);
    }

    #[test]
    fn test_parse_json_with_pages_key() {
        let json = r#"{"pages": [
            {"id": "overview", "title": "Overview"}
        ]}"#;
        let structure = WikiStructure::parse(json).unwrap();
        assert_eq!(structure.pages.len(), 1);
        assert_eq!(structure.pages[0].title, "Overview");
    }
}
