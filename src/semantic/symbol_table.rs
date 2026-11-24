//! Name lookup for flattened entity declarations.

use std::collections::HashMap;

use crate::ast::NodeId;
use crate::model::Entity;

#[derive(Clone, Debug, Default)]
pub struct SymbolTable {
    by_name: HashMap<String, Vec<NodeId>>,
}

impl SymbolTable {
    pub fn from_entities(entities: &[Entity]) -> Self {
        let mut table = Self::default();
        let mut index = 0;
        for entity in entities {
            table.add(entity, &mut index);
        }
        table
    }

    fn add(&mut self, entity: &Entity, index: &mut usize) {
        let id = NodeId::new(*index);
        *index += 1;
        self.by_name
            .entry(entity.name.clone())
            .or_default()
            .push(id);
        for member in &entity.members {
            self.add(member, index);
        }
    }

    pub fn get(&self, name: &str) -> &[NodeId] {
        self.by_name.get(name).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn len(&self) -> usize {
        self.by_name.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
