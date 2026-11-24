//! Type-dependency edges derived from fields and callable signatures.

use crate::ast::TypeRef;
use crate::model::Entity;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub owner: String,
    pub dependency: String,
}

pub fn dependencies(entities: &[Entity]) -> Vec<Dependency> {
    fn visit(entity: &Entity, out: &mut Vec<Dependency>) {
        for field in &entity.fields {
            for token in TypeRef::parse(&field.r#type).tokens() {
                let edge = Dependency {
                    owner: entity.name.clone(),
                    dependency: token.clone(),
                };
                if !out.contains(&edge) {
                    out.push(edge);
                }
            }
        }
        for member in &entity.members {
            visit(member, out);
        }
    }

    let mut out = Vec::new();
    for entity in entities {
        visit(entity, &mut out);
    }
    out
}
