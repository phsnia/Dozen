//! Extension registry for language plugins.

use std::path::Path;

use crate::lang::{Language, cpp::CppLanguage, python::PythonLanguage, rust::RustLanguage};

pub const EXTENSIONS: &[&str] = &[".py", ".rs", ".cpp", ".hpp", ".h", ".cc", ".cxx", ".hh"];

/// Return the lowercase extension including the leading dot.
pub fn suffix(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{}", extension.to_lowercase()))
        .unwrap_or_default()
}

pub fn is_supported(path: &Path) -> bool {
    EXTENSIONS.contains(&suffix(path).as_str())
}

pub fn language_for(path: &Path) -> Option<Box<dyn Language>> {
    match suffix(path).as_str() {
        ".py" => Some(Box::new(PythonLanguage)),
        ".rs" => Some(Box::new(RustLanguage)),
        ".cpp" | ".hpp" | ".h" | ".cc" | ".cxx" | ".hh" => Some(Box::new(CppLanguage)),
        _ => None,
    }
}
