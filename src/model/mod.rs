//! Language-agnostic documentation model.
//!
//! Every language plugin lowers its syntax tree into these types. The
//! renderers only ever see this model, which keeps Markdown and Mermaid output
//! independent of the source language.

use crate::ast::Visibility;

/// Kinds of declaration the documentation model can represent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    /// A source file or module.
    Module,
    /// A free function.
    Function,
    /// A function that belongs to a class or struct.
    Method,
    /// A class declaration.
    Class,
    /// A struct declaration.
    Struct,
    /// An enum declaration.
    Enum,
    /// A Rust trait or equivalent abstract contract.
    Trait,
    /// A C++ pure-virtual interface or similar abstract type.
    Interface,
}

impl EntityKind {
    /// Return the lowercase name used inside contract diagrams.
    pub const fn value(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Interface => "interface",
        }
    }

    /// Return the title-cased name used in reference headings.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Module => "Module",
            Self::Function => "Function",
            Self::Method => "Method",
            Self::Class => "Class",
            Self::Struct => "Struct",
            Self::Enum => "Enum",
            Self::Trait => "Trait",
            Self::Interface => "Interface",
        }
    }

    /// True for kinds rendered as a class-like block in diagrams.
    pub const fn is_aggregate(self) -> bool {
        matches!(
            self,
            Self::Class | Self::Struct | Self::Trait | Self::Interface
        )
    }

    /// True for kinds rendered as an enumeration block in diagrams.
    pub const fn is_enum(self) -> bool {
        matches!(self, Self::Enum)
    }

    /// True for kinds that can own call edges.
    pub const fn is_callable(self) -> bool {
        matches!(self, Self::Function | Self::Method)
    }

    /// True for kinds that contain other declarations.
    pub const fn is_container(self) -> bool {
        matches!(
            self,
            Self::Module | Self::Class | Self::Struct | Self::Trait | Self::Interface | Self::Enum
        )
    }
}

/// A documented parameter or field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    /// Parameter name as written in the source.
    pub name: String,
    /// Declared type, when the parser can infer one.
    pub r#type: String,
    /// Doxygen direction (`in`, `out`, or `in,out`).
    pub direction: String,
    /// Human-readable description collected from the doc block.
    pub description: String,
}

impl Param {
    /// Create an input parameter with no type or description yet.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            r#type: String::new(),
            direction: "in".to_owned(),
            description: String::new(),
        }
    }

    /// Append a continuation line to the description, collapsing whitespace.
    pub fn extend_description(&mut self, line: &str) {
        self.description = format!("{} {}", self.description, line).trim().to_owned();
    }
}

/// One enumerator or enum case.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Variant {
    /// Enumerator name.
    pub name: String,
    /// Documentation attached to the enumerator.
    pub description: String,
}

/// One data member of a class, struct, or interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Member name.
    pub name: String,
    /// Declared member type.
    pub r#type: String,
    /// Effective member visibility.
    pub visibility: Visibility,
    /// Whether the member is declared `static`.
    pub r#static: bool,
    /// Documentation attached to the member.
    pub description: String,
}

impl Field {
    /// Create a public instance field with no type or description yet.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            r#type: String::new(),
            visibility: Visibility::Public,
            r#static: false,
            description: String::new(),
        }
    }
}

/// Structured documentation extracted from a comment block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Doc {
    /// Short summary shown first in every rendered section.
    pub brief: String,
    /// Long-form description.
    pub details: String,
    /// Documented parameters, in source order.
    pub params: Vec<Param>,
    /// Documented return value.
    pub returns: String,
    /// Exceptions and their descriptions.
    pub throws: Vec<(String, String)>,
    /// Cross references collected from `@see`.
    pub see: Vec<String>,
    /// Notes collected from `@note`.
    pub notes: Vec<String>,
    /// Warnings collected from `@warning`.
    pub warnings: Vec<String>,
    /// Examples collected from `@example`.
    pub examples: Vec<String>,
}

impl Doc {
    /// True when the documentation carries structured contract content.
    ///
    /// The brief is intentionally excluded: a brief alone is already rendered
    /// in the member table and does not deserve a separate contract section.
    pub fn has_contract(&self) -> bool {
        !self.details.is_empty()
            || !self.params.is_empty()
            || !self.returns.is_empty()
            || !self.throws.is_empty()
            || !self.notes.is_empty()
            || !self.warnings.is_empty()
            || !self.examples.is_empty()
            || !self.see.is_empty()
    }
}

/// One documented declaration and its optional children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    /// Declared kind.
    pub kind: EntityKind,
    /// Declared name.
    pub name: String,
    /// Signature exactly as rendered in reference tables.
    pub signature: String,
    /// Declared return type.
    pub return_type: String,
    /// Structured documentation.
    pub doc: Doc,
    /// Whether the declaration is abstract.
    pub r#abstract: bool,
    /// Nested methods, functions, or type members.
    pub members: Vec<Entity>,
    /// Enum cases, for [`EntityKind::Enum`].
    pub variants: Vec<Variant>,
    /// Data members, for class-like declarations.
    pub fields: Vec<Field>,
    /// Base classes or inherited interfaces.
    pub bases: Vec<String>,
    /// Implemented interfaces or traits.
    pub implements: Vec<String>,
    /// Owning module stem assigned during conversion.
    pub module: String,
    /// Source path assigned during conversion.
    pub path: String,
    /// One-based source line, when known.
    pub line: usize,
    /// Function calls observed in the body.
    pub calls: Vec<String>,
    /// Included headers or imported modules.
    pub includes: Vec<String>,
}

impl Entity {
    /// Create an empty entity of `kind` with the given `name`.
    pub fn new(kind: EntityKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            signature: String::new(),
            return_type: String::new(),
            doc: Doc::default(),
            r#abstract: false,
            members: Vec::new(),
            variants: Vec::new(),
            fields: Vec::new(),
            bases: Vec::new(),
            implements: Vec::new(),
            module: String::new(),
            path: String::new(),
            line: 0,
            calls: Vec::new(),
            includes: Vec::new(),
        }
    }

    /// Attach the source path and module stem to this entity and its members.
    pub fn stamp(&mut self, path: &std::path::Path) {
        self.path = path.to_string_lossy().into_owned();
        self.module = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.members
            .iter_mut()
            .for_each(|member| member.stamp(path));
    }
}

/// Extension trait for building ordered, de-duplicated name lists.
pub trait UniquePush {
    /// Push `value` unless it is empty or already present.
    fn push_unique(&mut self, value: impl Into<String>);
}

impl UniquePush for Vec<String> {
    fn push_unique(&mut self, value: impl Into<String>) {
        let value = value.into();
        if !value.is_empty() && !self.contains(&value) {
            self.push(value);
        }
    }
}

/// Append a non-empty, unique string while preserving insertion order.
///
/// This is the legacy-compatible free-function form used by language parsers.
pub fn dedup_push(values: &mut Vec<String>, value: impl Into<String>) {
    values.push_unique(value);
}
