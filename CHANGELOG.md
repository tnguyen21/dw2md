# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-12

### Added
- Initial release
- MCP client for DeepWiki's JSON-RPC endpoint
- Fetch wiki structure and contents from any DeepWiki repository
- Compile to markdown or JSON output formats
- `--list` flag to print table of contents
- `--interactive` flag for multi-select page picker
- Page filtering via `--pages` and `--exclude` flags
- Configurable output options: `--no-toc`, `--no-metadata`
- Progress indicators and verbose debug mode
- Retry logic with exponential backoff
- 45 unit and integration tests
- Cross-platform support (Linux, macOS, Windows)
