//! Call-edge data extracted from the language-agnostic entity model.

use crate::model::Entity;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallEdge {
    pub caller: String,
    pub callee: String,
}

pub fn edges(entities: &[Entity]) -> Vec<CallEdge> {
    fn flatten(entity: &Entity, owner: &str, out: &mut Vec<CallEdge>) {
        let caller = if entity.kind.is_callable() {
            if owner.is_empty() {
                entity.name.clone()
            } else {
                format!("{owner}.{}", entity.name)
            }
        } else {
            owner.to_owned()
        };

        if entity.kind.is_callable() {
            for callee in &entity.calls {
                let edge = CallEdge {
                    caller: caller.clone(),
                    callee: callee.clone(),
                };
                if !out.contains(&edge) {
                    out.push(edge);
                }
            }
        }

        for member in &entity.members {
            flatten(member, &caller, out);
        }
    }

    let mut out = Vec::new();
    for entity in entities {
        flatten(entity, &entity.name, &mut out);
    }
    out
}
