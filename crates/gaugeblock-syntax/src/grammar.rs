//! The grammar table: which tree-sitter grammar reads which language.

use gaugeblock_contracts::Language;

/// What loading a grammar can fail with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxError {
    /// The language has no syntax tree worth walking (a declarative format).
    Unsupported {
        /// The language that was asked for.
        language: Language,
    },
    /// The grammar could not be loaded, or the parser rejected it.
    Grammar {
        /// The language whose grammar failed.
        language: Language,
        /// Underlying message.
        message: String,
    },
    /// `tree-sitter` returned no tree for the source.
    Parse {
        /// The language being parsed.
        language: Language,
    },
}

impl core::fmt::Display for SyntaxError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SyntaxError::Unsupported { language } => write!(
                formatter,
                "'{name}' is a declarative format with no syntax tree to validate",
                name = language_name(*language)
            ),
            SyntaxError::Grammar { language, message } => write!(
                formatter,
                "the {name} grammar could not be loaded: {message}",
                name = language_name(*language)
            ),
            SyntaxError::Parse { language } => {
                write!(
                    formatter,
                    "the parser returned no tree for {name}",
                    name = language_name(*language)
                )
            }
        }
    }
}

impl std::error::Error for SyntaxError {}

/// Human-readable name, so an error never prints a Rust variant name to a user.
#[must_use]
pub const fn language_name(language: Language) -> &'static str {
    match language {
        Language::Rust => "rust",
        Language::TypeScript => "typescript",
        Language::Tsx => "tsx",
        Language::JavaScript => "javascript",
        Language::Python => "python",
        // Declarative formats keep their wire names so a rejected request is legible.
        other => declarative_name(other),
    }
}

const fn declarative_name(language: Language) -> &'static str {
    match language {
        Language::Rust => "rust",
        Language::TypeScript => "typescript",
        Language::Tsx => "tsx",
        Language::JavaScript => "javascript",
        Language::Python => "python",
        Language::Sql => "sql",
        Language::Html => "html",
        Language::Css => "css",
        _ => "unknown",
    }
}

/// Loads the grammar for `language`.
///
/// `Tsx` and `JavaScript` both answer the TSX grammar: TSX is the superset that accepts JSX, which is the
/// reason a `.jsx` file exists.
///
/// # Errors
///
/// [`SyntaxError::Unsupported`] for a declarative format.
pub fn grammar_for(language: Language) -> Result<tree_sitter::Language, SyntaxError> {
    match language {
        Language::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
        Language::TypeScript => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        Language::Tsx | Language::JavaScript => Ok(tree_sitter_typescript::LANGUAGE_TSX.into()),
        Language::Python => Ok(tree_sitter_python::LANGUAGE.into()),
        other => Err(SyntaxError::Unsupported { language: other }),
    }
}

/// Builds a parser for `language`.
///
/// # Errors
///
/// [`SyntaxError::Unsupported`] for a declarative format, [`SyntaxError::Grammar`] when the parser rejects
/// the grammar it was handed.
pub fn parser_for(language: Language) -> Result<tree_sitter::Parser, SyntaxError> {
    let grammar = grammar_for(language)?;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar)
        .map_err(|error| SyntaxError::Grammar {
            language,
            message: error.to_string(),
        })?;
    Ok(parser)
}
