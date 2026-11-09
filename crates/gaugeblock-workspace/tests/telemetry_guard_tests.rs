//! `SpanGuard` (`GB-243`, AC2): one `O_APPEND` line per span, created on demand, failing open.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use gaugeblock_contracts::{SpanStatus, TelemetrySpan};
use gaugeblock_workspace::{GaugeblockPaths, SpanGuard, append_span_forced, now_ms};
use tempfile::TempDir;
use uuid::Uuid;

fn span() -> TelemetrySpan {
    TelemetrySpan::open(Uuid::from_u128(3), "miter", Uuid::from_u128(4), now_ms())
}

fn lines(path: &std::path::Path) -> Vec<String> {
    fs::read_to_string(path)
        .expect("recorder file")
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn a_guard_writes_exactly_one_line_on_drop() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("telemetry").join("events.jsonl");
    {
        let _guard = SpanGuard::open(&path, span());
    }

    let written = lines(&path);
    assert_eq!(written.len(), 1, "one span is one line");
    let parsed: TelemetrySpan = serde_json::from_str(&written[0]).expect("a span line");
    assert_eq!(parsed.binary, "miter");
}

#[test]
fn the_file_and_its_directory_are_created_when_missing() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("deep").join("nested").join("events.jsonl");
    assert!(!path.exists());

    append_span_forced(&path, &span()).expect("creates the file");

    assert!(path.is_file(), "emission creates the file");
}

#[test]
fn finish_writes_once_and_drop_writes_nothing_more() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("events.jsonl");

    let guard = SpanGuard::open(&path, span());
    guard.finish(SpanStatus::Ok);

    assert_eq!(lines(&path).len(), 1, "finish disarms the guard");
}

#[test]
fn an_early_return_still_records_the_span() {
    // The reason this is a guard: a `?` in an instrumented function must not lose the span.
    fn instrumented(path: &std::path::Path) -> Result<(), &'static str> {
        let _guard = SpanGuard::open(path, span());
        Err("this function bailed out early")
    }

    let temp = TempDir::new().unwrap();
    let path = temp.path().join("events.jsonl");
    let _ = instrumented(&path);

    let written = lines(&path);
    assert_eq!(written.len(), 1);
    let parsed: TelemetrySpan = serde_json::from_str(&written[0]).expect("a span line");
    assert_eq!(
        parsed.status,
        SpanStatus::Running,
        "an unfinished span is recorded as running, not silently dropped"
    );
}

#[test]
fn emission_fails_open_when_the_path_cannot_be_written() {
    // A file where a directory must be: unwritable, and no panic, no error reaching the caller.
    let temp = TempDir::new().unwrap();
    let blocker = temp.path().join("blocker");
    fs::write(&blocker, b"not a directory").unwrap();
    let unwritable = blocker.join("events.jsonl");

    let guard = SpanGuard::open(&unwritable, span());
    guard.finish(SpanStatus::Error);

    assert!(
        append_span_forced(&unwritable, &span()).is_err(),
        "the error exists"
    );
    assert!(!unwritable.exists(), "nothing was written");
}

#[test]
fn the_canonical_telemetry_path_is_where_the_guard_writes() {
    // Ties the two halves of AC2 together: the path comes from GaugeblockPaths and the guard writes it,
    // so a caller never spells `.gaugeblock/telemetry/events.jsonl` itself.
    let temp = TempDir::new().unwrap();
    let paths = GaugeblockPaths::at(temp.path());
    let events = paths.telemetry_events();

    assert_eq!(
        events,
        temp.path()
            .join(".gaugeblock")
            .join("telemetry")
            .join("events.jsonl")
    );

    SpanGuard::open(&events, span()).finish(SpanStatus::Ok);

    assert!(events.is_file(), "the guard creates the canonical log");
}

#[test]
fn spans_accumulate_rather_than_overwrite() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("events.jsonl");

    for _ in 0..3 {
        SpanGuard::open(&path, span()).finish(SpanStatus::Ok);
    }

    assert_eq!(lines(&path).len(), 3, "append only, never rewrite");
}
