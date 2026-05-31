use wp_block_mcp::validation::block_parser::{parse, ParsedBlock};
use serde_json::json;

#[test]
fn test_parse_simple_self_closing_block() {
    let markup = "<!-- wp:core/latest-posts /-->";
    let result = parse(markup);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_name, Some("core/latest-posts".to_string()));
    assert!(result[0].attrs.is_some());
    assert_eq!(result[0].inner_blocks.len(), 0);
    assert_eq!(result[0].inner_html, "");
}

#[test]
fn test_parse_self_closing_block_with_attrs() {
    let markup = r#"<!-- wp:core/latest-posts {"postsToShow":5,"displayPostDate":true} /-->"#;
    let result = parse(markup);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_name, Some("core/latest-posts".to_string()));
    assert_eq!(
        result[0].attrs,
        Some(json!({
            "postsToShow": 5,
            "displayPostDate": true
        }))
    );
}

#[test]
fn test_parse_block_with_content() {
    let markup = r#"<!-- wp:core/paragraph -->
<p>Hello World</p>
<!-- /wp:core/paragraph -->"#;
    let result = parse(markup);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_name, Some("core/paragraph".to_string()));
    assert_eq!(result[0].inner_html, "\n<p>Hello World</p>\n");
    assert_eq!(result[0].inner_content, vec![Some("\n<p>Hello World</p>\n".to_string())]);
}

#[test]
fn test_parse_nested_blocks() {
    let markup = r#"<!-- wp:core/columns -->
<div class="wp-block-columns">
<!-- wp:core/column -->
<div class="wp-block-column">
<!-- wp:core/paragraph -->
<p>Column 1</p>
<!-- /wp:core/paragraph -->
</div>
<!-- /wp:core/column -->
</div>
<!-- /wp:core/columns -->"#;
    let result = parse(markup);
    assert_eq!(result.len(), 1);
    
    let columns = &result[0];
    assert_eq!(columns.block_name, Some("core/columns".to_string()));
    assert_eq!(columns.inner_blocks.len(), 1);
    
    let column = &columns.inner_blocks[0];
    assert_eq!(column.block_name, Some("core/column".to_string()));
    assert_eq!(column.inner_blocks.len(), 1);
    
    let paragraph = &column.inner_blocks[0];
    assert_eq!(paragraph.block_name, Some("core/paragraph".to_string()));
    assert_eq!(paragraph.inner_html, "\n<p>Column 1</p>\n");
}

#[test]
fn test_parse_freeform_html() {
    let markup = r#"some raw text
<!-- wp:core/paragraph -->
<p>Block text</p>
<!-- /wp:core/paragraph -->
more raw text"#;
    let result = parse(markup);
    assert_eq!(result.len(), 3);
    
    assert_eq!(result[0].block_name, None);
    assert_eq!(result[0].inner_html, "some raw text\n");
    
    assert_eq!(result[1].block_name, Some("core/paragraph".to_string()));
    assert_eq!(result[1].inner_html, "\n<p>Block text</p>\n");
    
    assert_eq!(result[2].block_name, None);
    assert_eq!(result[2].inner_html, "\nmore raw text");
}

#[test]
fn test_parse_block_with_complex_json_in_attrs() {
    let markup = r#"<!-- wp:core/group {"style":{"spacing":{"padding":{"top":"30px"}}},"layout":{"type":"constrained"}} -->
<div class="wp-block-group"></div>
<!-- /wp:core/group -->"#;
    let result = parse(markup);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].block_name, Some("core/group".to_string()));
    
    let attrs = result[0].attrs.as_ref().unwrap();
    assert_eq!(attrs["style"]["spacing"]["padding"]["top"], "30px");
    assert_eq!(attrs["layout"]["type"], "constrained");
}

#[test]
fn test_parse_graceful_recovery_unclosed_block() {
    let markup = r#"<!-- wp:core/group -->
<div class="wp-block-group">
<!-- wp:core/paragraph -->
<p>Paragraph</p>"#;
    let result = parse(markup);
    // Should parse the opened blocks as best as possible
    assert!(!result.is_empty());
}
