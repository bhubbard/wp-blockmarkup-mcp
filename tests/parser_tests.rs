use wp_block_mcp::parsers::save::parse_save_file;
use wp_block_mcp::parsers::edit::parse_edit_file;
use wp_block_mcp::parsers::variations::parse_variations_file;
use std::fs::{self, create_dir_all};
use std::env;

#[test]
fn test_ast_parsers_save_and_edit() {
    // Create a temporary block folder
    let mut temp_dir = env::temp_dir();
    temp_dir.push("wp_block_mcp_test_block_ast");
    let _ = fs::remove_dir_all(&temp_dir);
    create_dir_all(&temp_dir).unwrap();

    // 1. Write mock save.js
    let save_js = r#"
import { useBlockProps, InnerBlocks } from '@wordpress/block-editor';

export default function save({ attributes }) {
    const { className, align } = attributes;
    return (
        <section { ...useBlockProps.save({ className: 'custom-class' }) }>
            <div className="inner-wrap">
                <InnerBlocks.Content />
            </div>
        </section>
    );
}
"#;
    fs::write(temp_dir.join("save.js"), save_js).unwrap();

    // 2. Write mock edit.js
    let edit_js = r#"
import { __ } from '@wordpress/i18n';
import { useBlockProps, InspectorControls } from '@wordpress/block-editor';
import { PanelBody, ToggleControl, TextControl, SelectControl } from '@wordpress/components';

export default function Edit({ attributes, setAttributes }) {
    const { isFeatured, title, size } = attributes;
    return (
        <div { ...useBlockProps() }>
            <InspectorControls>
                <PanelBody title={ __( 'Settings' ) }>
                    <ToggleControl
                        label={ __( 'Featured' ) }
                        checked={ isFeatured }
                        onChange={ ( val ) => setAttributes( { isFeatured: val } ) }
                    />
                    <TextControl
                        label={ __( 'Title' ) }
                        value={ title }
                        onChange={ ( val ) => setAttributes( { title: val } ) }
                    />
                    <SelectControl
                        label={ __( 'Size' ) }
                        value={ size }
                        options={ [
                            { label: 'Small', value: 'small' },
                            { label: 'Large', value: 'large' }
                        ] }
                        onChange={ ( val ) => setAttributes( { size: val } ) }
                    />
                </PanelBody>
            </InspectorControls>
            <p>Edit Mode</p>
        </div>
    );
}
"#;
    fs::write(temp_dir.join("edit.js"), edit_js).unwrap();

    // 3. Write mock variations.js
    let variations_js = r#"
const variations = [
    {
        name: 'featured-grid',
        title: 'Featured Grid',
        description: 'A preconfigured grid of featured items',
        isDefault: true,
        attributes: {
            columns: 3,
            isFeatured: true
        }
    }
];
export default variations;
"#;
    fs::write(temp_dir.join("variations.js"), variations_js).unwrap();

    // Run the save parser
    let save_data = parse_save_file(&temp_dir);
    println!("DEBUG save_data: {:?}", save_data);
    assert!(save_data.has_custom_save);
    assert_eq!(save_data.wrapper_element, Some("section".to_string()));
    assert!(save_data.uses_inner_blocks);

    // Run the edit parser
    let edit_data = parse_edit_file(&temp_dir);
    assert!(edit_data.imports.iter().any(|i| i.source == "@wordpress/components"));
    assert_eq!(edit_data.controls.len(), 3);

    let toggle = edit_data.controls.iter().find(|c| c.component == "ToggleControl").unwrap();
    assert_eq!(toggle.attribute_name, Some("isFeatured".to_string()));
    assert_eq!(toggle.label, Some("[Expression]".to_string()));

    let select = edit_data.controls.iter().find(|c| c.component == "SelectControl").unwrap();
    assert_eq!(select.attribute_name, Some("size".to_string()));
    assert_eq!(select.label, Some("[Expression]".to_string()));
    assert!(select.options.is_some());

    // Run the variations parser
    let variations_data = parse_variations_file(&temp_dir);
    assert_eq!(variations_data.len(), 1);
    assert_eq!(variations_data[0].name, "featured-grid");
    assert_eq!(variations_data[0].title.as_deref(), Some("Featured Grid"));

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
