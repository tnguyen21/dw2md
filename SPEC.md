# `dw2md` — DeepWiki to Markdown Compile

**Version:** 0.2.0
**Language:** Rust
**Purpose:** Crawl a DeepWiki repository and compile all pages into a single, LLM-friendly markdown file.

---

## Motivation

DeepWiki generates excellent structured documentation for open-source repositories, but there's no good way to grab an entire wiki and feed it to an LLM as context. The content lives across many pages on `deepwiki.com`, each rendered client-side via Next.js, making naive scraping impractical. `dw2md` solves this by talking directly to the official DeepWiki MCP server — a free, no-auth JSON-RPC endpoint — to pull the wiki structure and all page contents, then compile them into a clean markdown document optimized for LLM consumption.

---

## Core Workflow

The tool operates in three phases:

**Phase 1 — Resolve the repository.** The user provides either a full DeepWiki URL (`https://deepwiki.com/owner/repo`) or a shorthand (`owner/repo`). The tool parses out the owner and repo name.

**Phase 2 — Fetch structure and contents via MCP.** The tool connects to `https://mcp.deepwiki.com/mcp` using the MCP Streamable HTTP protocol (JSON-RPC 2.0 over HTTP POST). It first calls `read_wiki_structure` to get the table of contents, then calls `read_wiki_contents` for each page (or in batches, if the tool supports it). Concurrency is bounded and configurable.

**Phase 3 — Compile and emit.** All pages are assembled into a single markdown document in table-of-contents order, with metadata headers, navigation aids, and clean formatting. The result is written to stdout or a file.

---

## MCP Client Implementation

The tool implements a minimal MCP client targeting the Streamable HTTP transport. No session management is needed — the DeepWiki MCP server is stateless.

### Protocol Details

- **Endpoint:** `https://mcp.deepwiki.com/mcp`
- **Transport:** HTTP POST with JSON-RPC 2.0 body
- **Content-Type:** `application/json`
- **Accept:** `application/json, text/event-stream`
- **Auth:** None required

### Initialization Handshake

Before calling tools, the client must perform the MCP initialization sequence:

```
POST /mcp
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-03-26",
    "capabilities": {},
    "clientInfo": {
      "name": "dw2md",
      "version": "0.2.0"
    }
  }
}
```

The server responds with its capabilities and may return a `Mcp-Session-Id` header. If present, include it in subsequent requests. Follow up with the `notifications/initialized` notification:

```
POST /mcp
{
  "jsonrpc": "2.0",
  "method": "notifications/initialized",
  "params": {}
}
```

### Tool Calls

After initialization, call tools via `tools/call`:

```
POST /mcp
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "read_wiki_structure",
    "arguments": {
      "repo": "owner/repo"
    }
  }
}
```

The three available tools are:

| Tool                  | Purpose                                                              | Key Arguments                                                                               |
| --------------------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `read_wiki_structure` | Returns the wiki's table of contents (page titles, hierarchy, slugs) | `repo` (e.g. `"tinygrad/tinygrad"`)                                                         |
| `read_wiki_contents`  | Returns the markdown content for a specific page                     | `repo`, page identifier (slug or title — discover exact schema via `tools/list` at runtime) |
| `ask_question`        | Not used by `dw2md`, but available for future extensions             | `repo`, `question`                                                                          |

**Important:** The exact argument schemas for these tools should be discovered at runtime by calling `tools/list` during initialization. The schemas above are based on documentation and observed behavior, but the server is the source of truth. The tool should call `tools/list` on first run and cache the schemas, or at minimum handle schema mismatches gracefully.

### Response Handling

The server may respond with either `application/json` (a single JSON-RPC response) or `text/event-stream` (SSE). The client must handle both:

- **JSON response:** Parse directly as a JSON-RPC result.
- **SSE response:** Read `data:` lines, parse each as a JSON-RPC message, and concatenate text content blocks from the result.

Tool call results come back in the standard MCP content block format:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "...the actual markdown content..."
      }
    ]
  }
}
```

---

## CLI Interface

```
dw2md [OPTIONS] <REPO>
```

### Arguments

- `<REPO>` — Repository identifier. Accepts any of:
  - `owner/repo` (e.g. `tinygrad/tinygrad`)
  - `https://deepwiki.com/owner/repo`
  - `https://deepwiki.com/owner/repo/3.1-some-page` (extracts `owner/repo`, ignores page path)

### Options

| Flag                 | Short | Default    | Description                                                                 |
| -------------------- | ----- | ---------- | --------------------------------------------------------------------------- |
| `--output <FILE>`    | `-o`  | stdout     | Write output to a file instead of stdout                                    |
| `--concurrency <N>`  | `-j`  | `4`        | Max concurrent page fetches                                                 |
| `--format <FMT>`     | `-f`  | `markdown` | Output format: `markdown` or `json`                                         |
| `--timeout <SECS>`   | `-t`  | `30`       | Per-request timeout in seconds                                              |
| `--pages <FILTER>`   | `-p`  | all        | Comma-separated page slugs to include (e.g. `1-overview,3.1-data-pipeline`) |
| `--exclude <FILTER>` | `-x`  | none       | Comma-separated page slugs to exclude                                       |
| `--no-toc`           |       | false      | Omit the structure tree from output                                         |
| `--no-metadata`      |       | false      | Omit the metadata header block                                              |
| `--quiet`            | `-q`  | false      | Suppress progress output on stderr                                          |
| `--verbose`          | `-v`  | false      | Show detailed progress and debug info                                       |

### Examples

```bash
# Basic usage — prints to stdout
dw2md tinygrad/tinygrad

# Save to file with progress
dw2md tinygrad/tinygrad -o react-wiki.md

# Just the architecture sections
dw2md AsyncFuncAI/deepwiki-open -p 3-architecture,3.1-data-pipeline,3.2-rag-system

# As JSON for programmatic use
dw2md tinygrad/tinygrad -f json -o react-wiki.json

# From a full URL
dw2md https://deepwiki.com/tokio-rs/tokio -o tokio.md
```

---

## Output Format

### Markdown (default)

The markdown output is designed for LLM and agent workflows. The two key design goals are:

1. **Fast structural scanning** — an agent should be able to read the table of contents and understand the document's hierarchy without processing the full content.
2. **Selective extraction** — an agent should be able to `grep` for a section delimiter and extract only the sections relevant to its current task, rather than stuffing the entire document into context.

The compiled document follows this structure:

```markdown
<!-- dw2md v0.2.0 | tinygrad/tinygrad | 2026-02-12T15:30:00Z | 47 pages -->

# tinygrad/tinygrad — DeepWiki

> Compiled from https://deepwiki.com/tinygrad/tinygrad
> Generated: 2026-02-12T15:30:00Z | Pages: 47

## Structure

├── 1 Overview
│   ├── 1.1 Key Features
│   └── 1.2 System Requirements
├── 2 Getting Started
│   ...
└── 8 API Reference

## Contents

<<< SECTION: 1 Overview [1-overview] >>>

[page content with original heading levels preserved]

<<< SECTION: 1.1 Key Features [1-1-key-features] >>>

[page content]

<<< SECTION: 2 Getting Started [2-getting-started] >>>

[page content]
```

#### Section delimiters

Each page is preceded by a delimiter line with the format:

```
<<< SECTION: {title} [{slug}] >>>
```

This is designed to be trivially grep-able by agents and scripts:

```bash
# List all sections
grep "^<<< SECTION:" wiki.md

# Extract a specific section (content between two delimiters)
sed -n '/^<<< SECTION: 1 Overview/,/^<<< SECTION:/p' wiki.md

# Regex to capture title and slug
# ^<<< SECTION: (.+?) \[(.+?)\] >>>$
```

The slug in `[brackets]` is the same identifier used by `--pages` and `--exclude` flags, so an agent can discover slugs from the structure, then re-invoke `dw2md` with `--pages` to fetch only what it needs.

#### Tree table of contents

The `## Structure` section uses ASCII tree characters (`├──`, `└──`, `│`) — the same visual language as the Unix `tree` command. This is more scannable than indented bullet lists and conveys hierarchy at a glance.

When `--no-toc` is passed, the structure tree and the `## Contents` header are both omitted; section delimiters go directly after the metadata.

#### Design decisions

- **Original heading levels preserved** — page content keeps its source heading structure. No heading-level bumping is performed because the `<<< SECTION >>>` delimiter (not markdown heading depth) is the structural boundary. This saves tokens and avoids information loss.
- **Token efficient** — compared to the previous format: no repeated horizontal rules (`---`), no anchor link markup in the TOC, no extra `#` characters from heading bumping.
- **HTML comment metadata on line 1** — machine-parseable but invisible to most renderers. Lets a tool quickly identify the document without reading the whole thing.
- **Source annotations preserved** — DeepWiki pages contain `Sources: file.py:1-50` annotations linking to GitHub. These provide useful code location context for LLMs.
- **Mermaid blocks preserved** — left as fenced code blocks. Many LLMs can interpret these.

### JSON format

When `--format json` is specified:

```json
{
  "repo": "tinygrad/tinygrad",
  "url": "https://deepwiki.com/tinygrad/tinygrad",
  "generated_at": "2026-02-12T15:30:00Z",
  "tool_version": "0.2.0",
  "page_count": 47,
  "pages": [
    {
      "slug": "1-overview",
      "title": "1 Overview",
      "depth": 0,
      "content": "...markdown content..."
    },
    {
      "slug": "1-1-key-features",
      "title": "1.1 Key Features",
      "depth": 1,
      "content": "..."
    }
  ]
}
```

The JSON format is useful for downstream tooling — feeding individual pages into different context windows, building a retrieval index, etc.

---

## Architecture

### Crate Dependencies

| Crate                  | Purpose                                                   |
| ---------------------- | --------------------------------------------------------- |
| `clap`                 | CLI argument parsing (derive API)                         |
| `reqwest`              | HTTP client (with `rustls-tls` for no OpenSSL dependency) |
| `tokio`                | Async runtime                                             |
| `serde` / `serde_json` | JSON serialization                                        |
| `futures`              | Stream combinators for SSE parsing                        |
| `indicatif`            | Progress bars on stderr                                   |
| `anyhow`               | Error handling                                            |

### Module Layout

```
src/
├── main.rs          # CLI entry point, clap parsing
├── mcp/
│   ├── mod.rs       # MCP client: init, tool calls, response parsing
│   ├── transport.rs # HTTP transport layer, SSE handling
│   └── types.rs     # JSON-RPC and MCP type definitions
├── compiler/
│   ├── mod.rs       # Orchestrates fetch + compile pipeline
│   ├── markdown.rs  # Markdown output assembly, tree TOC, section delimiters
│   └── json.rs      # JSON output assembly
└── wiki/
    ├── mod.rs       # Wiki types: Page, Structure, etc.
    └── filter.rs    # Page include/exclude filtering
```

### Concurrency Model

Page fetching uses a `tokio::sync::Semaphore` to bound concurrency. The fetch pipeline:

1. Call `read_wiki_structure` → get ordered list of pages.
2. Apply include/exclude filters.
3. Spawn one task per page, gated by the semaphore.
4. Collect results into a `Vec<Page>` preserving the original order.
5. Pass to the compiler for output assembly.

The semaphore default of 4 is conservative to be respectful of the free DeepWiki MCP endpoint. Users can increase it, but the tool should document that aggressive concurrency may result in rate limiting.

---

## Error Handling

The tool should handle these failure modes gracefully:

- **Network errors / timeouts** — retry each page up to 3 times with exponential backoff (1s, 2s, 4s). After exhausting retries, log the failure and continue with remaining pages. The final output should note which pages failed.
- **MCP protocol errors** — if initialization fails, exit with a clear error message suggesting the endpoint may be down. If `tools/list` returns unexpected schemas, log a warning and attempt to proceed with best-guess arguments.
- **Repository not indexed** — if `read_wiki_structure` returns an error indicating the repo isn't on DeepWiki, print a helpful message: `"Repository 'owner/repo' is not indexed on DeepWiki. Visit https://deepwiki.com to request indexing."`
- **Partial failures** — the tool should always produce output for whatever pages it successfully fetched, appending a summary of failures at the end.

---

## Future Extensions

These are explicitly out of scope for v0.2.0 but worth designing around:

- **`--ask <QUESTION>`** — pipe the compiled wiki to the `ask_question` tool and print the answer. Useful for one-shot queries.
- **Caching** — store fetched wikis in `~/.cache/dw2md/` keyed by `owner/repo` with a TTL. Skip fetching if fresh.
- **Multiple repos** — accept multiple repo arguments and compile them into one document or separate files.
- **Diff mode** — compare a cached version with the current wiki and show what changed.
- **Piping to clipboard** — `dw2md owner/repo | pbcopy` already works since the default output is stdout.

---

## Build & Install

```bash
# From source
cargo install --path .

# Or directly from crates.io (once published)
cargo install dw2md
```

The binary should be statically linkable (no OpenSSL dependency thanks to `rustls`) and produce a single ~5MB binary.
