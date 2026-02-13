# dw2md

Grab an entire [DeepWiki](https://deepwiki.com) and compile it into a single markdown file — ready to drop into an LLM context window.

```bash
dw2md tinygrad/tinygrad -o tinygrad-wiki.md
```

DeepWiki generates excellent structured documentation for open-source repositories, but it's spread across dozens of client-rendered pages with no export button. `dw2md` talks directly to DeepWiki's MCP server to pull the full wiki structure and contents, then assembles everything into one clean document.

## Install

### From crates.io (recommended)

```bash
cargo install dw2md
```

### Homebrew (macOS/Linux)

```bash
brew install tnguyen21/dw2md/dw2md
```

### Debian/Ubuntu (.deb)

```bash
curl -LO https://github.com/tnguyen21/dw2md/releases/latest/download/dw2md_0.2.1_amd64.deb
sudo dpkg -i dw2md_0.2.1_amd64.deb
```

### From source

```bash
git clone https://github.com/tnguyen21/dw2md
cd dw2md
cargo install --path .
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/tnguyen21/dw2md/releases) for Linux, macOS (x86_64/ARM64), and Windows.

Produces a single static binary (~6MB, no OpenSSL dependency).

## Usage

```
dw2md [OPTIONS] <REPO>
```

`<REPO>` accepts any of:

| Format                   | Example                                           |
| ------------------------ | ------------------------------------------------- |
| `owner/repo`             | `tinygrad/tinygrad`                               |
| Full URL                 | `https://deepwiki.com/tinygrad/tinygrad`          |
| Page URL (extracts repo) | `https://deepwiki.com/tokio-rs/tokio/3.1-runtime` |

### Options

| Flag                 | Short | Default    | Description                                    |
| -------------------- | ----- | ---------- | ---------------------------------------------- |
| `--output <FILE>`    | `-o`  | stdout     | Write to file instead of stdout                |
| `--format <FMT>`     | `-f`  | `markdown` | Output format: `markdown` or `json`            |
| `--timeout <SECS>`   | `-t`  | `30`       | Per-request timeout in seconds                 |
| `--pages <FILTER>`   | `-p`  | all        | Comma-separated page slugs to include          |
| `--exclude <FILTER>` | `-x`  | none       | Comma-separated page slugs to exclude          |
| `--no-toc`           |       |            | Omit the structure tree from output             |
| `--no-metadata`      |       |            | Omit the metadata header                       |
| `--list`             | `-l`  |            | Print the table of contents and exit           |
| `--interactive`      | `-i`  |            | Interactively select which sections to include |
| `--quiet`            | `-q`  |            | Suppress progress on stderr                    |
| `--verbose`          | `-v`  |            | Show debug info                                |

## Examples

**Dump an entire wiki to a file:**

```bash
dw2md tokio-rs/tokio -o tokio-wiki.md
```

**Pipe straight to clipboard (macOS):**

```bash
dw2md tinygrad/tinygrad | pbcopy
```

**See what sections are available before downloading:**

```bash
dw2md tinygrad/tinygrad --list
```

```
├── 1 Overview [1-overview]
│   └── 1.1 Repository Structure and Packages [1-1-repository-structure-and-packages]
├── 2 Feature Flags System [2-feature-flags-system]
├── 3 Build System and Package Distribution [3-build-system-and-package-distribution]
│   ...
└── 7 Developer Tools and Debugging [7-developer-tools-and-debugging]
```

**Interactively pick which sections to include:**

```bash
dw2md tinygrad/tinygrad -i -o react-wiki.md
```

Shows a multi-select prompt where you can toggle sections on/off with space, then press enter to fetch only what you selected. All sections are selected by default.

**Only grab specific sections (if you know the slugs):**

```bash
dw2md tinygrad/tinygrad -p "4-react-reconciler,4-1-fiber-architecture-and-data-structures"
```

**Exclude sections you don't need:**

```bash
dw2md tinygrad/tinygrad -x "7-developer-tools-and-debugging"
```

**JSON output for programmatic use:**

```bash
dw2md tinygrad/tinygrad -f json -o react.json
```

**From a DeepWiki URL you already have open:**

```bash
dw2md https://deepwiki.com/anthropics/claude-code
```

**Minimal output — no metadata, no TOC, just content:**

```bash
dw2md tinygrad/tinygrad --no-toc --no-metadata
```

## Output

### Markdown (default)

The output format is designed for LLM and agent workflows — a tree-structured table of contents for fast orientation, and grep-friendly section delimiters so agents can selectively extract the sections they need.

```markdown
<!-- dw2md v0.2.0 | tinygrad/tinygrad | 2026-02-12T15:30:00Z | 29 pages -->

# tinygrad/tinygrad — DeepWiki

> Compiled from https://deepwiki.com/tinygrad/tinygrad
> Generated: 2026-02-12T15:30:00Z | Pages: 29

## Structure

├── 1 Overview
│   └── 1.1 Repository Structure and Packages
├── 2 Feature Flags System
├── 3 Build System and Package Distribution
│   ├── 3.1 Setup.py and Package Config
│   └── 3.2 CI/CD Pipeline
└── 4 Core Architecture

## Contents

<<< SECTION: 1 Overview [1-overview] >>>

[page content with original heading levels preserved]

<<< SECTION: 1.1 Repository Structure and Packages [1-1-repository-structure-and-packages] >>>

[page content]

<<< SECTION: 2 Feature Flags System [2-feature-flags-system] >>>

[page content]
```

#### Why this format?

The `<<< SECTION: Title [slug] >>>` delimiter is designed to be trivially grep-able:

```bash
# List all sections in a compiled wiki
grep "^<<< SECTION:" wiki.md

# Extract a specific section (content between two delimiters)
sed -n '/^<<< SECTION: 1 Overview/,/^<<< SECTION:/p' wiki.md

# Regex captures both title and slug in one match
# ^<<< SECTION: (.+?) \[(.+?)\] >>>$
```

This matters because LLMs and agents working with large documents need to scan structure cheaply, then pull in only the sections relevant to their current task — rather than stuffing the entire document into context.

Other format choices:

- **Tree TOC** — `├──`/`└──` characters show hierarchy at a glance (same as `tree` command), scannable faster than indented bullet lists
- **Original heading levels preserved** — no heading-level bumping; the delimiter handles section boundaries, so page content keeps its source structure
- **Token efficient** — no repeated horizontal rules (`---`), no anchor link markup, no extra `#` characters from heading bumping
- **HTML comment on line 1** — machine-parseable metadata invisible to most renderers
- **Mermaid blocks preserved** as fenced code blocks
- **Source annotations preserved** (`Sources: file.py:1-50`) for code location context

### JSON

With `--format json`:

```json
{
  "repo": "tinygrad/tinygrad",
  "url": "https://deepwiki.com/tinygrad/tinygrad",
  "generated_at": "2026-02-12T15:30:00Z",
  "tool_version": "0.2.0",
  "page_count": 29,
  "pages": [
    {
      "slug": "1-overview",
      "title": "1 Overview",
      "depth": 0,
      "content": "...markdown content..."
    }
  ]
}
```

Useful for feeding individual pages into separate context windows, building retrieval indexes, etc.

## How It Works

`dw2md` is a minimal MCP client that talks to DeepWiki's public JSON-RPC endpoint (`https://mcp.deepwiki.com/mcp`). No API key, no auth, no browser automation.

1. **Initialize** — MCP handshake with the server
2. **Fetch structure** — `read_wiki_structure` returns the table of contents
3. **Fetch content** — `read_wiki_contents` returns all pages in one response
4. **Match & compile** — split content by page markers, match to structure, assemble output

Failed requests are retried 3 times with exponential backoff (1s, 2s, 4s).

## Contributing

Contributions welcome! Please ensure:

- All tests pass: `cargo test`
- Code is formatted: `cargo fmt`
- Clippy is happy: `cargo clippy`

## License

MIT
