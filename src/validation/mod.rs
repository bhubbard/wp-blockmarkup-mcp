pub mod block_parser;
pub mod preset;
pub mod save_validator;

use rusqlite::Connection;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use block_parser::{parse, ParsedBlock};
use preset::validate_preset_references;
use save_validator::{validate_save_output, SaveValidationResult};
use crate::parsers::save::SaveData;
use crate::db::get_block_schema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSchemaAttribute {
    pub name: String,
    pub r#type: Option<String>,
    pub enum_values: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSchemaSupport {
    pub feature: String,
    pub config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSchema {
    pub block_name: String,
    pub block_type: String,
    pub attributes: Vec<ValidationSchemaAttribute>,
    pub supports: Vec<ValidationSchemaSupport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub parsed_blocks: Vec<ParsedBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupValidationResult {
    pub status: String, // "verified", "structural_only", "attributes_only", "invalid"
    pub tier1: StructuralResult,
    pub tier2: Option<SaveValidationResult>,
    pub summary: String,
}

pub fn validate_structural(conn: Option<&Connection>, markup: &str) -> StructuralResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if markup.trim().is_empty() {
        return StructuralResult {
            valid: false,
            errors: vec!["Empty or invalid markup".to_string()],
            warnings,
            parsed_blocks: Vec::new(),
        };
    }

    let parsed_blocks = parse(markup);
    if parsed_blocks.is_empty() {
        return StructuralResult {
            valid: false,
            errors: vec!["No blocks found in markup".to_string()],
            warnings,
            parsed_blocks,
        };
    }

    let real_blocks: Vec<&ParsedBlock> = parsed_blocks.iter().filter(|b| b.block_name.is_some()).collect();
    let freeform_blocks: Vec<&ParsedBlock> = parsed_blocks.iter().filter(|b| b.block_name.is_none()).collect();

    if real_blocks.is_empty() {
        return StructuralResult {
            valid: false,
            errors: vec!["No valid block comments found — only freeform HTML".to_string()],
            warnings,
            parsed_blocks,
        };
    }

    for fb in freeform_blocks {
        let content = fb.inner_html.trim();
        if !content.is_empty() {
            let truncated = if content.len() > 60 {
                format!("{}...", &content[..60])
            } else {
                content.to_string()
            };
            warnings.push(format!("Freeform HTML found outside block delimiters: \"{}\"", truncated));
        }
    }

    for block in &real_blocks {
        validate_block(conn, block, &mut errors, &mut warnings);
    }

    let preset_warnings = validate_preset_references(&parsed_blocks);
    warnings.extend(preset_warnings);

    StructuralResult {
        valid: errors.is_empty(),
        errors,
        warnings,
        parsed_blocks,
    }
}

fn validate_block(conn: Option<&Connection>, block: &ParsedBlock, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let block_name = block.block_name.as_ref().unwrap();

    if !block_name.contains('/') {
        warnings.push(format!("Block \"{}\" has no namespace — WordPress will assume \"core/{}\"", block_name, block_name));
    }

    if let Some(c) = conn {
        let schema = get_block_schema(c, block_name)
            .ok()
            .flatten()
            .or_else(|| {
                // Try alternate namespace prefix check
                let alt_name = if block_name.starts_with("core/") {
                    block_name.replace("core/", "")
                } else {
                    format!("core/{}", block_name)
                };
                get_block_schema(c, &alt_name).ok().flatten()
            });

        if let Some(s) = schema {
            let mut val_schema = ValidationSchema {
                block_name: s.block.block_name,
                block_type: s.block.block_type,
                attributes: s.attributes.iter().map(|a| ValidationSchemaAttribute {
                    name: a.name.clone(),
                    r#type: a.r#type.clone(),
                    enum_values: a.enum_values.clone(),
                }).collect(),
                supports: s.supports.iter().map(|sup| ValidationSchemaSupport {
                    feature: sup.feature.clone(),
                    config: sup.config.clone(),
                }).collect(),
            };
            
            let attrs_map = block.attrs.as_ref().and_then(|v| v.as_object());
            if let Some(attrs) = attrs_map {
                validate_attributes(&val_schema, attrs, errors, warnings);
            }
        } else {
            warnings.push(format!("Block \"{}\" not found in any indexed source — cannot verify attributes", block_name));
        }
    }

    for inner in &block.inner_blocks {
        validate_block(conn, inner, errors, warnings);
    }
}

fn validate_attributes(
    schema: &ValidationSchema,
    attrs: &serde_json::Map<String, Value>,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>
) {
    let mut known_attrs = std::collections::HashSet::new();
    for a in &schema.attributes {
        known_attrs.insert(a.name.as_str());
    }

    // Add global block attributes
    let global_attrs = [
        "className", "anchor", "style", "backgroundColor", "textColor",
        "gradient", "fontSize", "fontFamily", "align", "lock", "metadata",
    ];
    for ga in &global_attrs {
        known_attrs.insert(*ga);
    }

    // Derive supports
    for s in &schema.supports {
        let config_obj = s.config.as_ref().and_then(|c| serde_json::from_str::<Value>(c).ok());
        match s.feature.as_str() {
            "typography" => {
                if config_obj.and_then(|c| c.get("textAlign").map(|t| t != &Value::Bool(false))).unwrap_or(true) {
                    known_attrs.insert("textAlign");
                }
            }
            "layout" => {
                known_attrs.insert("layout");
            }
            "color" => {
                if config_obj.and_then(|c| c.get("link").map(|l| l != &Value::Bool(false))).unwrap_or(false) {
                    known_attrs.insert("linkColor");
                }
            }
            "spacing" => {
                if config_obj.and_then(|c| c.get("blockGap").map(|g| g != &Value::Bool(false))).unwrap_or(true) {
                    known_attrs.insert("blockGap");
                }
            }
            "border" => {
                known_attrs.insert("borderColor");
            }
            "position" => {
                known_attrs.insert("position");
            }
            _ => {}
        }
    }

    // Check unknown attrs
    for key in attrs.keys() {
        if !known_attrs.contains(key.as_str()) {
            warnings.push(format!(
                "Unknown attribute \"{}\" on {} — not in schema or global attributes",
                key, schema.block_name
            ));
        }
    }

    // Type checking
    for schema_attr in &schema.attributes {
        if let Some(val) = attrs.get(&schema_attr.name) {
            if let Some(ref attr_type_str) = schema_attr.r#type {
                if attr_type_str == "rich-text" || attr_type_str == "unknown" {
                    continue;
                }

                let actual_type = get_value_type_str(val);
                let allowed_types: Vec<&str> = attr_type_str.split('|').collect();

                let mut is_allowed = allowed_types.contains(&actual_type);
                if !is_allowed && actual_type == "number" && allowed_types.contains(&"integer") {
                    if val.is_i64() || val.as_f64().map(|f| f.fract() == 0.0).unwrap_or(false) {
                        is_allowed = true;
                    }
                }

                if !is_allowed {
                    errors.push(format!(
                        "Attribute \"{}\" on {}: expected {}, got {}",
                        schema_attr.name, schema.block_name, attr_type_str, actual_type
                    ));
                }
            }

            // Check enum
            if let Some(ref enum_str) = schema_attr.enum_values {
                if let Ok(Value::Array(allowed_arr)) = serde_json::from_str::<Value>(enum_str) {
                    if let Some(s) = val.as_str() {
                        let allowed_strings: Vec<&str> = allowed_arr.iter().filter_map(|v| v.as_str()).collect();
                        if !allowed_strings.contains(&s) {
                            errors.push(format!(
                                "Attribute \"{}\" on {}: value \"{}\" not in allowed values [{}]",
                                schema_attr.name, schema.block_name, s, allowed_strings.join(", ")
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn get_value_type_str(val: &Value) -> &'static str {
    match val {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub fn validate_markup(
    conn: Option<&Connection>,
    markup: &str,
    block_schema: Option<&ValidationSchema>,
    save_data: Option<&SaveData>
) -> MarkupValidationResult {
    let tier1 = validate_structural(conn, markup);

    if !tier1.valid {
        let errors_str = tier1.errors.join("; ");
        return MarkupValidationResult {
            status: "invalid".to_string(),
            summary: format!("INVALID — {} structural error(s): {}", tier1.errors.len(), errors_str),
            tier1,
            tier2: None,
        };
    }

    let block_type = block_schema.map(|s| s.block_type.as_str()).unwrap_or("static");
    if block_type == "dynamic" {
        let warnings_count = tier1.warnings.len();
        let summary = format!(
            "VALID (attributes only) — dynamic block, structural check passed{}",
            if warnings_count > 0 { format!(" with {} warning(s)", warnings_count) } else { "".to_string() }
        );
        return MarkupValidationResult {
            status: "attributes_only".to_string(),
            tier1,
            tier2: None,
            summary,
        };
    }

    if let Some(save) = save_data {
        let name = block_schema.map(|s| s.block_name.as_str()).unwrap_or("");
        let tier2 = validate_save_output(markup, name, Some(save));

        if tier2.status == "verified" {
            let summary = format!(
                "VERIFIED — passed structural and save function validation{}",
                if !tier1.warnings.is_empty() { format!(" ({} warning(s))", tier1.warnings.len()) } else { "".to_string() }
            );
            return MarkupValidationResult {
                status: "verified".to_string(),
                tier1,
                tier2: Some(tier2),
                summary,
            };
        }

        let issues_str = tier2.issues.join("; ");
        let summary = format!(
            "STRUCTURAL ONLY — passed Tier 1 but Tier 2 found {} issue(s): {}",
            tier2.issues.len(),
            issues_str
        );
        return MarkupValidationResult {
            status: "structural_only".to_string(),
            tier1,
            tier2: Some(tier2),
            summary,
        };
    }

    let summary = format!(
        "STRUCTURAL ONLY — passed structural validation (no save data for Tier 2){}",
        if !tier1.warnings.is_empty() { format!(" with {} warning(s)", tier1.warnings.len()) } else { "".to_string() }
    );
    MarkupValidationResult {
        status: "structural_only".to_string(),
        tier1,
        tier2: None,
        summary,
    }
}
