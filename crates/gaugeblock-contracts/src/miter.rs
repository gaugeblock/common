//! `miter` AST-slicing wire domain: languages, symbols, lexical scopes and verification reports.

use serde::{Deserialize, Serialize};

/// A source language or declarative format the suite reads.
///
/// This mirrors `miter`'s implemented taxonomy (13 formats), not an aspirational list: a
/// language appears here only when an extractor exists for it. Wire spelling is the lowercase
/// `name()` form miter already uses and the `dado` pattern packs already contain.
///
/// Deserialization is case-insensitive and accepts the short aliases `rs`, `ts`, `js` and `py`, so
/// hand-written packs stay lenient while serialization stays canonical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Rust.
    Rust,
    /// TypeScript.
    TypeScript,
    /// TSX — TypeScript with JSX.
    Tsx,
    /// JavaScript, including the `.mjs` / `.cjs` module flavours.
    JavaScript,
    /// Python.
    Python,
    /// SQL DDL: tables, views, indexes and columns.
    Sql,
    /// HTML documents: ids, custom elements, templates and landmarks.
    Html,
    /// CSS and SCSS: custom properties, selectors, keyframes and layers.
    Css,
    /// Markdown: the ATX heading outline.
    Markdown,
    /// Dockerfile / Containerfile: stages, build arguments and published ports.
    Dockerfile,
    /// Kubernetes and Docker Compose manifests.
    Yaml,
    /// HCL / OpenTofu / Terraform: resources, modules and variables.
    Hcl,
    /// `.env` key files (values are never recorded).
    Env,
}

impl Language {
    /// Canonical lowercase wire spelling.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Sql => "sql",
            Self::Html => "html",
            Self::Css => "css",
            Self::Markdown => "markdown",
            Self::Dockerfile => "dockerfile",
            Self::Yaml => "yaml",
            Self::Hcl => "hcl",
            Self::Env => "env",
        }
    }

    /// Parses a wire spelling case-insensitively, accepting the short aliases.
    pub fn from_wire(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "rust" | "rs" => Some(Self::Rust),
            "typescript" | "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "javascript" | "js" => Some(Self::JavaScript),
            "python" | "py" => Some(Self::Python),
            "sql" => Some(Self::Sql),
            "html" => Some(Self::Html),
            "css" | "scss" => Some(Self::Css),
            "markdown" | "md" => Some(Self::Markdown),
            "dockerfile" | "docker" => Some(Self::Dockerfile),
            "yaml" | "yml" => Some(Self::Yaml),
            "hcl" | "terraform" => Some(Self::Hcl),
            "env" => Some(Self::Env),
            _ => None,
        }
    }
}

impl<'de> serde::Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_wire(&raw)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown language: {raw}")))
    }
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
