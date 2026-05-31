use crate::utils::component_defaults::{DEFAULT_COLORS, DEFAULT_FONT_SIZES};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedExample {
    pub title: String,
    pub description: String,
    pub markup: String,
    pub attributes: serde_json::Value,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BlockData {
    pub block_name: String,
    pub save_wrapper: Option<String>,
    pub uses_inner_blocks: bool,
    pub supports_align: bool,
    pub supports_color: bool,
    pub supports_typography: bool,
    pub supports_spacing: bool,
}

pub fn generate_markup_examples(block: &BlockData) -> Vec<GeneratedExample> {
    let mut examples = Vec::new();
    let block_name = &block.block_name;

    // 1. Basic Usage
    examples.push(GeneratedExample {
        title: "Basic Usage".to_string(),
        description: "Minimal block with no additional attributes".to_string(),
        markup: generate_basic_markup(block),
        attributes: serde_json::Value::Object(serde_json::Map::new()),
        features: Vec::new(),
    });

    // 2. Alignment
    if block.supports_align {
        let mut map = serde_json::Map::new();
        map.insert("align".to_string(), serde_json::Value::String("center".to_string()));
        examples.push(GeneratedExample {
            title: "With Alignment".to_string(),
            description: "Block with text alignment".to_string(),
            markup: generate_alignment_example(block),
            attributes: serde_json::Value::Object(map),
            features: vec!["align".to_string()],
        });
    }

    // 3. Color
    if block.supports_color {
        let mut map = serde_json::Map::new();
        map.insert("backgroundColor".to_string(), serde_json::Value::String("pale-cyan-blue".to_string()));
        map.insert("textColor".to_string(), serde_json::Value::String("vivid-purple".to_string()));
        examples.push(GeneratedExample {
            title: "With Colors".to_string(),
            description: "Block with background and text colors".to_string(),
            markup: generate_color_example(block),
            attributes: serde_json::Value::Object(map),
            features: vec!["color".to_string()],
        });
    }

    // 4. Typography
    if block.supports_typography {
        let mut map = serde_json::Map::new();
        map.insert("fontSize".to_string(), serde_json::Value::String("large".to_string()));
        let mut style_map = serde_json::Map::new();
        let mut typo_map = serde_json::Map::new();
        typo_map.insert("lineHeight".to_string(), serde_json::Value::String("1.5".to_string()));
        style_map.insert("typography".to_string(), serde_json::Value::Object(typo_map));
        map.insert("style".to_string(), serde_json::Value::Object(style_map));

        examples.push(GeneratedExample {
            title: "With Typography".to_string(),
            description: "Block with custom typography settings".to_string(),
            markup: generate_typography_example(block),
            attributes: serde_json::Value::Object(map),
            features: vec!["typography".to_string()],
        });
    }

    // 5. Spacing
    if block.supports_spacing {
        let mut map = serde_json::Map::new();
        let mut style_map = serde_json::Map::new();
        let mut spacing_map = serde_json::Map::new();
        let mut padding_map = serde_json::Map::new();
        padding_map.insert("top".to_string(), serde_json::Value::String("2rem".to_string()));
        padding_map.insert("right".to_string(), serde_json::Value::String("2rem".to_string()));
        padding_map.insert("bottom".to_string(), serde_json::Value::String("2rem".to_string()));
        padding_map.insert("left".to_string(), serde_json::Value::String("2rem".to_string()));
        spacing_map.insert("padding".to_string(), serde_json::Value::Object(padding_map));
        style_map.insert("spacing".to_string(), serde_json::Value::Object(spacing_map));
        map.insert("style".to_string(), serde_json::Value::Object(style_map));

        examples.push(GeneratedExample {
            title: "With Spacing".to_string(),
            description: "Block with padding and margin".to_string(),
            markup: generate_spacing_example(block),
            attributes: serde_json::Value::Object(map),
            features: vec!["spacing".to_string()],
        });
    }

    // 6. Full Featured
    let mut features = Vec::new();
    if block.supports_align { features.push("align".to_string()); }
    if block.supports_color { features.push("color".to_string()); }
    if block.supports_typography { features.push("typography".to_string()); }
    if block.supports_spacing { features.push("spacing".to_string()); }

    examples.push(GeneratedExample {
        title: "Full Featured".to_string(),
        description: "Block with multiple attributes combined".to_string(),
        markup: generate_full_featured_example(block),
        attributes: build_full_attributes(block),
        features,
    });

    examples
}

fn get_wrapper(block: &BlockData) -> &'static str {
    if let Some(ref w) = block.save_wrapper {
        // Since wrapper element can be dynamically allocated, we match standard wrapper tags
        match w.as_str() {
            "h1" => "h1",
            "h2" => "h2",
            "h3" => "h3",
            "h4" => "h4",
            "h5" => "h5",
            "h6" => "h6",
            "p" => "p",
            "div" => "div",
            "span" => "span",
            "ul" => "ul",
            "li" => "li",
            "blockquote" => "blockquote",
            "figure" => "figure",
            "pre" => "pre",
            "table" => "table",
            other => {
                // If it is static but unknown, return a static div to satisfy lifetime
                if other.is_empty() { "div" } else { "div" }
            }
        }
    } else {
        match block.block_name.as_str() {
            "core/heading" => "h2",
            "core/button" => "div",
            "core/list" => "ul",
            "core/list-item" => "li",
            "core/quote" => "blockquote",
            "core/pullquote" => "figure",
            "core/verse" => "pre",
            _ => "div",
        }
    }
}

fn get_block_class(block_name: &str) -> String {
    format!("wp-block-{}", block_name.replace("core/", "").replace('/', "-"))
}

fn get_default_content(block_name: &str) -> &'static str {
    let block_type = block_name.split('/').last().unwrap_or("");
    match block_type {
        "paragraph" => "This is a paragraph block with example content.",
        "heading" => "Example Heading",
        "list" => "<li>List item 1</li><li>List item 2</li><li>List item 3</li>",
        "quote" => "<p>This is a quote block with inspiring content.</p>",
        "code" => "function example() {\n  return \"Hello World\";\n}",
        "preformatted" => "Preformatted text preserves   spaces and\nline breaks.",
        "pullquote" => "<p>A pull quote stands out from the rest of the content.</p>",
        "verse" => "Verse text\npreserves line breaks\nfor poetry",
        "button" => "<a class=\"wp-block-button__link\" href=\"#\">Click Me</a>",
        "table" => "<table><tbody><tr><td>Cell 1</td><td>Cell 2</td></tr></tbody></table>",
        _ => "",
    }
}

fn generate_basic_markup(block: &BlockData) -> String {
    let wrapper = get_wrapper(block);
    let content = get_default_content(&block.block_name);
    let block_class = get_block_class(&block.block_name);

    if content.is_empty() && block.uses_inner_blocks {
        format!(
            "<!-- wp:{} -->\n<{} class=\"{}\"><!-- wp:paragraph -->\n<p>Inner content</p>\n<!-- /wp:paragraph --></{}>\n<!-- /wp:{} -->",
            block.block_name, wrapper, block_class, wrapper, block.block_name
        )
    } else if content.is_empty() {
        format!(
            "<!-- wp:{} -->\n<{} class=\"{}\"></{}>\n<!-- /wp:{} -->",
            block.block_name, wrapper, block_class, wrapper, block.block_name
        )
    } else {
        format!(
            "<!-- wp:{} -->\n<{} class=\"{}\">{}</{}>\n<!-- /wp:{} -->",
            block.block_name, wrapper, block_class, content, wrapper, block.block_name
        )
    }
}

fn generate_alignment_example(block: &BlockData) -> String {
    let wrapper = get_wrapper(block);
    let content = get_default_content(&block.block_name);
    let block_class = get_block_class(&block.block_name);

    format!(
        "<!-- wp:{} {{\"align\":\"center\"}} -->\n<{} class=\"{} has-text-align-center\">{}</{}>\n<!-- /wp:{} -->",
        block.block_name, wrapper, block_class, content, wrapper, block.block_name
    )
}

fn generate_color_example(block: &BlockData) -> String {
    let wrapper = get_wrapper(block);
    let content = get_default_content(&block.block_name);
    let block_class = get_block_class(&block.block_name);
    let color = &DEFAULT_COLORS[9]; // pale cyan blue

    format!(
        "<!-- wp:{} {{\"backgroundColor\":\"{}\",\"textColor\":\"vivid-purple\"}} -->\n<{} class=\"{} has-{}-background-color has-vivid-purple-color has-text-color has-background\">{}</{}>\n<!-- /wp:{} -->",
        block.block_name, color.slug, wrapper, block_class, color.slug, content, wrapper, block.block_name
    )
}

fn generate_typography_example(block: &BlockData) -> String {
    let wrapper = get_wrapper(block);
    let content = get_default_content(&block.block_name);
    let block_class = get_block_class(&block.block_name);
    let size = &DEFAULT_FONT_SIZES[2]; // large

    format!(
        "<!-- wp:{} {{\"fontSize\":\"{}\",\"style\":{{\"typography\":{{\"lineHeight\":\"1.5\"}}}}}} -->\n<{} class=\"{} has-{}-font-size\" style=\"line-height:1.5\">{}</{}>\n<!-- /wp:{} -->",
        block.block_name, size.slug, wrapper, block_class, size.slug, content, wrapper, block.block_name
    )
}

fn generate_spacing_example(block: &BlockData) -> String {
    let wrapper = get_wrapper(block);
    let content = get_default_content(&block.block_name);
    let block_class = get_block_class(&block.block_name);

    format!(
        "<!-- wp:{} {{\"style\":{{\"spacing\":{{\"padding\":{{\"top\":\"2rem\",\"right\":\"2rem\",\"bottom\":\"2rem\",\"left\":\"2rem\"}},\"margin\":{{\"top\":\"1rem\",\"bottom\":\"1rem\"}}}}}}}} -->\n<{} class=\"{}\" style=\"margin-top:1rem;margin-bottom:1rem;padding-top:2rem;padding-right:2rem;padding-bottom:2rem;padding-left:2rem\">{}</{}>\n<!-- /wp:{} -->",
        block.block_name, wrapper, block_class, content, wrapper, block.block_name
    )
}

fn generate_full_featured_example(block: &BlockData) -> String {
    let wrapper = get_wrapper(block);
    let content = get_default_content(&block.block_name);
    let attributes = build_full_attributes(block);
    let mut classes = vec![get_block_class(&block.block_name)];
    let mut styles = Vec::new();

    if block.supports_align {
        classes.push("has-text-align-center".to_string());
    }
    if block.supports_color {
        classes.push("has-pale-cyan-blue-background-color".to_string());
        classes.push("has-vivid-purple-color".to_string());
        classes.push("has-text-color".to_string());
        classes.push("has-background".to_string());
    }
    if block.supports_typography {
        classes.push("has-large-font-size".to_string());
        styles.push("line-height:1.6");
    }
    if block.supports_spacing {
        styles.push("padding-top:1.5rem");
        styles.push("padding-right:1.5rem");
        styles.push("padding-bottom:1.5rem");
        styles.push("padding-left:1.5rem");
    }

    let attr_string = serde_json::to_string(&attributes).unwrap_or_default();
    let class_string = classes.join(" ");
    let style_string = if !styles.is_empty() {
        format!(" style=\"{}\"", styles.join(";"))
    } else {
        String::new()
    };

    format!(
        "<!-- wp:{} {} -->\n<{} class=\"{}\"{}>{}</{}>\n<!-- /wp:{} -->",
        block.block_name, attr_string, wrapper, class_string, style_string, content, wrapper, block.block_name
    )
}

fn build_full_attributes(block: &BlockData) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    if block.supports_align {
        map.insert("align".to_string(), serde_json::Value::String("center".to_string()));
    }
    if block.supports_color {
        map.insert("backgroundColor".to_string(), serde_json::Value::String("pale-cyan-blue".to_string()));
        map.insert("textColor".to_string(), serde_json::Value::String("vivid-purple".to_string()));
    }

    let mut style_map = serde_json::Map::new();

    if block.supports_typography {
        map.insert("fontSize".to_string(), serde_json::Value::String("large".to_string()));
        let mut typo_map = serde_json::Map::new();
        typo_map.insert("lineHeight".to_string(), serde_json::Value::String("1.6".to_string()));
        style_map.insert("typography".to_string(), serde_json::Value::Object(typo_map));
    }

    if block.supports_spacing {
        let mut spacing_map = serde_json::Map::new();
        let mut padding_map = serde_json::Map::new();
        padding_map.insert("top".to_string(), serde_json::Value::String("1.5rem".to_string()));
        padding_map.insert("right".to_string(), serde_json::Value::String("1.5rem".to_string()));
        padding_map.insert("bottom".to_string(), serde_json::Value::String("1.5rem".to_string()));
        padding_map.insert("left".to_string(), serde_json::Value::String("1.5rem".to_string()));
        spacing_map.insert("padding".to_string(), serde_json::Value::Object(padding_map));
        style_map.insert("spacing".to_string(), serde_json::Value::Object(spacing_map));
    }

    if !style_map.is_empty() {
        map.insert("style".to_string(), serde_json::Value::Object(style_map));
    }

    serde_json::Value::Object(map)
}

pub fn generate_dynamic_markup(block_name: &str, attributes: &serde_json::Value) -> String {
    let attr_string = if attributes.is_object() && !attributes.as_object().unwrap().is_empty() {
        format!(" {}", serde_json::to_string(attributes).unwrap_or_default())
    } else {
        String::new()
    };
    format!("<!-- wp:{}{} /-->", block_name, attr_string)
}
