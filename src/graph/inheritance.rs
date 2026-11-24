//! Inheritance and implementation edges in the entity model.

use crate::model::Entity;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Relation {
    pub child: String,
    pub parent: String,
    pub implementation: bool,
}

pub fn relations(entities: &[Entity]) -> Vec<Relation> {
    fn flatten(entity: &Entity, out: &mut Vec<Relation>) {
        for parent in &entity.bases {
            let relation = Relation {
                child: entity.name.clone(),
                parent: parent.clone(),
                implementation: false,
            };
            if !out.contains(&relation) {
                out.push(relation);
            }
        }
        for parent in &entity.implements {
            let relation = Relation {
                child: entity.name.clone(),
                parent: parent.clone(),
                implementation: true,
            };
            if !out.contains(&relation) {
                out.push(relation);
            }
        }
        for member in &entity.members {
            flatten(member, out);
        }
    }

    let mut out = Vec::new();
    for entity in entities {
        flatten(entity, &mut out);
    }
    out
}
