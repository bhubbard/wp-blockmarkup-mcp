use wp_block_mcp::validation::save_validator::validate_save_output;
use wp_block_mcp::parsers::save::SaveData;
use serde_json::json;

fn create_mock_save_data(
    has_custom_save: bool,
    wrapper_element: Option<&str>,
    uses_inner_blocks: bool,
) -> SaveData {
    SaveData {
        block_type: if has_custom_save { "static".to_string() } else { "dynamic".to_string() },
        has_custom_save,
        returns_null: !has_custom_save,
        wrapper_element: wrapper_element.map(|s| s.to_string()),
        uses_inner_blocks,
        class_patterns: Vec::new(),
        style_patterns: Vec::new(),
        attribute_usage: Vec::new(),
    }
}

#[test]
fn test_save_validator_dynamic_block() {
    let save_data = create_mock_save_data(false, None, false);
    let markup = "<!-- wp:core/latest-posts /-->";
    let res = validate_save_output(markup, "core/latest-posts", Some(&save_data));
    assert!(res.valid);
    assert_eq!(res.status, "attributes_only");
}

#[test]
fn test_save_validator_wrapper_element_match() {
    let save_data = create_mock_save_data(true, Some("div"), false);
    
    // Correct wrapper
    let ok_markup = "<!-- wp:core/group -->\n<div class=\"wp-block-group\"></div>\n<!-- /wp:core/group -->";
    let res_ok = validate_save_output(ok_markup, "core/group", Some(&save_data));
    assert!(res_ok.valid);

    // Wrong wrapper
    let bad_markup = "<!-- wp:core/group -->\n<section class=\"wp-block-group\"></section>\n<!-- /wp:core/group -->";
    let res_bad = validate_save_output(bad_markup, "core/group", Some(&save_data));
    assert!(!res_bad.valid);
    assert!(res_bad.issues[0].contains("Expected wrapper <div> but found <section>"));
}

#[test]
fn test_save_validator_missing_wp_block_class() {
    let save_data = create_mock_save_data(true, Some("div"), false);
    let markup = "<!-- wp:core/group -->\n<div class=\"some-other-class\"></div>\n<!-- /wp:core/group -->";
    let res = validate_save_output(markup, "core/group", Some(&save_data));
    assert!(!res.valid);
    assert!(res.issues.iter().any(|i| i.contains("Expected CSS class \"wp-block-group\" not found")));
}

#[test]
fn test_save_validator_optional_wp_block_class_for_simple_elements() {
    let save_data = create_mock_save_data(true, Some("p"), false);
    let markup = "<!-- wp:core/paragraph -->\n<p class=\"custom-class\">Text</p>\n<!-- /wp:core/paragraph -->";
    let res = validate_save_output(markup, "core/paragraph", Some(&save_data));
    assert!(res.valid); // simple elements (like p) have optional block classes
}

#[test]
fn test_save_validator_color_feature_classes() {
    let save_data = create_mock_save_data(true, Some("div"), false);
    
    // Correct text and bg colors
    let ok_markup = r#"<!-- wp:core/group {"backgroundColor":"red","textColor":"blue"} -->
<div class="wp-block-group has-red-background-color has-background has-blue-color has-text-color"></div>
<!-- /wp:core/group -->"#;
    let res_ok = validate_save_output(ok_markup, "core/group", Some(&save_data));
    assert!(res_ok.valid);

    // Missing background color classes
    let bad_markup = r#"<!-- wp:core/group {"backgroundColor":"red","textColor":"blue"} -->
<div class="wp-block-group has-blue-color has-text-color"></div>
<!-- /wp:core/group -->"#;
    let res_bad = validate_save_output(bad_markup, "core/group", Some(&save_data));
    assert!(!res_bad.valid);
    assert!(res_bad.issues.iter().any(|i| i.contains("Background color attribute \"red\" set but class \"has-red-background-color\" not in HTML")));
    assert!(res_bad.issues.iter().any(|i| i.contains("Background color set but \"has-background\" class missing")));
}

#[test]
fn test_save_validator_style_attribute_padding_and_margin() {
    let save_data = create_mock_save_data(true, Some("div"), false);

    // Correct padding
    let ok_markup = r#"<!-- wp:core/group {"style":{"spacing":{"padding":{"top":"10px"}}}} -->
<div class="wp-block-group" style="padding-top:10px"></div>
<!-- /wp:core/group -->"#;
    let res_ok = validate_save_output(ok_markup, "core/group", Some(&save_data));
    assert!(res_ok.valid);

    // Missing padding in style tag
    let bad_markup = r#"<!-- wp:core/group {"style":{"spacing":{"padding":{"top":"10px"}}}} -->
<div class="wp-block-group" style="color:red"></div>
<!-- /wp:core/group -->"#;
    let res_bad = validate_save_output(bad_markup, "core/group", Some(&save_data));
    assert!(!res_bad.valid);
    assert!(res_bad.issues.iter().any(|i| i.contains("Spacing padding set in attrs but not found in style attribute")));
}

#[test]
fn test_save_validator_missing_closing_delimiter() {
    let save_data = create_mock_save_data(true, Some("div"), false);
    let markup = "<!-- wp:core/group -->\n<div class=\"wp-block-group\"></div>";
    let res = validate_save_output(markup, "core/group", Some(&save_data));
    assert!(!res.valid);
    assert!(res.issues.iter().any(|i| i.contains("Missing closing block comment delimiter")));
}
