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

impl core::fmt::Display for TelemetryTier {
    /// Renders the tier as the snake_case name used in the wire form and in the store.
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let name = match self {
            Self::Bronze => "bronze",
            Self::Silver => "silver",
            Self::Gold => "gold",
            Self::Diamond => "diamond",
        };
        formatter.write_str(name)
    }
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

/// Synchronisation state of a spooled trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// Awaiting transmission to a hub.
    #[default]
    Pending,
    /// Successfully synchronised.
    Synced,
    /// Rejected by schema validation or deduplication.
    Rejected,
}

impl core::fmt::Display for SyncStatus {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::Pending => "pending",
            Self::Synced => "synced",
            Self::Rejected => "rejected",
        })
    }
}

/// One line of the spool outbox: the trace plus its identity and sync state.
///
/// Every writer (`storypole verify`, `joiner fill`, `benchwork`) appends this shape and every reader
/// (`rabbet` ingestion) parses it, so the outbox is one format rather than three (`GB-182`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpoolEntry {
    /// The anonymised trace payload.
    pub trace: AnonymizedSynthesisTrace,
    /// SHA-256 of the canonical trace JSON, used for deduplication.
    pub content_hash: String,
    /// Current synchronisation state.
    pub sync_status: SyncStatus,
    /// When the entry was spooled, in epoch milliseconds.
    pub recorded_at_ms: u64,
}

impl SpoolEntry {
    /// Builds an entry, computing `content_hash` the one canonical way.
    ///
    /// The hash is SHA-256 over `serde_json` bytes of the trace, which is what `joiner` wrote before
    /// this type was shared - so hashes already recorded stay comparable.
    ///
    /// # Errors
    ///
    /// Returns the serialiser error if the trace cannot be encoded.
    pub fn from_trace(
        trace: AnonymizedSynthesisTrace,
        recorded_at_ms: u64,
    ) -> Result<Self, serde_json::Error> {
        let content_hash = Self::hash_trace(&trace)?;
        Ok(Self {
            trace,
            content_hash,
            sync_status: SyncStatus::Pending,
            recorded_at_ms,
        })
    }

    /// Computes the canonical content hash of a trace.
    ///
    /// # Errors
    ///
    /// Returns the serialiser error if the trace cannot be encoded.
    pub fn hash_trace(trace: &AnonymizedSynthesisTrace) -> Result<String, serde_json::Error> {
        use sha2::{Digest, Sha256};
        let bytes = serde_json::to_vec(trace)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }
}
