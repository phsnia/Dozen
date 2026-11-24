//! @brief Shared Tree-sitter parser entry points.
//! @details Creates pinned language parsers and rejects incomplete syntax trees.

use crate::lang::Language;
use crate::model::Entity;
use tree_sitter::{Parser, Tree};

/// @brief Create a parser for Python source.
/// @return A configured Python parser.
pub fn python_parser() -> anyhow::Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into())?;
    Ok(parser)
}

/// @brief Parse Python source and reject trees containing syntax errors.
/// @param[in] source Python source text.
/// @return Parsed tree, or `None` for invalid syntax.
pub fn parse_python(source: &str) -> Option<Tree> {
    let mut parser = python_parser().ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    (!root.has_error() && !has_missing(&root)).then_some(tree)
}

/// @brief Create a parser for Rust source.
/// @return A configured Rust parser.
pub fn rust_parser() -> anyhow::Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
    Ok(parser)
}

/// @brief Parse Rust source and reject trees containing syntax errors.
/// @param[in] source Rust source text.
/// @return Parsed tree, or `None` for invalid syntax.
pub fn parse_rust(source: &str) -> Option<Tree> {
    let mut parser = rust_parser().ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    (!root.has_error() && !has_missing(&root)).then_some(tree)
}

/// @brief Create a parser for C++ source.
/// @return A configured C++ parser.
pub fn cpp_parser() -> anyhow::Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?;
    Ok(parser)
}

/// @brief Parse C++ source and reject syntax-error trees.
/// @param[in] source C++ source text.
/// @return Parsed tree, or `None` for invalid syntax.
pub fn parse_cpp(source: &str) -> Option<Tree> {
    let mut parser = cpp_parser().ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    (!root.has_error() && !has_missing(&root)).then_some(tree)
}

/// @brief Create a parser for Bash source.
/// @return A configured Bash parser.
pub fn bash_parser() -> anyhow::Result<Parser> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_bash::LANGUAGE.into())?;
    Ok(parser)
}

/// @brief Parse Bash source and reject syntax-error trees.
/// @param[in] source Bash source text.
/// @return Parsed tree, or `None` for invalid syntax.
pub fn parse_bash(source: &str) -> Option<Tree> {
    let mut parser = bash_parser().ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    (!root.has_error() && !has_missing(&root)).then_some(tree)
}

fn has_missing(node: &tree_sitter::Node<'_>) -> bool {
    if node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if has_missing(&child) {
            return true;
        }
    }
    false
}

/// Parse source with an existing language plugin.
///
/// This is a compatibility facade; language lowering remains implemented by
/// the existing plugin registry and is not duplicated here.
pub fn parse(language: &dyn Language, source: &str, filename: &str) -> Vec<Entity> {
    language.parse(source, filename)
}
