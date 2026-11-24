//! @brief Bash source parsing through Tree-sitter.
//! @details Lowers Bash functions and source/include commands into shared entities.

use tree_sitter::Node;

use crate::documentation::DOC_PARSER;
use crate::lang::Language;
use crate::model::{Entity, EntityKind, dedup_push};
use crate::parser::tree_sitter::parse_bash;

/// @brief Parse Bash functions and module metadata through Tree-sitter.
pub struct BashLanguage;

impl Language for BashLanguage {
    /// @brief Return Bash source extensions.
    /// @return `.sh` and `.bash` extensions.
    fn extensions(&self) -> &'static [&'static str] {
        &[".sh", ".bash"]
    }

    /// @brief Parse one Bash source file.
    /// @param[in] source Bash source text.
    /// @param[in] filename Source filename used as the module name.
    /// @return Contract entities, or an empty list for invalid syntax.
    fn parse(&self, source: &str, filename: &str) -> Vec<Entity> {
        let Some(tree) = parse_bash(source) else {
            return Vec::new();
        };
        let root = tree.root_node();
        let mut entities = Vec::new();
        let mut module = Entity::new(EntityKind::Module, filename);
        module.line = 1;
        module.doc = DOC_PARSER.parse(&module_doc(source, &root));
        module.includes = includes(source, &root);
        entities.push(module);

        walk_functions(source, &root, &mut entities);
        entities
    }
}

fn text_at(source: &str, node: &Node<'_>) -> String {
    source.get(node.byte_range()).unwrap_or_default().to_owned()
}

fn module_doc(source: &str, _root: &Node<'_>) -> String {
    let mut lines = Vec::new();
    for line in source.lines() {
        let stripped = line.trim();
        if stripped.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if let Some(comment) = stripped.strip_prefix('#') {
            let text = comment.trim();
            if !text.is_empty() {
                lines.push(text);
            }
        } else {
            break;
        }
    }
    lines.join("\n")
}

fn preceding_doc(source: &str, node: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = node.prev_named_sibling();
    let mut expected_row = node.start_position().row;
    while let Some(comment) = current {
        if comment.kind() == "comment" {
            if comment.end_position().row + 1 != expected_row {
                break;
            }
            let text = text_at(source, &comment)
                .trim()
                .trim_start_matches('#')
                .trim()
                .to_owned();
            if !text.is_empty() {
                out.push(text);
            }
            expected_row = comment.start_position().row;
            current = comment.prev_named_sibling();
        } else {
            break;
        }
    }
    out.reverse();
    out
}

fn walk_functions(source: &str, node: &Node<'_>, entities: &mut Vec<Entity>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "function_definition" {
            let name = child
                .child_by_field_name("name")
                .map(|name| text_at(source, &name))
                .unwrap_or_default();
            let mut entity = Entity::new(EntityKind::Function, name);
            entity.signature = format!("{}()", entity.name);
            entity.doc = DOC_PARSER.parse(&preceding_doc(source, &child).join("\n"));
            entity.line = child.start_position().row + 1;
            entity.calls = calls(source, &child);
            entities.push(entity);
        }
        walk_functions(source, &child, entities);
    }
}

fn includes(source: &str, root: &Node<'_>) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        if matches!(node.kind(), "command" | "declaration_command") {
            let text = text_at(source, &node);
            if (text.trim_start().starts_with("source ") || text.trim_start().starts_with(". "))
                && let Some(target) = text.split_whitespace().nth(1)
            {
                dedup_push(
                    &mut out,
                    target.trim_matches(|ch| matches!(ch, '"' | '\'' | ';')),
                );
            }
        }
        collect_includes(source, &node, &mut out);
    }
    out
}

fn collect_includes(source: &str, node: &Node<'_>, out: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "command" {
            let text = text_at(source, &child);
            if (text.trim_start().starts_with("source ") || text.trim_start().starts_with(". "))
                && let Some(target) = text.split_whitespace().nth(1)
            {
                dedup_push(
                    out,
                    target.trim_matches(|ch| matches!(ch, '"' | '\'' | ';')),
                );
            }
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
    if node.kind() == "command" {
        let text = text_at(source, node);
        if let Some(name) = text.split_whitespace().next()
            && !matches!(name, "echo" | "printf" | "return" | "exit" | "local")
        {
            dedup_push(out, name.trim_matches(|ch| matches!(ch, '"' | '\'' | ';')));
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
    fn parses_bash_function() {
        let source = "# Greet someone.\ngreet() {\n  echo hello\n}\n";
        let entities = BashLanguage.parse(source, "script.sh");
        assert_eq!(entities[0].kind, EntityKind::Module);
        assert!(entities.iter().any(|entity| entity.name == "greet"));
    }

    #[test]
    fn rejects_invalid_bash_syntax() {
        assert!(
            BashLanguage
                .parse("if true; then\n", "broken.sh")
                .is_empty()
        );
    }

    #[test]
    fn separates_module_and_function_docs() {
        let source = std::fs::read_to_string("tests/fixtures/animal.sh").unwrap();
        let entities = BashLanguage.parse(&source, "animal.sh");
        let module = entities
            .iter()
            .find(|entity| entity.kind == EntityKind::Module)
            .unwrap();
        assert_eq!(module.doc.brief, "Animal hierarchy shell helpers.");
        assert_eq!(
            module.doc.details,
            "Demonstrates Bash functions, calls, and comments."
        );
        let make_sound = entities
            .iter()
            .find(|entity| entity.name == "make_sound")
            .unwrap();
        assert_eq!(
            make_sound.doc.brief,
            "Produce the characteristic animal sound."
        );
    }
}
