//! Language plugin abstraction and shared source helpers.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::Entity;

pub mod cpp;
pub mod python;
pub mod registry;
pub mod rust;

/// A source-language plugin that lowers one file into the shared model.
pub trait Language {
    /// File extensions handled by this plugin, including the leading dot.
    fn extensions(&self) -> &'static [&'static str];

    /// Parse `source`, using `filename` as the module entity's name.
    fn parse(&self, source: &str, filename: &str) -> Vec<Entity>;
}

static BLOCK_START_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*/\*\*").unwrap());
static BLOCK_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\*\s?").unwrap());
pub(crate) static LINE_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"//.*$").unwrap());

/// Count the lines occupied by the block starting at `start`.
///
/// Braces are tracked per character so one-line blocks (`fn foo() {}`) are
/// handled. Without braces, a `;` ends the block.
pub fn consume_block(lines: &[&str], start: usize) -> usize {
    if start >= lines.len() {
        return 0;
    }

    let mut depth: i64 = 0;
    let mut opened = false;
    for (offset, line) in lines[start..].iter().enumerate() {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if opened && depth <= 0 {
            return offset + 1;
        }
        if !opened && line.contains(';') {
            return offset + 1;
        }
    }

    lines.len() - start
}

/// Read a `/** ... */` block starting at `start`.
///
/// Returns `([], 0)` when the line does not start a block comment.
pub fn read_block_comment(lines: &[&str], start: usize) -> (Vec<String>, usize) {
    if start >= lines.len() {
        return (Vec::new(), 0);
    }

    let raw = lines[start];
    let Some(found) = BLOCK_START_RE.find(raw) else {
        return (Vec::new(), 0);
    };

    let mut out = Vec::new();
    let mut index = start;
    while index < lines.len() {
        let text = if index > start {
            lines[index]
        } else {
            &raw[found.end()..]
        };

        if let Some(close) = text.find("*/") {
            let line = BLOCK_PREFIX_RE
                .replace(&text[..close], "")
                .trim_end()
                .to_owned();
            if !line.trim().is_empty() {
                out.push(line);
            }
            return (out, index - start + 1);
        }

        let line = BLOCK_PREFIX_RE.replace(text, "").trim_end().to_owned();
        if !line.trim().is_empty() {
            out.push(line);
        }
        index += 1;
    }

    (out, lines.len() - start)
}

/// Slice `lines[start + 1 .. start + consumed - 1]` safely.
pub fn body_slice<'a>(lines: &'a [&'a str], start: usize, consumed: usize) -> &'a [&'a str] {
    let begin = (start + 1).min(lines.len());
    let end = start
        .saturating_add(consumed)
        .saturating_sub(1)
        .min(lines.len());
    if begin < end { &lines[begin..end] } else { &[] }
}

/// Approximate Python's `str.isidentifier` for the inputs the tools see.
pub(crate) fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '_' && !first.is_alphabetic() {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumes_braced_and_semicolon_blocks() {
        let lines = vec!["fn a() {", "  x();", "}", "fn b();"];
        assert_eq!(consume_block(&lines, 0), 3);
        assert_eq!(consume_block(&lines, 3), 1);
    }

    #[test]
    fn reads_block_comments() {
        let lines = vec!["    /**", "     * Hello", "     * World", "     */"];
        let (block, consumed) = read_block_comment(&lines, 0);
        assert_eq!(block, vec!["Hello", "World"]);
        assert_eq!(consumed, 4);
    }
}
