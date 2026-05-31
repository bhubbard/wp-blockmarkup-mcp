use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedVariation {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub attributes: serde_json::Value, // object map
    pub inner_blocks: Vec<serde_json::Value>,
    pub scope: Vec<String>,
    pub keywords: Vec<String>,
    pub is_default: bool,
}

pub fn parse_variations_file(block_path: &Path) -> Vec<ParsedVariation> {
    let variations_path = block_path.join("variations.js");
    let mut variations = Vec::new();

    if !variations_path.exists() {
        return variations;
    }

    let content = match fs::read_to_string(&variations_path) {
        Ok(c) => c,
        Err(_) => return variations,
    };

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(&variations_path)
        .unwrap_or_default()
        .with_jsx(true)
        .with_typescript(true);
    let parser_return = Parser::new(&allocator, &content, source_type).parse();

    if !parser_return.errors.is_empty() {
        return variations;
    }

    let mut visitor = VariationsVisitor {
        variations: &mut variations,
    };
    visitor.visit_program(&parser_return.program);

    variations
}

struct VariationsVisitor<'a> {
    variations: &'a mut Vec<ParsedVariation>,
}

impl<'a> Visit<'a> for VariationsVisitor<'a> {
    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        if let ExportDefaultDeclarationKind::ArrayExpression(arr) = &decl.declaration {
            extract_variations_from_array(arr, self.variations);
        }
    }

    fn visit_variable_declaration(&mut self, decl: &VariableDeclaration<'a>) {
        for declarator in &decl.declarations {
            if let BindingPattern::BindingIdentifier(ident) = &declarator.id {
                if ident.name == "variations" {
                    if let Some(Expression::ArrayExpression(arr)) = &declarator.init {
                        extract_variations_from_array(arr, self.variations);
                    }
                }
            }
        }
    }
}

fn extract_variations_from_array(arr: &ArrayExpression, variations: &mut Vec<ParsedVariation>) {
    for el in &arr.elements {
        if let ArrayExpressionElement::ObjectExpression(obj) = el {
            if let Some(variation) = extract_variation_object(obj) {
                variations.push(variation);
            }
        }
    }
}

fn extract_variation_object(obj: &ObjectExpression) -> Option<ParsedVariation> {
    let mut name = None;
    let mut title = None;
    let mut description = None;
    let mut attributes = serde_json::Value::Object(serde_json::Map::new());
    let mut inner_blocks = Vec::new();
    let mut scope = Vec::new();
    let mut keywords = Vec::new();
    let mut is_default = false;

    for prop in &obj.properties {
        if let ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
            let key = match &obj_prop.key {
                PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
                PropertyKey::StringLiteral(s) => Some(s.value.to_string()),
                _ => None,
            };

            if let Some(ref k) = key {
                match k.as_str() {
                    "name" => name = get_string_value(&obj_prop.value),
                    "title" => title = get_string_value(&obj_prop.value),
                    "description" => description = get_string_value(&obj_prop.value),
                    "attributes" => {
                        if let Expression::ObjectExpression(attr_obj) = &obj_prop.value {
                            attributes = extract_attributes_object(attr_obj);
                        }
                    }
                    "innerBlocks" => {
                        if let Expression::ArrayExpression(blocks_arr) = &obj_prop.value {
                            inner_blocks = extract_inner_blocks_array(blocks_arr);
                        }
                    }
                    "scope" => {
                        if let Expression::ArrayExpression(scope_arr) = &obj_prop.value {
                            scope = scope_arr.elements.iter().filter_map(|el| {
                                get_array_element_string(el)
                            }).collect();
                        }
                    }
                    "keywords" => {
                        if let Expression::ArrayExpression(kw_arr) = &obj_prop.value {
                            keywords = kw_arr.elements.iter().filter_map(|el| {
                                get_array_element_string(el)
                            }).collect();
                        }
                    }
                    "isDefault" => {
                        if let Expression::BooleanLiteral(b) = &obj_prop.value {
                            is_default = b.value;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if let Some(n) = name {
        Some(ParsedVariation {
            name: n,
            title,
            description,
            attributes,
            inner_blocks,
            scope,
            keywords,
            is_default,
        })
    } else {
        None
    }
}

fn extract_attributes_object(obj: &ObjectExpression) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for prop in &obj.properties {
        if let ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
            let key = match &obj_prop.key {
                PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
                PropertyKey::StringLiteral(s) => Some(s.value.to_string()),
                _ => None,
            };
            if let Some(ref k) = key {
                let value = get_property_value(&obj_prop.value);
                map.insert(k.clone(), value);
            }
        }
    }
    serde_json::Value::Object(map)
}

fn extract_inner_blocks_array(arr: &ArrayExpression) -> Vec<serde_json::Value> {
    let mut list = Vec::new();
    for el in &arr.elements {
        if let ArrayExpressionElement::ArrayExpression(block_arr) = el {
            if block_arr.elements.len() >= 2 {
                let mut map = serde_json::Map::new();
                
                let block_name = if let Some(first_el) = block_arr.elements.get(0) {
                    get_array_element_string(first_el)
                } else {
                    None
                };

                let block_attrs = if let Some(ArrayExpressionElement::ObjectExpression(obj)) = block_arr.elements.get(1) {
                    extract_attributes_object(obj)
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                };

                let nested_blocks = if block_arr.elements.len() > 2 {
                    if let Some(ArrayExpressionElement::ArrayExpression(nested_arr)) = block_arr.elements.get(2) {
                        extract_inner_blocks_array(nested_arr)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                map.insert("blockName".to_string(), block_name.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
                map.insert("attributes".to_string(), block_attrs);
                map.insert("innerBlocks".to_string(), serde_json::Value::Array(nested_blocks));
                list.push(serde_json::Value::Object(map));
            }
        }
    }
    list
}

fn get_array_element_string(el: &ArrayExpressionElement) -> Option<String> {
    match el {
        ArrayExpressionElement::StringLiteral(s) => Some(s.value.to_string()),
        ArrayExpressionElement::TemplateLiteral(t) if t.quasis.len() == 1 => {
            t.quasis.first().and_then(|q| q.value.cooked.as_ref().map(|s| s.to_string()))
        }
        _ => None,
    }
}

fn get_string_value(expr: &Expression) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.to_string()),
        Expression::TemplateLiteral(t) if t.quasis.len() == 1 => {
            t.quasis.first().and_then(|q| q.value.cooked.as_ref().map(|s| s.to_string()))
        }
        _ => None,
    }
}

fn get_property_value(expr: &Expression) -> serde_json::Value {
    match expr {
        Expression::StringLiteral(s) => serde_json::Value::String(s.value.to_string()),
        Expression::NumericLiteral(n) => serde_json::Value::Number(serde_json::Number::from_f64(n.value).unwrap()),
        Expression::BooleanLiteral(b) => serde_json::Value::Bool(b.value),
        Expression::NullLiteral(_) => serde_json::Value::Null,
        Expression::ArrayExpression(arr) => {
            let list = arr.elements.iter().filter_map(|el| {
                match el {
                    ArrayExpressionElement::SpreadElement(_) => None,
                    ArrayExpressionElement::Elision(_) => None,
                    other => Some(get_property_value_from_element(other)),
                }
            }).collect();
            serde_json::Value::Array(list)
        }
        Expression::ObjectExpression(obj) => extract_attributes_object(obj),
        _ => serde_json::Value::String("[Complex Value]".to_string()),
    }
}

fn get_property_value_from_element(el: &ArrayExpressionElement) -> serde_json::Value {
    match el {
        ArrayExpressionElement::StringLiteral(s) => serde_json::Value::String(s.value.to_string()),
        ArrayExpressionElement::NumericLiteral(n) => serde_json::Value::Number(serde_json::Number::from_f64(n.value).unwrap()),
        ArrayExpressionElement::BooleanLiteral(b) => serde_json::Value::Bool(b.value),
        ArrayExpressionElement::NullLiteral(_) => serde_json::Value::Null,
        ArrayExpressionElement::ArrayExpression(arr) => {
            let list = arr.elements.iter().filter_map(|el| {
                match el {
                    ArrayExpressionElement::SpreadElement(_) => None,
                    ArrayExpressionElement::Elision(_) => None,
                    other => Some(get_property_value_from_element(other)),
                }
            }).collect();
            serde_json::Value::Array(list)
        }
        ArrayExpressionElement::ObjectExpression(obj) => extract_attributes_object(obj),
        _ => serde_json::Value::String("[Complex Value]".to_string()),
    }
}
