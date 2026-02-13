use serde::Serialize;

use crate::wiki::Page;

#[derive(Serialize)]
pub struct JsonOutput {
    pub repo: String,
    pub url: String,
    pub generated_at: String,
    pub tool_version: String,
    pub page_count: usize,
    pub pages: Vec<JsonPage>,
}

#[derive(Serialize)]
pub struct JsonPage {
    pub slug: String,
    pub title: String,
    pub depth: usize,
    pub content: String,
}

/// Compile pages into JSON output.
pub fn compile(repo: &str, pages: &[Page]) -> String {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let page_count = pages.iter().filter(|p| p.content.is_some()).count();

    let output = JsonOutput {
        repo: repo.to_string(),
        url: format!("https://deepwiki.com/{}", repo),
        generated_at: now,
        tool_version: "0.2.0".to_string(),
        page_count,
        pages: pages
            .iter()
            .filter(|p| p.content.is_some())
            .map(|p| JsonPage {
                slug: p.slug.clone(),
                title: p.title.clone(),
                depth: p.depth,
                content: p.content.clone().unwrap_or_default(),
            })
            .collect(),
    };

    serde_json::to_string_pretty(&output).expect("Failed to serialize JSON output")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_compile() {
        let pages = vec![
            Page {
                slug: "1-overview".into(),
                title: "Overview".into(),
                depth: 0,
                content: Some("Hello world.".into()),
                error: None,
            },
            Page {
                slug: "2-setup".into(),
                title: "Setup".into(),
                depth: 1,
                content: Some("Install it.".into()),
                error: None,
            },
        ];

        let result = compile("test/repo", &pages);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["repo"], "test/repo");
        assert_eq!(parsed["url"], "https://deepwiki.com/test/repo");
        assert_eq!(parsed["tool_version"], "0.2.0");
        assert_eq!(parsed["page_count"], 2);
        assert_eq!(parsed["pages"][0]["slug"], "1-overview");
        assert_eq!(parsed["pages"][0]["content"], "Hello world.");
        assert_eq!(parsed["pages"][1]["depth"], 1);
    }

    #[test]
    fn test_json_compile_skips_failed_pages() {
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
                title: "Broken".into(),
                depth: 0,
                content: None,
                error: Some("timeout".into()),
            },
        ];

        let result = compile("test/repo", &pages);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["page_count"], 1);
        assert_eq!(parsed["pages"].as_array().unwrap().len(), 1);
    }
}
