//! Wire-shape regression tests for the shared spool outbox record (`GB-182`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::{
    AnonymizedSynthesisTrace, Language, SpoolEntry, SyncStatus, TelemetryTier, VerificationReport,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn trace(task_key: Option<&str>, head: Option<&str>) -> AnonymizedSynthesisTrace {
    AnonymizedSynthesisTrace {
        trace_id: Uuid::from_u128(42),
        language: Language::Rust,
        tier: TelemetryTier::Silver,
        selected_pattern_id: None,
        anonymized_scope_signature: "fn tokenize(input: &str) -> Vec<Token>".to_string(),
        filled_holes: vec![("body".to_string(), "input.split_whitespace()".to_string())],
        rejected_attempt: None,
        verification: VerificationReport {
            ast_valid: true,
            typecheck_passed: true,
            tests_passed: Some(true),
            diagnostics: vec![],
            duration_ms: 7,
        },
        task_key: task_key.map(str::to_string),
        head_commit_sha: head.map(str::to_string),
    }
}

#[test]
fn test_spool_entry_field_names_and_snake_case_status() {
    let entry = SpoolEntry::from_trace(trace(Some("GB-182"), Some("abc123")), 1_700_000_000_000)
        .expect("entry");
    let value = serde_json::to_value(&entry).expect("value");

    for key in ["trace", "content_hash", "sync_status", "recorded_at_ms"] {
        assert!(value.get(key).is_some(), "missing SpoolEntry.{key}");
    }
    assert_eq!(value["sync_status"], "pending", "status is snake_case");
    assert_eq!(value["recorded_at_ms"], 1_700_000_000_000_u64);
    assert_eq!(value["trace"]["task_key"], "GB-182");
    assert_eq!(value["trace"]["head_commit_sha"], "abc123");
}

#[test]
fn test_spool_entry_roundtrips() {
    let entry = SpoolEntry::from_trace(trace(Some("GB-182"), Some("deadbeef")), 12).expect("entry");
    let json = serde_json::to_string(&entry).expect("json");
    assert!(!json.contains('\n'), "one entry is exactly one line");
    let parsed: SpoolEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(entry, parsed);
}

#[test]
fn test_every_sync_status_roundtrips_as_snake_case() {
    for (status, wire) in [
        (SyncStatus::Pending, "pending"),
        (SyncStatus::Synced, "synced"),
        (SyncStatus::Rejected, "rejected"),
    ] {
        let encoded = serde_json::to_string(&status).expect("encode");
        assert_eq!(encoded, format!("\"{wire}\""));
        let decoded: SyncStatus = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded, status);
        assert_eq!(status.to_string(), wire);
    }
    assert_eq!(SyncStatus::default(), SyncStatus::Pending);
}

#[test]
fn test_hash_is_sha256_over_canonical_trace_json() {
    let subject = trace(Some("GB-182"), Some("abc123"));
    let entry = SpoolEntry::from_trace(subject.clone(), 1).expect("entry");

    // Recomputed independently of the implementation: sha256 over `serde_json` bytes, lowercase hex.
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&subject).expect("bytes"));
    let expected = format!("{:x}", hasher.finalize());

    assert_eq!(entry.content_hash, expected);
    assert_eq!(entry.content_hash.len(), 64);
    assert_eq!(
        SpoolEntry::hash_trace(&subject).expect("hash"),
        entry.content_hash,
        "from_trace must not diverge from hash_trace"
    );
    assert_eq!(
        entry.sync_status,
        SyncStatus::Pending,
        "a freshly spooled entry is awaiting sync"
    );
}
