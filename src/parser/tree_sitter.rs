//! Source text helpers for parser implementations.

use crate::lang::Language;
use crate::model::Entity;

/// Parse source with an existing language plugin.
///
/// This is a compatibility facade; language lowering remains implemented by
/// the existing plugin registry and is not duplicated here.
pub fn parse(language: &dyn Language, source: &str, filename: &str) -> Vec<Entity> {
    language.parse(source, filename)
}
