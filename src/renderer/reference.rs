//! Markdown reference renderer.

use crate::model::{Entity, EntityKind, Field, Param};
use crate::renderer::markdown::{bullets, cell, code_list, table};

/// Render a parsed entity tree as Markdown.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReferenceRenderer;

impl ReferenceRenderer {
    /// Render every entity, separating top-level sections with horizontal rules.
    pub fn render(&self, entities: &[Entity]) -> String {
        entities
            .iter()
            .map(|entity| self.entity(entity))
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }

    /// Dispatch one entity to the renderer for its kind.
    fn entity(&self, entity: &Entity) -> String {
        match entity.kind {
            EntityKind::Enum => self.enum_reference(entity),
            kind if kind.is_aggregate() => self.class_reference(entity),
            EntityKind::Module => self.module_reference(entity),
            _ => self.function(entity),
        }
    }

    /// Render a module as its brief, details, and examples.
    fn module_reference(&self, entity: &Entity) -> String {
        let example = if entity.doc.examples.is_empty() {
            String::new()
        } else {
            self.example_section(entity, "### Example")
        };
        [
            entity.doc.brief.clone(),
            entity.doc.details.clone(),
            example,
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
    }

    /// Render a function as a full API reference section.
    fn function(&self, entity: &Entity) -> String {
        let doc = &entity.doc;
        let mut parts = vec![format!("## `{}()`", entity.name)];

        if !entity.signature.is_empty() {
            parts.push(format!("```\n{}\n```", entity.signature));
        }
        if !doc.brief.is_empty() {
            parts.push(doc.brief.clone());
        }
        if !doc.details.is_empty() {
            parts.push(format!("### Details\n\n{}", doc.details));
        }
        if !doc.params.is_empty() {
            parts.push(format!(
                "### Parameters\n\n{}",
                self.params_table(&doc.params)
            ));
        }
        if !doc.returns.is_empty() {
            parts.push(format!("### Returns\n\n{}", doc.returns));
        }
        if !doc.throws.is_empty() {
            parts.push(format!(
                "### Exceptions\n\n{}",
                self.throws_table(&doc.throws)
            ));
        }
        if !doc.notes.is_empty() {
            parts.push(format!("### Notes\n\n{}", self.quote_blocks(&doc.notes)));
        }
        if !doc.warnings.is_empty() {
            parts.push(format!(
                "### Warnings\n\n{}",
                self.quote_blocks(&doc.warnings)
            ));
        }
        if !doc.examples.is_empty() {
            parts.push(self.example_section(entity, "### Example"));
        }
        if !doc.see.is_empty() {
            parts.push(format!("### See Also\n\n{}", bullets(&doc.see)));
        }

        parts
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Render a class-like declaration as a reference section with tables.
    fn class_reference(&self, entity: &Entity) -> String {
        let mut title = format!("`{}` {} Reference", entity.name, entity.kind.title());
        if entity.r#abstract {
            title.push_str(" *abstract*");
        }

        let mut parts = vec![format!("## {title}")];
        if !entity.doc.brief.is_empty() {
            parts.push(entity.doc.brief.clone());
        }
        if !entity.doc.details.is_empty() {
            parts.push(entity.doc.details.clone());
        }
        if let Some(relations) = self.relations(entity) {
            parts.push(relations);
        }
        if !entity.doc.examples.is_empty() {
            parts.push(self.example_section(entity, "### Example"));
        }
        if !entity.fields.is_empty() {
            parts.push(format!(
                "### Fields\n\n{}",
                self.fields_table(&entity.fields)
            ));
        }
        if !entity.members.is_empty() {
            parts.push(format!(
                "### Public Member Functions\n\n{}",
                self.members_table(&entity.members)
            ));
            parts.extend(self.member_contracts(&entity.members));
        }

        parts.join("\n\n")
    }

    /// Render an enum as a reference section with an enumerator table.
    fn enum_reference(&self, entity: &Entity) -> String {
        let mut parts = vec![format!("## `{}` Enum Reference", entity.name)];
        if !entity.doc.brief.is_empty() {
            parts.push(entity.doc.brief.clone());
        }
        if !entity.doc.details.is_empty() {
            parts.push(entity.doc.details.clone());
        }
        if !entity.variants.is_empty() {
            let rows = entity
                .variants
                .iter()
                .map(|variant| {
                    format!(
                        "| `{}` | {} |",
                        cell(&variant.name),
                        cell(&variant.description)
                    )
                })
                .collect::<Vec<_>>();
            parts.push(format!(
                "### Enumerators\n\n{}",
                table(
                    "| Enumerator | Description |",
                    "|------------|-------------|",
                    rows
                )
            ));
        }
        parts.join("\n\n")
    }

    /// Render inheritance and interface relations, when present.
    fn relations(&self, entity: &Entity) -> Option<String> {
        let mut relations = Vec::new();
        if !entity.bases.is_empty() {
            relations.push(format!("**Inherits:** {}", code_list(&entity.bases)));
        }
        if !entity.implements.is_empty() {
            relations.push(format!("**Implements:** {}", code_list(&entity.implements)));
        }
        (!relations.is_empty()).then(|| relations.join("\n\n"))
    }

    /// Render the contracts of members that have structured documentation.
    fn member_contracts(&self, members: &[Entity]) -> Option<String> {
        let contracts: Vec<String> = members
            .iter()
            .filter(|member| member.doc.has_contract())
            .map(|member| self.member_contract(member))
            .collect();
        (!contracts.is_empty())
            .then(|| format!("### Method Contracts\n\n{}", contracts.join("\n\n")))
    }

    /// Render one method contract entry.
    fn member_contract(&self, member: &Entity) -> String {
        let doc = &member.doc;
        let mut parts = vec![format!("#### `{}()`", member.name)];

        if !doc.details.is_empty() {
            parts.push(doc.details.clone());
        }
        if !doc.params.is_empty() {
            parts.push(format!(
                "**Parameters**\n\n{}",
                self.params_table(&doc.params)
            ));
        }
        if !doc.returns.is_empty() {
            parts.push(format!("**Returns:** {}", doc.returns));
        }
        if !doc.throws.is_empty() {
            parts.push(format!(
                "**Exceptions**\n\n{}",
                self.throws_table(&doc.throws)
            ));
        }
        if !doc.notes.is_empty() {
            parts.push(format!("**Notes**\n\n{}", self.quote_blocks(&doc.notes)));
        }
        if !doc.warnings.is_empty() {
            parts.push(format!(
                "**Warnings**\n\n{}",
                self.quote_blocks(&doc.warnings)
            ));
        }
        if !doc.examples.is_empty() {
            parts.push(self.example_section(member, "#### Example"));
        }
        if !doc.see.is_empty() {
            parts.push(format!("**See Also:** {}", code_list(&doc.see)));
        }

        parts.join("\n\n")
    }

    /// Render an example section, leaving already-fenced code untouched.
    fn example_section(&self, entity: &Entity, heading: &str) -> String {
        let examples = entity
            .doc
            .examples
            .iter()
            .map(|example| example.trim_matches('\n'))
            .filter(|content| !content.trim().is_empty())
            .map(|content| {
                if content.contains("```") {
                    content.to_owned()
                } else {
                    format!("```\n{content}\n```")
                }
            })
            .collect::<Vec<_>>();

        if examples.is_empty() {
            String::new()
        } else {
            std::iter::once(heading.to_owned())
                .chain(examples)
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }

    /// Render the public member function table.
    fn members_table(&self, members: &[Entity]) -> String {
        let rows = members.iter().map(|member| {
            let prefix = if member.r#abstract { "abstract " } else { "" };
            let mut content = format!("{prefix}`{}`", member.signature);
            if !member.doc.brief.is_empty() {
                content.push_str("<br>");
                content.push_str(&member.doc.brief);
            }
            format!("| `{}` | {} |", cell(&member.return_type), cell(&content))
        });
        table(
            "| Return Type | Signature & Description |",
            "|-------------|-------------------------|",
            rows,
        )
    }

    /// Render the data member table.
    fn fields_table(&self, fields: &[Field]) -> String {
        let rows = fields.iter().map(|field| {
            let flags = if field.r#static { "static " } else { "" };
            format!(
                "| {}{} | `{}` | `{}` | {} |",
                flags,
                cell(field.visibility.as_str()),
                cell(&field.r#type),
                cell(&field.name),
                cell(&field.description)
            )
        });
        table(
            "| Visibility | Type | Name | Description |",
            "|------------|------|------|-------------|",
            rows,
        )
    }

    /// Render the parameter table.
    fn params_table(&self, params: &[Param]) -> String {
        let rows = params.iter().map(|param| {
            format!(
                "| [{}] | `{}` | `{}` | {} |",
                cell(&param.direction),
                cell(&param.name),
                cell(&param.r#type),
                cell(&param.description)
            )
        });
        table(
            "| Direction | Name | Type | Description |",
            "|-----------|------|------|-------------|",
            rows,
        )
    }

    /// Render the exception table.
    fn throws_table(&self, throws: &[(String, String)]) -> String {
        let rows = throws.iter().map(|(exception, description)| {
            format!("| `{}` | {} |", cell(exception), cell(description))
        });
        table(
            "| Exception | Description |",
            "|-----------|-------------|",
            rows,
        )
    }

    /// Render list entries as block quotes.
    fn quote_blocks(&self, items: &[String]) -> String {
        items
            .iter()
            .map(|item| format!("> {item}"))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
