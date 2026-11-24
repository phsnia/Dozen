//! Extension registry for language plugins.

use std::path::Path;

use crate::lang::{Language, cpp::CppLanguage, python::PythonLanguage, rust::RustLanguage};

/// Every extension the tool accepts, including the leading dot.
pub const EXTENSIONS: &[&str] = &[".py", ".rs", ".cpp", ".hpp", ".h", ".cc", ".cxx", ".hh"];

/// Maps file paths to the language plugin that can parse them.
#[derive(Clone, Copy, Debug, Default)]
pub struct LanguageRegistry;

impl LanguageRegistry {
    /// Create a stateless registry.
    pub fn new() -> Self {
        Self
    }

    /// Return the lowercase extension including the leading dot.
    pub fn suffix(&self, path: &Path) -> String {
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!(".{}", extension.to_lowercase()))
            .unwrap_or_default()
    }

    /// True when the tool has a parser for `path`.
    pub fn is_supported(&self, path: &Path) -> bool {
        EXTENSIONS.contains(&self.suffix(path).as_str())
    }

    /// Return the parser for `path`, if the extension is supported.
    pub fn language_for(&self, path: &Path) -> Option<Box<dyn Language>> {
        match self.suffix(path).as_str() {
            ".py" => Some(Box::new(PythonLanguage)),
            ".rs" => Some(Box::new(RustLanguage)),
            ".cpp" | ".hpp" | ".h" | ".cc" | ".cxx" | ".hh" => Some(Box::new(CppLanguage)),
            _ => None,
        }
    }
}

/// Shared stateless registry instance.
pub static REGISTRY: LanguageRegistry = LanguageRegistry;
