//! AST-Aware Secret Protection & Egress Sanitization.
//!
//! Lives in the suite foundation rather than in `miter` because redaction is not a miter concern:
//! `rabbet` and `joiner` need it, and reaching into another product's engine for a std-only helper is
//! a lateral edge that buys nothing (`GB-204`, `GB-158`).
//!
//! Preserves symbol and variable names for semantic queries while scrubbing
//! raw secret values and credentials to `"[REDACTED_SECRET]"`.

pub mod entropy;
pub mod redact;
mod scrub;

pub use entropy::{is_high_entropy_secret, shannon_entropy};
pub use redact::{REDACTED_SECRET, is_secret_literal, is_sensitive_identifier};
pub use scrub::scrub_source_text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrub_env_line() {
        let line = "STRIPE_SECRET_KEY=sk_live_1234567890abcdef";
        assert_eq!(
            scrub_source_text(line),
            "STRIPE_SECRET_KEY=\"[REDACTED_SECRET]\""
        );

        let export_line = "export DB_PASSWORD=\"supersecret\"";
        assert_eq!(
            scrub_source_text(export_line),
            "export DB_PASSWORD=\"[REDACTED_SECRET]\""
        );
    }

    #[test]
    fn test_scrub_code_assignment() {
        let rust_line = "pub const STRIPE_KEY: &str = \"sk_live_51Nz987654321abcdef\";";
        assert_eq!(
            scrub_source_text(rust_line),
            "pub const STRIPE_KEY: &str = \"[REDACTED_SECRET]\";"
        );
        let ts_line = "const defaultHeader = \"ghp_1234567890abcdefghijklmnopqrstuvwxyz\";";
        assert_eq!(
            scrub_source_text(ts_line),
            "const defaultHeader = \"[REDACTED_SECRET]\";"
        );
    }

    #[test]
    fn test_benign_code_preservation() {
        let code = "pub struct SyntaxToken { pub kind: String }";
        assert_eq!(scrub_source_text(code), code);
        let const_tok = "const MAX_TOKENS: usize = 4096;";
        assert_eq!(scrub_source_text(const_tok), const_tok);
    }
}
