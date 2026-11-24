//! Resolve source type expressions against a symbol table.

use crate::ast::{NodeId, TypeRef};
use crate::semantic::SymbolTable;

#[derive(Clone, Debug, Default)]
pub struct TypeResolver;

impl TypeResolver {
    pub fn resolve<'a>(&self, table: &'a SymbolTable, raw: &str) -> &'a [NodeId] {
        table.get(TypeRef::parse(raw).base())
    }
}
