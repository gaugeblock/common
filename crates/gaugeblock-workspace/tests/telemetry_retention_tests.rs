//! Retention and rotation for the raw span log (`GB-344`).
//!
//! The defect these pin: the flight recorder was one append-only file with no policy, so it grew forever and
//! nothing summarised what it had held. Rotation bounds it; the aggregate line means the spans aging out
//! leave a record of what they were.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use gaugeblock_contracts::{SpanStatus, TelemetrySpan};
use gaugeblock_workspace::{
    Retention, aggregates_path, append_span_forced, now_ms, retention_for, rotate_if_needed,
};

/// A suite root with a `[telemetry]` policy and the events path under it.
fn suite(root: &Path, max_bytes: u64, keep: usize) -> std::path::PathBuf {
    std::fs::create_dir_all(root.join(".gaugeblock")).expect("suite dir");
    std::fs::write(
        root.join(".gaugeblock").join("config.toml"),
        format!("[telemetry]\nmax_bytes = {max_bytes}\nkeep_rotations = {keep}\n"),
    )
    .expect("config");
    root.join(".gaugeblock")
        .join("telemetry")
        .join("events.jsonl")
}

fn span(binary: &str, duration_ms: u64) -> TelemetrySpan {
    TelemetrySpan {
        span_id: uuid::Uuid::nil(),
        trace_id: uuid::Uuid::nil(),
        parent_span_id: None,
        binary: binary.to_string(),
        command: Some("list".to_string()),
        task_key: None,
        branch: Some("main".to_string()),
        repo: Some("suite".to_string()),
        status: SpanStatus::Ok,
        started_at_ms: now_ms(),
        duration_ms: Some(duration_ms),
        first_diagnostic: None,
        worktree_diff_hash: None,
    }
}

#[test]
fn the_policy_is_read_from_the_suite_config() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let events = suite(tmp.path(), 4096, 2);
    let policy = retention_for(&events);
    assert_eq!(policy.max_bytes, 4096);
    assert_eq!(policy.keep, 2);
}

#[test]
fn a_nonsense_policy_falls_back_rather_than_rotating_on_every_append() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let events = suite(tmp.path(), 0, 3);
    let policy = retention_for(&events);
    assert_eq!(
        policy.max_bytes,
        Retention::default().max_bytes,
        "zero is a configuration mistake, not an instruction to churn"
    );
}

#[test]
fn reaching_the_size_rotates_the_file_and_leaves_an_aggregate() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let events = suite(tmp.path(), 400, 3);

    // Each span is ~250 bytes, so a few appends cross the threshold.
    for index in 0..6 {
        append_span_forced(&events, &span("miter", 10 * index)).expect("append");
    }

    let dir = events.parent().expect("telemetry dir");
    assert!(
        dir.join("events.1.jsonl").is_file(),
        "the full file was rotated aside"
    );
    let aggregate =
        std::fs::read_to_string(aggregates_path(tmp.path())).expect("aggregate written");
    let first: serde_json::Value =
        serde_json::from_str(aggregate.lines().next().expect("one line")).expect("json");
    assert!(
        first["spans"].as_u64().expect("count") > 0,
        "the aggregate counts the spans it rotated"
    );
    assert_eq!(
        first["by_binary"][0]["binary"].as_str(),
        Some("miter"),
        "and says who they came from"
    );
}

#[test]
fn only_the_configured_number_of_rotations_is_kept() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let events = suite(tmp.path(), 300, 2);
    for _ in 0..12 {
        append_span_forced(&events, &span("dado", 5)).expect("append");
    }

    let dir = events.parent().expect("telemetry dir");
    assert!(dir.join("events.1.jsonl").is_file(), "newest rotation");
    assert!(dir.join("events.2.jsonl").is_file(), "older rotation");
    assert!(
        !dir.join("events.3.jsonl").exists(),
        "beyond the policy nothing is kept"
    );
}

#[test]
fn a_log_below_the_threshold_is_left_alone() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let events = suite(tmp.path(), 1024 * 1024, 3);
    append_span_forced(&events, &span("storypole", 7)).expect("append");
    rotate_if_needed(&events);

    let dir = events.parent().expect("telemetry dir");
    assert!(events.is_file(), "the file is still the current one");
    assert!(
        !dir.join("events.1.jsonl").exists(),
        "and nothing was rotated"
    );
}
