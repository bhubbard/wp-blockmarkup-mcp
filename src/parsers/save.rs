use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassPattern {
    #[serde(skip_serializing_if = "Option::is_none", rename = "className")]
    pub class_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#static: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StylePattern {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "valueExpression")]
    pub value_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveData {
    pub has_custom_save: bool,
    pub returns_null: bool,
    pub block_type: String,
    pub wrapper_element: Option<String>,
    pub class_patterns: Vec<ClassPattern>,
    pub style_patterns: Vec<StylePattern>,
    pub uses_inner_blocks: bool,
    pub attribute_usage: Vec<String>,
}

pub fn parse_save_file(block_path: &Path) -> SaveData {
    let save_path = block_path.join("save.js");
    let has_render_php = block_path.join("render.php").exists();

    let mut result = SaveData {
        has_custom_save: false,
        returns_null: false,
        block_type: "static".to_string(),
        wrapper_element: None,
        class_patterns: Vec::new(),
        style_patterns: Vec::new(),
        uses_inner_blocks: false,
        attribute_usage: Vec::new(),
    };

    if !save_path.exists() {
        result.returns_null = true;
        result.block_type = if has_render_php { "dynamic".to_string() } else { "unknown".to_string() };
        return result;
    }

    let content = match fs::read_to_string(&save_path) {
        Ok(c) => c,
        Err(_) => {
            result.returns_null = true;
            result.block_type = "dynamic".to_string();
            return result;
        }
    };

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(&save_path)
        .unwrap_or_default()
        .with_jsx(true)
        .with_typescript(true);
    let parser_return = Parser::new(&allocator, &content, source_type).parse();

    if !parser_return.errors.is_empty() {
        result.returns_null = true;
        result.block_type = "dynamic".to_string();
        return result;
    }

    let mut visitor = SaveVisitor {
        result: &mut result,
        save_function_found: false,
    };
    visitor.visit_program(&parser_return.program);

    if !visitor.save_function_found {
        result.returns_null = true;
        result.block_type = "dynamic".to_string();
    } else if result.returns_null {
        result.block_type = "dynamic".to_string();
    } else {
        result.has_custom_save = true;
        result.block_type = if has_render_php { "hybrid".to_string() } else { "static".to_string() };
    }

    result
}

struct SaveVisitor<'a> {
    result: &'a mut SaveData,
    save_function_found: bool,
}

impl<'a> Visit<'a> for SaveVisitor<'a> {
    fn visit_export_default_declaration(&mut self, decl: &ExportDefaultDeclaration<'a>) {
        match &decl.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
                self.save_function_found = true;
                if let Some(ref first_param) = func.params.items.first() {
                    extract_attributes_usage(&first_param.pattern, &mut self.result.attribute_usage);
                }
                if let Some(ref body) = func.body {
                    walk::walk_function_body(self, body);
                }
            }
            ExportDefaultDeclarationKind::ArrowFunctionExpression(arrow) => {
                self.save_function_found = true;
                if let Some(ref first_param) = arrow.params.items.first() {
                    extract_attributes_usage(&first_param.pattern, &mut self.result.attribute_usage);
                }
                walk::walk_function_body(self, &arrow.body);
            }
            _ => {}
        }
    }

    fn visit_function(&mut self, func: &Function<'a>, flags: oxc_semantic::ScopeFlags) {
        if let Some(ref ident) = func.id {
            if ident.name == "save" && !self.save_function_found {
                self.save_function_found = true;
                if let Some(ref first_param) = func.params.items.first() {
                    extract_attributes_usage(&first_param.pattern, &mut self.result.attribute_usage);
                }
                if let Some(ref body) = func.body {
                    walk::walk_function_body(self, body);
                }
            }
        }
    }

    fn visit_return_statement(&mut self, stmt: &ReturnStatement<'a>) {
        if self.save_function_found {
            match &stmt.argument {
                None => {
                    self.result.returns_null = true;
                }
                Some(Expression::NullLiteral(_)) => {
                    self.result.returns_null = true;
                }
                Some(expr) => {
                    analyze_jsx_structure(expr, self.result);
                }
            }
        }
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if self.save_function_found {
            if let Expression::Identifier(ident) = &call.callee {
                if ident.name == "clsx" || ident.name == "classnames" {
                    extract_class_patterns(call, self.result);
                }
            }
        }
        walk::walk_call_expression(self, call);
    }
}

fn extract_attributes_usage(pattern: &BindingPattern, result: &mut Vec<String>) {
    if let BindingPattern::ObjectPattern(obj_pat) = pattern {
        for prop in &obj_pat.properties {
            if let PropertyKey::StaticIdentifier(ident) = &prop.key {
                if ident.name == "attributes" {
                    if let BindingPattern::ObjectPattern(attr_pat) = &prop.value {
                        for attr_prop in &attr_pat.properties {
                            if let PropertyKey::StaticIdentifier(attr_ident) = &attr_prop.key {
                                result.push(attr_ident.name.to_string());
                            }
                        }
                    }
                }
            }
        }
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

fn analyze_jsx_structure(expr: &Expression, result: &mut SaveData) {
    match expr {
        Expression::JSXElement(elem) => {
            analyze_jsx_element(elem, result);
        }
        Expression::JSXFragment(frag) => {
            for child in &frag.children {
                if let JSXChild::Element(child_elem) = child {
                    analyze_jsx_element(child_elem, result);
                }
            }
        }
        Expression::ParenthesizedExpression(paren) => {
            analyze_jsx_structure(&paren.expression, result);
        }
        _ => {}
    }
}

fn analyze_jsx_element(elem: &JSXElement, result: &mut SaveData) {
    let tag_name = get_jsx_element_name(&elem.opening_element.name);
    if let Some(ref name) = tag_name {
        if !name.contains('.') {
            if let Some(first_char) = name.chars().next() {
                if first_char.is_lowercase() && result.wrapper_element.is_none() {
                    result.wrapper_element = Some(name.clone());
                }
            }
        }

        if name.contains("InnerBlocks") {
            result.uses_inner_blocks = true;
        }
    }

    for attr in &elem.opening_element.attributes {
        match attr {
            JSXAttributeItem::Attribute(jsx_attr) => {
                let attr_name = match &jsx_attr.name {
                    JSXAttributeName::Identifier(ident) => ident.name.as_str(),
                    _ => "",
                };

                if attr_name == "className" {
                    if let Some(val) = &jsx_attr.value {
                        extract_class_from_attribute(val, result);
                    }
                } else if attr_name == "style" {
                    if let Some(val) = &jsx_attr.value {
                        extract_style_from_attribute(val, result);
                    }
                }
            }
            JSXAttributeItem::SpreadAttribute(spread) => {
                if let Expression::CallExpression(call) = &spread.argument {
                    if let Expression::StaticMemberExpression(member) = &call.callee {
                        if let Expression::Identifier(obj_ident) = &member.object {
                            if obj_ident.name == "useBlockProps" && member.property.name == "save" {
                                result.class_patterns.push(ClassPattern {
                                    class_name: None,
                                    conditional: None,
                                    condition: None,
                                    r#static: None,
                                    source: Some("useBlockProps.save()".to_string()),
                                    expression: None,
                                    description: Some("Uses WordPress block props with dynamic classes".to_string()),
                                    dynamic: Some(true),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    for child in &elem.children {
        if let JSXChild::Element(child_elem) = child {
            analyze_jsx_element(child_elem, result);
        }
    }
}

fn extract_class_from_attribute(value: &JSXAttributeValue, result: &mut SaveData) {
    match value {
        JSXAttributeValue::StringLiteral(str_lit) => {
            result.class_patterns.push(ClassPattern {
                class_name: Some(str_lit.value.to_string()),
                conditional: Some(false),
                condition: None,
                r#static: Some(true),
                source: None,
                expression: None,
                description: None,
                dynamic: None,
            });
        }
        JSXAttributeValue::ExpressionContainer(container) => {
            if let Some(expr) = container.expression.as_expression() {
                result.class_patterns.push(ClassPattern {
                    class_name: None,
                    conditional: None,
                    condition: None,
                    r#static: None,
                    source: Some("dynamic".to_string()),
                    expression: Some(get_expression_string(expr)),
                    description: None,
                    dynamic: None,
                });
            }
        }
        _ => {}
    }
}

fn extract_style_from_attribute(value: &JSXAttributeValue, result: &mut SaveData) {
    if let JSXAttributeValue::ExpressionContainer(container) = value {
        if let Some(expr) = container.expression.as_expression() {
            if let Expression::ObjectExpression(obj_expr) = expr {
                for prop in &obj_expr.properties {
                    if let ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
                        let prop_name = match &obj_prop.key {
                            PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
                            PropertyKey::StringLiteral(str_lit) => Some(str_lit.value.to_string()),
                            _ => None,
                        };
                        if let Some(name) = prop_name {
                            result.style_patterns.push(StylePattern {
                                property: Some(name),
                                value_expression: Some(get_expression_string(&obj_prop.value)),
                                dynamic: None,
                                expression: None,
                            });
                        }
                    }
                }
            } else {
                result.style_patterns.push(StylePattern {
                    property: None,
                    value_expression: None,
                    dynamic: Some(true),
                    expression: Some(get_expression_string(expr)),
                });
            }
        }
    }
}

fn get_expression_string(expr: &Expression) -> String {
    match expr {
        Expression::Identifier(ident) => ident.name.to_string(),
        Expression::StringLiteral(str_lit) => format!("\"{}\"", str_lit.value),
        Expression::NumericLiteral(num_lit) => num_lit.value.to_string(),
        Expression::BooleanLiteral(bool_lit) => bool_lit.value.to_string(),
        Expression::StaticMemberExpression(member) => {
            let obj = get_expression_string(&member.object);
            format!("{}.{}", obj, member.property.name)
        }
        Expression::TemplateLiteral(_) => "`template`".to_string(),
        Expression::ConditionalExpression(_) => "condition ? a : b".to_string(),
        _ => "[Expression]".to_string(),
    }
}

fn extract_class_patterns(call: &CallExpression, result: &mut SaveData) {
    for arg in &call.arguments {
        if let Some(expr) = arg.as_expression() {
            match expr {
                Expression::ObjectExpression(obj_expr) => {
                    for prop in &obj_expr.properties {
                        if let ObjectPropertyKind::ObjectProperty(obj_prop) = prop {
                            let class_name = match &obj_prop.key {
                                PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
                                PropertyKey::StringLiteral(str_lit) => Some(str_lit.value.to_string()),
                                _ => None,
                            };
                            if let Some(name) = class_name {
                                result.class_patterns.push(ClassPattern {
                                    class_name: Some(name),
                                    conditional: Some(true),
                                    condition: Some(get_expression_string(&obj_prop.value)),
                                    r#static: None,
                                    source: None,
                                    expression: None,
                                    description: None,
                                    dynamic: None,
                                });
                            }
                        }
                    }
                }
                Expression::StringLiteral(str_lit) => {
                    result.class_patterns.push(ClassPattern {
                        class_name: Some(str_lit.value.to_string()),
                        conditional: Some(false),
                        condition: None,
                        r#static: None,
                        source: None,
                        expression: None,
                        description: None,
                        dynamic: None,
                    });
                }
                _ => {}
            }
        }
    }
}
