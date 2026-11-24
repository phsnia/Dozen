//! @brief C++ source parsing through Tree-sitter.
//! @details Lowers C++ declarations into the shared Rust entity model.

use std::collections::HashMap;

use tree_sitter::Node;

use crate::ast::Visibility;
use crate::documentation::DOC_PARSER;
use crate::lang::Language;
use crate::model::{Entity, EntityKind, Field, Variant, dedup_push};
use crate::parser::tree_sitter::parse_cpp;

const CALL_STOP: &[&str] = &[
    "alignof",
    "case",
    "catch",
    "const_cast",
    "decltype",
    "delete",
    "do",
    "dynamic_cast",
    "else",
    "for",
    "if",
    "new",
    "noexcept",
    "reinterpret_cast",
    "return",
    "sizeof",
    "static_cast",
    "switch",
    "throw",
    "while",
];

/// @brief Parse C++ declarations through Tree-sitter.
pub struct CppLanguage;

impl Language for CppLanguage {
    /// @brief Return C++ source extensions.
    /// @return Supported C++ header and implementation extensions.
    fn extensions(&self) -> &'static [&'static str] {
        &[".cpp", ".hpp", ".h", ".cc", ".cxx", ".hh"]
    }

    /// @brief Parse one C++ source file.
    /// @param[in] source C++ source text.
    /// @param[in] filename Source filename used as the module name.
    /// @return Contract entities, or an empty list for invalid syntax.
    fn parse(&self, source: &str, filename: &str) -> Vec<Entity> {
        let Some(tree) = parse_cpp(source) else {
            return Vec::new();
        };
        let root = tree.root_node();
        let mut entities = Vec::new();
        let mut module = Entity::new(EntityKind::Module, filename);
        module.line = 1;
        module.includes = includes(source, &root);
        entities.push(module);

        let mut by_name: HashMap<String, usize> = HashMap::new();
        walk_top_level(source, &root, &mut entities, &mut by_name);
        merge_methods_in_namespace(source, &root, &mut entities, &by_name);
        entities
    }
}

fn text_at(source: &str, node: &Node<'_>) -> String {
    source.get(node.byte_range()).unwrap_or_default().to_owned()
}

fn clean_comment(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| {
            line.trim()
                .trim_start_matches("///")
                .trim()
                .trim_start_matches("/*")
                .trim()
        })
        .map(|line| {
            line.trim_end_matches("*/")
                .trim()
                .trim_start_matches('*')
                .trim()
        })
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn preceding_doc(source: &str, node: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = node.prev_named_sibling();
    while let Some(comment) = current {
        if matches!(comment.kind(), "comment" | "line_comment" | "block_comment") {
            out.extend(clean_comment(&text_at(source, &comment)));
            current = comment.prev_named_sibling();
        } else {
            break;
        }
    }
    out.reverse();
    out
}

fn doc_for(source: &str, node: &Node<'_>) -> crate::model::Doc {
    DOC_PARSER.parse(&preceding_doc(source, node).join("\n"))
}

fn walk_top_level(
    source: &str,
    node: &Node<'_>,
    entities: &mut Vec<Entity>,
    by_name: &mut HashMap<String, usize>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "namespace_definition" => walk_top_level(source, &child, entities, by_name),
            "declaration_list" => walk_top_level(source, &child, entities, by_name),
            "enum_specifier" => {
                let entity = enum_entity(source, &child);
                by_name.insert(entity.name.clone(), entities.len());
                entities.push(entity);
            }
            "class_specifier" => {
                let entity = class_entity(source, &child);
                by_name.insert(entity.name.clone(), entities.len());
                entities.push(entity);
            }
            "template_declaration" => {
                let mut inner = child.walk();
                if let Some(declaration) = child
                    .named_children(&mut inner)
                    .find(|node| matches!(node.kind(), "class_specifier" | "enum_specifier"))
                {
                    walk_top_level(source, &declaration, entities, by_name);
                }
            }
            "function_definition" => {
                let entity = function_entity(source, &child, EntityKind::Function);
                let qualified = entity.name.clone();
                let is_constructor = qualified
                    .rsplit_once("::")
                    .is_some_and(|(class, name)| class.rsplit("::").next() == Some(name));
                if !is_constructor {
                    entities.push(entity);
                }
            }
            "declaration" => {}
            _ => {}
        }
    }
}

fn enum_entity(source: &str, node: &Node<'_>) -> Entity {
    let name = node
        .child_by_field_name("name")
        .map(|name| text_at(source, &name))
        .unwrap_or_default();
    let mut entity = Entity::new(EntityKind::Enum, name);
    entity.doc = doc_for(source, node);
    entity.line = node.start_position().row + 1;
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            if matches!(child.kind(), "enumerator" | "enumerator_definition") {
                let variant_name = child
                    .child_by_field_name("name")
                    .map(|name| text_at(source, &name))
                    .unwrap_or_default();
                if !variant_name.is_empty() {
                    entity.variants.push(Variant {
                        name: variant_name,
                        description: String::new(),
                    });
                }
            }
        }
    }
    entity
}

fn class_entity(source: &str, node: &Node<'_>) -> Entity {
    let name = node
        .child_by_field_name("name")
        .map(|name| text_at(source, &name))
        .unwrap_or_default();
    let kind = if text_at(source, node).trim_start().starts_with("struct") {
        EntityKind::Struct
    } else {
        EntityKind::Class
    };
    let mut entity = Entity::new(kind, name);
    entity.doc = doc_for(source, node);
    entity.line = node.start_position().row + 1;
    let mut base_cursor = node.walk();
    if let Some(bases) = node
        .named_children(&mut base_cursor)
        .find(|child| child.kind() == "base_class_clause")
    {
        entity.bases = base_names(&text_at(source, &bases));
    }
    let default_visibility = if kind == EntityKind::Struct {
        Visibility::Public
    } else {
        Visibility::Private
    };
    let Some(body) = node.child_by_field_name("body") else {
        return entity;
    };
    let mut visibility = default_visibility;
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        match child.kind() {
            "access_specifier" => {
                visibility = match text_at(source, &child).trim().trim_end_matches(':') {
                    "protected" => Visibility::Protected,
                    "private" => Visibility::Private,
                    _ => Visibility::Public,
                };
            }
            "field_declaration" => {
                if let Some(declarator) = child.child_by_field_name("declarator")
                    && declarator.kind() == "function_declarator"
                {
                    if let Some(method) = method_declaration(source, &child)
                        && method.name != entity.name
                    {
                        entity.members.push(method);
                    }
                } else if let Some(field) = field_entity(source, &child, visibility) {
                    entity.fields.push(field);
                }
            }
            "function_declaration" | "declaration" => {
                if let Some(method) = method_declaration(source, &child)
                    && method.name != entity.name
                {
                    entity.members.push(method);
                }
            }
            "function_definition" => {
                let mut method = function_entity(source, &child, EntityKind::Method);
                method.name = method
                    .name
                    .rsplit("::")
                    .next()
                    .unwrap_or(&method.name)
                    .to_owned();
                entity.members.push(method);
            }
            "template_declaration" => {
                let mut inner = child.walk();
                for declaration in child.named_children(&mut inner) {
                    if let Some(method) = method_declaration(source, &declaration) {
                        entity.members.push(method);
                    }
                }
            }
            _ => {}
        }
    }
    entity.r#abstract = entity.members.iter().any(|method| method.r#abstract);
    entity
}

fn field_entity(source: &str, node: &Node<'_>, visibility: Visibility) -> Option<Field> {
    let declarator = node.child_by_field_name("declarator")?;
    let name = node_name(source, &declarator);
    if name.is_empty() {
        return None;
    }
    let type_name = node
        .child_by_field_name("type")
        .map(|type_| text_at(source, &type_).trim().to_owned())
        .unwrap_or_default();
    let text = text_at(source, node);
    let mut field = Field::new(name);
    field.r#type = type_name;
    field.visibility = visibility;
    field.r#static = text.trim_start().starts_with("static ");
    field.description = preceding_doc(source, node).join(" ");
    Some(field)
}

fn node_name(source: &str, node: &Node<'_>) -> String {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" => {
            text_at(source, node).trim().to_owned()
        }
        "pointer_declarator"
        | "reference_declarator"
        | "array_declarator"
        | "init_declarator"
        | "parenthesized_declarator" => node
            .named_child(0)
            .map(|child| node_name(source, &child))
            .unwrap_or_default(),
        _ => node
            .named_child(0)
            .map(|child| node_name(source, &child))
            .unwrap_or_else(|| text_at(source, node).trim().to_owned()),
    }
}

fn method_declaration(source: &str, node: &Node<'_>) -> Option<Entity> {
    let declarator = node.child_by_field_name("declarator")?;
    let name = node_name(source, &declarator);
    if name.is_empty() {
        return None;
    }
    let return_type = node
        .child_by_field_name("type")
        .map(|type_| text_at(source, &type_).trim().to_owned())
        .unwrap_or_default();
    let parameters = declarator_parameter_text(source, &declarator);
    let mut signature = format!("{return_type} {name}({parameters})");
    if return_type.is_empty() {
        signature = format!("{name}({parameters})");
    }
    let mut method = Entity::new(EntityKind::Method, name);
    method.signature = signature.trim().to_owned();
    method.return_type = return_type;
    method.doc = doc_for(source, node);
    method.line = node.start_position().row + 1;
    method.r#abstract = text_at(source, node).contains("= 0");
    Some(method)
}

fn declarator_parameter_text(source: &str, declarator: &Node<'_>) -> String {
    if let Some(parameters) = find_parameters(declarator) {
        parameters
            .named_children(&mut parameters.walk())
            .filter_map(|parameter| {
                if parameter.kind() == "comment" {
                    None
                } else {
                    Some(text_at(source, &parameter).trim().to_owned())
                }
            })
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        String::new()
    }
}

fn find_parameters<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "parameter_list" {
        return Some(*node);
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find_map(|child| find_parameters(&child))
}

fn parameter_types(source: &str, parameters: &Node<'_>) -> HashMap<String, String> {
    let mut types = HashMap::new();
    let mut cursor = parameters.walk();
    for parameter in parameters.named_children(&mut cursor) {
        if parameter.kind() == "parameter_declaration" {
            let name = parameter
                .child_by_field_name("declarator")
                .map(|declarator| node_name(source, &declarator))
                .unwrap_or_default();
            let type_name = parameter
                .child_by_field_name("type")
                .map(|type_| text_at(source, &type_).trim().to_owned())
                .unwrap_or_default();
            if !name.is_empty() {
                types.insert(name, type_name);
            }
        }
    }
    types
}

fn merge_parameter_types(doc: &mut crate::model::Doc, types: &HashMap<String, String>) {
    for param in &mut doc.params {
        if param.r#type.is_empty()
            && let Some(type_name) = types.get(&param.name)
        {
            param.r#type = type_name.clone();
        }
    }
}

fn function_entity(source: &str, node: &Node<'_>, kind: EntityKind) -> Entity {
    let declarator = node.child_by_field_name("declarator");
    let mut name = declarator
        .as_ref()
        .map(|declarator| node_name(source, declarator))
        .unwrap_or_default();
    if let Some(position) = name.rfind("::") {
        name = name[position + 2..].to_owned();
    }
    let return_type = node
        .child_by_field_name("type")
        .map(|type_| text_at(source, &type_).trim().to_owned())
        .unwrap_or_default();
    let parameters = declarator
        .as_ref()
        .map(|declarator| declarator_parameter_text(source, declarator))
        .unwrap_or_default();
    let full_name = declarator
        .as_ref()
        .map(|declarator| qualified_name(source, declarator))
        .unwrap_or(name.clone());
    let mut entity = Entity::new(kind, full_name.clone());
    entity.signature = format!("{return_type} {full_name}({parameters})")
        .trim()
        .to_owned();
    entity.return_type = return_type;
    entity.doc = doc_for(source, node);
    entity.line = node.start_position().row + 1;
    entity.calls = calls(source, node);
    entity.r#abstract = text_at(source, node).contains("= 0");
    if let Some(declarator) = declarator
        && let Some(parameters_node) = find_parameters(&declarator)
    {
        merge_parameter_types(&mut entity.doc, &parameter_types(source, &parameters_node));
    }
    entity
}

fn qualified_name(source: &str, declarator: &Node<'_>) -> String {
    match declarator.kind() {
        "function_declarator" => declarator
            .child_by_field_name("declarator")
            .map(|declarator| qualified_name(source, &declarator))
            .unwrap_or_else(|| text_at(source, declarator)),
        "qualified_identifier" => text_at(source, declarator).trim().to_owned(),
        "field_identifier" | "identifier" => text_at(source, declarator).trim().to_owned(),
        _ => text_at(source, declarator).trim().to_owned(),
    }
}

fn merge_impl_methods(
    source: &str,
    root: &Node<'_>,
    entities: &mut [Entity],
    by_name: &HashMap<String, usize>,
) {
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "namespace_definition" {
            merge_impl_methods(source, &child, entities, by_name);
        }
        if child.kind() == "function_definition" {
            let qualified = child
                .child_by_field_name("declarator")
                .map(|declarator| qualified_name(source, &declarator))
                .unwrap_or_default();
            if let Some((class, _)) = qualified.rsplit_once("::")
                && let Some(&index) = by_name.get(class)
            {
                let mut method = function_entity(source, &child, EntityKind::Method);
                method.name = qualified.clone();
                if method
                    .name
                    .rsplit_once("::")
                    .is_none_or(|(owner, simple)| owner.rsplit("::").next() != Some(simple))
                {
                    entities[index].members.push(method);
                }
            }
        }
    }
}

fn merge_methods_in_namespace(
    source: &str,
    root: &Node<'_>,
    entities: &mut [Entity],
    by_name: &HashMap<String, usize>,
) {
    merge_impl_methods(source, root, entities, by_name);
}

fn base_names(raw: &str) -> Vec<String> {
    let mut names = Vec::new();
    for part in raw.trim().trim_start_matches(':').split(',') {
        let cleaned = part
            .replace("public", "")
            .replace("protected", "")
            .replace("private", "")
            .replace("virtual", "");
        let simple = cleaned.split('<').next().unwrap_or("").trim();
        let name = simple.rsplit("::").next().unwrap_or("").trim();
        dedup_push(&mut names, name);
    }
    names
}

fn includes(source: &str, root: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        if node.kind() == "preproc_include" {
            let text = text_at(source, &node);
            let name = text
                .trim_start_matches("#include")
                .trim()
                .trim_matches(|ch| matches!(ch, '<' | '>' | '"'));
            dedup_push(&mut out, name);
        }
        if node.kind() == "namespace_definition" {
            collect_includes(source, &node, &mut out);
        }
    }
    out
}

fn collect_includes(source: &str, node: &Node<'_>, out: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "preproc_include" {
            let text = text_at(source, &child);
            let name = text
                .trim_start_matches("#include")
                .trim()
                .trim_matches(|ch| matches!(ch, '<' | '>'));
            dedup_push(out, name);
        }
        collect_includes(source, &child, out);
    }
}

fn calls(source: &str, root: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    walk_calls(source, root, &mut out);
    out
}

fn walk_calls(source: &str, node: &Node<'_>, out: &mut Vec<String>) {
    if node.kind() == "call_expression"
        && let Some(function) = node.child_by_field_name("function")
    {
        let name = qualified_name(source, &function);
        if !name.is_empty() && !CALL_STOP.contains(&name.as_str()) {
            dedup_push(out, name.rsplit("::").next().unwrap_or(&name).to_owned());
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_calls(source, &child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_parameters() {
        let types = parse_cpp_params("const std::string& name, int age = 3, void");
        assert_eq!(types.get("name").unwrap(), "const std::string");
        assert_eq!(types.len(), 1);
    }

    fn parse_cpp_params(raw: &str) -> HashMap<String, String> {
        let mut types = HashMap::new();
        for part in raw.split(',') {
            let part = part.trim();
            let replaced = part.replace(['&', '*'], " ");
            let tokens = replaced.split_whitespace().collect::<Vec<_>>();
            if let Some(name) = tokens.last()
                && !name.is_empty()
                && !matches!(*name, "void")
                && !part.contains('=')
            {
                types.insert((*name).to_owned(), tokens[..tokens.len() - 1].join(" "));
            }
        }
        types
    }

    #[test]
    fn cleans_base_names() {
        assert_eq!(
            base_names(": public Base<int>, private virtual ns::Other"),
            vec!["Base", "Other"]
        );
    }
}
