//! @brief Rust source parsing through Tree-sitter.
//! @details Lowers Rust declarations and impl methods into the shared entity model.

use std::collections::HashMap;

use tree_sitter::Node;

use crate::ast::Visibility;
use crate::documentation::DOC_PARSER;
use crate::lang::Language;
use crate::model::{Entity, EntityKind, Field, Variant, dedup_push};
use crate::parser::tree_sitter::parse_rust;

const CALL_STOP: &[&str] = &[
    "if", "for", "while", "match", "loop", "return", "let", "fn", "impl", "struct", "enum",
    "trait", "use", "mod", "as", "in", "where", "move", "async", "await", "dyn", "Some", "None",
    "Ok", "Err", "Self", "self", "super", "crate",
];

/// @brief Parse Rust items from documentation comments and Tree-sitter syntax nodes.
pub struct RustLanguage;

impl Language for RustLanguage {
    /// @brief Return Rust source extensions.
    /// @return The `.rs` extension.
    fn extensions(&self) -> &'static [&'static str] {
        &[".rs"]
    }

    /// @brief Parse one Rust source file.
    /// @param[in] source Rust source text.
    /// @param[in] filename Source filename used as the module name.
    /// @return Contract entities, or an empty list for invalid syntax.
    fn parse(&self, source: &str, filename: &str) -> Vec<Entity> {
        let Some(tree) = parse_rust(source) else {
            return Vec::new();
        };
        let root = tree.root_node();
        let mut entities = Vec::new();
        let mut module = Entity::new(EntityKind::Module, filename);
        module.line = 1;
        module.includes = includes(source, &root);
        entities.push(module);

        let mut by_name: HashMap<String, usize> = HashMap::new();
        let mut cursor = root.walk();
        for node in root.named_children(&mut cursor) {
            match node.kind() {
                "enum_item" => {
                    let entity = enum_entity(source, &node);
                    by_name.insert(entity.name.clone(), entities.len());
                    entities.push(entity);
                }
                "struct_item" => {
                    let entity = struct_entity(source, &node);
                    by_name.insert(entity.name.clone(), entities.len());
                    entities.push(entity);
                }
                "trait_item" => {
                    let entity = trait_entity(source, &node);
                    by_name.insert(entity.name.clone(), entities.len());
                    entities.push(entity);
                }
                "impl_item" => merge_impl(source, &node, &mut entities, &by_name),
                "function_item" => {
                    let entity = function_entity(source, &node, EntityKind::Function);
                    entities.push(entity);
                }
                "extern_crate_declaration" | "use_declaration" | "mod_item" => {}
                _ => {}
            }
        }
        entities
    }
}

fn text_at(source: &str, node: &Node<'_>) -> String {
    source.get(node.byte_range()).unwrap_or_default().to_owned()
}

fn node_name(source: &str, node: &Node<'_>) -> String {
    node.child_by_field_name("name")
        .map(|name| text_at(source, &name))
        .unwrap_or_default()
}

fn preceding_doc(source: &str, node: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = node.prev_sibling();
    while let Some(comment) = current {
        if matches!(comment.kind(), "line_comment" | "block_comment") {
            let text = text_at(source, &comment);
            if comment.kind() == "block_comment" {
                out.extend(clean_block_comment(&text));
            } else {
                out.push(clean_line_comment(&text));
            }
            current = comment.prev_sibling();
        } else if comment.kind() == "attribute_item" {
            current = comment.prev_sibling();
        } else {
            break;
        }
    }
    out.reverse();
    out
}

fn clean_line_comment(text: &str) -> String {
    text.lines()
        .map(|line| {
            let mut rest = line.trim_start_matches("//").trim_start_matches('/');
            if let Some(stripped) = rest.strip_prefix(' ') {
                rest = stripped;
            }
            rest.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn clean_block_comment(text: &str) -> Vec<String> {
    text.trim()
        .strip_prefix("/**")
        .or_else(|| text.trim().strip_prefix("/*"))
        .unwrap_or(text.trim())
        .strip_suffix("*/")
        .unwrap_or(text.trim())
        .lines()
        .map(|line| line.trim().trim_start_matches('*').trim())
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn doc_for(source: &str, node: &Node<'_>) -> crate::model::Doc {
    DOC_PARSER.parse(&preceding_doc(source, node).join("\n"))
}

fn visibility(node: &Node<'_>) -> Visibility {
    let mut cursor = node.walk();
    if node
        .children(&mut cursor)
        .any(|child| child.kind() == "visibility_modifier")
    {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

fn struct_entity(source: &str, node: &Node<'_>) -> Entity {
    let mut entity = Entity::new(EntityKind::Struct, node_name(source, node));
    entity.doc = doc_for(source, node);
    entity.line = node.start_position().row + 1;
    let Some(body) = node.child_by_field_name("body") else {
        return entity;
    };
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        if child.kind() == "field_declaration" {
            let name = child
                .child_by_field_name("name")
                .map(|name| text_at(source, &name))
                .unwrap_or_default();
            let type_name = child
                .child_by_field_name("type")
                .map(|type_| text_at(source, &type_))
                .unwrap_or_default();
            let mut field = Field::new(name);
            field.r#type = type_name;
            field.visibility = visibility(&child);
            field.description = preceding_doc(source, &child).join(" ");
            entity.fields.push(field);
        }
    }
    entity
}

fn enum_entity(source: &str, node: &Node<'_>) -> Entity {
    let mut entity = Entity::new(EntityKind::Enum, node_name(source, node));
    entity.doc = doc_for(source, node);
    entity.line = node.start_position().row + 1;
    let Some(body) = node.child_by_field_name("body") else {
        return entity;
    };
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        if child.kind() == "enum_variant" {
            let name = child
                .child_by_field_name("name")
                .map(|name| text_at(source, &name))
                .unwrap_or_default();
            entity.variants.push(Variant {
                name,
                description: preceding_doc(source, &child).join(" "),
            });
        }
    }
    entity
}

fn trait_entity(source: &str, node: &Node<'_>) -> Entity {
    let mut entity = Entity::new(EntityKind::Trait, node_name(source, node));
    entity.doc = doc_for(source, node);
    entity.line = node.start_position().row + 1;
    if let Some(bounds) = node.child_by_field_name("bounds") {
        let text = text_at(source, &bounds);
        entity.bases = text
            .trim()
            .trim_start_matches(':')
            .trim()
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_owned)
            .collect();
    }
    let Some(body) = node.child_by_field_name("body") else {
        return entity;
    };
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        if matches!(child.kind(), "function_signature_item" | "function_item") {
            let mut method = function_entity(source, &child, EntityKind::Method);
            method.r#abstract = child.kind() == "function_signature_item";
            entity.members.push(method);
        }
    }
    entity
}

fn function_entity(source: &str, node: &Node<'_>, kind: EntityKind) -> Entity {
    let name = node_name(source, node);
    let parameters = node
        .child_by_field_name("parameters")
        .map(|parameters| parameter_text(source, &parameters))
        .unwrap_or_default();
    let return_type = node
        .child_by_field_name("return_type")
        .map(|return_type| text_at(source, &return_type).trim().to_owned())
        .unwrap_or_default();
    let mut signature = format!("fn {name}({parameters})");
    if !return_type.is_empty() {
        signature.push_str(" -> ");
        signature.push_str(&return_type);
    }
    let mut entity = Entity::new(kind, name);
    entity.signature = signature;
    entity.return_type = return_type;
    entity.doc = doc_for(source, node);
    entity.line = node.start_position().row + 1;
    entity.calls = calls(source, node);
    entity.r#abstract = node.kind() == "function_signature_item";
    entity
}

fn parameter_text(source: &str, parameters: &Node<'_>) -> String {
    let mut args = Vec::new();
    let mut cursor = parameters.walk();
    for parameter in parameters.named_children(&mut cursor) {
        let text = text_at(source, &parameter).trim().to_owned();
        if !text.is_empty() {
            args.push(text);
        }
    }
    args.join(", ")
}

fn merge_impl(
    source: &str,
    node: &Node<'_>,
    entities: &mut [Entity],
    by_name: &HashMap<String, usize>,
) {
    let target = node
        .child_by_field_name("type")
        .map(|target| text_at(source, &target).trim().to_owned())
        .unwrap_or_default();
    let Some(&entity_index) = by_name.get(&target) else {
        return;
    };
    let trait_name = node
        .child_by_field_name("trait")
        .map(|trait_| text_at(source, &trait_).trim().to_owned());
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let mut methods = Vec::new();
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        if matches!(child.kind(), "function_item" | "function_signature_item") {
            methods.push(function_entity(source, &child, EntityKind::Method));
        }
    }
    let entity = &mut entities[entity_index];
    entity.members.extend(methods);
    if let Some(trait_name) = trait_name {
        dedup_push(&mut entity.implements, trait_name);
    }
}

fn includes(source: &str, root: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        match node.kind() {
            "use_declaration" => {
                let text = text_at(source, &node);
                let target = text
                    .trim()
                    .strip_prefix("use ")
                    .unwrap_or(&text)
                    .trim()
                    .trim_end_matches(';')
                    .trim();
                let target = target.split('{').next().unwrap_or(target).trim();
                dedup_push(&mut out, target);
            }
            "mod_item" => {
                if let Some(name) = node.child_by_field_name("name") {
                    dedup_push(&mut out, text_at(source, &name));
                }
            }
            "extern_crate_declaration" => {
                if let Some(name) = node.child_by_field_name("name") {
                    dedup_push(&mut out, text_at(source, &name));
                }
            }
            _ => {}
        }
    }
    out
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
        let name = call_name(source, &function);
        if !name.is_empty() && !CALL_STOP.contains(&name.as_str()) {
            dedup_push(out, name);
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_calls(source, &child, out);
    }
}

fn call_name(source: &str, node: &Node<'_>) -> String {
    match node.kind() {
        "identifier" => text_at(source, node),
        "field_expression" => node
            .child_by_field_name("field")
            .map(|field| text_at(source, &field))
            .unwrap_or_default(),
        "scoped_identifier" => node
            .child_by_field_name("name")
            .map(|name| text_at(source, &name))
            .unwrap_or_default(),
        "type_identifier" => text_at(source, node),
        _ => text_at(source, node),
    }
}

/// Split a parameter list on top-level commas.
pub fn split_args(raw: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut buffer = String::new();
    for ch in raw.chars() {
        if matches!(ch, '<' | '(' | '[') {
            depth += 1;
        } else if matches!(ch, '>' | ')' | ']') {
            depth -= 1;
        }
        if ch == ',' && depth == 0 {
            parts.push(buffer.trim().to_owned());
            buffer.clear();
        } else {
            buffer.push(ch);
        }
    }
    if !buffer.trim().is_empty() {
        parts.push(buffer.trim().to_owned());
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_animal_fixture() {
        let source = std::fs::read_to_string("tests/fixtures/animal.rs").unwrap();
        let entities = RustLanguage.parse(&source, "animal.rs");
        assert_eq!(entities[0].kind, EntityKind::Module);
        assert!(entities.iter().any(|entity| entity.name == "Animal"));
        let animal = entities
            .iter()
            .find(|entity| entity.name == "Animal")
            .unwrap();
        assert_eq!(animal.fields.len(), 3);
        assert_eq!(animal.members.len(), 3);
    }

    #[test]
    fn splits_nested_arguments() {
        assert_eq!(
            split_args("a: Vec<u8>, b: (i32, i32), c: &str"),
            vec!["a: Vec<u8>", "b: (i32, i32)", "c: &str"]
        );
    }

    #[test]
    fn parses_mermaid_renderer_impl() {
        let source =
            std::fs::read_to_string("src/renderer/mermaid.rs").expect("read mermaid renderer");
        let entities = RustLanguage.parse(&source, "mermaid.rs");
        let renderer = entities
            .iter()
            .find(|entity| entity.name == "MermaidRenderer")
            .expect("MermaidRenderer entity");
        assert_eq!(renderer.members.len(), 12);
    }
}
