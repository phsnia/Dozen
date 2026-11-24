//! Semantic analysis shared across language plugins and renderers.

pub mod analyzer;
pub mod symbol_table;
pub mod type_resolver;

pub use symbol_table::SymbolTable;
pub use type_resolver::TypeResolver;
