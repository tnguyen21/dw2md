pub mod json;
pub mod markdown;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;

use crate::mcp::McpClient;
use crate::wiki::{Page, WikiStructure, filter};

/// Configuration for the compilation pipeline.
pub struct CompileConfig {
    pub repo: String,
    pub concurrency: usize,
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

    let client = Arc::new(client);

    if config.verbose {
        eprintln!("[dw2md] Connected to MCP server");
    }

    // Fetch wiki structure
    if !config.quiet {
        eprintln!("[dw2md] Fetching wiki structure for {}...", config.repo);
    }

    let structure_text = client
        .call_tool(
            "read_wiki_structure",
            serde_json::json!({"repoName": config.repo.clone()}),
        )
        .await
        .context(format!(
            "Failed to fetch wiki structure. Repository '{}' may not be indexed on DeepWiki. \
             Visit https://deepwiki.com to request indexing.",
            config.repo
        ))?;

    if config.verbose {
        eprintln!(
            "[dw2md] Raw structure response ({} bytes):\n{}",
            structure_text.len(),
            &structure_text[..structure_text.len().min(500)]
        );
    }

    let structure = WikiStructure::parse(&structure_text)
        .context("Failed to parse wiki structure")?;

    if config.verbose {
        eprintln!("[dw2md] Found {} pages in structure", structure.pages.len());
    }

    // Apply filters
    let mut pages = filter::filter_pages(structure.pages, &config.include, &config.exclude);

    if !config.quiet {
        eprintln!(
            "[dw2md] Fetching {} pages (concurrency: {})...",
            pages.len(),
            config.concurrency
        );
    }

    // Set up progress bar
    let progress = if !config.quiet {
        let pb = ProgressBar::new(pages.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("##-"),
        );
        Some(pb)
    } else {
        None
    };

    // Fetch all page contents concurrently
    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let mut handles = Vec::new();

    for (idx, page) in pages.iter().enumerate() {
        let client = client.clone();
        let sem = semaphore.clone();
        let repo = config.repo.clone();
        let slug = page.slug.clone();
        let title = page.title.clone();
        let progress = progress.clone();
        let verbose = config.verbose;

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            if verbose {
                eprintln!("[dw2md] Fetching page: {} ({})", title, slug);
            }

            let result = fetch_page_with_retry(&client, &repo, &slug, 3).await;

            if let Some(pb) = &progress {
                pb.set_message(title.clone());
                pb.inc(1);
            }

            (idx, result)
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        let (idx, result) = handle.await.context("Task panicked")?;
        match result {
            Ok(content) => {
                pages[idx].content = Some(content);
            }
            Err(err) => {
                let msg = format!("{:#}", err);
                if !config.quiet {
                    eprintln!("[dw2md] Warning: Failed to fetch '{}': {}", pages[idx].slug, msg);
                }
                pages[idx].error = Some(msg);
            }
        }
    }

    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    let success_count = pages.iter().filter(|p| p.content.is_some()).count();
    let fail_count = pages.iter().filter(|p| p.error.is_some()).count();

    if !config.quiet {
        eprintln!(
            "[dw2md] Done: {} pages fetched, {} failed",
            success_count, fail_count
        );
    }

    Ok(pages)
}

/// Fetch a single page with retry logic (exponential backoff).
async fn fetch_page_with_retry(
    client: &McpClient,
    repo: &str,
    slug: &str,
    max_retries: u32,
) -> Result<String> {
    let mut last_err = None;

    for attempt in 0..max_retries {
        if attempt > 0 {
            let backoff = Duration::from_secs(1 << (attempt - 1));
            tokio::time::sleep(backoff).await;
        }

        match client
            .call_tool(
                "read_wiki_contents",
                serde_json::json!({
                    "repoName": repo,
                    "pagePath": slug,
                }),
            )
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
