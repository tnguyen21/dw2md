pub mod json;
pub mod markdown;

use std::time::Duration;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

use crate::mcp::McpClient;
use crate::wiki::{Page, WikiStructure, filter, merge_content, split_pages};

/// Configuration for the compilation pipeline.
pub struct CompileConfig {
    pub repo: String,
    pub timeout: Duration,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub quiet: bool,
    pub verbose: bool,
}

/// Fetch the wiki structure and all page contents.
pub async fn fetch_wiki(config: &CompileConfig) -> Result<Vec<Page>> {
    let client = McpClient::connect(config.timeout)
        .await
        .context("Failed to connect to DeepWiki MCP server")?;

    if config.verbose {
        eprintln!("[dw2md] Connected to MCP server");
    }

    // Phase 1: Fetch wiki structure
    if !config.quiet {
        eprintln!("[dw2md] Fetching wiki structure for {}...", config.repo);
    }

    let structure_text = call_with_retry(&client, "read_wiki_structure", &config.repo, 3)
        .await
        .context(format!(
            "Failed to fetch wiki structure. Repository '{}' may not be indexed on DeepWiki. \
             Visit https://deepwiki.com to request indexing.",
            config.repo
        ))?;

    if config.verbose {
        eprintln!(
            "[dw2md] Raw structure ({} bytes):\n{}",
            structure_text.len(),
            &structure_text[..structure_text.len().min(500)]
        );
    }

    let structure = WikiStructure::parse(&structure_text)
        .context("Failed to parse wiki structure")?;

    if config.verbose {
        eprintln!("[dw2md] Found {} pages in structure", structure.pages.len());
    }

    // Apply include/exclude filters
    let mut pages = filter::filter_pages(structure.pages, &config.include, &config.exclude);
    let total_pages = pages.len();

    if !config.quiet {
        eprintln!("[dw2md] {} pages to fetch", total_pages);
    }

    // Phase 2: Fetch all page contents (single API call returns everything)
    let progress = if !config.quiet {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message("Fetching page contents...");
        pb.enable_steady_tick(Duration::from_millis(100));
        Some(pb)
    } else {
        None
    };

    let contents_text = call_with_retry(&client, "read_wiki_contents", &config.repo, 3)
        .await
        .context("Failed to fetch wiki contents")?;

    if let Some(pb) = &progress {
        pb.set_message("Parsing pages...");
    }

    if config.verbose {
        eprintln!(
            "[dw2md] Raw content ({} bytes)",
            contents_text.len()
        );
    }

    // Phase 3: Split content by "# Page:" markers and match to structure
    let content_pages = split_pages(&contents_text);

    if config.verbose {
        eprintln!(
            "[dw2md] Found {} pages in content response",
            content_pages.len()
        );
        for (title, content) in &content_pages {
            eprintln!("  - \"{}\" ({} bytes)", title, content.len());
        }
    }

    merge_content(&mut pages, &content_pages);

    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    let matched = pages.iter().filter(|p| p.content.is_some()).count();
    let unmatched = total_pages - matched;

    if !config.quiet {
        eprintln!(
            "[dw2md] Done: {} pages matched, {} unmatched",
            matched, unmatched
        );
    }

    if unmatched > 0 && config.verbose {
        for page in &pages {
            if page.content.is_none() {
                eprintln!("[dw2md] Warning: No content matched for '{}'", page.title);
            }
        }
    }

    Ok(pages)
}

/// Call an MCP tool with retry logic (exponential backoff).
async fn call_with_retry(
    client: &McpClient,
    tool: &str,
    repo: &str,
    max_retries: u32,
) -> Result<String> {
    let mut last_err = None;

    for attempt in 0..max_retries {
        if attempt > 0 {
            let backoff = Duration::from_secs(1 << (attempt - 1));
            tokio::time::sleep(backoff).await;
        }

        match client
            .call_tool(tool, serde_json::json!({"repoName": repo}))
            .await
        {
            Ok(content) => return Ok(content),
            Err(err) => {
                last_err = Some(err);
            }
        }
    }

    Err(last_err.unwrap())
}
