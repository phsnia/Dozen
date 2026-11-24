//! Rust source parsing from doc comments and declaration heuristics.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::ast::Visibility;
use crate::documentation::{DOC_PARSER, split_lines};
use crate::lang::{LINE_COMMENT_RE, Language, body_slice, consume_block, read_block_comment};
use crate::model::{Entity, EntityKind, Field, Variant, dedup_push};

const VIS: &str = r"(?:pub(?:\([^)]*\))?\s+)?";

static DOC_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*///\s?(.*)$").unwrap());
static ATTR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*#\[.*\]\s*$").unwrap());

static FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"^\s*{VIS}(?:(?:const|async|unsafe|extern(?:\s+"[^"]*")?)\s+)*fn\s+(\w+)\s*(?:<[^>]*>)?\s*\((.*?)\)(?:\s*->\s*([^{{;]+?))?\s*(?:\{{|;|where|$)"#
    ))
    .unwrap()
});
static STRUCT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^\s*{VIS}struct\s+(\w+)")).unwrap());
static ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^\s*{VIS}enum\s+(\w+)")).unwrap());
static TRAIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^\s*{VIS}(?:unsafe\s+)?trait\s+(\w+)(?:\s*:\s*([^;]+?))?\s*(?:\{{|;|$)"
    ))
    .unwrap()
});
static IMPL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*impl(?:\s*<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap());
static VARIANT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:#\[.*\]\s*)?(\w+)\s*(?:[({,]|$)").unwrap());
static FIELD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(pub(?:\([^)]*\))?\s+)?(\w+)\s*:\s*(.+?),?\s*$").unwrap());
static USE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:pub\s+)?use\s+([^;]+);").unwrap());
static MOD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:pub\s+)?mod\s+(\w+)\s*;").unwrap());
static EXTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*extern\s+crate\s+(\w+)").unwrap());
static CALL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b([A-Za-z_]\w*)\s*\(").unwrap());

const CALL_STOP: &[&str] = &[
    "if", "for", "while", "match", "loop", "return", "let", "fn", "impl", "struct", "enum",
    "trait", "use", "mod", "as", "in", "where", "move", "async", "await", "dyn", "Some", "None",
    "Ok", "Err", "Self", "self", "super", "crate",
];

/// Parse Rust items from `///` and `/** ... */` doc comments.
pub struct RustLanguage;

impl Language for RustLanguage {
    /// Rust uses a single `.rs` extension.
    fn extensions(&self) -> &'static [&'static str] {
        &[".rs"]
    }

    /// Scan doc comments and emit Rust entities in source order.
    fn parse(&self, source: &str, filename: &str) -> Vec<Entity> {
        let lines = split_lines(source);
        let mut module = Entity::new(EntityKind::Module, filename);
        module.line = 1;
        module.includes = includes(&lines);

        let mut entities = vec![module];
        let mut by_name: HashMap<String, usize> = HashMap::new();
        let mut pending: Vec<String> = Vec::new();
        let mut index = 0;

        while index < lines.len() {
            let line = lines[index];

            if let Some(caps) = DOC_LINE.captures(line) {
                pending.push(caps[1].to_owned());
                index += 1;
                continue;
            }

            if line.contains("/**") {
                let (block, consumed) = read_block_comment(&lines, index);
                if consumed > 0 {
                    pending.extend(block);
                    index += consumed;
                    continue;
                }
            }

            let stripped = line.trim();
            if stripped.is_empty() || stripped.starts_with("//") || ATTR.is_match(line) {
                index += 1;
                continue;
            }

            if let Some(caps) = FN.captures(line) {
                let (entity, consumed) = parse_fn(&caps, &lines, index, &pending);
                let name = entity.name.clone();
                entities.push(entity);
                by_name.insert(name, entities.len() - 1);
                index += consumed;
                pending.clear();
                continue;
            }

            if let Some(caps) = STRUCT.captures(line) {
                let name = caps[1].to_owned();
                let mut entity = Entity::new(EntityKind::Struct, &name);
                entity.doc = DOC_PARSER.parse(&pending.join("\n"));
                let consumed = consume_block(&lines, index);
                entity.fields = parse_rust_fields(body_slice(&lines, index, consumed));
                entity.line = index + 1;
                entities.push(entity);
                by_name.insert(name, entities.len() - 1);
                index += consumed;
                pending.clear();
                continue;
            }

            if let Some(caps) = ENUM.captures(line) {
                let name = caps[1].to_owned();
                let mut entity = Entity::new(EntityKind::Enum, &name);
                entity.doc = DOC_PARSER.parse(&pending.join("\n"));
                let consumed = consume_block(&lines, index);
                entity.variants = parse_variants(body_slice(&lines, index, consumed));
                entity.line = index + 1;
                entities.push(entity);
                by_name.insert(name, entities.len() - 1);
                index += consumed;
                pending.clear();
                continue;
            }

            if let Some(caps) = TRAIT.captures(line) {
                let name = caps[1].to_owned();
                let mut entity = Entity::new(EntityKind::Trait, &name);
                entity.doc = DOC_PARSER.parse(&pending.join("\n"));
                let bounds = caps.get(2).map_or("", |m| m.as_str()).trim();
                entity.bases = bounds
                    .split('+')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect();
                let consumed = consume_block(&lines, index);
                entity.members = parse_impl_body(body_slice(&lines, index, consumed));
                entity.line = index + 1;
                entities.push(entity);
                by_name.insert(name, entities.len() - 1);
                index += consumed;
                pending.clear();
                continue;
            }

            if let Some(caps) = IMPL.captures(line) {
                let consumed = consume_block(&lines, index);
                let inner = body_slice(&lines, index, consumed);
                let methods = parse_impl_body(inner);
                let trait_name = caps.get(1).map(|m| m.as_str());
                let target = caps[2].to_owned();
                if let Some(&entity_index) = by_name.get(&target) {
                    let entity = &mut entities[entity_index];
                    entity.members.extend(methods);
                    if let Some(trait_name) = trait_name {
                        dedup_push(&mut entity.implements, trait_name);
                    }
                }
                index += consumed;
                pending.clear();
                continue;
            }

            pending.clear();
            index += 1;
        }

        entities
    }
}

/// Build a function entity from a regex match and its parameter body.
fn parse_fn(
    caps: &regex::Captures<'_>,
    lines: &[&str],
    start: usize,
    doc_lines: &[String],
) -> (Entity, usize) {
    let name = caps[1].to_owned();
    let args = split_args(caps.get(2).map_or("", |m| m.as_str()));
    let ret = caps.get(3).map_or("", |m| m.as_str()).trim().to_owned();
    let mut signature = format!("fn {name}({})", args.join(", "));
    if !ret.is_empty() {
        signature.push_str(" -> ");
        signature.push_str(&ret);
    }

    let consumed = consume_block(lines, start);
    let mut entity = Entity::new(EntityKind::Function, name);
    entity.signature = signature;
    entity.return_type = ret;
    entity.doc = DOC_PARSER.parse(&doc_lines.join("\n"));
    entity.calls = calls_from_lines(body_slice(lines, start, consumed));
    entity.line = start + 1;
    (entity, consumed)
}

/// Parse documented fields from a struct or enum body.
fn parse_rust_fields(lines: &[&str]) -> Vec<Field> {
    let mut out = Vec::new();
    let mut pending: Vec<String> = Vec::new();

    for raw in lines {
        if let Some(caps) = DOC_LINE.captures(raw) {
            pending.push(caps[1].to_owned());
            continue;
        }

        let stripped = LINE_COMMENT_RE.replace(raw, "");
        let stripped = stripped.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        let Some(caps) = FIELD.captures(stripped) else {
            pending.clear();
            continue;
        };

        let visibility = if caps.get(1).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let mut field = Field::new(&caps[2]);
        field.r#type = caps[3].trim().trim_end_matches(',').to_owned();
        field.visibility = visibility;
        field.description = pending.join(" ");
        out.push(field);
        pending.clear();
    }

    out
}

/// Parse enum variants and their attached doc comments.
fn parse_variants(lines: &[&str]) -> Vec<Variant> {
    let mut out = Vec::new();
    let mut pending: Vec<String> = Vec::new();

    for raw in lines {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(caps) = DOC_LINE.captures(raw) {
            pending.push(caps[1].to_owned());
            continue;
        }
        if let Some(caps) = VARIANT.captures(line) {
            let name = &caps[1];
            if name.chars().next().is_some_and(char::is_alphabetic) {
                out.push(Variant {
                    name: name.to_owned(),
                    description: pending.join(" "),
                });
                pending.clear();
            }
        }
    }

    out
}

/// Parse the methods declared inside an `impl` block.
fn parse_impl_body(lines: &[&str]) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let raw = lines[index];

        if let Some(caps) = DOC_LINE.captures(raw) {
            pending.push(caps[1].to_owned());
            index += 1;
            continue;
        }

        let stripped = raw.trim();
        if stripped.is_empty() || stripped.starts_with("//") || ATTR.is_match(raw) {
            index += 1;
            continue;
        }

        if let Some(caps) = FN.captures(stripped) {
            let name = caps[1].to_owned();
            let args = split_args(caps.get(2).map_or("", |m| m.as_str()));
            let ret = caps.get(3).map_or("", |m| m.as_str()).trim().to_owned();
            let mut signature = format!("fn {name}({})", args.join(", "));
            if !ret.is_empty() {
                signature.push_str(" -> ");
                signature.push_str(&ret);
            }

            let consumed = consume_block(lines, index);
            let block_end = (index + consumed).min(lines.len());
            let block = &lines[index..block_end];
            let mut entity = Entity::new(EntityKind::Method, name);
            entity.signature = signature;
            entity.return_type = ret;
            entity.doc = DOC_PARSER.parse(&pending.join("\n"));
            entity.r#abstract = !block.iter().any(|line| line.contains('{'));
            entity.calls = calls_from_lines(body_slice(lines, index, consumed));
            out.push(entity);
            pending.clear();
            index += consumed;
            continue;
        }

        pending.clear();
        index += 1;
    }

    out
}

/// Collect `use` and `extern crate` targets as module includes.
fn includes(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::new();

    for raw in lines {
        let target = if let Some(caps) = USE.captures(raw) {
            let value = caps[1].split('{').next().unwrap_or("");
            value.trim_end_matches([':', ',', ' ']).trim().to_owned()
        } else if let Some(caps) = MOD.captures(raw).or_else(|| EXTERN.captures(raw)) {
            caps[1].to_owned()
        } else {
            String::new()
        };

        dedup_push(&mut out, target);
    }

    out
}

/// Extract function calls from a block, skipping keywords and control flow.
fn calls_from_lines(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in lines {
        let code = LINE_COMMENT_RE.replace(raw, "");
        for caps in CALL.captures_iter(&code) {
            let name = &caps[1];
            if !CALL_STOP.contains(&name) {
                dedup_push(&mut out, name);
            }
        }
    }
    out
}

/// Split a parameter list on top-level commas.
fn split_args(raw: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth: i32 = 0;
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
        assert!(entities.iter().any(|e| e.name == "Animal"));
        let animal = entities.iter().find(|e| e.name == "Animal").unwrap();
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
}
