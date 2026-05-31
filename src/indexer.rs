use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use sha2::{Sha256, Digest};
use walkdir::WalkDir;
use serde::{Serialize, Deserialize};

use crate::db::{
    self, Block, BlockAttribute, BlockSupport, MarkupExample, Variation, Source,
    list_sources, get_source, get_block_by_name, upsert_block, mark_blocks_removed,
    insert_attributes, insert_supports, insert_markup_example, insert_variation,
    get_indexed_file, upsert_indexed_file, update_block_validation,
};
use crate::source_manager::fetch_source;
use crate::parsers::{
    parse_block_json, parse_save_file, parse_edit_file, parse_variations_file,
};
use crate::utils::markup_generator::{
    generate_markup_examples, generate_dynamic_markup, BlockData, GeneratedExample,
};
use crate::validation::{
    validate_markup, ValidationSchema, ValidationSchemaAttribute, ValidationSchemaSupport,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub sources_processed: i64,
    pub blocks_discovered: i64,
    pub blocks_inserted: i64,
    pub blocks_updated: i64,
    pub blocks_skipped: i64,
    pub blocks_removed: i64,
    pub static_blocks: i64,
    pub dynamic_blocks: i64,
    pub hybrid_blocks: i64,
    pub total_attributes: i64,
    pub total_variations: i64,
    pub total_examples: i64,
    pub verified_examples: i64,
    pub structural_only_examples: i64,
    pub errors: Vec<String>,
}

pub fn index_sources(conn: &mut rusqlite::Connection, source_name: Option<&str>, force: bool) -> Result<IndexStats, String> {
    let sources = if let Some(name) = source_name {
        let source = get_source(conn, name).map_err(|e| e.to_string())?;
        let s = source.ok_or_else(|| format!("Source not found: {}", name))?;
        if !s.enabled {
            return Err(format!("Source \"{}\" is disabled", name));
        }
        vec![s]
    } else {
        list_sources(conn).map_err(|e| e.to_string())?
            .into_iter()
            .filter(|s| s.enabled)
            .collect()
    };

    if sources.is_empty() {
        return Err("No enabled sources found. Add a source first.".to_string());
    }

    let mut stats = IndexStats {
        sources_processed: 0,
        blocks_discovered: 0,
        blocks_inserted: 0,
        blocks_updated: 0,
        blocks_skipped: 0,
        blocks_removed: 0,
        static_blocks: 0,
        dynamic_blocks: 0,
        hybrid_blocks: 0,
        total_attributes: 0,
        total_variations: 0,
        total_examples: 0,
        verified_examples: 0,
        structural_only_examples: 0,
        errors: Vec::new(),
    };

    for source in sources {
        match fetch_source(&source) {
            Ok(local_path) => {
                println!("Indexing source: {} from {:?}", source.name, local_path);
                if let Err(err) = index_source(conn, &source, &local_path, force, &mut stats) {
                    let msg = format!("Error processing source \"{}\": {}", source.name, err);
                    println!("{}", msg);
                    stats.errors.push(msg);
                } else {
                    stats.sources_processed += 1;
                }
            }
            Err(err) => {
                let msg = format!("Error fetching source \"{}\": {}", source.name, err);
                println!("{}", msg);
                stats.errors.push(msg);
            }
        }
    }

    Ok(stats)
}

fn index_source(
    conn: &mut rusqlite::Connection,
    source: &Source,
    local_path: &Path,
    force: bool,
    stats: &mut IndexStats
) -> Result<(), String> {
    let mut block_json_dirs = Vec::new();

    // Walk directory to discover block.json files
    let walker = WalkDir::new(local_path).into_iter();
    for entry in walker.filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        name != "node_modules" && name != "vendor" && name != "dist" && name != "build" && name != ".git" && name != "tests" && name != "test" && name != "__tests__" && name != "spec"
    }) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() && entry.file_name() == "block.json" {
            if let Some(parent) = entry.path().parent() {
                block_json_dirs.push(parent.to_path_buf());
            }
        }
    }

    println!("Found {} block.json files in {}", block_json_dirs.len(), source.name);

    let mut active_block_ids = Vec::new();

    for full_block_dir in block_json_dirs {
        // Get relative path for key indexing
        let relative_path = full_block_dir.strip_prefix(local_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "".to_string());

        let dir_hash = compute_block_dir_hash(&full_block_dir);

        // Incremental skip check
        if !force {
            if let Ok(Some((_, hash))) = get_indexed_file(conn, source.id, &relative_path) {
                if hash == dir_hash {
                    if let Some(block_json) = parse_block_json(&full_block_dir) {
                        if let Some(name) = block_json.get("name").and_then(|v| v.as_str()) {
                            if let Ok(Some(existing)) = get_block_by_name(conn, name) {
                                active_block_ids.push(existing.id);
                            }
                        }
                    }
                    stats.blocks_skipped += 1;
                    continue;
                }
            }
        }

        // Parse block.json
        let block_json = match parse_block_json(&full_block_dir) {
            Some(v) => v,
            None => {
                println!("  Skipping {:?} — invalid block.json", relative_path);
                continue;
            }
        };

        let block_name = match block_json.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => {
                println!("  Skipping {:?} — no valid block name", relative_path);
                continue;
            }
        };

        stats.blocks_discovered += 1;

        // Parse save.js, edit.js, variations.js
        let save_data = parse_save_file(&full_block_dir);
        let edit_data = parse_edit_file(&full_block_dir);
        let variations_data = parse_variations_file(&full_block_dir);

        let block_type = save_data.block_type.clone();
        let confidence = calculate_confidence(true, edit_data.controls.len(), save_data.has_custom_save, save_data.returns_null);

        let db_block = Block {
            id: 0,
            source_id: source.id,
            block_name: block_name.clone(),
            title: block_json.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
            description: block_json.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
            category: block_json.get("category").and_then(|v| v.as_str()).map(|s| s.to_string()),
            block_type: block_type.clone(),
            api_version: block_json.get("apiVersion").and_then(|v| v.as_i64()),
            validation_status: "unverified".to_string(),
            confidence,
            file_path: relative_path.clone(),
            content_hash: Some(dir_hash.clone()),
            status: "active".to_string(),
            removed_at: None,
            first_seen_at: "".to_string(),
            last_seen_at: "".to_string(),
        };

        let upsert_res = match upsert_block(conn, &db_block) {
            Ok(r) => r,
            Err(e) => {
                stats.errors.push(format!("Upsert failed for {}: {}", block_name, e));
                continue;
            }
        };

        let block_id = upsert_res.id;
        active_block_ids.push(block_id);

        match upsert_res.action {
            db::UpsertAction::Inserted => stats.blocks_inserted += 1,
            db::UpsertAction::Updated => stats.blocks_updated += 1,
            db::UpsertAction::Skipped => {
                stats.blocks_skipped += 1;
                continue;
            }
        }

        if block_type == "static" { stats.static_blocks += 1; }
        else if block_type == "dynamic" { stats.dynamic_blocks += 1; }
        else if block_type == "hybrid" { stats.hybrid_blocks += 1; }

        // Save attributes
        if let Some(attrs) = block_json.get("attributes").and_then(|v| v.as_object()) {
            if !attrs.is_empty() {
                let _ = insert_attributes(conn, block_id, attrs);
                stats.total_attributes += attrs.len() as i64;
            }
        }

        // Save supports
        if let Some(supports) = block_json.get("supports").and_then(|v| v.as_object()) {
            let filtered_supports: serde_json::Map<String, serde_json::Value> = supports.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            if !filtered_supports.is_empty() {
                let _ = insert_supports(conn, block_id, &filtered_supports);
            }
        }

        // Save variations
        for var in &variations_data {
            let db_var = Variation {
                id: 0,
                block_id,
                name: var.name.clone(),
                title: var.title.clone(),
                description: var.description.clone(),
                attributes: Some(serde_json::to_string(&var.attributes).unwrap_or_default()),
                inner_blocks: Some(serde_json::to_string(&var.inner_blocks).unwrap_or_default()),
                scope: Some(serde_json::to_string(&var.scope).unwrap_or_default()),
                markup: None,
            };
            let _ = insert_variation(conn, &db_var);
            stats.total_variations += 1;
        }

        // Markup examples generation and validation
        let mut block_validation_status = "unverified".to_string();
        let mut example_statuses = Vec::new();

        if block_type == "dynamic" {
            let dyn_markup = generate_dynamic_markup(&block_name, &serde_json::Value::Object(serde_json::Map::new()));
            let example = MarkupExample {
                id: 0,
                block_id,
                title: "Dynamic Block".to_string(),
                description: Some("Self-closing comment format — HTML rendered server-side by PHP".to_string()),
                markup: dyn_markup,
                attributes_json: Some("{}".to_string()),
                validation_status: "attributes_only".to_string(),
                features_used: Some("[]".to_string()),
            };
            let _ = insert_markup_example(conn, &example);
            stats.total_examples += 1;
            block_validation_status = "attributes_only".to_string();
        } else {
            let val_schema = ValidationSchema {
                block_name: block_name.clone(),
                block_type: block_type.clone(),
                attributes: block_json.get("attributes")
                    .and_then(|v| v.as_object())
                    .unwrap_or(&serde_json::Map::new())
                    .iter()
                    .map(|(k, v)| {
                        let attr_type = if v.is_object() {
                            v.get("type").and_then(|t| {
                                if t.is_array() {
                                    Some(t.as_array().unwrap().iter().filter_map(|val| val.as_str()).collect::<Vec<_>>().join("|"))
                                } else {
                                    t.as_str().map(|s| s.to_string())
                                }
                            })
                        } else {
                            None
                        };
                        let enum_val = v.get("enum").map(|e| e.to_string());
                        ValidationSchemaAttribute {
                            name: k.clone(),
                            r#type: attr_type,
                            enum_values: enum_val,
                        }
                    })
                    .collect(),
                supports: block_json.get("supports")
                    .and_then(|v| v.as_object())
                    .unwrap_or(&serde_json::Map::new())
                    .iter()
                    .map(|(k, v)| ValidationSchemaSupport {
                        feature: k.clone(),
                        config: Some(v.to_string()),
                    })
                    .collect(),
            };

            let block_data = BlockData {
                block_name: block_name.clone(),
                save_wrapper: save_data.wrapper_element.clone(),
                uses_inner_blocks: save_data.uses_inner_blocks,
                supports_align: block_json.get("supports").and_then(|v| v.get("align")).map(|v| !v.is_null() && v != &serde_json::Value::Bool(false)).unwrap_or(false),
                supports_color: block_json.get("supports").and_then(|v| v.get("color")).map(|v| !v.is_null() && v != &serde_json::Value::Bool(false)).unwrap_or(false),
                supports_typography: block_json.get("supports").and_then(|v| v.get("typography")).map(|v| !v.is_null() && v != &serde_json::Value::Bool(false)).unwrap_or(false),
                supports_spacing: block_json.get("supports").and_then(|v| v.get("spacing")).map(|v| !v.is_null() && v != &serde_json::Value::Bool(false)).unwrap_or(false),
            };

            let generated = generate_markup_examples(&block_data);
            for g in generated {
                let val_res = validate_markup(None, &g.markup, Some(&val_schema), Some(&save_data));
                example_statuses.push(val_res.status.clone());

                if val_res.status == "verified" {
                    stats.verified_examples += 1;
                } else if val_res.status == "structural_only" {
                    stats.structural_only_examples += 1;
                }

                let db_example = MarkupExample {
                    id: 0,
                    block_id,
                    title: g.title,
                    description: Some(g.description),
                    markup: g.markup,
                    attributes_json: Some(serde_json::to_string(&g.attributes).unwrap_or_default()),
                    validation_status: val_res.status,
                    features_used: Some(serde_json::to_string(&g.features).unwrap_or_default()),
                };
                let _ = insert_markup_example(conn, &db_example);
                stats.total_examples += 1;
            }

            if !example_statuses.is_empty() && example_statuses.iter().all(|s| s == "verified") {
                block_validation_status = "verified".to_string();
            } else if example_statuses.iter().any(|s| s == "verified") {
                block_validation_status = "structural_only".to_string();
            }
        }

        let _ = update_block_validation(conn, block_id, &block_validation_status);

        // Update indexed file timestamp
        let mtime = fs::metadata(full_block_dir.join("block.json"))
            .and_then(|meta| meta.modified())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        let _ = upsert_indexed_file(conn, source.id, &relative_path, mtime, &dir_hash);
        println!("  ✓ {} ({}, confidence: {}%)", block_name, block_type, confidence);
    }

    // Mark blocks removed that are no longer in active list
    let removed = mark_blocks_removed(conn, source.id, &active_block_ids).map_err(|e| e.to_string())?;
    stats.blocks_removed += removed;
    if removed > 0 {
        println!("  Removed {} blocks no longer in source", removed);
    }

    Ok(())
}

fn compute_block_dir_hash(block_dir: &Path) -> String {
    let mut hasher = Sha256::new();
    let files = ["block.json", "save.js", "edit.js", "variations.js"];
    for file in &files {
        let path = block_dir.join(file);
        if let Ok(content) = fs::read_to_string(&path) {
            hasher.update(content.as_bytes());
        } else {
            hasher.update(format!("__missing__{}", file).as_bytes());
        }
    }
    let result = hasher.finalize();
    let hex_string: String = result.iter().map(|b| format!("{:02x}", b)).collect();
    hex_string[..16].to_string()
}

fn calculate_confidence(has_block_json: bool, edit_controls_count: usize, has_custom_save: bool, returns_null: bool) -> i64 {
    let mut score = 0;
    let mut factors = 0;

    if has_block_json {
        score += 100;
        factors += 1;
    }

    if edit_controls_count > 0 {
        score += 80;
    }
    factors += 1;

    if has_custom_save {
        score += 70;
    } else if returns_null {
        score += 60;
    }
    factors += 1;

    if factors > 0 {
        score / factors
    } else {
        0
    }
}
