//! Markdown reference renderer.

use crate::model::{Entity, EntityKind, Field, Param};

/// Render a parsed entity tree as Markdown.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceRenderer;

impl ReferenceRenderer {
    pub fn render(&self, entities: &[Entity]) -> String {
        entities
            .iter()
            .map(|entity| self.entity(entity))
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }

    fn entity(&self, entity: &Entity) -> String {
        if entity.kind == EntityKind::Enum {
            return self.enum_reference(entity);
        }
        if matches!(
            entity.kind,
            EntityKind::Class | EntityKind::Struct | EntityKind::Trait | EntityKind::Interface
        ) {
            return self.class_reference(entity);
        }
        if entity.kind == EntityKind::Module {
            let mut parts = Vec::new();
            if !entity.doc.brief.is_empty() {
                parts.push(entity.doc.brief.clone());
            }
            if !entity.doc.details.is_empty() {
                parts.push(entity.doc.details.clone());
            }
            if !entity.doc.examples.is_empty() {
                parts.push(example_section(entity, "### Example"));
            }
            return parts.join("\n\n");
        }
        self.function(entity)
    }

    fn function(&self, entity: &Entity) -> String {
        let mut out = vec![format!("## `{}()`", entity.name)];
        if !entity.signature.is_empty() {
            out.push(format!("```\n{}\n```", entity.signature));
        }
        if !entity.doc.brief.is_empty() {
            out.push(entity.doc.brief.clone());
        }
        if !entity.doc.details.is_empty() {
            out.push(format!("### Details\n\n{}", entity.doc.details));
        }
        if !entity.doc.params.is_empty() {
            out.push(format!(
                "### Parameters\n\n{}",
                params_rows(&entity.doc.params)
            ));
        }
        if !entity.doc.returns.is_empty() {
            out.push(format!("### Returns\n\n{}", entity.doc.returns));
        }
        if !entity.doc.throws.is_empty() {
            out.push(format!(
                "### Exceptions\n\n{}",
                throws_rows(&entity.doc.throws)
            ));
        }
        if !entity.doc.notes.is_empty() {
            out.push(format!("### Notes\n\n{}", quote_blocks(&entity.doc.notes)));
        }
        if !entity.doc.warnings.is_empty() {
            out.push(format!(
                "### Warnings\n\n{}",
                quote_blocks(&entity.doc.warnings)
            ));
        }
        if !entity.doc.examples.is_empty() {
            out.push(example_section(entity, "### Example"));
        }
        if !entity.doc.see.is_empty() {
            out.push(format!(
                "### See Also\n\n{}",
                entity
                    .doc
                    .see
                    .iter()
                    .map(|item| format!("- `{item}`"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        out.into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn class_reference(&self, entity: &Entity) -> String {
        let mut title = format!("`{}` {} Reference", entity.name, entity.kind.title());
        if entity.r#abstract {
            title.push_str(" *abstract*");
        }

        let mut out = vec![format!("## {title}")];
        if !entity.doc.brief.is_empty() {
            out.push(entity.doc.brief.clone());
        }
        if !entity.doc.details.is_empty() {
            out.push(entity.doc.details.clone());
        }

        let mut relations = Vec::new();
        if !entity.bases.is_empty() {
            relations.push(format!(
                "**Inherits:** {}",
                entity
                    .bases
                    .iter()
                    .map(|base| format!("`{base}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !entity.implements.is_empty() {
            relations.push(format!(
                "**Implements:** {}",
                entity
                    .implements
                    .iter()
                    .map(|item| format!("`{item}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !relations.is_empty() {
            out.push(relations.join("\n\n"));
        }

        if !entity.doc.examples.is_empty() {
            out.push(example_section(entity, "### Example"));
        }
        if !entity.fields.is_empty() {
            out.push(format!("### Fields\n\n{}", fields_table(&entity.fields)));
        }
        if !entity.members.is_empty() {
            out.push(format!(
                "### Public Member Functions\n\n{}",
                members_table(&entity.members)
            ));
            let contracts: Vec<&Entity> = entity
                .members
                .iter()
                .filter(|member| needs_contract(member))
                .collect();
            if !contracts.is_empty() {
                out.push(format!(
                    "### Method Contracts\n\n{}",
                    contracts
                        .iter()
                        .map(|member| member_contract(member))
                        .collect::<Vec<_>>()
                        .join("\n\n")
                ));
            }
        }

        out.join("\n\n")
    }

    fn enum_reference(&self, entity: &Entity) -> String {
        let mut out = vec![format!("## `{}` Enum Reference", entity.name)];
        if !entity.doc.brief.is_empty() {
            out.push(entity.doc.brief.clone());
        }
        if !entity.doc.details.is_empty() {
            out.push(entity.doc.details.clone());
        }
        if !entity.variants.is_empty() {
            let mut rows = vec![
                "| Enumerator | Description |".to_owned(),
                "|------------|-------------|".to_owned(),
            ];
            for variant in &entity.variants {
                rows.push(format!(
                    "| `{}` | {} |",
                    cell(&variant.name),
                    cell(&variant.description)
                ));
            }
            out.push(format!("### Enumerators\n\n{}", rows.join("\n")));
        }
        out.join("\n\n")
    }
}

fn needs_contract(member: &Entity) -> bool {
    let doc = &member.doc;
    !doc.details.is_empty()
        || !doc.params.is_empty()
        || !doc.returns.is_empty()
        || !doc.throws.is_empty()
        || !doc.notes.is_empty()
        || !doc.warnings.is_empty()
        || !doc.examples.is_empty()
        || !doc.see.is_empty()
}

fn member_contract(member: &Entity) -> String {
    let mut parts = vec![format!("#### `{}()`", member.name)];
    if !member.doc.details.is_empty() {
        parts.push(member.doc.details.clone());
    }
    if !member.doc.params.is_empty() {
        parts.push(format!(
            "**Parameters**\n\n{}",
            params_rows(&member.doc.params)
        ));
    }
    if !member.doc.returns.is_empty() {
        parts.push(format!("**Returns:** {}", member.doc.returns));
    }
    if !member.doc.throws.is_empty() {
        parts.push(format!(
            "**Exceptions**\n\n{}",
            throws_rows(&member.doc.throws)
        ));
    }
    if !member.doc.notes.is_empty() {
        parts.push(format!("**Notes**\n\n{}", quote_blocks(&member.doc.notes)));
    }
    if !member.doc.warnings.is_empty() {
        parts.push(format!(
            "**Warnings**\n\n{}",
            quote_blocks(&member.doc.warnings)
        ));
    }
    if !member.doc.examples.is_empty() {
        parts.push(example_section(member, "#### Example"));
    }
    if !member.doc.see.is_empty() {
        parts.push(format!(
            "**See Also:** {}",
            member
                .doc
                .see
                .iter()
                .map(|item| format!("`{item}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    parts.join("\n\n")
}

fn example_section(entity: &Entity, heading: &str) -> String {
    let mut parts = vec![heading.to_owned()];
    for example in &entity.doc.examples {
        let content = example.trim_matches('\n');
        if content.trim().is_empty() {
            continue;
        }
        if content.contains("```") {
            parts.push(content.to_owned());
        } else {
            parts.push(format!("```\n{content}\n```"));
        }
    }
    if parts.len() > 1 {
        parts.join("\n\n")
    } else {
        String::new()
    }
}

fn members_table(members: &[Entity]) -> String {
    let mut rows = vec![
        "| Return Type | Signature & Description |".to_owned(),
        "|-------------|-------------------------|".to_owned(),
    ];
    for member in members {
        let prefix = if member.r#abstract { "abstract " } else { "" };
        let mut content = format!("{prefix}`{}`", member.signature);
        if !member.doc.brief.is_empty() {
            content.push_str("<br>");
            content.push_str(&member.doc.brief);
        }
        rows.push(format!(
            "| `{}` | {} |",
            cell(&member.return_type),
            cell(&content)
        ));
    }
    rows.join("\n")
}

fn fields_table(fields: &[Field]) -> String {
    let mut rows = vec![
        "| Visibility | Type | Name | Description |".to_owned(),
        "|------------|------|------|-------------|".to_owned(),
    ];
    for field in fields {
        let flags = if field.r#static { "static " } else { "" };
        rows.push(format!(
            "| {}{} | `{}` | `{}` | {} |",
            flags,
            cell(&field.visibility),
            cell(&field.r#type),
            cell(&field.name),
            cell(&field.description)
        ));
    }
    rows.join("\n")
}

fn params_rows(params: &[Param]) -> String {
    let mut rows = vec![
        "| Direction | Name | Type | Description |".to_owned(),
        "|-----------|------|------|-------------|".to_owned(),
    ];
    for param in params {
        rows.push(format!(
            "| [{}] | `{}` | `{}` | {} |",
            cell(&param.direction),
            cell(&param.name),
            cell(&param.r#type),
            cell(&param.description)
        ));
    }
    rows.join("\n")
}

fn throws_rows(throws: &[(String, String)]) -> String {
    let mut rows = vec![
        "| Exception | Description |".to_owned(),
        "|-----------|-------------|".to_owned(),
    ];
    for (exception, description) in throws {
        rows.push(format!("| `{}` | {} |", cell(exception), cell(description)));
    }
    rows.join("\n")
}

fn quote_blocks(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("> {item}"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn cell(value: &str) -> String {
    value.replace('|', "\\|")
}
