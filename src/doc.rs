//! Parse Doxygen-style documentation tags into the shared [`Doc`] model.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{Doc, Param};

static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*[@\\](\w+)(?:\[([^\]]+)\])?(?:\s+(.*))?$").unwrap());

const TAG_ALIASES: &[(&str, &str)] = &[
    ("return", "returns"),
    ("returns", "returns"),
    ("exception", "throws"),
    ("raises", "throws"),
];

const LIST_TAGS: &[(&str, &str)] = &[
    ("see", "see"),
    ("note", "notes"),
    ("warning", "warnings"),
    ("example", "examples"),
];

fn alias(tag: &str) -> &str {
    TAG_ALIASES
        .iter()
        .find_map(|(name, target)| (*name == tag).then_some(*target))
        .unwrap_or(tag)
}

fn list_field(tag: &str) -> Option<&'static str> {
    LIST_TAGS
        .iter()
        .find_map(|(name, target)| (*name == tag).then_some(*target))
}

/// Split text the way Python's `str.splitlines` does.
pub fn split_lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut index = 0;

    while index < text.len() {
        let ch = text[index..].chars().next().expect("valid UTF-8");
        let width = ch.len_utf8();
        let is_break = matches!(
            ch,
            '\n' | '\r'
                | '\u{000b}'
                | '\u{000c}'
                | '\u{001c}'
                | '\u{001d}'
                | '\u{001e}'
                | '\u{0085}'
                | '\u{2028}'
                | '\u{2029}'
        );

        if is_break {
            out.push(&text[start..index]);
            index += width;
            if ch == '\r' && text[index..].starts_with('\n') {
                index += 1;
            }
            start = index;
        } else {
            index += width;
        }
    }

    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// Expand tabs using Python's default tab size and tab stops.
pub fn expand_tabs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut column = 0;

    for ch in text.chars() {
        match ch {
            '\t' => {
                let spaces = 8 - (column % 8);
                out.extend(std::iter::repeat_n(' ', spaces));
                column += spaces;
            }
            '\n' | '\r' => {
                out.push(ch);
                column = 0;
            }
            _ => {
                out.push(ch);
                column += 1;
            }
        }
    }

    out
}

/// Equivalent of Python's `inspect.cleandoc`.
pub fn cleandoc(text: &str) -> String {
    let expanded = expand_tabs(text);
    let mut lines: Vec<String> = expanded.split('\n').map(str::to_owned).collect();

    let mut margin = usize::MAX;
    for line in lines.iter().skip(1) {
        let content = line.trim_start();
        if !content.is_empty() {
            let indent = line.chars().count() - content.chars().count();
            margin = margin.min(indent);
        }
    }

    if let Some(first) = lines.first_mut() {
        *first = first.trim_start().to_owned();
    }
    if margin != usize::MAX {
        for line in lines.iter_mut().skip(1) {
            *line = line.chars().skip(margin).collect();
        }
    }

    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    while lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }

    lines.join("\n")
}

/// Convert a comment block into a structured [`Doc`] model.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocParser;

impl DocParser {
    pub fn parse(&self, text: &str) -> Doc {
        let mut doc = Doc::default();
        if text.trim().is_empty() {
            return doc;
        }

        let lines = split_lines(text);
        let first_tag = lines.iter().position(|line| TAG_RE.is_match(line.trim()));

        let (leading, tag_lines): (&[&str], &[&str]) = match first_tag {
            Some(index) => (&lines[..index], &lines[index..]),
            None => (&lines[..], &[]),
        };

        let paragraphs = Self::paragraphs(leading);
        if let Some(first) = paragraphs.first() {
            doc.brief = first.clone();
            if paragraphs.len() > 1 {
                doc.details = paragraphs[1..].join("\n\n");
            }
        }

        let mut tag: Option<String> = None;
        let mut last_param: Option<usize> = None;
        let mut throw_index: Option<usize> = None;
        let mut list_name: Option<String> = None;
        let mut list_index: Option<usize> = None;

        for raw in tag_lines {
            let line = raw.trim();
            if line.is_empty() {
                if tag.as_deref() == Some("details") {
                    doc.details.push('\n');
                } else if list_name.as_deref() == Some("examples")
                    && let Some(index) = list_index
                {
                    doc.examples[index].push('\n');
                }
                continue;
            }

            if let Some(caps) = TAG_RE.captures(line) {
                let raw_tag = caps.get(1).map_or("", |m| m.as_str()).to_lowercase();
                let resolved = alias(&raw_tag).to_owned();
                let direction = caps.get(2).map_or("in", |m| m.as_str()).to_owned();
                let rest = caps.get(3).map_or("", |m| m.as_str()).trim().to_owned();

                if resolved == "brief" {
                    doc.brief = rest;
                } else if resolved == "details" {
                    doc.details = rest;
                } else if resolved == "param" {
                    let (name, _, desc) = partition_space(&rest);
                    let mut param = Param::new(name);
                    param.direction = direction;
                    param.description = desc.trim().to_owned();
                    doc.params.push(param);
                    last_param = Some(doc.params.len() - 1);
                } else if resolved == "returns" {
                    doc.returns = rest;
                } else if resolved == "throws" {
                    let (exc, _, desc) = partition_space(&rest);
                    doc.throws.push((exc.to_owned(), desc.trim().to_owned()));
                    throw_index = Some(doc.throws.len() - 1);
                } else if let Some(field) = list_field(&resolved) {
                    list_name = Some(field.to_owned());
                    list_index = Some(push_list(&mut doc, field, rest));
                } else {
                    tag = None;
                    continue;
                }
                tag = Some(resolved);
                continue;
            }

            match tag.as_deref() {
                Some("brief") => {
                    doc.brief.push(' ');
                    doc.brief.push_str(line);
                }
                Some("details") => {
                    doc.details.push('\n');
                    doc.details.push_str(raw.trim_end());
                }
                Some("param") => {
                    if let Some(index) = last_param {
                        let param = &mut doc.params[index];
                        param.description =
                            format!("{} {}", param.description, line).trim().to_owned();
                    }
                }
                Some("returns") => {
                    doc.returns.push(' ');
                    doc.returns.push_str(line);
                }
                Some("throws") => {
                    if let Some(index) = throw_index {
                        let (exc, desc) = doc.throws[index].clone();
                        doc.throws[index] = (exc, format!("{desc} {line}").trim().to_owned());
                    }
                }
                _ => {
                    if let Some(name) = &list_name {
                        if let Some(index) = list_index {
                            let values = list_vec(&mut doc, name);
                            if name == "examples" {
                                values[index].push('\n');
                                values[index].push_str(raw.trim_end());
                            } else {
                                values[index].push(' ');
                                values[index].push_str(line);
                            }
                        }
                    } else {
                        doc.brief = format!("{} {}", doc.brief, line).trim().to_owned();
                    }
                }
            }
        }

        doc.brief = doc.brief.trim().to_owned();
        doc.details = doc.details.trim_matches('\n').to_owned();
        doc
    }

    fn paragraphs(lines: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        let mut buffer: Vec<&str> = Vec::new();

        for raw in lines {
            let line = raw.trim();
            if line.is_empty() {
                if !buffer.is_empty() {
                    out.push(buffer.join(" "));
                    buffer.clear();
                }
            } else {
                buffer.push(line);
            }
        }
        if !buffer.is_empty() {
            out.push(buffer.join(" "));
        }
        out
    }
}

fn partition_space(value: &str) -> (&str, &str, &str) {
    match value.find(' ') {
        Some(index) => (&value[..index], " ", &value[index + 1..]),
        None => (value, "", ""),
    }
}

fn list_vec<'a>(doc: &'a mut Doc, name: &str) -> &'a mut Vec<String> {
    match name {
        "see" => &mut doc.see,
        "notes" => &mut doc.notes,
        "warnings" => &mut doc.warnings,
        "examples" => &mut doc.examples,
        _ => unreachable!("unknown list field"),
    }
}

fn push_list(doc: &mut Doc, name: &str, value: String) -> usize {
    let values = list_vec(doc, name);
    values.push(value);
    values.len() - 1
}

pub static DOC_PARSER: DocParser = DocParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_brief_and_paragraphs() {
        let doc = DOC_PARSER.parse("First line\nsecond line.\n\nSecond paragraph.");
        assert_eq!(doc.brief, "First line second line.");
        assert_eq!(doc.details, "Second paragraph.");
    }

    #[test]
    fn parses_tags_and_aliases() {
        let doc = DOC_PARSER
            .parse("@brief Thing\n@param[in] x the x\n@return a value\n@raises ValueError bad");
        assert_eq!(doc.brief, "Thing");
        assert_eq!(doc.params.len(), 1);
        assert_eq!(doc.params[0].name, "x");
        assert_eq!(doc.params[0].direction, "in");
        assert_eq!(doc.returns, "a value");
        assert_eq!(doc.throws, vec![("ValueError".into(), "bad".into())]);
    }

    #[test]
    fn preserves_blank_lines_in_details_and_examples() {
        let doc = DOC_PARSER.parse("@details\nline one\n\nline two\n@example\nfirst\n\nsecond");
        assert_eq!(doc.details, "line one\n\nline two");
        assert_eq!(doc.examples, vec!["\nfirst\n\nsecond"]);
    }

    #[test]
    fn cleandoc_matches_python_semantics() {
        assert_eq!(
            cleandoc("\n    First line.\n        indented\n    last\n\n"),
            "First line.\n    indented\nlast"
        );
    }

    #[test]
    fn split_lines_handles_python_separators() {
        assert_eq!(split_lines("a\nb\rc\r\nd"), vec!["a", "b", "c", "d"]);
        assert_eq!(split_lines("a\n"), vec!["a"]);
        assert_eq!(split_lines("a\n\n"), vec!["a", ""]);
    }
}
