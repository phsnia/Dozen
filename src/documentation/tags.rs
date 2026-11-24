//! Tag vocabulary and the stateful builder that assembles one [`Doc`].

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

/// One parsed `@tag`, including its optional direction and inline value.
pub struct TagLine<'a> {
    /// Resolved tag.
    pub tag: Tag,
    /// Direction capture, defaulting to `in`.
    pub direction: &'a str,
    /// Inline text after the tag, already trimmed.
    pub value: &'a str,
}

impl<'a> TagLine<'a> {
    /// Parse a trimmed line, returning `None` when it is not a tag line.
    pub fn parse(line: &'a str) -> Option<Self> {
        let captures = TAG_RE.captures(line)?;
        let name = captures
            .get(1)
            .map_or("", |found| found.as_str())
            .to_lowercase();
        Some(Self {
            tag: Tag::from_name(&name),
            direction: captures.get(2).map_or("in", |found| found.as_str()),
            value: captures
                .get(3)
                .map_or("", |found| found.as_str())
                .trim(),
        })
    }

    /// Report whether a trimmed line starts with a documentation tag.
    pub fn is_tag(line: &str) -> bool {
        TAG_RE.is_match(line)
    }
}

/// Builder state for one tagged documentation block.
pub(crate) struct DocBuilder {
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
    pub fn new(leading: &[&str]) -> Self {
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
    pub fn consume(&mut self, raw: &str) {
        let line = raw.trim();
        if line.is_empty() {
            self.blank_line();
        } else if let Some(tag_line) = TagLine::parse(line) {
            self.begin_tag(&tag_line);
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
    fn begin_tag(&mut self, tag_line: &TagLine<'_>) {
        match tag_line.tag {
            Tag::Brief => self.doc.brief = tag_line.value.to_owned(),
            Tag::Details => self.doc.details = tag_line.value.to_owned(),
            Tag::Param => {
                let index = self.push_param(tag_line.value, tag_line.direction);
                self.param = Some(index);
            }
            Tag::Returns => self.doc.returns = tag_line.value.to_owned(),
            Tag::Throws => {
                let index = self.push_throw(tag_line.value);
                self.throw = Some(index);
            }
            Tag::Unknown => {
                self.active = None;
                return;
            }
            Tag::See | Tag::Note | Tag::Warning | Tag::Example => {
                let kind = tag_line.tag.list_kind().expect("list tag");
                let index = self.push_list(kind, tag_line.value);
                self.list = Some((kind, index));
            }
        }

        self.active = Some(tag_line.tag);
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
        let (name, _, description) = Self::partition_space(value);
        let mut param = Param::new(name);
        param.direction = direction.to_owned();
        param.description = description.trim().to_owned();
        self.doc.params.push(param);
        self.doc.params.len() - 1
    }

    /// Push an exception and return its index.
    fn push_throw(&mut self, value: &str) -> usize {
        let (exception, _, description) = Self::partition_space(value);
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
    pub fn finish(mut self) -> Doc {
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

    /// Split `value` on the first ASCII space, mirroring Python's `partition`.
    fn partition_space(value: &str) -> (&str, &str, &str) {
        match value.find(' ') {
            Some(index) => (&value[..index], " ", &value[index + 1..]),
            None => (value, "", ""),
        }
    }
}
