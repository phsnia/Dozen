//! Declaration attributes shared by the model and the renderers.

/// Effective member visibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Visible to every caller.
    #[default]
    Public,
    /// Visible to subclasses.
    Protected,
    /// Visible only inside the declaring type.
    Private,
    /// Visible inside the declaring package.
    Package,
}

impl Visibility {
    /// Return the lowercase spelling used in reference tables.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Private => "private",
            Self::Package => "package",
        }
    }

    /// Return the Mermaid class-diagram marker for this visibility.
    pub const fn marker(self) -> char {
        match self {
            Self::Public => '+',
            Self::Protected => '#',
            Self::Private => '-',
            Self::Package => '~',
        }
    }

    /// Resolve a source-language access keyword.
    pub fn from_keyword(value: &str) -> Self {
        match value {
            "protected" => Self::Protected,
            "private" => Self::Private,
            "package" => Self::Package,
            _ => Self::Public,
        }
    }
}

/// Non-type declaration modifiers carried by a member.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// The member is declared `static`.
    pub r#static: bool,
    /// The member has no concrete implementation.
    pub r#abstract: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keywords_to_visibility() {
        assert_eq!(Visibility::from_keyword("private"), Visibility::Private);
        assert_eq!(Visibility::from_keyword("protected"), Visibility::Protected);
        assert_eq!(Visibility::from_keyword("public"), Visibility::Public);
        assert_eq!(Visibility::from_keyword("other"), Visibility::Public);
    }
}
