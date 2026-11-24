//! Parse Doxygen-style documentation tags into the shared [`Doc`] model.
//!
//! The parser is deliberately a small state machine: `dox.py` splits a comment
//! into a leading summary and a sequence of tagged lines, then appends
//! continuation lines to whichever field the last tag selected.

use std::sync::LazyLock;

use regex::Regex;

use crate::model::{Doc, Param};

static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*[@\\](\w+)(?:\[([^\]]+)\])?(?:\s+(.*))?$").unwrap());

/// A recognised documentation tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    /// `@brief` — short summary.
    Brief,
    /// `@details` — long description.
    Details,
    /// `@param[in]` — parameter documentation.
    Param,
    /// `@return` / `@returns` — return value documentation.
    Returns,
    /// `@throws` / `@exception` / `@raises` — exception documentation.
    Throws,
    /// `@see` — cross-reference list entry.
    See,
    /// `@note` — note list entry.
    Note,
    /// `@warning` — warning list entry.
    Warning,
    /// `@example` — example block.
    Example,
    /// Any other tag; ignored without changing the active field.
    Unknown,
}

impl Tag {
    /// Resolve a lowercased tag name, applying `dox.py`'s aliases.
    pub fn from_name(name: &str) -> Self {
        match name {
            "brief" => Self::Brief,
            "details" => Self::Details,
            "param" => Self::Param,
            "return" | "returns" => Self::Returns,
            "throws" | "exception" | "raises" => Self::Throws,
            "see" => Self::See,
            "note" => Self::Note,
            "warning" => Self::Warning,
            "example" => Self::Example,
            _ => Self::Unknown,
        }
    }

    /// Return the list kind this tag writes to, when it is a list tag.
    pub const fn list_kind(self) -> Option<ListKind> {
        match self {
            Self::See => Some(ListKind::See),
            Self::Note => Some(ListKind::Note),
            Self::Warning => Some(ListKind::Warning),
            Self::Example => Some(ListKind::Example),
            _ => None,
        }
    }
}

/// A repeated documentation list carried by [`Doc`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    /// `@see` references.
    See,
    /// `@note` notes.
    Note,
    /// `@warning` warnings.
    Warning,
    /// `@example` examples.
    Example,
}

/// Builder state for one tagged documentation block.
struct DocBuilder {
    /// Document assembled so far.
    doc: Doc,
    /// Tag whose continuation lines are currently being consumed.
    active: Option<Tag>,
    /// Index of the parameter receiving continuation lines.
    param: Option<usize>,
    /// Index of the exception receiving continuation lines.
    throw: Option<usize>,
    /// Current list and the index receiving continuation lines.
    list: Option<(ListKind, usize)>,
}

impl DocBuilder {
    /// Seed the builder with the untagged leading paragraphs.
    fn new(leading: &[&str]) -> Self {
        let mut paragraphs = Self::paragraphs(leading).into_iter();
        let mut doc = Doc::default();
        if let Some(brief) = paragraphs.next() {
            doc.brief = brief;
            doc.details = paragraphs.collect::<Vec<_>>().join("\n\n");
        }
        Self {
            doc,
            active: None,
            param: None,
            throw: None,
            list: None,
        }
    }

    /// Consume one tagged line.
    fn consume(&mut self, raw: &str) {
        let line = raw.trim();
        if line.is_empty() {
            self.blank_line();
        } else if let Some(captures) = TAG_RE.captures(line) {
            self.begin_tag(&captures);
        } else {
            self.continuation(raw, line);
        }
    }

    /// Handle a blank line inside a details block or example.
    fn blank_line(&mut self) {
        if self.active == Some(Tag::Details) {
            self.doc.details.push('\n');
        } else if let Some((ListKind::Example, index)) = self.list {
            self.doc.examples[index].push('\n');
        }
    }

    /// Start a new tag, seeding the selected field with its inline value.
    fn begin_tag(&mut self, captures: &regex::Captures<'_>) {
        let raw_name = captures
            .get(1)
            .map_or("", |found| found.as_str())
            .to_lowercase();
        let tag = Tag::from_name(&raw_name);
        let direction = captures.get(2).map_or("in", |found| found.as_str());
        let rest = captures.get(3).map_or("", |found| found.as_str()).trim();

        match tag {
            Tag::Brief => self.doc.brief = rest.to_owned(),
            Tag::Details => self.doc.details = rest.to_owned(),
            Tag::Param => {
                let index = self.push_param(rest, direction);
                self.param = Some(index);
            }
            Tag::Returns => self.doc.returns = rest.to_owned(),
            Tag::Throws => {
                let index = self.push_throw(rest);
                self.throw = Some(index);
            }
            Tag::Unknown => {
                self.active = None;
                return;
            }
            Tag::See | Tag::Note | Tag::Warning | Tag::Example => {
                let kind = tag.list_kind().expect("list tag");
                let index = self.push_list(kind, rest);
                self.list = Some((kind, index));
            }
        }

        self.active = Some(tag);
    }

    /// Append a plain continuation line to the active field.
    fn continuation(&mut self, raw: &str, line: &str) {
        match self.active {
            Some(Tag::Brief) => {
                self.doc.brief.push(' ');
                self.doc.brief.push_str(line);
            }
            Some(Tag::Details) => {
                self.doc.details.push('\n');
                self.doc.details.push_str(raw.trim_end());
            }
            Some(Tag::Param) => {
                if let Some(index) = self.param {
                    self.doc.params[index].extend_description(line);
                }
            }
            Some(Tag::Returns) => {
                self.doc.returns.push(' ');
                self.doc.returns.push_str(line);
            }
            Some(Tag::Throws) => {
                if let Some(index) = self.throw {
                    let (exception, description) = self.doc.throws[index].clone();
                    self.doc.throws[index] =
                        (exception, format!("{description} {line}").trim().to_owned());
                }
            }
            _ => self.append_unstructured(raw, line),
        }
    }

    /// Handle text that either extends the active list or the brief summary.
    fn append_unstructured(&mut self, raw: &str, line: &str) {
        let Some((kind, index)) = self.list else {
            self.doc.brief = format!("{} {}", self.doc.brief, line).trim().to_owned();
            return;
        };

        match kind {
            ListKind::Example => {
                self.doc.examples[index].push('\n');
                self.doc.examples[index].push_str(raw.trim_end());
            }
            _ => {
                self.list_mut(kind)[index].push(' ');
                self.list_mut(kind)[index].push_str(line);
            }
        }
    }

    /// Push a parameter and return its index.
    fn push_param(&mut self, value: &str, direction: &str) -> usize {
        let (name, _, description) = partition_space(value);
        let mut param = Param::new(name);
        param.direction = direction.to_owned();
        param.description = description.trim().to_owned();
        self.doc.params.push(param);
        self.doc.params.len() - 1
    }

    /// Push an exception and return its index.
    fn push_throw(&mut self, value: &str) -> usize {
        let (exception, _, description) = partition_space(value);
        self.doc
            .throws
            .push((exception.to_owned(), description.trim().to_owned()));
        self.doc.throws.len() - 1
    }

    /// Push a list entry and return its index.
    fn push_list(&mut self, kind: ListKind, value: &str) -> usize {
        let values = self.list_mut(kind);
        values.push(value.to_owned());
        values.len() - 1
    }

    /// Borrow one of the repeated lists by kind.
    fn list_mut(&mut self, kind: ListKind) -> &mut Vec<String> {
        match kind {
            ListKind::See => &mut self.doc.see,
            ListKind::Note => &mut self.doc.notes,
            ListKind::Warning => &mut self.doc.warnings,
            ListKind::Example => &mut self.doc.examples,
        }
    }

    /// Finish the document by trimming the fields `dox.py` trims.
    fn finish(mut self) -> Doc {
        self.doc.brief = self.doc.brief.trim().to_owned();
        self.doc.details = self.doc.details.trim_matches('\n').to_owned();
        self.doc
    }

    /// Split untagged lines into whitespace-collapsed paragraphs.
    fn paragraphs(lines: &[&str]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.trim())
            .fold(Vec::<String>::new(), |mut paragraphs, line| {
                if line.is_empty() {
                    paragraphs.push(String::new());
                    return paragraphs;
                }
                let Some(current) = paragraphs.last_mut() else {
                    paragraphs.push(line.to_owned());
                    return paragraphs;
                };
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(line);
                paragraphs
            })
            .into_iter()
            .filter(|paragraph| !paragraph.is_empty())
            .collect()
    }
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

    let margin = lines
        .iter()
        .skip(1)
        .filter(|line| !line.trim_start().is_empty())
        .map(|line| line.chars().count() - line.trim_start().chars().count())
        .min()
        .unwrap_or(usize::MAX);

    if let Some(first) = lines.first_mut() {
        *first = first.trim_start().to_owned();
    }
    if margin != usize::MAX {
        lines
            .iter_mut()
            .skip(1)
            .for_each(|line| *line = line.chars().skip(margin).collect());
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
    /// Parse one comment block into its brief, details, and tagged fields.
    pub fn parse(&self, text: &str) -> Doc {
        if text.trim().is_empty() {
            return Doc::default();
        }

        let lines = split_lines(text);
        let first_tag = lines.iter().position(|line| TAG_RE.is_match(line.trim()));
        let (leading, tagged): (&[&str], &[&str]) = match first_tag {
            Some(index) => (&lines[..index], &lines[index..]),
            None => (&lines[..], &[]),
        };

        let mut builder = DocBuilder::new(leading);
        tagged.iter().for_each(|line| builder.consume(line));
        builder.finish()
    }
}

/// Shared stateless parser instance.
pub static DOC_PARSER: DocParser = DocParser;

/// Split `value` on the first ASCII space, mirroring Python's `partition(" ")`.
fn partition_space(value: &str) -> (&str, &str, &str) {
    match value.find(' ') {
        Some(index) => (&value[..index], " ", &value[index + 1..]),
        None => (value, "", ""),
    }
}

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
