//! `TelemetrySpan` (AC1 of `GB-243`): the flight-recorder line every process appends.
//!
//! The property that matters is not that it round-trips, but that an absent optional field produces no
//! key at all — the log is appended to by every CLI and MCP server, so `null` padding would triple its
//! size and add fields a reader never asked for.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::{SpanStatus, TelemetrySpan};
use uuid::Uuid;

#[test]
fn an_open_span_omits_every_absent_optional_field() {
    let trace_id = Uuid::from_u128(7);
    let span = TelemetrySpan::open(Uuid::from_u128(7), "miter", trace_id, 1_700_000_000_000);
    let json = serde_json::to_value(&span).expect("serialise");

    let object = json.as_object().expect("an object");
    for absent in [
        "parent_span_id",
        "command",
        "task_key",
        "branch",
        "duration_ms",
        "first_diagnostic",
        "worktree_diff_hash",
    ] {
        assert!(
            !object.contains_key(absent),
            "`{absent}` must be omitted when absent, not written as null"
        );
    }
    assert_eq!(object["status"], "running");
    assert_eq!(object["binary"], "miter");
}

#[test]
fn a_span_round_trips_with_its_context() {
    let mut span = TelemetrySpan::open(
        Uuid::from_u128(7),
        "outfeed",
        Uuid::from_u128(7),
        1_700_000_000_000,
    );
    span.parent_span_id = Some(Uuid::from_u128(7));
    span.command = Some("gate".to_string());
    span.task_key = Some("GB-243".to_string());
    span.branch = Some("feat/gb-243".to_string());
    span.status = SpanStatus::Error;
    span.duration_ms = Some(42);
    span.first_diagnostic = Some("error[E0425]: cannot find value".to_string());
    span.worktree_diff_hash = Some("abc123".to_string());

    let json = serde_json::to_string(&span).expect("serialise");
    let back: TelemetrySpan = serde_json::from_str(&json).expect("deserialise");

    assert_eq!(back, span);
}

#[test]
fn an_unknown_field_from_a_newer_writer_is_ignored() {
    // A span written by a later version must not break today's reader; the log is append-only and
    // shared by every process in the suite.
    let span = TelemetrySpan::open(Uuid::from_u128(7), "chalkline", Uuid::from_u128(7), 5);
    let mut json = serde_json::to_value(&span).expect("serialise");
    json["a_field_from_the_future"] = serde_json::json!(true);

    let parsed: TelemetrySpan = serde_json::from_value(json).expect("unknown fields are ignored");
    assert_eq!(parsed.span_id, span.span_id);
}

#[test]
fn a_minimal_line_deserialises_without_optional_keys() {
    let json = r#"{
        "span_id": "00000000-0000-0000-0000-000000000001",
        "trace_id": "00000000-0000-0000-0000-000000000002",
        "binary": "storypole",
        "status": "ok",
        "started_at_ms": 10
    }"#;

    let span: TelemetrySpan = serde_json::from_str(json).expect("deserialise a bare span");
    assert_eq!(span.status, SpanStatus::Ok);
    assert!(span.command.is_none() && span.duration_ms.is_none());
}
