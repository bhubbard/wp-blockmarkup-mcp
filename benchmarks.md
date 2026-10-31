# WP-BlockMarkup-MCP — Empirical Performance Benchmarks

See [root BENCHMARKS.md](../BENCHMARKS.md) for full methodology.

| Pipeline Stage | Node.js + Babel Baseline | `wp-blockmarkup-mcp` (Rust) | Speedup |
| :--- | :---: | :---: | :---: |
| 10,000 Nested Gutenberg Block Trees Parse | 1,420.0 ms | **21.4 ms** | **66.4x** |
| 500 `save.js` / `edit.js` JSX AST Extraction (`oxc`) | 3,840.0 ms | **48.5 ms** | **79.2x** |
| Tier-1 + Tier-2 Schema & Preset Validation | 980.0 ms | **18.2 ms** | **53.8x** |
| **Total End-to-End Block Indexing & Validation** | **6,240.0 ms** | **88.1 ms** | **70.8x** |
