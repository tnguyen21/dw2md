mod compiler;
mod mcp;
mod wiki;

use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Parser;

/// Crawl a DeepWiki repository and compile all pages into a single, LLM-friendly markdown file.
#[derive(Parser, Debug)]
#[command(name = "dw2md", version, about)]
struct Cli {
    /// Repository identifier: owner/repo, https://deepwiki.com/owner/repo, or a full page URL
    repo: String,

    /// Write output to a file instead of stdout
    #[arg(short, long)]
    output: Option<String>,

    /// Output format: markdown or json
    #[arg(short, long, default_value = "markdown")]
    format: OutputFormat,

    /// Per-request timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// Comma-separated page slugs to include
    #[arg(short, long, value_delimiter = ',')]
    pages: Option<Vec<String>>,

    /// Comma-separated page slugs to exclude
    #[arg(short = 'x', long, value_delimiter = ',')]
    exclude: Option<Vec<String>>,

    /// Omit the generated table of contents from output
    #[arg(long)]
    no_toc: bool,

    /// Omit the metadata header block
    #[arg(long)]
    no_metadata: bool,

    /// Suppress progress output on stderr
    #[arg(short, long)]
    quiet: bool,

    /// Show detailed progress and debug info
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Markdown,
    Json,
}

/// Parse a repository identifier from various input formats.
///
/// Accepts:
/// - `owner/repo`
/// - `https://deepwiki.com/owner/repo`
/// - `https://deepwiki.com/owner/repo/3.1-some-page`
fn parse_repo(input: &str) -> Result<String> {
    let input = input.trim().trim_end_matches('/');

    // Try URL format
    if input.starts_with("http://") || input.starts_with("https://") {
        let url = input
            .strip_prefix("https://deepwiki.com/")
            .or_else(|| input.strip_prefix("http://deepwiki.com/"))
            .context("URL must be a deepwiki.com URL (e.g., https://deepwiki.com/owner/repo)")?;

        let parts: Vec<&str> = url.splitn(3, '/').collect();
        if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
            bail!("URL must contain owner/repo (e.g., https://deepwiki.com/owner/repo)");
        }

        return Ok(format!("{}/{}", parts[0], parts[1]));
    }

    // Try owner/repo format
    let parts: Vec<&str> = input.splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        bail!(
            "Invalid repository identifier '{}'. Expected owner/repo or https://deepwiki.com/owner/repo",
            input
        );
    }

    Ok(input.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let repo = parse_repo(&cli.repo)?;

    let config = compiler::CompileConfig {
        repo: repo.clone(),
        timeout: Duration::from_secs(cli.timeout),
        include: cli.pages,
        exclude: cli.exclude,
        quiet: cli.quiet,
        verbose: cli.verbose,
    };

    let pages = compiler::fetch_wiki(&config).await?;

    let output = match cli.format {
        OutputFormat::Markdown => {
            compiler::markdown::compile(&repo, &pages, !cli.no_toc, !cli.no_metadata)
        }
        OutputFormat::Json => compiler::json::compile(&repo, &pages),
    };

    if let Some(path) = cli.output {
        std::fs::write(&path, &output)
            .with_context(|| format!("Failed to write output to {}", path))?;
        if !cli.quiet {
            eprintln!("[dw2md] Output written to {}", path);
        }
    } else {
        print!("{}", output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_simple() {
        assert_eq!(parse_repo("facebook/react").unwrap(), "facebook/react");
    }

    #[test]
    fn test_parse_repo_url() {
        assert_eq!(
            parse_repo("https://deepwiki.com/facebook/react").unwrap(),
            "facebook/react"
        );
    }

    #[test]
    fn test_parse_repo_url_with_page() {
        assert_eq!(
            parse_repo("https://deepwiki.com/tokio-rs/tokio/3.1-runtime").unwrap(),
            "tokio-rs/tokio"
        );
    }

    #[test]
    fn test_parse_repo_url_trailing_slash() {
        assert_eq!(
            parse_repo("https://deepwiki.com/owner/repo/").unwrap(),
            "owner/repo"
        );
    }

    #[test]
    fn test_parse_repo_invalid() {
        assert!(parse_repo("just-a-name").is_err());
        assert!(parse_repo("").is_err());
        assert!(parse_repo("/repo").is_err());
        assert!(parse_repo("owner/").is_err());
    }

    #[test]
    fn test_parse_repo_wrong_host() {
        assert!(parse_repo("https://github.com/owner/repo").is_err());
    }
}
