//! `dado` and `batten` wire domain: pattern templates, typed holes and grammar constraints.

use serde::{Deserialize, Serialize};

use crate::miter::Language;

/// A typed hole that a synthesis model must fill inside a pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedHole {
    /// Identifier of the hole within its pattern.
    pub hole_id: String,
    /// Type the filled expression must satisfy.
    pub expected_type: String,
    /// AST node kind the hole occupies (e.g. `expression`, `match_arm_body`, `predicate`).
    pub ast_node_kind: String,
    /// Token budget for the filled expression.
    pub max_tokens: u16,
    /// Whether a bare literal is an acceptable fill.
    pub allow_literals: bool,
}

/// A deterministic AST pattern with typed holes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DadoPattern {
    /// Versioned pattern identifier (e.g. `rust.axum.handler.sqlx_tx.v1`).
    pub pattern_id: String,
    /// Language the pattern renders into.
    pub language: Language,
    /// Human-readable description of what the pattern produces.
    pub description: String,
    /// Template source containing the holes.
    pub template_source: String,
    /// Imports the rendered result requires.
    pub required_imports: Vec<String>,
    /// Holes the synthesis model must fill.
    pub holes: Vec<TypedHole>,
}

/// Grammar dialect used to constrain a hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrammarFormat {
    /// GBNF grammar (llama.cpp family).
    Gbnf,
    /// Lark grammar.
    Lark,
    /// Regular expression.
    Regex,
}

/// Grammar constraint produced by `batten` from a lexical scope plus a typed hole.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledGrammarConstraint {
    /// Hole this constraint applies to.
    pub hole_id: String,
    /// Dialect of `grammar_definition`.
    pub format: GrammarFormat,
    /// Raw GBNF or Lark grammar string.
    pub grammar_definition: String,
    /// Identifiers the grammar accepts at this hole.
    pub allowed_identifiers: Vec<String>,
}
