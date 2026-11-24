//! Language-agnostic syntax primitives.
//!
//! The parser layer lowers source text into [`crate::model`] entities. The
//! semantic and graph layers then use the small primitives in this module to
//! reason about type references, visibility, and node identity without knowing
//! which language produced them.

pub mod attribute;
pub mod node;
pub mod types;

pub use attribute::{Modifiers, Visibility};
pub use node::NodeId;
pub use types::TypeRef;
