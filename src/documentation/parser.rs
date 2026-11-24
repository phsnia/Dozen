//! Comment-block parser that drives the tag builder.

use crate::documentation::DocText;
use crate::documentation::tags::{DocBuilder, TagLine};
use crate::model::Doc;

/// Convert a comment block into a structured [`Doc`] model.
#[derive(Clone, Copy, Debug, Default)]
pub struct DocParser;

impl DocParser {
    /// Parse one comment block into its brief, details, and tagged fields.
    pub fn parse(&self, text: &str) -> Doc {
        if text.trim().is_empty() {
            return Doc::default();
        }

        let lines = DocText::lines(text);
        let first_tag = lines
            .iter()
            .position(|line| TagLine::is_tag(line.trim()));
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
}
