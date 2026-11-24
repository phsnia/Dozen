//! Type expressions shared by semantic analysis and diagram rendering.

use std::sync::LazyLock;

use regex::Regex;

static TYPE_TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[A-Za-z_]\w*").unwrap());
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Scalar type names that never become dependency edges.
const BUILTIN_TYPES: &[&str] = &[
    "a", "bool", "char", "double", "f32", "f64", "float", "i8", "i16", "i32", "i64", "i128", "int",
    "isize", "long", "Self", "short", "signed", "std", "str", "String", "u8", "u16", "u32", "u64",
    "u128", "unsigned", "usize", "void",
];

/// One parsed source-level type expression.
///
/// The type system only needs three views of a type: the original spelling,
/// the unqualified base name used for symbol lookup, and the identifier tokens
/// that may refer to user-declared types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRef {
    /// Original type text as written in the source.
    raw: String,
    /// Base name with generics and `::` qualification removed.
    base: String,
    /// Non-builtin identifier tokens referenced by the type.
    tokens: Vec<String>,
}

impl TypeRef {
    /// Parse a raw type expression into its lookup and display views.
    pub fn parse(raw: &str) -> Self {
        let base = raw
            .split('<')
            .next()
            .unwrap_or("")
            .trim()
            .rsplit("::")
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        let tokens = TYPE_TOKEN
            .find_iter(raw)
            .map(|found| found.as_str())
            .filter(|token| !BUILTIN_TYPES.contains(token))
            .map(str::to_owned)
            .collect();

        Self {
            raw: raw.to_owned(),
            base,
            tokens,
        }
    }

    /// Return the type exactly as written in the source.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Return the unqualified base name used for symbol lookup.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Return the user-declared identifier tokens referenced by this type.
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }

    /// Render the type for a Mermaid class-block field.
    pub fn display(&self) -> String {
        WHITESPACE
            .replace_all(
                &self
                    .raw
                    .replace(['&', '\''], "")
                    .replace(['<', '>', '[', ']'], "~")
                    .replace('|', " or "),
                " ",
            )
            .trim()
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_base_and_tokens() {
        let type_ref = TypeRef::parse("&'a std::collections::HashMap<String, Animal>");
        assert_eq!(type_ref.base(), "HashMap");
        assert_eq!(type_ref.tokens(), ["collections", "HashMap", "Animal"]);
    }

    #[test]
    fn renders_mermaid_safe_display_text() {
        assert_eq!(TypeRef::parse("Vec<&'a Animal>").display(), "Vec~a Animal~");
    }
}
