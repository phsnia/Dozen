//! Node identity shared by semantic tables and graphs.

/// Stable index-based identity for a declaration inside one analysis pass.
///
/// Python's renderer used `id(entity)`; the port uses the insertion order of a
/// node list so identifiers stay deterministic across runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(usize);

impl NodeId {
    /// Create an identifier for the node at `index`.
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Return the zero-based insertion index.
    pub const fn index(self) -> usize {
        self.0
    }
}

impl From<usize> for NodeId {
    fn from(index: usize) -> Self {
        Self::new(index)
    }
}
