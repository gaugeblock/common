//! Retention and rotation for the raw span log (`GB-344`).
//!
//! The flight recorder is one append-only file, and an append-only file with no policy becomes the reason a
//! checkout is enormous. Two configured numbers bound it: `max_bytes` (rotate when the file reaches it) and
//! `keep_rotations` (how many rotated files to keep).
//!
//! **A span cannot be torn.** A span is written as one line by one `O_APPEND` write, so rotation between
//! lines is the only thing it can do, and no reader sees half a record. The trace-across-a-boundary worry
//! `GB-344` raises belongs to the *spool*, where a record is assembled over several writes.
//!
//! **Rotation loses nothing a reader wanted.** Each rotation writes one aggregate line beside the
//! rotations: how many spans, bytes, and per-binary counts with duration totals. The raw file is derived
//! state that ages out; the aggregate is the summary that survives it, so "what did the suite do last week"
//! has an answer after the spans are gone.
//!
//! One honest limit: another process may be mid-append when the file is rotated, and its write lands in the
//! rotated file because the descriptor it holds still points at that inode. The cost is one line in the
//! wrong file, not corruption, and a lock would cost every append a syscall to prevent it.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::paths::GaugeblockPaths;
use crate::suite_config::SuiteConfig;

/// Rotation policy for the raw span log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retention {
    /// Rotate when the file reaches this size.
    pub max_bytes: u64,
    /// How many rotated files to keep. `0` keeps none, which drops history at each rotation.
    pub keep: usize,
}

/// 8 MiB: roughly a month of a busy suite's spans, and small enough that nobody notices it in a checkout.
pub const DEFAULT_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Three rotations: with the default size, about a month of current history plus three of older spans.
pub const DEFAULT_KEEP: usize = 3;

impl Default for Retention {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
            keep: DEFAULT_KEEP,
        }
    }
}

/// The policy for the log at `events_path`, from `[telemetry] max_bytes` and `keep_rotations`.
///
/// Absence means the default, and a nonsense value (zero, negative) is treated as absence rather than as
/// "rotate on every append": a configuration mistake must not turn the write path into a churn loop.
#[must_use]
pub fn retention_for(events_path: &Path) -> Retention {
    let Some(suite_root) = suite_root_of(events_path) else {
        return Retention::default();
    };
    let Ok(config) = SuiteConfig::load(&suite_root) else {
        return Retention::default();
    };
    let max_bytes = config
        .integer("telemetry", "max_bytes")
        .filter(|value| *value > 0)
        .map_or(DEFAULT_MAX_BYTES, |value| value as u64);
    let keep = config
        .integer("telemetry", "keep_rotations")
        .filter(|value| *value >= 0)
        .map_or(DEFAULT_KEEP, |value| value as usize);
    Retention { max_bytes, keep }
}

/// Rotates `events_path` when it has reached the configured size.
///
/// Never an error the caller must handle: a rotation that cannot happen is "no rotation", which is the
/// same fail-open rule every other part of emission follows.
pub fn rotate_if_needed(events_path: &Path) {
    let retention = retention_for(events_path);
    let Ok(metadata) = std::fs::metadata(events_path) else {
        return;
    };
    if metadata.len() < retention.max_bytes {
        return;
    }

    // The aggregate must describe the file that is about to be renamed, so it is computed before the shift
    // and written only if the shift succeeded - a summary of a rotation that did not happen would be a
    // record of an event that did not occur.
    let summary = aggregate_line(events_path, metadata.len());
    if shift(events_path, retention.keep).is_err() {
        return;
    }
    let (Some(line), Some(dir)) = (summary, events_path.parent()) else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("aggregates.jsonl"))
    else {
        return;
    };
    let _ = file.write_all(line.as_bytes());
    let _ = file.write_all(b"\n");
}

/// Renames `events.jsonl` to `events.1.jsonl`, shifting the older ones along and dropping the excess.
fn shift(events_path: &Path, keep: usize) -> std::io::Result<()> {
    let dir = events_path
        .parent()
        .ok_or_else(|| std::io::Error::other("events path has no parent"))?;
    if keep == 0 {
        return std::fs::remove_file(events_path);
    }
    // Oldest first, so `events.N` is freed before `events.N-1` wants to become it.
    let _ = std::fs::remove_file(dir.join(format!("events.{keep}.jsonl")));
    for index in (1..keep).rev() {
        let from = dir.join(format!("events.{index}.jsonl"));
        let to = dir.join(format!("events.{}.jsonl", index + 1));
        if from.is_file() {
            std::fs::rename(&from, &to)?;
        }
    }
    std::fs::rename(events_path, dir.join("events.1.jsonl"))
}

/// One JSON line summarising the file about to be rotated, or `None` when it cannot be read.
fn aggregate_line(path: &Path, bytes: u64) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut counts: std::collections::BTreeMap<String, (u64, u64)> =
        std::collections::BTreeMap::new();
    let mut spans = 0u64;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        spans += 1;
        let binary = value
            .get("binary")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let duration = value
            .get("duration_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let entry = counts.entry(binary).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += duration;
    }
    let by_binary: Vec<serde_json::Value> = counts
        .into_iter()
        .map(|(binary, (count, duration_ms))| {
            serde_json::json!({"binary": binary, "spans": count, "duration_ms": duration_ms})
        })
        .collect();
    Some(
        serde_json::json!({
            "rotated_at_epoch_ms": crate::telemetry::now_ms(),
            "spans": spans,
            "bytes": bytes,
            "by_binary": by_binary,
        })
        .to_string(),
    )
}

/// Where the aggregates live for a suite root, for a caller that wants to read them.
#[must_use]
pub fn aggregates_path(suite_root: &Path) -> PathBuf {
    let events = GaugeblockPaths::at(suite_root).telemetry_events();
    events
        .parent()
        .map_or(events.clone(), |dir| dir.join("aggregates.jsonl"))
}

/// `…/telemetry/events.jsonl` → the suite root two levels up.
fn suite_root_of(events_path: &Path) -> Option<PathBuf> {
    let telemetry_dir = events_path.parent()?;
    let dot = telemetry_dir.parent()?;
    Some(dot.parent()?.to_path_buf())
}
