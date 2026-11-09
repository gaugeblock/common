//! One span per CLI invocation (`GB-339`).
//!
//! One of twelve tools emits spans today, and only for provider completions, so the panel observes inference and
//! nothing else. Telemetry is meant to be suite-wide, so every binary gets the same shape: a span that opens where the
//! process starts, records which command ran, where, and how long it took, and closes with the exit status.
//!
//! Deliberately one mechanism rather than nine: a tool that invents its own span shape makes the log unqueryable, and
//! a tool that forgets to emit is invisible - which is the state this exists to end.
//!
//! Nothing here can fail a command. A missing sink, an unwritable directory or an absent checkout all mean "no span",
//! never "the tool did not run".

use std::path::{Path, PathBuf};

use gaugeblock_contracts::{SpanStatus, TelemetrySpan};
use uuid::Uuid;

use crate::{GaugeblockPaths, Membership, SpanGuard, ambient_trace_id, now_ms};

/// Where this process should append spans (`GB-338`).
///
/// Telemetry is **derived, untracked** state, so it centralises: a suite of twelve repositories has one log, at the
/// suite root, because there is nothing to merge and every worktree of a member already shares it through the common
/// dir. The path follows the declared suite rather than the working directory, which is what made a member's spans
/// invisible to a panel tailing the root.
///
/// Falls back to the repository's own directory when no declared suite lists the caller: a lone checkout must still
/// record its own spans, and a user with several unrelated repositories has no suite to centralise into. `None`
/// outside any checkout, where there is nowhere meaningful to write.
#[must_use]
pub fn sink_path(start: &Path) -> Option<PathBuf> {
    // Canonicalised first because `Membership::containing` walks ancestors, and a relative path stops at the
    // current directory - the defect that cost GB-327 a session.
    let canonical = std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    if let Some(membership) = Membership::containing(&canonical) {
        return Some(GaugeblockPaths::at(&membership.suite_root).telemetry_events());
    }
    GaugeblockPaths::discover(&canonical)
        .ok()
        .map(|paths| paths.telemetry_events())
}

/// An invocation in flight, written when it finishes or is dropped.
#[derive(Debug)]
pub struct InvocationSpan {
    guard: Option<SpanGuard>,
}

impl InvocationSpan {
    /// Opens a span for `binary` running `command`, from the current directory.
    ///
    /// `command` is the subcommand when there is one, because "miter audit" and "miter verify" are different
    /// operations that happen to share an executable.
    #[must_use]
    pub fn start(binary: &str, command: Option<&str>) -> Self {
        // The policy is consulted here, at the entry point, because `SpanGuard::open` takes a path and writes -
        // it has no opinion about whether writing is allowed. Without this line the whole emission policy was
        // inert on the only production path: `SP-2`'s config switch did nothing, and `GAUGEBLOCK_NO_TELEMETRY`
        // did not stop an invocation span either. Found by setting `enabled = false` and watching the log grow.
        if !crate::telemetry::emission_enabled() {
            return Self { guard: None };
        }
        let cwd = std::env::current_dir().unwrap_or_default();
        let Some(sink) = sink_path(&cwd) else {
            return Self { guard: None };
        };

        let trace_id = ambient_trace_id().unwrap_or_else(|| {
            // No `v4` feature is enabled here, so the id is built from the process and the clock: unique for
            // our purposes - two spans in one process differ by the millisecond - and honest about not being
            // random.
            let pid = u128::from(std::process::id());
            Uuid::from_u128((pid << 64) | u128::from(now_ms()))
        });
        // A span id distinct from the trace id, from the same deterministic source (no `v4` feature here).
        let span_id = Uuid::from_u128(u128::from(now_ms()) ^ u128::from(std::process::id()));
        let mut span = TelemetrySpan::open(span_id, binary, trace_id, now_ms());
        span.command = command.map(str::to_string);
        span.repo = repository_of(&cwd);
        span.branch = branch_of(&cwd);
        Self {
            guard: Some(SpanGuard::open(sink, span)),
        }
    }

    /// Closes the span with the process outcome.
    ///
    /// Separate from `Drop` so a `main` can say what happened; dropping without finishing records the span as
    /// running, which is what an early `?` used to look like.
    pub fn finish(mut self, success: bool) {
        if let Some(guard) = self.guard.take() {
            guard.finish(if success {
                SpanStatus::Ok
            } else {
                SpanStatus::Error
            });
        }
    }
}

/// The repository name a span belongs to, which the telemetry record can no longer infer from a path (`GB-337`).
///
/// A declared suite answers with the member's name; anything else falls back to the checkout's directory name, which
/// is what a lone repository's name is.
fn repository_of(cwd: &std::path::Path) -> Option<String> {
    let canonical = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    if let Some(membership) = Membership::containing(&canonical) {
        return Some(membership.member.name.clone());
    }
    let root = gaugeblock_git::Repository::discover(&canonical)
        .ok()
        .map(|repo| repo.root().to_path_buf())?;
    root.file_name().map(|name| name.to_string_lossy().into())
}

/// The current branch, when git can say.
fn branch_of(cwd: &std::path::Path) -> Option<String> {
    let canonical = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    let repo = gaugeblock_git::Repository::discover(&canonical).ok()?;
    repo.current_branch().ok().flatten()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn a_span_carries_the_command_that_ran() {
        // The shape the panel groups by: binary plus command, because `miter audit` and `miter verify` are different
        // operations sharing one executable.
        let mut span = TelemetrySpan::open(Uuid::from_u128(1), "miter", Uuid::from_u128(2), 1_000);
        span.command = Some("audit".to_string());

        let line = serde_json::to_string(&span).unwrap();

        assert!(line.contains(r#""binary":"miter""#), "{line}");
        assert!(line.contains(r#""command":"audit""#), "{line}");
    }

    #[test]
    fn an_invocation_outside_a_checkout_still_opens_no_span() {
        // `start` must never fail a command: with no sink there is nothing to record, and the tool still runs.
        let temp = tempfile::tempdir().unwrap();
        assert!(sink_path(temp.path()).is_none());
    }
}
