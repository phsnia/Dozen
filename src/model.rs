//! Language-agnostic documentation model.

/// Kinds of declaration the documentation model can represent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Module,
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Interface,
}

impl EntityKind {
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
}

/// A documented parameter or field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub r#type: String,
    pub direction: String,
    pub description: String,
}

impl Param {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            r#type: String::new(),
            direction: "in".to_owned(),
            description: String::new(),
        }
    }
}

/// One enumerator or enum case.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Variant {
    pub name: String,
    pub description: String,
}

/// One data member of a class, struct, or interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub r#type: String,
    pub visibility: String,
    pub r#static: bool,
    pub description: String,
}

impl Field {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            r#type: String::new(),
            visibility: "public".to_owned(),
            r#static: false,
            description: String::new(),
        }
    }
}

/// Structured documentation extracted from a comment block.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Doc {
    pub brief: String,
    pub details: String,
    pub params: Vec<Param>,
    pub returns: String,
    pub throws: Vec<(String, String)>,
    pub see: Vec<String>,
    pub notes: Vec<String>,
    pub warnings: Vec<String>,
    pub examples: Vec<String>,
}

/// One documented declaration and its optional children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity {
    pub kind: EntityKind,
    pub name: String,
    pub signature: String,
    pub return_type: String,
    pub doc: Doc,
    pub r#abstract: bool,
    pub members: Vec<Entity>,
    pub variants: Vec<Variant>,
    pub fields: Vec<Field>,
    pub bases: Vec<String>,
    pub implements: Vec<String>,
    pub module: String,
    pub path: String,
    pub line: usize,
    pub calls: Vec<String>,
    pub includes: Vec<String>,
}

impl Entity {
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
}

pub fn dedup_push(values: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !value.is_empty() && !values.contains(&value) {
        values.push(value);
    }
}
