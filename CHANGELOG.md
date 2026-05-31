# Changelog

All notable changes to this project will be documented in this file.

## [1.0.0] - 2026-05-30

### Added
- Recreated and ported the entire Node.js/TypeScript `wp-blockmarkup-mcp` project to Rust.
- Implemented recursive-descent Gutenberg block parser in Rust matching WordPress core block syntax.
- Implemented Model Context Protocol (MCP) server endpoints (`search_blocks`, `get_block_schema`, `get_block_markup`, `validate_markup`, `list_block_attributes`, `search_variations`) over stdio.
- Added WordPress.org plugin repository indexing using `curl` and `unzip` to fetch block plugin zips by slug.
- Added PHP `render.php` parsing to support `get_block_wrapper_attributes` dynamic class and style merging.
- Added dynamic JSON Schema attribute validation leveraging the `jsonschema` crate.
- Added recursive block linter subcommand (`wp-blocks lint`) that reports file paths, line numbers, and errors/warnings.
- Added in-place auto-formatter subcommand (`wp-blocks format`) to auto-fix camelCase and non-kebab-case preset variables.
- Added interactive visual block preview sandbox server (`wp-blocks preview`) utilizing standard `TcpListener` and CDN styling.
- Wrote 29 integration tests validating block parsing, save schema checks, preset rules, and AST analysis.
