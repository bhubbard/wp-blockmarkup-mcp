use crate::validation::block_parser::{parse, ParsedBlock};
use crate::parsers::save::SaveData;
use crate::db::Block;
use regex::Regex;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveValidationResult {
    pub valid: bool,
    pub status: String, // "verified", "unverified", "attributes_only"
    pub issues: Vec<String>,
    pub details: Vec<String>,
}

pub fn validate_save_output(
    markup: &str,
    block_name: &str,
    save_data: Option<&SaveData>,
) -> SaveValidationResult {
    let mut issues = Vec::new();
    let mut details = Vec::new();

    let save = match save_data {
        None => {
            return SaveValidationResult {
                valid: true,
                status: "attributes_only".to_string(),
                issues: Vec::new(),
                details: vec!["No save data available for Tier 2".to_string()],
            };
        }
        Some(s) => s,
    };

    if !save.has_custom_save && save.wrapper_element.is_none() {
        return SaveValidationResult {
            valid: true,
            status: "attributes_only".to_string(),
            issues: Vec::new(),
            details: vec!["Dynamic block — save function returns null, only attribute validation applies".to_string()],
        };
    }

    let has_dynamic_wrapper = save.class_patterns.iter().any(|c| c.source.as_deref() == Some("get_block_wrapper_attributes()"));

    // Parse the markup to extract blocks
    let parsed_blocks = parse(markup);
    let block = match parsed_blocks.iter().find(|b| b.block_name.is_some()) {
        None => {
            return SaveValidationResult {
                valid: false,
                status: "unverified".to_string(),
                issues: vec!["No block found in parsed markup".to_string()],
                details: Vec::new(),
            };
        }
        Some(b) => b,
    };

    let inner_html = block.inner_html.trim();

    // 1. Validate wrapper element
    if let Some(ref wrapper) = save.wrapper_element {
        let wrapper_pattern = format!(r"^<{}([\s>])", wrapper);
        let re = Regex::new(&wrapper_pattern).unwrap();
        if !inner_html.is_empty() && !re.is_match(inner_html) {
            let actual_match = Regex::new(r"^<([a-zA-Z0-9]+)").unwrap().captures(inner_html);
            let actual = actual_match.map(|c| c[1].to_string()).unwrap_or_else(|| "unknown".to_string());
            issues.push(format!("Expected wrapper <{}> but found <{}>", wrapper, actual));
        } else {
            details.push(format!("Wrapper element <{}> matches", wrapper));
        }
    }

    // 2. Validate CSS class structure
    let expected_block_class = format!(
        "wp-block-{}",
        block_name
            .replace("core/", "")
            .replace('/', "-")
    );

    if !inner_html.is_empty() && inner_html.contains("class=") {
        let class_re = Regex::new(r#"class="([^"]*)""#).unwrap();
        if let Some(caps) = class_re.captures(inner_html) {
            let classes: Vec<&str> = caps[1].split_whitespace().collect();

            if has_dynamic_wrapper {
                details.push("Block uses get_block_wrapper_attributes() — strict class verification bypassed".to_string());
            } else {
                // Check block class
                if !classes.contains(&expected_block_class.as_str()) {
                    let is_simple_element = ["p", "h1", "h2", "h3", "h4", "h5", "h6", "pre", "code"]
                        .contains(&save.wrapper_element.as_deref().unwrap_or(""));
                    if !is_simple_element {
                        issues.push(format!(
                            "Expected CSS class \"{}\" not found. Present classes: {}",
                            expected_block_class,
                            classes.join(", ")
                        ));
                    } else {
                        details.push("Simple element — block class optional".to_string());
                    }
                } else {
                    details.push(format!("Block class \"{}\" present", expected_block_class));
                }

                // Check color/align feature classes if attrs are set
                if let Some(ref attrs) = block.attrs {
                    if let Some(bg_color) = attrs.get("backgroundColor").and_then(|v| v.as_str()) {
                        let bg_class = format!("has-{}-background-color", bg_color);
                        if !classes.contains(&bg_class.as_str()) {
                            issues.push(format!("Background color attribute \"{}\" set but class \"{}\" not in HTML", bg_color, bg_class));
                        }
                        if !classes.contains(&"has-background") {
                            issues.push("Background color set but \"has-background\" class missing".to_string());
                        }
                    }

                    if let Some(text_color) = attrs.get("textColor").and_then(|v| v.as_str()) {
                        let text_class = format!("has-{}-color", text_color);
                        if !classes.contains(&text_class.as_str()) {
                            issues.push(format!("Text color attribute \"{}\" set but class \"{}\" not in HTML", text_color, text_class));
                        }
                        if !classes.contains(&"has-text-color") {
                            issues.push("Text color set but \"has-text-color\" class missing".to_string());
                        }
                    }

                    if let Some(font_size) = attrs.get("fontSize").and_then(|v| v.as_str()) {
                        let font_class = format!("has-{}-font-size", font_size);
                        if !classes.contains(&font_class.as_str()) {
                            issues.push(format!("Font size attribute \"{}\" set but class \"{}\" not in HTML", font_size, font_class));
                        }
                    }

                    if let Some(align) = attrs.get("align").and_then(|v| v.as_str()) {
                        let align_class = format!("has-text-align-{}", align);
                        let align_class2 = format!("align{}", align);
                        if !classes.contains(&align_class.as_str()) && !classes.contains(&align_class2.as_str()) {
                            issues.push(format!("Align attribute \"{}\" set but neither \"{}\" nor \"{}\" found in classes", align, align_class, align_class2));
                        }
                    }
                }
            }
        }
    }

    // 3. InnerBlocks usage
    if save.uses_inner_blocks {
        if block.inner_blocks.is_empty() && !block.inner_html.contains("<!-- wp:") {
            details.push("Save uses InnerBlocks.Content — basic example has no inner blocks".to_string());
        } else {
            details.push(format!("Save uses InnerBlocks.Content — {} inner block(s) present", block.inner_blocks.len()));
        }
    }

    // 4. Style attribute check
    if let Some(ref attrs) = block.attrs {
        if let Some(style_obj) = attrs.get("style").and_then(|v| v.as_object()) {
            let style_match = Regex::new(r#"style="([^"]*)""#).unwrap().captures(inner_html);
            if let Some(caps) = style_match {
                let style_str = &caps[1];
                
                if has_dynamic_wrapper {
                    details.push("Block uses get_block_wrapper_attributes() — strict style verification bypassed".to_string());
                } else {
                    // Check spacing padding
                    if let Some(padding) = style_obj.get("spacing").and_then(|s| s.get("padding")).and_then(|p| p.as_object()) {
                        if !padding.is_empty() {
                            let padding_props = ["padding-top", "padding-right", "padding-bottom", "padding-left"];
                            let mut has_padding = false;
                            for prop in &padding_props {
                                if style_str.contains(prop) {
                                    has_padding = true;
                                    break;
                                }
                            }
                            if !has_padding && !style_str.contains("padding:") {
                                issues.push("Spacing padding set in attrs but not found in style attribute".to_string());
                            }
                        }
                    }

                    // Check spacing margin
                    if let Some(margin) = style_obj.get("spacing").and_then(|s| s.get("margin")).and_then(|m| m.as_object()) {
                        let margin_props = ["margin-top", "margin-bottom"];
                        for prop in &margin_props {
                            let short_key = prop.replace("margin-", "");
                            if margin.get(&short_key).is_some() && !style_str.contains(prop) && !style_str.contains("margin:") {
                                issues.push(format!("Spacing margin set in attrs but \"{}\" not found in style attribute", prop));
                            }
                        }
                    }

                    // Check typography
                    if let Some(typo) = style_obj.get("typography").and_then(|t| t.as_object()) {
                        if typo.get("lineHeight").is_some() && !style_str.contains("line-height") {
                            issues.push("Typography lineHeight set in attrs but \"line-height\" not in style attribute".to_string());
                        }
                    }

                    details.push("Style attribute present and checked against attributes".to_string());
                }
            } else if style_obj.get("spacing").is_some() || style_obj.get("typography").is_some() {
                if !has_dynamic_wrapper {
                    issues.push("Style attributes set in comment but no style attribute found in HTML".to_string());
                }
            }
        }
    }

    // 5. Closing delimiter presence
    if !markup.contains(&format!("<!-- /wp:{}", block_name)) && !markup.contains("/-->") {
        issues.push("Missing closing block comment delimiter".to_string());
    }

    let valid = issues.is_empty();
    SaveValidationResult {
        valid,
        status: if valid { "verified".to_string() } else { "unverified".to_string() },
        issues,
        details,
    }
}
