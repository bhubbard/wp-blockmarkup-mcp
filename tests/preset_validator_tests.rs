use wp_block_mcp::validation::block_parser::parse;
use wp_block_mcp::validation::preset::{wp_to_kebab_case, validate_preset_references};

#[test]
fn test_wp_to_kebab_case_digit_to_letter_boundary() {
    assert_eq!(wp_to_kebab_case("30x"), "30-x");
    assert_eq!(wp_to_kebab_case("16x"), "16-x");
}

#[test]
fn test_wp_to_kebab_case_letter_to_digit_boundary() {
    assert_eq!(wp_to_kebab_case("x30"), "x-30");
}

#[test]
fn test_wp_to_kebab_case_camel_case_boundary() {
    assert_eq!(wp_to_kebab_case("fontSize"), "font-size");
}

#[test]
fn test_wp_to_kebab_case_no_boundary() {
    assert_eq!(wp_to_kebab_case("primary"), "primary");
    assert_eq!(wp_to_kebab_case("lg"), "lg");
}

#[test]
fn test_wp_to_kebab_case_pre_hyphenated_slug_with_digit_to_letter() {
    assert_eq!(wp_to_kebab_case("0-5x"), "0-5-x");
}

#[test]
fn test_flags_digit_to_letter_mismatch_in_inline_style() {
    let markup = r#"<!-- wp:group {"style":{"spacing":{"padding":{"top":"var:preset|spacing|30x"}}}} -->
<div class="wp-block-group" style="padding-top:var(--wp--preset--spacing--30x)"></div>
<!-- /wp:group -->"#;
    let parsed = parse(markup);
    let warnings = validate_preset_references(&parsed);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("core/group"));
    assert!(warnings[0].contains("style.spacing.padding.top"));
    assert!(warnings[0].contains("--wp--preset--spacing--30x"));
    assert!(warnings[0].contains("--wp--preset--spacing--30-x"));
}

#[test]
fn test_passes_when_inline_style_uses_canonical_kebab_form() {
    let markup = r#"<!-- wp:group {"style":{"spacing":{"padding":{"top":"var:preset|spacing|30x"}}}} -->
<div class="wp-block-group" style="padding-top:var(--wp--preset--spacing--30-x)"></div>
<!-- /wp:group -->"#;
    let parsed = parse(markup);
    let warnings = validate_preset_references(&parsed);
    assert!(warnings.is_empty());
}

#[test]
fn test_passes_when_slug_has_no_digit_letter_boundary() {
    let markup = r#"<!-- wp:group {"backgroundColor":"primary","style":{"color":{"background":"var:preset|color|primary"}}} -->
<div class="wp-block-group" style="background:var(--wp--preset--color--primary)"></div>
<!-- /wp:group -->"#;
    let parsed = parse(markup);
    let warnings = validate_preset_references(&parsed);
    assert!(warnings.is_empty());
}

#[test]
fn test_handles_0_5x_decimal_with_x_suffix() {
    let ok_markup = r#"<!-- wp:group {"style":{"spacing":{"padding":{"top":"var:preset|spacing|0-5x"}}}} -->
<div class="wp-block-group" style="padding-top:var(--wp--preset--spacing--0-5-x)"></div>
<!-- /wp:group -->"#;
    let parsed_ok = parse(ok_markup);
    assert!(validate_preset_references(&parsed_ok).is_empty());

    let bad_markup = r#"<!-- wp:group {"style":{"spacing":{"padding":{"top":"var:preset|spacing|0-5x"}}}} -->
<div class="wp-block-group" style="padding-top:var(--wp--preset--spacing--0-5x)"></div>
<!-- /wp:group -->"#;
    let parsed_bad = parse(bad_markup);
    let warnings = validate_preset_references(&parsed_bad);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("--wp--preset--spacing--0-5-x"));
}

#[test]
fn test_validates_multiple_preset_references_independently() {
    let markup = r#"<!-- wp:group {"style":{"spacing":{"padding":{"top":"var:preset|spacing|30x","right":"var:preset|spacing|16x"}}}} -->
<div class="wp-block-group" style="padding-top:var(--wp--preset--spacing--30x);padding-right:var(--wp--preset--spacing--16x)"></div>
<!-- /wp:group -->"#;
    let parsed = parse(markup);
    let warnings = validate_preset_references(&parsed);
    assert_eq!(warnings.len(), 2);
    assert!(warnings.iter().any(|w| w.contains("30x") && w.contains("30-x")));
    assert!(warnings.iter().any(|w| w.contains("16x") && w.contains("16-x")));
}

#[test]
fn test_no_preset_references_means_no_warnings() {
    let markup = r#"<!-- wp:paragraph -->
<p>Hello world</p>
<!-- /wp:paragraph -->"#;
    let parsed = parse(markup);
    let warnings = validate_preset_references(&parsed);
    assert!(warnings.is_empty());
}

#[test]
fn test_recurses_into_inner_blocks() {
    let markup = r#"<!-- wp:group -->
<div class="wp-block-group">
<!-- wp:paragraph {"style":{"spacing":{"padding":{"top":"var:preset|spacing|30x"}}}} -->
<p style="padding-top:var(--wp--preset--spacing--30x)">x</p>
<!-- /wp:paragraph -->
</div>
<!-- /wp:group -->"#;
    let parsed = parse(markup);
    let warnings = validate_preset_references(&parsed);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("core/paragraph"));
}

#[test]
fn test_does_not_flag_when_preset_ref_exists_in_attrs_but_not_in_inline_style() {
    let markup = r#"<!-- wp:latest-posts {"style":{"spacing":{"padding":{"top":"var:preset|spacing|30x"}}}} /-->"#;
    let parsed = parse(markup);
    let warnings = validate_preset_references(&parsed);
    assert!(warnings.is_empty());
}

#[test]
fn test_real_world_reproducer_hero_pattern_with_two_mismatches() {
    let markup = r#"<!-- wp:group {"className":"pl-hero","style":{"spacing":{"padding":{"top":"var:preset|spacing|30x","right":"var:preset|spacing|16x"}}},"backgroundColor":"gray-900","layout":{"type":"constrained"}} -->
<div class="wp-block-group pl-hero has-gray-900-background-color has-background" style="padding-top:var(--wp--preset--spacing--30x);padding-right:var(--wp--preset--spacing--16x)">
</div>
<!-- /wp:group -->"#;
    let parsed = parse(markup);
    let warnings = validate_preset_references(&parsed);
    assert_eq!(warnings.len(), 2);
}
