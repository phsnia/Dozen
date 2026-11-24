//! High-level semantic pass used by graph and symbol analyses.

use crate::model::Entity;
use crate::semantic::SymbolTable;

#[derive(Clone, Debug)]
pub struct Analysis {
    pub symbols: SymbolTable,
}

pub fn analyze(entities: &[Entity]) -> Analysis {
    Analysis {
        symbols: SymbolTable::from_entities(entities),
    }
}
