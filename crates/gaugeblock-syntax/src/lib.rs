//! Grammar loading and syntax diagnostics for the suite's shared `Language` vocabulary.
//!
//! This crate exists because two products need the same answer to "does this code parse?" (`GB-246`):
//! `miter` reads code to index it, and `dado` validates a splice before writing it. Leaving the grammar
//! table in `miter` made `dado` depend on another product for a question that is neither product's
//! property, so the table lives here and both reach down to it.
//!
//! The vocabulary itself is not duplicated: [`gaugeblock_contracts::Language`] is the wire enum, and a
//! second enum here would be a second table that can disagree with the first.
//!
//! Declarative formats (SQL, HTML, CSS, Markdown, Docker, HCL, Kubernetes) have no syntax tree to walk for
//! this purpose; they are rejected as [`SyntaxError::Unsupported`] rather than silently accepted.

pub mod grammar;
pub mod syntax;

pub use grammar::{SyntaxError, grammar_for};
pub use syntax::{SyntaxDiagnostic, SyntaxDiagnosticKind, span_within_node, syntax_diagnostics};
