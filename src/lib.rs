//! `dozen` is a pure-Rust port of `dox.py` for turning Doxygen-style source
//! comments into Markdown API references and Mermaid diagrams.
//!
//! The public modules expose the same pipeline the binary uses, which keeps
//! integration tests and external tooling on the exact production code path.

pub mod ast;
pub mod cli;
pub mod documentation;
pub mod engine;
pub mod graph;
pub mod lang;
pub mod model;
pub mod parser;
pub mod renderer;
pub mod semantic;
