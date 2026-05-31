use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content},
    tool, tool_router,
    transport::io::stdio,
    ErrorData as McpError, ServiceExt,
};
use serde::Deserialize;
use schemars::JsonSchema;
use wp_block_mcp::db::{
    self, get_connection,
    get_block_schema as get_block_schema_db,
    get_block_markup as get_block_markup_db,
    search_blocks as search_blocks_db,
    SearchOpts,
};
use wp_block_mcp::validation::validate_structural;

#[derive(Deserialize, JsonSchema, Default)]
struct SearchBlocksArgs {
    query: String,
    source: Option<String>,
    category: Option<String>,
    block_type: Option<String>,
    include_removed: Option<bool>,
    limit: Option<i64>,
}

#[derive(Deserialize, JsonSchema, Default)]
struct GetBlockSchemaArgs {
    block_name: String,
}

#[derive(Deserialize, JsonSchema, Default)]
struct GetBlockMarkupArgs {
    block_name: String,
    features: Option<Vec<String>>,
    validated_only: Option<bool>,
}

#[derive(Deserialize, JsonSchema, Default)]
struct ValidateMarkupArgs {
    markup: String,
}

#[derive(Deserialize, JsonSchema, Default)]
struct ListBlockAttributesArgs {
    block_name: String,
}

#[derive(Deserialize, JsonSchema, Default)]
struct SearchVariationsArgs {
    query: String,
    limit: Option<i64>,
}

#[derive(Clone)]
struct Server {
    tool_router: ToolRouter<Self>,
}

#[tool_router(server_handler)]
impl Server {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "search_blocks",
        description = "Search Gutenberg blocks across all indexed sources using full-text search. Returns BM25-ranked results with block metadata, type classification, and validation status."
    )]
    async fn search_blocks(
        &self,
        Parameters(args): Parameters<SearchBlocksArgs>,
    ) -> Result<CallToolResult, McpError> {
        let conn = get_connection().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let opts = SearchOpts {
            source: args.source,
            category: args.category,
            block_type: args.block_type,
            include_removed: args.include_removed.unwrap_or(false),
            limit: args.limit.unwrap_or(20),
        };

        let results = search_blocks_db(&conn, &args.query, opts)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if results.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No blocks found matching \"{}\". Try broader search terms or check source indexing with the CLI.",
                args.query
            ))]));
        }

        let mut formatted = Vec::new();
        for (i, r) in results.iter().enumerate() {
            let mut lines = vec![
                format!("### {}. {}", i + 1, r.block.block_name),
                format!(
                    "- **Title:** {} | **Category:** {}",
                    r.block.title.as_deref().unwrap_or("—"),
                    r.block.category.as_deref().unwrap_or("—")
                ),
                format!(
                    "- **Type:** {} | **Validation:** {} | **Confidence:** {}%",
                    r.block.block_type, r.block.validation_status, r.block.confidence
                ),
                format!("- **Source:** {}", r.source_name),
            ];
            if let Some(ref desc) = r.block.description {
                lines.push(format!("- **Description:** {}", desc));
            }
            if r.block.status == "removed" {
                lines.push("- **Status:** REMOVED".to_string());
            }
            formatted.push(lines.join("\n"));
        }

        let text = format!(
            "Found {} block(s) matching \"{}\":\n\n{}",
            results.len(),
            args.query,
            formatted.join("\n\n")
        );
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "get_block_schema",
        description = "Get the complete schema for a Gutenberg block: all attributes with types/defaults, support configurations, variations, and validation status. Use the full block name (e.g. \"core/paragraph\")."
    )]
    async fn get_block_schema(
        &self,
        Parameters(args): Parameters<GetBlockSchemaArgs>,
    ) -> Result<CallToolResult, McpError> {
        let conn = get_connection().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let schema = get_block_schema_db(&conn, &args.block_name)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if schema.is_none() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "Block \"{}\" not found. Use search_blocks to find blocks first.",
                args.block_name
            ))]));
        }
        let s = schema.unwrap();

        let mut sections = vec![
            format!("## {}", s.block.block_name),
            format!(
                "**Title:** {} | **Category:** {}",
                s.block.title.as_deref().unwrap_or("—"),
                s.block.category.as_deref().unwrap_or("—")
            ),
            format!(
                "**Type:** {} | **Validation:** {} | **Confidence:** {}%",
                s.block.block_type, s.block.validation_status, s.block.confidence
            ),
        ];

        if let Some(ref desc) = s.block.description {
            sections.push(format!("**Description:** {}", desc));
        }

        if !s.attributes.is_empty() {
            sections.push(format!("\n### Attributes ({})", s.attributes.len()));
            let mut attr_lines = Vec::new();
            for a in &s.attributes {
                let mut parts = vec![format!("- **{}**: `{}`", a.name, a.r#type.as_deref().unwrap_or("?"))];
                if let Some(ref def) = a.default_val {
                    parts.push(format!("(default: {})", def));
                }
                if let Some(ref enums) = a.enum_values {
                    parts.push(format!("[{}]", enums));
                }
                if let Some(ref src) = a.source {
                    parts.push(format!("source: {}", src));
                }
                if let Some(ref sel) = a.selector {
                    parts.push(format!("selector: {}", sel));
                }
                attr_lines.push(parts.join(" "));
            }
            sections.push(attr_lines.join("\n"));
        }

        if !s.supports.is_empty() {
            sections.push(format!("\n### Supports ({})", s.supports.len()));
            let mut sup_lines = Vec::new();
            for sup in &s.supports {
                sup_lines.push(format!(
                    "- **{}**: {}",
                    sup.feature,
                    sup.config.as_deref().unwrap_or("")
                ));
            }
            sections.push(sup_lines.join("\n"));
        }

        if !s.variations.is_empty() {
            sections.push(format!("\n### Variations ({})", s.variations.len()));
            let mut var_lines = Vec::new();
            for v in &s.variations {
                let desc = v.description.as_ref().map(|d| format!(" — {}", d)).unwrap_or_default();
                var_lines.push(format!(
                    "- **{}**: {}{}",
                    v.name,
                    v.title.as_deref().unwrap_or("—"),
                    desc
                ));
            }
            sections.push(var_lines.join("\n"));
        }

        Ok(CallToolResult::success(vec![Content::text(sections.join("\n"))]))
    }

    #[tool(
        name = "get_block_markup",
        description = "Get validated markup examples for a Gutenberg block. Optionally filter by features used (color, typography, spacing, align). Every returned example has been extracted from the block's actual source."
    )]
    async fn get_block_markup(
        &self,
        Parameters(args): Parameters<GetBlockMarkupArgs>,
    ) -> Result<CallToolResult, McpError> {
        let conn = get_connection().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let opts = db::MarkupOpts {
            features: args.features,
            validated_only: args.validated_only.unwrap_or(false),
        };
        let examples = get_block_markup_db(&conn, &args.block_name, opts)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if examples.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No markup examples found for \"{}\".",
                args.block_name
            ))]));
        }

        let mut formatted = Vec::new();
        for (i, e) in examples.iter().enumerate() {
            let features_used: Vec<String> = e
                .features_used
                .as_ref()
                .and_then(|f| serde_json::from_str(f).ok())
                .unwrap_or_default();
            let feature_str = if !features_used.is_empty() {
                format!(" [{}]", features_used.join(", "))
            } else {
                "".to_string()
            };
            let desc = e.description.as_ref().map(|d| format!("{}\n", d)).unwrap_or_default();
            formatted.push(format!(
                "### {}. {}{} ({})\n{}\n```html\n{}\n```",
                i + 1,
                e.title,
                feature_str,
                e.validation_status,
                desc,
                e.markup
            ));
        }

        let text = format!(
            "{} markup example(s) for \"{}\":\n\n{}",
            examples.len(),
            args.block_name,
            formatted.join("\n\n")
        );
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "validate_markup",
        description = "Validate raw Gutenberg block markup. Checks structural format (comment delimiters, JSON attributes) and verifies the block name exists in indexed sources. For static blocks, checks attribute names/types against the block schema. Also catches preset reference mismatches where var:preset|group|slug expands to a non-kebab CSS variable in inline style (a common cause of editor \"block recovery\" prompts)."
    )]
    async fn validate_markup(
        &self,
        Parameters(args): Parameters<ValidateMarkupArgs>,
    ) -> Result<CallToolResult, McpError> {
        let conn = get_connection().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let result = validate_structural(Some(&conn), &args.markup);

        if !result.valid {
            let mut text = format!("INVALID — {} error(s):\n", result.errors.len());
            for err in &result.errors {
                text.push_str(&format!("  - {}\n", err));
            }
            if !result.warnings.is_empty() {
                text.push_str("\nWarnings:\n");
                for w in &result.warnings {
                    text.push_str(&format!("  - {}\n", w));
                }
            }
            return Ok(CallToolResult::success(vec![Content::text(text)]));
        }

        let blocks: Vec<_> = result.parsed_blocks.iter().filter(|b| b.block_name.is_some()).collect();
        let block_names: Vec<String> = blocks.iter().map(|b| b.block_name.clone().unwrap()).collect();

        // Check block types for status
        let mut schemas = Vec::new();
        for name in &block_names {
            if let Ok(Some(s)) = get_block_schema_db(&conn, name) {
                schemas.push(s);
            }
        }
        let all_dynamic = !schemas.is_empty() && schemas.iter().all(|s| s.block.block_type == "dynamic");

        let status = if all_dynamic {
            "VALID (attributes only — dynamic block)"
        } else if !result.warnings.is_empty() {
            "VALID (with warnings)"
        } else {
            "VALID"
        };

        let mut text = format!(
            "{} — Markup passes structural validation using the WordPress block parser.\n",
            status
        );
        text.push_str(&format!("\nBlocks found: {}\n", block_names.join(", ")));

        if !result.warnings.is_empty() {
            text.push_str("\nWarnings:\n");
            for w in &result.warnings {
                text.push_str(&format!("  - {}\n", w));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "list_block_attributes",
        description = "List all attributes for a Gutenberg block with their types, defaults, allowed values, sources, and selectors. Useful for constructing specific attribute combinations."
    )]
    async fn list_block_attributes(
        &self,
        Parameters(args): Parameters<ListBlockAttributesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let conn = get_connection().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let attrs = db::list_block_attributes(&conn, &args.block_name)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if attrs.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No attributes found for \"{}\". The block may not exist or may have no defined attributes.",
                args.block_name
            ))]));
        }

        let mut formatted = Vec::new();
        for a in &attrs {
            let mut lines = vec![format!("### {}", a.name)];
            lines.push(format!("- **Type:** {}", a.r#type.as_deref().unwrap_or("unknown")));
            if let Some(ref def) = a.default_val {
                lines.push(format!("- **Default:** {}", def));
            }
            if let Some(ref src) = a.source {
                lines.push(format!("- **Source:** {}", src));
            }
            if let Some(ref sel) = a.selector {
                lines.push(format!("- **Selector:** {}", sel));
            }
            if let Some(ref enums) = a.enum_values {
                lines.push(format!("- **Allowed values:** {}", enums));
            }
            formatted.push(lines.join("\n"));
        }

        let text = format!(
            "{} attribute(s) for \"{}\":\n\n{}",
            attrs.len(),
            args.block_name,
            formatted.join("\n\n")
        );
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        name = "search_variations",
        description = "Search Gutenberg block variations by name or description. Returns matching variations with their pre-configured attributes and parent block information."
    )]
    async fn search_variations(
        &self,
        Parameters(args): Parameters<SearchVariationsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let conn = get_connection().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let results = db::search_variations(&conn, &args.query, args.limit.unwrap_or(20))
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if results.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No variations found matching \"{}\".",
                args.query
            ))]));
        }

        let mut formatted = Vec::new();
        for (i, v) in results.iter().enumerate() {
            let title_or_name = v.variation.title.as_deref().unwrap_or(&v.variation.name);
            let mut lines = vec![
                format!("### {}. {}", i + 1, title_or_name),
                format!(
                    "- **Block:** {} ({})",
                    v.block_name,
                    v.block_title.as_deref().unwrap_or("—")
                ),
                format!("- **Variation:** {}", v.variation.name),
                format!("- **Source:** {}", v.source_name),
            ];
            if let Some(ref desc) = v.variation.description {
                lines.push(format!("- **Description:** {}", desc));
            }
            if let Some(ref attrs) = v.variation.attributes {
                lines.push(format!("- **Attributes:** {}", attrs));
            }
            if let Some(ref markup) = v.variation.markup {
                lines.push(format!("- **Markup:**\n```html\n{}\n```", markup));
            }
            formatted.push(lines.join("\n"));
        }

        let text = format!(
            "Found {} variation(s) matching \"{}\":\n\n{}",
            results.len(),
            args.query,
            formatted.join("\n\n")
        );
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CRITICAL: Redirect logs to stderr to avoid corrupting the stdio protocol stream
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let server = Server::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
