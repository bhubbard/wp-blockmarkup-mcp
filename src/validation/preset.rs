use serde_json::Value;
use crate::validation::block_parser::ParsedBlock;
use regex::Regex;

pub fn wp_to_kebab_case(input: &str) -> String {
    let re = Regex::new(r"([a-z0-9])([A-Z])|([0-9])([a-zA-Z])|([a-zA-Z])([0-9])").unwrap();
    let result = re.replace_all(input, |caps: &regex::Captures| {
        let g1 = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let g2 = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let g3 = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        let g4 = caps.get(4).map(|m| m.as_str()).unwrap_or("");
        let g5 = caps.get(5).map(|m| m.as_str()).unwrap_or("");
        let g6 = caps.get(6).map(|m| m.as_str()).unwrap_or("");

        format!(
            "{}{}{}-{}{}{}",
            g1, g3, g5, g2, g4, g6
        )
    });
    result.to_lowercase()
}

#[derive(Debug)]
pub struct PresetRef {
    pub path: String,
    pub group: String,
    pub slug: String,
}

fn collect_preset_refs(value: &Value, path: &mut Vec<String>, refs: &mut Vec<PresetRef>) {
    match value {
        Value::String(s) => {
            let re = Regex::new(r"^var:preset\|([a-z0-9-]+)\|(.+)$").unwrap();
            if let Some(caps) = re.captures(s) {
                refs.push(PresetRef {
                    path: path.join("."),
                    group: caps[1].to_string(),
                    slug: caps[2].to_string(),
                });
            }
        }
        Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                path.push(i.to_string());
                collect_preset_refs(val, path, refs);
                path.pop();
            }
        }
        Value::Object(obj) => {
            for (k, val) in obj {
                path.push(k.clone());
                collect_preset_refs(val, path, refs);
                path.pop();
            }
        }
        _ => {}
    }
}

fn check_block(block: &ParsedBlock, warnings: &mut Vec<String>) {
    if let Some(ref block_name) = block.block_name {
        let mut refs = Vec::new();
        let mut path = Vec::new();
        if let Some(ref attrs) = block.attrs {
            collect_preset_refs(attrs, &mut path, &mut refs);
        }

        for ref_info in refs {
            let kebab_slug = wp_to_kebab_case(&ref_info.slug);
            if kebab_slug == ref_info.slug {
                continue;
            }

            let canonical_var = format!("--wp--preset--{}--{}", ref_info.group, kebab_slug);
            let wrong_var = format!("--wp--preset--{}--{}", ref_info.group, ref_info.slug);

            let wrong_pattern = format!("var({})", wrong_var);
            if block.inner_html.contains(&wrong_pattern) {
                warnings.push(format!(
                    "Block \"{}\" — attribute path \"{}\" references preset \"{}.{}\" but inline style uses \"{}\" instead of the canonical \"{}\". The editor will flag this as invalid on load.",
                    block_name, ref_info.path, ref_info.group, ref_info.slug, wrong_var, canonical_var
                ));
            }
        }
    }

    for inner in &block.inner_blocks {
        check_block(inner, warnings);
    }
}

pub fn validate_preset_references(parsed_blocks: &[ParsedBlock]) -> Vec<String> {
    let mut warnings = Vec::new();
    for block in parsed_blocks {
        check_block(block, &mut warnings);
    }
    warnings
}

pub fn collect_all_preset_refs(blocks: &[ParsedBlock]) -> Vec<PresetRef> {
    let mut refs = Vec::new();
    for b in blocks {
        collect_refs_recursive(b, &mut refs);
    }
    refs
}

fn collect_refs_recursive(block: &ParsedBlock, refs: &mut Vec<PresetRef>) {
    let mut path = Vec::new();
    if let Some(ref attrs) = block.attrs {
        collect_preset_refs(attrs, &mut path, refs);
    }
    for inner in &block.inner_blocks {
        collect_refs_recursive(inner, refs);
    }
}
