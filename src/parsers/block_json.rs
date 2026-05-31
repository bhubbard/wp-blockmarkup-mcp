use std::fs;
use std::path::Path;
use serde_json::Value;

pub fn parse_block_json(block_path: &Path) -> Option<Value> {
    let block_json_path = block_path.join("block.json");
    if !block_json_path.exists() {
        return None;
    }

    let content = match fs::read_to_string(&block_json_path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let raw: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return None,
    };

    // Extract attributes
    let raw_attributes = raw.get("attributes").and_then(|v| v.as_object());
    let mut attributes = serde_json::Map::new();
    if let Some(attrs) = raw_attributes {
        for (name, config) in attrs {
            let mut extracted = serde_json::Map::new();
            
            let attr_type = config.get("type").cloned().unwrap_or(Value::String("unknown".to_string()));
            let default_val = config.get("default").cloned().unwrap_or(Value::Null);
            let source = config.get("source").cloned().unwrap_or(Value::Null);
            let selector = config.get("selector").cloned().unwrap_or(Value::Null);
            let attribute = config.get("attribute").cloned().unwrap_or(Value::Null);
            let query = config.get("query").cloned().unwrap_or(Value::Null);
            let enum_val = config.get("enum").cloned().unwrap_or(Value::Null);
            let role = config.get("role").cloned().unwrap_or(Value::Null);
            let multiline = config.get("multiline").cloned().unwrap_or(Value::Null);

            extracted.insert("type".to_string(), attr_type);
            extracted.insert("default".to_string(), default_val);
            extracted.insert("source".to_string(), source);
            extracted.insert("selector".to_string(), selector);
            extracted.insert("attribute".to_string(), attribute);
            extracted.insert("query".to_string(), query);
            extracted.insert("enum".to_string(), enum_val);
            extracted.insert("role".to_string(), role);
            extracted.insert("multiline".to_string(), multiline);

            attributes.insert(name.clone(), Value::Object(extracted));
        }
    }

    // Extract supports
    let raw_supports = raw.get("supports").and_then(|v| v.as_object());
    let mut supports = serde_json::Map::new();
    
    let border = raw_supports.and_then(|s| s.get("__experimentalBorder").or_else(|| s.get("border")));
    supports.insert("border".to_string(), extract_border_support(border));

    if let Some(s) = raw_supports {
        supports.insert("align".to_string(), extract_align_support(s.get("align")));
        supports.insert("alignWide".to_string(), s.get("alignWide").cloned().unwrap_or(Value::Null));
        supports.insert("anchor".to_string(), s.get("anchor").cloned().unwrap_or(Value::Null));
        supports.insert("ariaLabel".to_string(), s.get("ariaLabel").cloned().unwrap_or(Value::Null));
        supports.insert("className".to_string(), s.get("className").cloned().unwrap_or(Value::Null));
        supports.insert("color".to_string(), extract_color_support(s.get("color")));
        supports.insert("customClassName".to_string(), s.get("customClassName").cloned().unwrap_or(Value::Null));
        supports.insert("defaultStylePicker".to_string(), s.get("defaultStylePicker").cloned().unwrap_or(Value::Null));
        supports.insert("html".to_string(), s.get("html").cloned().unwrap_or(Value::Null));
        supports.insert("inserter".to_string(), s.get("inserter").cloned().unwrap_or(Value::Null));
        supports.insert("multiple".to_string(), s.get("multiple").cloned().unwrap_or(Value::Null));
        supports.insert("reusable".to_string(), s.get("reusable").cloned().unwrap_or(Value::Null));
        supports.insert("spacing".to_string(), extract_spacing_support(s.get("spacing")));
        supports.insert("typography".to_string(), extract_typography_support(s.get("typography")));
        supports.insert("dimensions".to_string(), extract_dimensions_support(s.get("dimensions")));
        supports.insert("position".to_string(), extract_position_support(s.get("position")));
        supports.insert("layout".to_string(), s.get("layout").cloned().unwrap_or(Value::Null));
        supports.insert("interactivity".to_string(), s.get("interactivity").cloned().unwrap_or(Value::Null));
        supports.insert("lock".to_string(), s.get("lock").cloned().unwrap_or(Value::Null));
        supports.insert("splitting".to_string(), s.get("splitting").cloned().unwrap_or(Value::Null));
    } else {
        // default empty support keys
        let keys = vec![
            "align", "alignWide", "anchor", "ariaLabel", "className", "color",
            "customClassName", "defaultStylePicker", "html", "inserter", "multiple",
            "reusable", "spacing", "typography", "dimensions", "position",
            "layout", "interactivity", "lock", "splitting"
        ];
        for k in keys {
            supports.insert(k.to_string(), Value::Null);
        }
    }

    let mut result = serde_json::Map::new();
    result.insert("name".to_string(), raw.get("name").cloned().unwrap_or(Value::Null));
    result.insert("title".to_string(), raw.get("title").cloned().unwrap_or(Value::Null));
    result.insert("category".to_string(), raw.get("category").cloned().unwrap_or(Value::Null));
    result.insert("description".to_string(), raw.get("description").cloned().unwrap_or(Value::Null));
    
    let keywords = raw.get("keywords").and_then(|k| k.as_array()).cloned().unwrap_or_default();
    result.insert("keywords".to_string(), Value::Array(keywords));
    
    result.insert("attributes".to_string(), Value::Object(attributes));
    result.insert("supports".to_string(), Value::Object(supports));
    
    result.insert("parent".to_string(), raw.get("parent").cloned().unwrap_or(Value::Null));
    result.insert("ancestor".to_string(), raw.get("ancestor").cloned().unwrap_or(Value::Null));
    result.insert("providesContext".to_string(), raw.get("providesContext").cloned().unwrap_or(Value::Null));
    
    let uses_context = raw.get("usesContext").and_then(|u| u.as_array()).cloned().unwrap_or_default();
    result.insert("usesContext".to_string(), Value::Array(uses_context));
    
    result.insert("example".to_string(), raw.get("example").cloned().unwrap_or(Value::Null));
    result.insert("apiVersion".to_string(), raw.get("apiVersion").cloned().unwrap_or(Value::from(2)));

    Some(Value::Object(result))
}

fn extract_align_support(align: Option<&Value>) -> Value {
    match align {
        None => Value::Null,
        Some(v) => {
            if v.is_boolean() {
                if v.as_bool().unwrap() {
                    serde_json::json!(["left", "center", "right", "wide", "full"])
                } else {
                    Value::Bool(false)
                }
            } else if v.is_array() {
                v.clone()
            } else {
                Value::Null
            }
        }
    }
}

fn extract_color_support(color: Option<&Value>) -> Value {
    match color {
        None => Value::Null,
        Some(v) => {
            if v.is_boolean() {
                if v.as_bool().unwrap() {
                    serde_json::json!({ "enabled": true })
                } else {
                    Value::Null
                }
            } else if v.is_object() {
                let mut map = serde_json::Map::new();
                map.insert("enabled".to_string(), Value::Bool(true));
                if let Some(obj) = v.as_object() {
                    map.insert("background".to_string(), obj.get("background").cloned().unwrap_or(Value::Null));
                    map.insert("gradients".to_string(), obj.get("gradients").cloned().unwrap_or(Value::Null));
                    map.insert("text".to_string(), obj.get("text").cloned().unwrap_or(Value::Null));
                    map.insert("link".to_string(), obj.get("link").cloned().unwrap_or(Value::Null));
                    map.insert("button".to_string(), obj.get("button").cloned().unwrap_or(Value::Null));
                    map.insert("heading".to_string(), obj.get("heading").cloned().unwrap_or(Value::Null));
                }
                Value::Object(map)
            } else {
                Value::Null
            }
        }
    }
}

fn extract_spacing_support(spacing: Option<&Value>) -> Value {
    match spacing {
        None => Value::Null,
        Some(v) => {
            if v.is_boolean() {
                if v.as_bool().unwrap() {
                    serde_json::json!({ "enabled": true })
                } else {
                    Value::Null
                }
            } else if v.is_object() {
                let mut map = serde_json::Map::new();
                map.insert("enabled".to_string(), Value::Bool(true));
                if let Some(obj) = v.as_object() {
                    map.insert("margin".to_string(), obj.get("margin").cloned().unwrap_or(Value::Null));
                    map.insert("padding".to_string(), obj.get("padding").cloned().unwrap_or(Value::Null));
                    map.insert("blockGap".to_string(), obj.get("blockGap").cloned().unwrap_or(Value::Null));
                }
                Value::Object(map)
            } else {
                Value::Null
            }
        }
    }
}

fn extract_typography_support(typography: Option<&Value>) -> Value {
    match typography {
        None => Value::Null,
        Some(v) => {
            if v.is_boolean() {
                if v.as_bool().unwrap() {
                    serde_json::json!({ "enabled": true })
                } else {
                    Value::Null
                }
            } else if v.is_object() {
                let mut map = serde_json::Map::new();
                map.insert("enabled".to_string(), Value::Bool(true));
                if let Some(obj) = v.as_object() {
                    map.insert("fontSize".to_string(), obj.get("fontSize").cloned().unwrap_or(Value::Null));
                    map.insert("lineHeight".to_string(), obj.get("lineHeight").cloned().unwrap_or(Value::Null));
                    map.insert("textAlign".to_string(), obj.get("textAlign").cloned().unwrap_or(Value::Null));
                    map.insert("fontFamily".to_string(), obj.get("__experimentalFontFamily").cloned().unwrap_or(Value::Null));
                    map.insert("fontStyle".to_string(), obj.get("__experimentalFontStyle").cloned().unwrap_or(Value::Null));
                    map.insert("fontWeight".to_string(), obj.get("__experimentalFontWeight").cloned().unwrap_or(Value::Null));
                    map.insert("letterSpacing".to_string(), obj.get("__experimentalLetterSpacing").cloned().unwrap_or(Value::Null));
                    map.insert("textDecoration".to_string(), obj.get("__experimentalTextDecoration").cloned().unwrap_or(Value::Null));
                    map.insert("textTransform".to_string(), obj.get("__experimentalTextTransform").cloned().unwrap_or(Value::Null));
                    map.insert("writingMode".to_string(), obj.get("__experimentalWritingMode").cloned().unwrap_or(Value::Null));
                }
                Value::Object(map)
            } else {
                Value::Null
            }
        }
    }
}

fn extract_border_support(border: Option<&Value>) -> Value {
    match border {
        None => Value::Null,
        Some(v) => {
            if v.is_boolean() {
                if v.as_bool().unwrap() {
                    serde_json::json!({ "enabled": true })
                } else {
                    Value::Null
                }
            } else if v.is_object() {
                let mut map = serde_json::Map::new();
                map.insert("enabled".to_string(), Value::Bool(true));
                if let Some(obj) = v.as_object() {
                    map.insert("color".to_string(), obj.get("color").cloned().unwrap_or(Value::Null));
                    map.insert("radius".to_string(), obj.get("radius").cloned().unwrap_or(Value::Null));
                    map.insert("style".to_string(), obj.get("style").cloned().unwrap_or(Value::Null));
                    map.insert("width".to_string(), obj.get("width").cloned().unwrap_or(Value::Null));
                }
                Value::Object(map)
            } else {
                Value::Null
            }
        }
    }
}

fn extract_dimensions_support(dimensions: Option<&Value>) -> Value {
    match dimensions {
        None => Value::Null,
        Some(v) => {
            if v.is_boolean() {
                if v.as_bool().unwrap() {
                    serde_json::json!({ "enabled": true })
                } else {
                    Value::Null
                }
            } else if v.is_object() {
                let mut map = serde_json::Map::new();
                map.insert("enabled".to_string(), Value::Bool(true));
                if let Some(obj) = v.as_object() {
                    map.insert("minHeight".to_string(), obj.get("minHeight").cloned().unwrap_or(Value::Null));
                    map.insert("aspectRatio".to_string(), obj.get("aspectRatio").cloned().unwrap_or(Value::Null));
                }
                Value::Object(map)
            } else {
                Value::Null
            }
        }
    }
}

fn extract_position_support(position: Option<&Value>) -> Value {
    match position {
        None => Value::Null,
        Some(v) => {
            if v.is_boolean() {
                if v.as_bool().unwrap() {
                    serde_json::json!({ "enabled": true })
                } else {
                    Value::Null
                }
            } else if v.is_object() {
                let mut map = serde_json::Map::new();
                map.insert("enabled".to_string(), Value::Bool(true));
                if let Some(obj) = v.as_object() {
                    map.insert("sticky".to_string(), obj.get("sticky").cloned().unwrap_or(Value::Null));
                }
                Value::Object(map)
            } else {
                Value::Null
            }
        }
    }
}
