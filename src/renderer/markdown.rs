//! @brief Shared Markdown rendering primitives.
//! @details Provides reusable escaping, table, list, heading, and code-fence
//! helpers. Entity-specific output is delegated to [`ReferenceRenderer`] so
//! Markdown formatting is implemented once.

use crate::model::Entity;
use crate::renderer::reference::ReferenceRenderer;

/// @brief Stateful entry point for Markdown reference rendering.
#[derive(Clone, Copy, Debug, Default)]
pub struct MarkdownRenderer {
    reference: ReferenceRenderer,
}

impl MarkdownRenderer {
    /// @brief Create a Markdown renderer.
    /// @return A renderer ready to emit API reference Markdown.
    pub fn new() -> Self {
        Self::default()
    }

    /// @brief Render one or more entities as a complete Markdown document.
    /// @param[in] entities Entity tree to render.
    /// @return Rendered Markdown, or an empty string for an empty input.
    pub fn render(&self, entities: &[Entity]) -> String {
        self.reference.render(entities)
    }
}

/// @brief Escape a pipe so it cannot end a Markdown table cell.
/// @param[in] value Raw table-cell text.
/// @return Text with Markdown table separators escaped.
pub fn escape_table_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

/// @brief Escape a pipe for a table cell.
///
/// @note Retained as a concise helper name for renderer call sites.
pub fn cell(value: &str) -> String {
    escape_table_cell(value)
}

/// @brief Join a header, separator, and rows into a Markdown table.
/// @param[in] header Markdown table header row.
/// @param[in] rule Markdown table separator row.
/// @param[in] rows Table body rows.
/// @return A complete Markdown table.
pub fn table(header: &str, rule: &str, rows: impl IntoIterator<Item = String>) -> String {
    std::iter::once(header.to_owned())
        .chain(std::iter::once(rule.to_owned()))
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

/// @brief Render items as inline code separated by commas.
/// @param[in] items Names or values to format.
/// @return A comma-separated Markdown inline-code list.
pub fn code_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// @brief Render items as Markdown bullet entries.
/// @param[in] items Names or values to format.
/// @return Markdown bullet lines.
pub fn bullets(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("- `{item}`"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// @brief Render a Markdown heading.
/// @param[in] level Heading level from one through six.
/// @param[in] text Heading text.
/// @return A heading line, clamped to the Markdown heading range.
pub fn heading(level: usize, text: &str) -> String {
    format!("{} {text}", "#".repeat(level.clamp(1, 6)))
}

/// @brief Wrap content in a fenced Markdown code block.
/// @param[in] content Code or documentation example text.
/// @param[in] language Optional fence language.
/// @return A fenced code block.
pub fn fenced_code(content: &str, language: Option<&str>) -> String {
    let language = language.unwrap_or_default();
    format!("```{language}\n{content}\n```")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_markdown_primitives() {
        assert_eq!(escape_table_cell("a|b"), "a\\|b");
        assert_eq!(heading(2, "Details"), "## Details");
        assert_eq!(
            fenced_code("let value = 1;", Some("rust")),
            "```rust\nlet value = 1;\n```"
        );
        assert_eq!(
            table(
                "| Name |",
                "|------|",
                [String::from("| `item` |")].into_iter()
            ),
            "| Name |\n|------|\n| `item` |"
        );
    }
}
