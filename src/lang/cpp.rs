//! C++ declaration parsing from doc comments and column-zero heuristics.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::ast::Visibility;
use crate::documentation::{DOC_PARSER, split_lines};
use crate::lang::{
    LINE_COMMENT_RE, Language, body_slice, consume_block, is_identifier, read_block_comment,
};
use crate::model::{Entity, EntityKind, Field, Variant, dedup_push};

static DOC_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*///\s?(.*)$").unwrap());

static FN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:template\s*<[^>]*>\s*)?(?:(?:virtual|static|inline|constexpr|explicit|friend)\s+)*([\w:]+(?:\s*<[^>]*>)?(?:\s*[*&])?)\s+(?:(\w+)::)?(\w+)\s*\(([^)]*)\)\s*(?:const)?\s*(?:override)?\s*(?:=\s*0)?\s*(?:\{|;)",
    )
    .unwrap()
});
static CLASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:template\s*<[^>]*>\s*)?(class|struct)\s+(\w+)\s*(?::\s*([^{]+))?\s*\{")
        .unwrap()
});
static ENUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:enum(?:\s+class)?)\s+(\w+)\s*\{").unwrap());
static ABSTRACT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"=\s*0\s*;").unwrap());
static FIELD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:(?:static|mutable|constexpr|inline|const)\s+)*([\w:<>,\s\*&]+?)\s+(\w+)\s*(?:\[[^\]]*\])?\s*(?:=\s*[^;]+)?;$",
    )
    .unwrap()
});
static INCLUDE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*#\s*include\s*[<"]([^>"]+)[>"]"#).unwrap());
static CALL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b([A-Za-z_]\w*)\s*\(").unwrap());
static ACCESS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(public|protected|private)\s*:\s*$").unwrap());
static BASE_ACCESS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(public|protected|private|virtual)\b").unwrap());
static STATIC_FIELD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^static\b").unwrap());

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

/// Parse C++ declarations from `///` and `/** ... */` comments.
pub struct CppLanguage;

impl Language for CppLanguage {
    /// C++ headers and implementation files share one parser.
    fn extensions(&self) -> &'static [&'static str] {
        &[".cpp", ".hpp", ".h", ".cc", ".cxx", ".hh"]
    }

    /// Scan doc comments and emit C++ entities in source order.
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
            let raw = lines[index];

            if let Some(caps) = DOC_LINE.captures(raw) {
                pending.push(caps[1].to_owned());
                index += 1;
                continue;
            }

            if raw.contains("/**") {
                let (block, consumed) = read_block_comment(&lines, index);
                if consumed > 0 {
                    pending.extend(block);
                    index += consumed;
                    continue;
                }
            }

            let stripped = raw.trim();
            if stripped.is_empty()
                || stripped.starts_with("//")
                || stripped.starts_with("/*")
                || stripped.starts_with('#')
            {
                index += 1;
                continue;
            }

            if let Some(caps) = FN.captures(raw) {
                let ret = caps[1].trim().to_owned();
                let class = caps.get(2).map(|m| m.as_str());
                let simple_name = caps[3].to_owned();
                let args_raw = caps[4].to_owned();
                let full_name = match class {
                    Some(class) => format!("{class}::{simple_name}"),
                    None => simple_name.clone(),
                };
                let signature = format!("{ret} {full_name}({})", args_raw.trim());
                let params = parse_cpp_params(&args_raw);
                let mut doc = DOC_PARSER.parse(&pending.join("\n"));
                merge_types(&mut doc, &params);
                let abstract_ = ABSTRACT.is_match(raw);
                let consumed = consume_block(&lines, index);
                let calls = calls_from_lines(body_slice(&lines, index, consumed));

                let mut entity = if class.is_some() {
                    Entity::new(EntityKind::Method, &full_name)
                } else {
                    Entity::new(EntityKind::Function, &simple_name)
                };
                entity.signature = signature;
                entity.return_type = ret;
                entity.doc = doc;
                entity.r#abstract = abstract_;
                entity.line = index + 1;
                entity.calls = calls;

                if let Some(class) = class {
                    if let Some(&class_index) = by_name.get(class) {
                        entities[class_index].members.push(entity);
                    } else {
                        entities.push(entity);
                    }
                } else {
                    entities.push(entity);
                    by_name.insert(simple_name, entities.len() - 1);
                }

                index += consumed;
                pending.clear();
                continue;
            }

            if let Some(caps) = CLASS.captures(raw) {
                let keyword = &caps[1];
                let name = caps[2].to_owned();
                let base_clause = caps.get(3).map(|m| m.as_str());
                let kind = if keyword == "class" {
                    EntityKind::Class
                } else {
                    EntityKind::Struct
                };
                let default_visibility = if kind == EntityKind::Struct {
                    Visibility::Public
                } else {
                    Visibility::Private
                };
                let doc = DOC_PARSER.parse(&pending.join("\n"));
                let consumed = consume_block(&lines, index);
                let (members, fields) =
                    parse_class_body(body_slice(&lines, index, consumed), default_visibility);
                let abstract_ = members.iter().any(|member| member.r#abstract);
                let mut entity = Entity::new(kind, &name);
                entity.doc = doc;
                entity.r#abstract = abstract_;
                entity.members = members;
                entity.fields = fields;
                entity.bases = base_names(base_clause);
                entity.line = index + 1;
                entities.push(entity);
                by_name.insert(name, entities.len() - 1);
                index += consumed;
                pending.clear();
                continue;
            }

            if let Some(caps) = ENUM.captures(raw) {
                let name = caps[1].to_owned();
                let doc = DOC_PARSER.parse(&pending.join("\n"));
                let consumed = consume_block(&lines, index);
                let body = body_slice(&lines, index, consumed);
                let mut variants = Vec::new();
                for line in body {
                    for value in line.split(',') {
                        let value = value.trim();
                        if !value.is_empty() && is_identifier(value) {
                            variants.push(Variant {
                                name: value.to_owned(),
                                description: String::new(),
                            });
                        }
                    }
                }
                let mut entity = Entity::new(EntityKind::Enum, &name);
                entity.doc = doc;
                entity.variants = variants;
                entity.line = index + 1;
                entities.push(entity);
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

/// Parse members from a class or struct body using the current visibility.
fn parse_class_body(lines: &[&str], default_visibility: Visibility) -> (Vec<Entity>, Vec<Field>) {
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut visibility = default_visibility;
    let mut index = 0;

    while index < lines.len() {
        let raw = lines[index];

        if let Some(caps) = DOC_LINE.captures(raw) {
            pending.push(caps[1].to_owned());
            index += 1;
            continue;
        }

        let stripped = LINE_COMMENT_RE.replace(raw, "");
        let stripped = stripped.trim();
        if stripped.is_empty() || stripped.starts_with("/*") {
            index += 1;
            continue;
        }

        if let Some(caps) = ACCESS.captures(stripped) {
            visibility = Visibility::from_keyword(&caps[1]);
            pending.clear();
            index += 1;
            continue;
        }

        if stripped.starts_with("using ")
            || stripped.starts_with("typedef ")
            || stripped.starts_with("friend ")
            || stripped.starts_with("template ")
        {
            pending.clear();
            index += 1;
            continue;
        }

        if let Some(caps) = FN.captures(stripped) {
            let ret = caps[1].trim().to_owned();
            let name = caps[3].to_owned();
            let signature = format!("{ret} {name}({})", caps[4].trim());
            let abstract_ = ABSTRACT.is_match(stripped);
            let doc = DOC_PARSER.parse(&pending.join("\n"));
            let consumed = consume_block(lines, index);
            let mut entity = Entity::new(EntityKind::Method, name);
            entity.signature = signature;
            entity.return_type = ret;
            entity.doc = doc;
            entity.r#abstract = abstract_;
            entity.calls = calls_from_lines(body_slice(lines, index, consumed));
            methods.push(entity);
            pending.clear();
            index += consumed;
            continue;
        }

        if stripped.contains('(') || stripped.contains('{') || stripped.contains('}') {
            pending.clear();
            index += 1;
            continue;
        }

        if let Some(caps) = FIELD.captures(stripped) {
            let type_ = caps[1].trim();
            if type_ != "return" && type_ != "delete" {
                let mut field = Field::new(&caps[2]);
                field.r#type = type_.to_owned();
                field.visibility = visibility;
                field.r#static = STATIC_FIELD.is_match(stripped);
                field.description = pending.join(" ");
                fields.push(field);
            }
            pending.clear();
            index += 1;
            continue;
        }

        pending.clear();
        index += 1;
    }

    (methods, fields)
}

/// Split a base-class clause into individual base names.
fn base_names(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw else {
        return Vec::new();
    };

    let mut names = Vec::new();
    for part in raw.split(',') {
        let cleaned = BASE_ACCESS.replace_all(part, "");
        let simple = cleaned.split('<').next().unwrap_or("").trim();
        let name = simple.rsplit("::").next().unwrap_or("").trim();
        dedup_push(&mut names, name);
    }
    names
}

/// Collect `#include` targets as module includes.
fn includes(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for raw in lines {
        if let Some(caps) = INCLUDE.captures(raw) {
            dedup_push(&mut out, &caps[1]);
        }
    }
    out
}

/// Extract function calls from a block, skipping C++ keywords and casts.
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

/// Map parameter names to their declared types.
fn parse_cpp_params(raw: &str) -> HashMap<String, String> {
    let mut types = HashMap::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() || part == "void" {
            continue;
        }
        let replaced = part.replace(['&', '*'], " ");
        let tokens: Vec<&str> = replaced.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let mut name = tokens[tokens.len() - 1];
        if let Some((before, _)) = name.split_once('=') {
            name = before.trim();
        }
        let type_ = tokens[..tokens.len() - 1].join(" ").trim().to_owned();
        if is_identifier(name) {
            types.insert(name.to_owned(), type_);
        }
    }
    types
}

/// Fill missing parameter types from the parsed declaration.
fn merge_types(doc: &mut crate::model::Doc, types: &HashMap<String, String>) {
    for param in &mut doc.params {
        if param.r#type.is_empty()
            && let Some(type_) = types.get(&param.name)
        {
            param.r#type = type_.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_parameters() {
        let types = parse_cpp_params("const std::string& name, int age = 3, void");
        assert_eq!(types.get("name").unwrap(), "const std::string");
        assert_eq!(types.get("age"), None);
        assert_eq!(types.len(), 1);
    }

    #[test]
    fn cleans_base_names() {
        assert_eq!(
            base_names(Some("public Base<int>, private virtual ns::Other")),
            vec!["Base", "Other"]
        );
    }
}
