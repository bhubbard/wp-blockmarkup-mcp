use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, GetSpan};
use regex::Regex;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ControlInfo {
    pub component: String,
    pub attribute_name: Option<String>,
    pub label: Option<String>,
    pub help: Option<String>,
    pub possible_values: Option<Vec<String>>,
    pub r#type: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub step: Option<String>,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportInfo {
    pub source: String,
    pub specifiers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditData {
    pub controls: Vec<ControlInfo>,
    pub imports: Vec<ImportInfo>,
}

struct ControlPattern {
    attribute: Option<&'static str>,
    possible_values: Option<&'static [&'static str]>,
    r#type: Option<&'static str>,
    has_min_max: bool,
    has_options: bool,
    multiline: bool,
}

fn get_control_patterns() -> std::collections::HashMap<&'static str, ControlPattern> {
    let mut m = std::collections::HashMap::new();
    m.insert("AlignmentControl", ControlPattern { attribute: Some("align"), possible_values: Some(&["left", "center", "right", "justify"]), r#type: None, has_min_max: false, has_options: false, multiline: false });
    m.insert("BlockAlignmentControl", ControlPattern { attribute: Some("align"), possible_values: Some(&["left", "center", "right", "wide", "full", "none"]), r#type: None, has_min_max: false, has_options: false, multiline: false });
    m.insert("BlockVerticalAlignmentControl", ControlPattern { attribute: Some("verticalAlignment"), possible_values: Some(&["top", "center", "bottom", "stretch"]), r#type: None, has_min_max: false, has_options: false, multiline: false });
    m.insert("ColorPalette", ControlPattern { attribute: Some("color"), possible_values: None, r#type: Some("color"), has_min_max: false, has_options: false, multiline: false });
    m.insert("ColorPicker", ControlPattern { attribute: Some("color"), possible_values: None, r#type: Some("color"), has_min_max: false, has_options: false, multiline: false });
    m.insert("FontSizePicker", ControlPattern { attribute: Some("fontSize"), possible_values: Some(&["small", "medium", "large", "x-large"]), r#type: None, has_min_max: false, has_options: false, multiline: false });
    m.insert("ToggleControl", ControlPattern { attribute: None, possible_values: None, r#type: Some("boolean"), has_min_max: false, has_options: false, multiline: false });
    m.insert("RangeControl", ControlPattern { attribute: None, possible_values: None, r#type: Some("number"), has_min_max: true, has_options: false, multiline: false });
    m.insert("SelectControl", ControlPattern { attribute: None, possible_values: None, r#type: Some("select"), has_min_max: false, has_options: true, multiline: false });
    m.insert("RadioControl", ControlPattern { attribute: None, possible_values: None, r#type: Some("radio"), has_min_max: false, has_options: true, multiline: false });
    m.insert("TextControl", ControlPattern { attribute: None, possible_values: None, r#type: Some("string"), has_min_max: false, has_options: false, multiline: false });
    m.insert("TextareaControl", ControlPattern { attribute: None, possible_values: None, r#type: Some("string"), has_min_max: false, has_options: false, multiline: true });
    m.insert("URLInput", ControlPattern { attribute: Some("url"), possible_values: None, r#type: Some("string"), has_min_max: false, has_options: false, multiline: false });
    m.insert("URLPopover", ControlPattern { attribute: Some("url"), possible_values: None, r#type: Some("string"), has_min_max: false, has_options: false, multiline: false });
    m.insert("MediaUpload", ControlPattern { attribute: Some("mediaId"), possible_values: None, r#type: Some("number"), has_min_max: false, has_options: false, multiline: false });
    m.insert("ImageSizeControl", ControlPattern { attribute: Some("sizeSlug"), possible_values: Some(&["thumbnail", "medium", "large", "full"]), r#type: None, has_min_max: false, has_options: false, multiline: false });
    m
}

pub fn parse_edit_file(block_path: &Path) -> EditData {
    let edit_path = block_path.join("edit.js");
    
    let mut result = EditData {
        controls: Vec::new(),
        imports: Vec::new(),
    };

    if !edit_path.exists() {
        return result;
    }

    let content = match fs::read_to_string(&edit_path) {
        Ok(c) => c,
        Err(_) => return result,
    };

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(&edit_path)
        .unwrap_or_default()
        .with_jsx(true)
        .with_typescript(true);
    let parser_return = Parser::new(&allocator, &content, source_type).parse();

    if !parser_return.errors.is_empty() {
        return result;
    }

    let mut visitor = EditVisitor {
        content: &content,
        result: &mut result,
    };
    visitor.visit_program(&parser_return.program);

    result
}

struct EditVisitor<'a> {
    content: &'a str,
    result: &'a mut EditData,
}

impl<'a> Visit<'a> for EditVisitor<'a> {
    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
        let source = decl.source.value.to_string();
        let mut specifiers = Vec::new();
        if let Some(ref specs) = decl.specifiers {
            for spec in specs {
                match spec {
                    ImportDeclarationSpecifier::ImportSpecifier(s) => {
                        specifiers.push(s.imported.name().to_string());
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                        specifiers.push(format!("default:{}", s.local.name));
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                        specifiers.push(format!("*:{}", s.local.name));
                    }
                }
            }
        }
        self.result.imports.push(ImportInfo { source, specifiers });
    }

    fn visit_jsx_element(&mut self, elem: &JSXElement<'a>) {
        let tag_name = get_jsx_element_name(&elem.opening_element.name);
        if let Some(ref name) = tag_name {
            let patterns = get_control_patterns();
            if let Some(pattern) = patterns.get(name.as_str()) {
                let mut control = ControlInfo {
                    component: name.clone(),
                    attribute_name: pattern.attribute.map(|s| s.to_string()),
                    label: None,
                    help: None,
                    possible_values: pattern.possible_values.map(|v| v.iter().map(|s| s.to_string()).collect()),
                    r#type: pattern.r#type.map(|s| s.to_string()),
                    min: None,
                    max: None,
                    step: None,
                    options: None,
                };

                // Scan JSX attributes
                for attr in &elem.opening_element.attributes {
                    if let JSXAttributeItem::Attribute(jsx_attr) = attr {
                        let prop_name = match &jsx_attr.name {
                            JSXAttributeName::Identifier(ident) => ident.name.as_str(),
                            _ => "",
                        };

                        if let Some(ref val) = jsx_attr.value {
                            let prop_value_str = get_attribute_value_str(val);
                            
                            match prop_name {
                                "onChange" => {
                                    // Slice the actual source code of the onChange expression to search setAttributes
                                    let span = val.span();
                                    let raw_expr = &self.content[span.start as usize..span.end as usize];
                                    let re = Regex::new(r"setAttributes\s*\(\s*\{[\s\n]*([a-zA-Z_$][a-zA-Z0-9_$]*)").unwrap();
                                    if let Some(cap) = re.captures(raw_expr) {
                                        control.attribute_name = Some(cap[1].to_string());
                                    }
                                }
                                "label" => control.label = prop_value_str,
                                "help" => control.help = prop_value_str,
                                "min" => control.min = prop_value_str,
                                "max" => control.max = prop_value_str,
                                "step" => control.step = prop_value_str,
                                "options" => {
                                    if let JSXAttributeValue::ExpressionContainer(container) = val {
                                        if let Some(expr) = container.expression.as_expression() {
                                            control.options = extract_options(expr);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                self.result.controls.push(control);
            }
        }

        // Recurse children
        walk::walk_jsx_element(self, elem);
    }
}

fn get_jsx_element_name(name: &JSXElementName) -> Option<String> {
    match name {
        JSXElementName::Identifier(ident) => Some(ident.name.to_string()),
        JSXElementName::IdentifierReference(ident) => Some(ident.name.to_string()),
        JSXElementName::ThisExpression(_) => Some("this".to_string()),
        JSXElementName::NamespacedName(ns) => Some(format!("{}:{}", ns.namespace.name, ns.name.name)),
        JSXElementName::MemberExpression(mem) => {
            let mut parts = Vec::new();
            let mut current = mem;
            loop {
                parts.insert(0, current.property.name.to_string());
                if let Some(ident) = current.object.get_identifier() {
                    parts.insert(0, ident.name.to_string());
                    break;
                } else if let JSXMemberExpressionObject::MemberExpression(sub_mem) = &current.object {
                    current = sub_mem;
                } else {
                    break;
                }
            }
            Some(parts.join("."))
        }
    }
}

fn get_attribute_value_str(value: &JSXAttributeValue) -> Option<String> {
    match value {
        JSXAttributeValue::StringLiteral(str_lit) => Some(str_lit.value.to_string()),
        JSXAttributeValue::ExpressionContainer(container) => {
            if let Some(expr) = container.expression.as_expression() {
                match expr {
                    Expression::StringLiteral(s) => Some(s.value.to_string()),
                    Expression::NumericLiteral(n) => Some(n.value.to_string()),
                    Expression::BooleanLiteral(b) => Some(b.value.to_string()),
                    Expression::TemplateLiteral(t) => {
                        let mut res = String::new();
                        for (i, q) in t.quasis.iter().enumerate() {
                            res.push_str(q.value.raw.as_str());
                            if i < t.expressions.len() {
                                res.push_str("${...}");
                            }
                        }
                        Some(res)
                    }
                    _ => Some("[Expression]".to_string()),
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_options(expr: &Expression) -> Option<serde_json::Value> {
    if let Expression::ArrayExpression(arr) = expr {
        let mut list = Vec::new();
        for el in &arr.elements {
            if let ArrayExpressionElement::ObjectExpression(obj) = el {
                let mut map = serde_json::Map::new();
                for prop in &obj.properties {
                    if let ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
                        let key = match &obj_prop.key {
                            PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
                            _ => None,
                        };
                        if let Some(k) = key {
                            match &obj_prop.value {
                                Expression::StringLiteral(s) => {
                                    map.insert(k, serde_json::Value::String(s.value.to_string()));
                                }
                                Expression::NumericLiteral(n) => {
                                    map.insert(k, serde_json::Value::Number(serde_json::Number::from_f64(n.value).unwrap()));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                list.push(serde_json::Value::Object(map));
            }
        }
        Some(serde_json::Value::Array(list))
    } else {
        None
    }
}
