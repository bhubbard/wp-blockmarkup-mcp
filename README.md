# WordPress Block Markup MCP & CLI (Rust)

A fast, lightweight, and modern Rust port of [wp-blockmarkup-mcp](https://github.com/pluginslab/wp-blockmarkup-mcp). This tool functions as both a Model Context Protocol (MCP) server for LLMs (like Claude Desktop) and a robust Command Line Interface (CLI) for auditing, linting, formatting, searching, and previewing Gutenberg block markups.

## Features

1. **Model Context Protocol (MCP) Support**: Exposes Gutenberg block schemas, examples, variations, and validation tools to LLMs.
2. **WordPress.org Zip Indexing**: Download and index block plugins directly from WordPress.org by plugin slug.
3. **PHP `render.php` Wrapper Detection**: Parses dynamic block wrapper templates using `get_block_wrapper_attributes()` to bypass strict static class/style validations.
4. **Dynamic JSON Schema Attribute Validation**: Strictly validates parsed block attributes against dynamically generated JSON Schemas using the `jsonschema` crate.
5. **CI Block Linting CLI (`wp-blocks lint`)**: Recursively audits files (`.html`, `.php`, `.json`) in a directory for block structure and attribute violations, outputting exact line numbers.
6. **Auto-Formatting/Repair (`wp-blocks format`)**: Auto-corrects camelCase or wrong preset references (e.g., `vividRed` $\rightarrow$ `vivid-red`) inside both comments (`var:preset|color|vividRed`) and inline styles (`--wp--preset--color--vividRed`) in-place.
7. **Interactive Visual Preview Sandbox (`wp-blocks preview`)**: Injects Gutenberg theme and block CDN styles to render visual block sandbox previews in your default browser on a local port.

---

## Installation & Build

Ensure you have Rust and Cargo installed. Clone the repository and build:

```bash
cargo build --release
```

The output executables will be generated at:
* MCP Server: `target/release/wp-blockmarkup-mcp`
* CLI Tool: `target/release/wp-blocks`

---

## CLI Usage

### 1. Add and Index a Source
Add a plugin or theme repository as a block source. Supports WordPress.org slug, GitHub, or local folders.

```bash
# Add from WordPress.org repository
./target/release/wp-blocks source:add --name coblocks --type wordpress-org --repo coblocks

# Add public GitHub repository
./target/release/wp-blocks source:add --name gutenberg --type github-public --repo https://github.com/WordPress/gutenberg.git --branch trunk

# Add local directory source
./target/release/wp-blocks source:add --name local-theme --type local-folder --path /path/to/my-theme
```

### 2. Manage and Run Indexer
```bash
# List all configured sources
./target/release/wp-blocks source:list

# Re-index all sources (or a specific one)
./target/release/wp-blocks index --source coblocks --force

# View statistics
./target/release/wp-blocks stats
```

### 3. Search and View Schema
```bash
# Search indexed blocks
./target/release/wp-blocks search accordion

# Display full block schema (attributes, supports, variations, examples)
./target/release/wp-blocks schema core/accordion
```

### 4. Lint and Validate Block Markup
```bash
# Validate single block markup string
./target/release/wp-blocks validate '<!-- wp:core/accordion {"autoclose":true} --><div class="wp-block-accordion"></div><!-- /wp:core/accordion -->'

# Recursively lint a directory for markup violations
./target/release/wp-blocks lint --path /path/to/theme
```

### 5. Format / Auto-Fix Markup
Fixes invalid camelCase variable slugs (e.g. `var(--wp--preset--color--vividRed)` $\rightarrow$ `var(--wp--preset--color--vivid-red)`) in-place:
```bash
./target/release/wp-blocks format --path /path/to/theme
```

### 6. Interactive Visual Sandbox
Validate and preview block markup inside a CDN-styled visual browser sandbox:
```bash
./target/release/wp-blocks preview '<!-- wp:core/accordion {"showIcon":true} --><div class="wp-block-accordion"></div><!-- /wp:core/accordion -->'
```

---

## MCP Server Integration

To use this with Claude Desktop, add the server configuration to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "wp-blockmarkup": {
      "command": "/Users/bhubbard/PROJECTS/wp-block-mcp/target/release/wp-blockmarkup-mcp",
      "args": []
    }
  }
}
```

### Exposed MCP Tools

* **`search_blocks`**: Full-text search across indexed block metadata.
* **`get_block_schema`**: Fetches full attributes, layout supports, variations, and HTML markup examples.
* **`get_block_markup`**: Generates valid block markups for any block.
* **`validate_markup`**: Runs structural validation and dynamic attribute type checks against Gutenberg database.
* **`list_block_attributes`**: Retrieves raw attribute schemas for a block.
* **`search_variations`**: Finds configured block variations by name or category.
