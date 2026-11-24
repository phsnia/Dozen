//! Graph types shared by diagram-oriented analyses.

pub mod call_graph;
pub mod dependency;
pub mod inheritance;

/// A directed graph edge with an optional label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge<T> {
    pub from: T,
    pub to: T,
    pub label: Option<String>,
}

impl<T> Edge<T> {
    pub fn new(from: T, to: T) -> Self {
        Self {
            from,
            to,
            label: None,
        }
    }

    pub fn labelled(from: T, to: T, label: impl Into<String>) -> Self {
        Self {
            from,
            to,
            label: Some(label.into()),
        }
    }
}
