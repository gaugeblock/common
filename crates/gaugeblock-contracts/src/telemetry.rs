//! Synthesis telemetry: the trace records `joiner` emits, `storypole` captures and `rabbet` mines.
//!
//! These moved out of `joiner-core` (GB-165) for one reason: `storypole verify` must append a trace
//! without depending on `joiner`. A second definition on the storypole side is exactly how the
//! duplicate-parser drift in `GB-148` began, so the record lives where every writer can reach it.
//!
//! The `language` field is this crate's wire [`Language`], not `miter_engine::Language`: producers
//! convert at the boundary (`impl From<PolyglotLanguage> for Language`), which is the same split the
//! rest of the suite already uses. Serialisation is unchanged - lowercase names on both sides.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::miter::{Language, VerificationReport};

/// Quality tier assigned to an anonymized synthesis trace per `CONTRACTS.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryTier {
    /// Compiled and typechecked cleanly on attempt #1 without tests.
    Bronze,
    /// Passed compiler checks and unit tests on attempt #1.
    Silver,
    /// Required self-repair retry or Oracle fallback (contains DPO preference pair).
    Gold,
    /// Survived unchanged in main across subsequent release commits.
    Diamond,
}

/// Wire contract representing an AST-anonymized synthesis run per `CONTRACTS.md`.
///
/// The provenance fields are optional and default when absent, so every record written before
/// `GB-165` still deserialises and every existing consumer keeps compiling untouched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnonymizedSynthesisTrace {
    /// Globally unique trace identifier.
    pub trace_id: Uuid,
    /// Programming language of the synthesized code.
    pub language: Language,
    /// Quality tier achieved by this synthesis run.
    pub tier: TelemetryTier,
    /// Selected `dado` pattern ID, if applicable.
    pub selected_pattern_id: Option<String>,
    /// Normalized anonymized lexical scope signature.
    pub anonymized_scope_signature: String,
    /// Completed hole expressions `(hole_id, anonymized_expression)`.
    pub filled_holes: Vec<(String, String)>,
    /// Rejected candidate attempt (populated for Gold DPO pairs).
    pub rejected_attempt: Option<String>,
    /// Detailed verification report.
    pub verification: VerificationReport,
    /// Storypole ticket the trace came from, when captured by `storypole verify`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_key: Option<String>,
    /// HEAD commit the trace was captured at; `(task_key, head_commit_sha)` is the idempotency key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_commit_sha: Option<String>,
}
