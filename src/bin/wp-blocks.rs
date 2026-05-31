use clap::{Parser, Subcommand};
use std::error::Error;
use wp_block_mcp::db::{
    self, add_source, get_connection, get_source, list_sources, remove_source, is_source_indexed,
    search_blocks, get_block_schema, get_stats, rebuild_fts_index, Source, SearchOpts,
};
use wp_block_mcp::indexer::{index_sources, IndexStats};
use wp_block_mcp::validation::block_parser::parse;
use wp_block_mcp::validation::validate_structural;

#[derive(Parser)]
#[command(name = "wp-blocks")]
#[command(author, version = "0.1.0", about = "Gutenberg block markup indexer and search CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(name = "source:add", about = "Add a new block source and index it")]
    SourceAdd {
        #[arg(long, required = true)]
        name: String,
        #[arg(long, required = true)]
        r#type: String, // github-public, github-private, local-folder
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        subfolder: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long = "token-env")]
        token_env: Option<String>,
        #[arg(long, default_value = "main")]
        branch: String,
        #[arg(long = "no-index", default_value_t = false, action = clap::ArgAction::SetTrue)]
        no_index: bool,
    },
    #[command(name = "source:list", about = "List all configured sources")]
    SourceList,
    #[command(name = "source:remove", about = "Remove a source and all its indexed data")]
    SourceRemove {
        name: String,
    },
    #[command(name = "index", about = "Re-index all enabled sources (or a specific one)")]
    Index {
        #[arg(long)]
        source: Option<String>,
        #[arg(long, default_value_t = false, action = clap::ArgAction::SetTrue)]
        force: bool,
    },
    #[command(name = "search", about = "Search indexed blocks by name, title, description, or category")]
    Search {
        query: String,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        r#type: Option<String>, // static, dynamic, hybrid
        #[arg(long, default_value = "20")]
        limit: i64,
        #[arg(long = "include-removed", default_value_t = false, action = clap::ArgAction::SetTrue)]
        include_removed: bool,
    },
    #[command(name = "schema", about = "Show full schema for a block (attributes, supports, variations, examples)")]
    Schema {
        #[arg(name = "block-name")]
        block_name: String,
    },
    #[command(name = "stats", about = "Show indexing statistics and validation coverage")]
    Stats,
    #[command(name = "validate", about = "Validate raw Gutenberg block markup (structural check + attribute verification)")]
    Validate {
        markup: String,
    },
    #[command(name = "rebuild-index", about = "Rebuild FTS indexes (recovery for out-of-sync full-text search)")]
    RebuildIndex,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let mut conn = get_connection()?;

    match cli.command {
        Commands::SourceAdd {
            name,
            r#type,
            repo,
            subfolder,
            path,
            token_env,
            branch,
            no_index,
        } => {
            let existing = get_source(&conn, &name)?;
            if existing.is_some() {
                eprintln!("Source \"{}\" already exists. Remove it first.", name);
                std::process::exit(1);
            }

            let source = Source {
                id: 0,
                name: name.clone(),
                r#type,
                repo_url: repo,
                subfolder,
                local_path: path,
                token_env_var: token_env,
                branch: Some(branch),
                enabled: true,
            };

            add_source(&conn, &source)?;
            println!("Source \"{}\" added successfully.", name);

            if !no_index {
                println!("\nIndexing \"{}\"...", name);
                match index_sources(&mut conn, Some(&name), false) {
                    Ok(stats) => print_index_stats(&stats),
                    Err(e) => {
                        eprintln!("Error indexing: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::SourceList => {
            let sources = list_sources(&conn)?;
            if sources.is_empty() {
                println!("No sources configured. Use \"wp-blocks source:add\" to add one.");
                return Ok(());
            }

            println!("\n{:<25} {:<18} {:<10} {:<10} Details", "Name", "Type", "Branch", "Indexed");
            println!("{}", "-".repeat(95));

            for s in sources {
                let details = s.repo_url.clone().or_else(|| s.local_path.clone()).unwrap_or_default();
                let subfolder = s.subfolder.as_ref().map(|sub| format!(" [{}]", sub)).unwrap_or_default();
                let indexed = if is_source_indexed(&conn, s.id)? { "yes" } else { "no" };
                let branch_name = s.branch.as_deref().unwrap_or("main");
                println!(
                    "{:<25} {:<18} {:<10} {:<10} {}{}",
                    s.name, s.r#type, branch_name, indexed, details, subfolder
                );
            }
            println!();
        }
        Commands::SourceRemove { name } => {
            let removed = remove_source(&conn, &name)?;
            if removed.is_none() {
                eprintln!("Source \"{}\" not found.", name);
                std::process::exit(1);
            }
            println!("Source \"{}\" removed along with all indexed data.", name);
        }
        Commands::Index { source, force } => {
            match index_sources(&mut conn, source.as_deref(), force) {
                Ok(stats) => print_index_stats(&stats),
                Err(e) => {
                    eprintln!("Error indexing: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Search {
            query,
            source,
            category,
            r#type,
            limit,
            include_removed,
        } => {
            let opts = SearchOpts {
                source,
                category,
                block_type: r#type,
                include_removed,
                limit,
            };
            let results = search_blocks(&conn, &query, opts)?;

            if results.is_empty() {
                println!("No blocks found matching \"{}\".", query);
                return Ok(());
            }

            println!("\nFound {} block(s) matching \"{}\":\n", results.len(), query);

            for r in results {
                println!("  {}", r.block.block_name);
                println!(
                    "    Title: {} | Category: {}",
                    r.block.title.as_deref().unwrap_or("—"),
                    r.block.category.as_deref().unwrap_or("—")
                );
                println!(
                    "    Type: {} | Validation: {} | Confidence: {}%",
                    r.block.block_type, r.block.validation_status, r.block.confidence
                );
                println!("    Source: {}", r.source_name);
                if let Some(desc) = r.block.description {
                    println!("    Description: {}", desc);
                }
                if r.block.status == "removed" {
                    println!("    Status: REMOVED");
                }
                println!();
            }
        }
        Commands::Schema { block_name } => {
            let schema = get_block_schema(&conn, &block_name)?;
            if schema.is_none() {
                eprintln!("Block \"{}\" not found.", block_name);
                std::process::exit(1);
            }
            let schema = schema.unwrap();

            println!("\n## {}", schema.block.block_name);
            println!("Title: {}", schema.block.title.as_deref().unwrap_or("—"));
            println!("Category: {}", schema.block.category.as_deref().unwrap_or("—"));
            println!(
                "Type: {} | Validation: {} | Confidence: {}%",
                schema.block.block_type, schema.block.validation_status, schema.block.confidence
            );
            if let Some(desc) = schema.block.description {
                println!("Description: {}", desc);
            }

            if !schema.attributes.is_empty() {
                println!("\nAttributes ({}):", schema.attributes.len());
                for a in schema.attributes {
                    let def = a.default_val.as_ref().map(|d| format!(" = {}", d)).unwrap_or_default();
                    let enums = a.enum_values.as_ref().map(|e| format!(" [{}]", e)).unwrap_or_default();
                    let attr_type = a.r#type.as_deref().unwrap_or("?");
                    println!("  {}: {}{}{}", a.name, attr_type, def, enums);
                }
            }

            if !schema.supports.is_empty() {
                println!("\nSupports ({}):", schema.supports.len());
                for s in schema.supports {
                    let config_str = s.config.as_deref().unwrap_or("");
                    println!("  {}: {}", s.feature, config_str);
                }
            }

            if !schema.variations.is_empty() {
                println!("\nVariations ({}):", schema.variations.len());
                for v in schema.variations {
                    println!("  {}: {}", v.name, v.title.as_deref().unwrap_or("—"));
                }
            }

            if !schema.examples.is_empty() {
                println!("\nMarkup Examples ({}):", schema.examples.len());
                for e in schema.examples {
                    println!("\n  --- {} ({}) ---", e.title, e.validation_status);
                    let indented: Vec<String> = e.markup.split('\n').map(|l| format!("  {}", l)).collect();
                    println!("{}", indented.join("\n"));
                }
            }
            println!();
        }
        Commands::Stats => {
            let stats = get_stats(&conn)?;

            println!("\nOverall Statistics:");
            println!("  Sources:          {}", stats.totals.sources);
            println!("  Active blocks:    {}", stats.totals.active_blocks);
            println!("  Removed blocks:   {}", stats.totals.removed_blocks);
            println!("  Static blocks:    {}", stats.totals.static_blocks);
            println!("  Dynamic blocks:   {}", stats.totals.dynamic_blocks);
            println!("  Hybrid blocks:    {}", stats.totals.hybrid_blocks);
            println!("  Verified blocks:  {}", stats.totals.verified_blocks);
            println!("  Total attributes: {}", stats.totals.total_attributes);
            println!("  Total variations: {}", stats.totals.total_variations);
            println!("  Total examples:   {}", stats.totals.total_examples);

            if !stats.per_source.is_empty() {
                println!("\nPer Source:");
                println!(
                    "  {:<25} {:<8} {:<8} {:<9} {:<10} {:<7} Vars",
                    "Name", "Blocks", "Static", "Dynamic", "Verified", "Attrs"
                );
                println!("  {}", "-".repeat(80));
                for s in stats.per_source {
                    println!(
                        "  {:<25} {:<8} {:<8} {:<9} {:<10} {:<7} {}",
                        s.name, s.blocks, s.static_blocks, s.dynamic_blocks, s.verified_blocks, s.attributes, s.variations
                    );
                }
            }
            println!();
        }
        Commands::Validate { markup } => {
            let result = validate_structural(Some(&conn), &markup);

            if result.valid {
                println!("\nVALID — Structural validation passed.");
            } else {
                println!("\nINVALID — {} error(s):", result.errors.len());
                for err in result.errors {
                    println!("  - {}", err);
                }
            }

            if !result.warnings.is_empty() {
                println!("\nWarnings ({}):", result.warnings.len());
                for w in result.warnings {
                    println!("  - {}", w);
                }
            }

            let blocks: Vec<_> = result.parsed_blocks.iter().filter(|b| b.block_name.is_some()).collect();
            if !blocks.is_empty() {
                println!("\nParsed {} block(s):", blocks.len());
                for b in blocks {
                    let block_name = b.block_name.as_deref().unwrap_or("—");
                    let attr_count = b.attrs.as_ref().and_then(|a| a.as_object()).map(|o| o.len()).unwrap_or(0);
                    println!(
                        "  - {} ({} attributes, {} inner blocks)",
                        block_name, attr_count, b.inner_blocks.len()
                    );
                }
            }
            println!();
        }
        Commands::RebuildIndex => {
            let mut conn_mut = get_connection()?;
            rebuild_fts_index(&mut conn_mut)?;
            println!("FTS indexes rebuilt successfully.");
        }
    }

    Ok(())
}

fn print_index_stats(stats: &IndexStats) {
    println!("\nIndexing complete:");
    println!("  Sources processed:  {}", stats.sources_processed);
    println!("  Blocks discovered:  {}", stats.blocks_discovered);
    println!("  Blocks inserted:    {}", stats.blocks_inserted);
    println!("  Blocks updated:     {}", stats.blocks_updated);
    println!("  Blocks unchanged:   {}", stats.blocks_skipped);
    println!("  Blocks removed:     {}", stats.blocks_removed);
    println!(
        "  Block types:        static={}, dynamic={}, hybrid={}",
        stats.static_blocks, stats.dynamic_blocks, stats.hybrid_blocks
    );
    println!("  Total attributes:   {}", stats.total_attributes);
    println!("  Total variations:   {}", stats.total_variations);
    println!("  Total examples:     {}", stats.total_examples);
    println!("  Verified examples:  {}", stats.verified_examples);
    println!("  Structural only:    {}", stats.structural_only_examples);

    if !stats.errors.is_empty() {
        println!("\n  Errors ({}):", stats.errors.len());
        for err in &stats.errors {
            println!("    - {}", err);
        }
    }
}
