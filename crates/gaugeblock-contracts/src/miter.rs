//! `miter` AST-slicing wire domain: languages, symbols, lexical scopes and verification reports.

use serde::{Deserialize, Serialize};

/// Source language understood by the suite's polyglot extractors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// Rust.
    Rust,
    /// TypeScript / TSX / JavaScript.
    TypeScript,
    /// Python.
    Python,
    /// Go.
    Go,
    /// C.
    C,
    /// C++.
    Cpp,
}

/// Classification of a symbol binding inside a lexical scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// Local variable binding.
    LocalVar,
    /// Function parameter.
    Parameter,
    /// Function or method.
    Function,
    /// Type, struct or class.
    TypeOrStruct,
    /// Trait or interface.
    TraitOrInterface,
    /// Constant.
    Constant,
}

/// A single name binding visible at a point in the AST.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolBinding {
    /// Bound name.
    pub name: String,
    /// Declared type rendered as source text.
    pub ty: String,
    /// Whether the binding can be reassigned.
    pub is_mutable: bool,
    /// Binding classification.
    pub kind: SymbolKind,
}

/// Lexical scope extracted at a byte/line offset to feed `dado` and `batten`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalScope {
    /// Repository-relative path of the file the scope belongs to.
    pub repo_relative_path: String,
    /// Language of the enclosing file.
    pub language: Language,
    /// Innermost enclosing symbol, when one exists.
    pub enclosing_symbol: Option<String>,
    /// Bindings visible in the scope.
    pub bindings: Vec<SymbolBinding>,
    /// Imports available at the offset.
    pub available_imports: Vec<String>,
}

/// Result of validating a synthesized AST splice or running compiler checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReport {
    /// Whether the splice produced a syntactically valid AST.
    pub ast_valid: bool,
    /// Whether the type checker accepted the result.
    pub typecheck_passed: bool,
    /// Whether tests ran and passed; `None` when tests were not requested.
    pub tests_passed: Option<bool>,
    /// Compiler diagnostics raised while verifying.
    pub diagnostics: Vec<CompilerDiagnostic>,
    /// Wall-clock verification duration in milliseconds.
    pub duration_ms: u64,
}

/// A single compiler diagnostic anchored to a source position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerDiagnostic {
    /// File the diagnostic refers to.
    pub file: String,
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
    /// Compiler error code (e.g. `E0308`), when the compiler supplies one.
    pub code: Option<String>,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Suggested fix, when the compiler offers one.
    pub suggestion: Option<String>,
}
