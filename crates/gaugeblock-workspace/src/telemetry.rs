//! Flight-recorder emission: one `O_APPEND` line per span, and never a failure that reaches a caller
//! (`GB-243`, AC2).
//!
//! Three rules, and each of them is a decision rather than a style preference:
//!
//! 1. **Append only, one line per span.** The file is shared by every process in the suite - CLIs, MCP
//!    servers, the release gate - so a write must not rewrite or reorder. `OpenOptions::append` gives
//!    that for free: the kernel serialises appends and no lock is needed.
//! 2. **Fail open.** Observability must never be the reason a build fails. Every error here is
//!    swallowed deliberately: a read-only checkout, a full disk, a path that is not writable - all of
//!    them mean "no telemetry", never "the command failed". Nothing in this module returns a `Result`
//!    to a caller, because there is no correct action for one to take.
//! 3. **Exactly once.** [`SpanGuard::finish`] writes and disarms; `Drop` writes only if `finish` never
//!    ran, so an early return or a `?` that unwinds still records the span, and it is recorded once.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use std::time::{SystemTime, UNIX_EPOCH};

use gaugeblock_contracts::{SpanStatus, TelemetrySpan};

/// Trace id of the run this process belongs to, set by a parent so child spans join it (`GB-243`).
pub const TRACE_ENV: &str = "GAUGEBLOCK_TRACE_ID";

/// Reads the ambient trace id, if the caller was started inside an instrumented run.
///
/// Returns `None` when the variable is absent or is not a UUID. Minting a replacement is deliberately
/// *not* done here: whether to reuse one id per process (`outfeed gate`, one trace per run) or mint per
/// call (`sawhorse`, one trace per uninstrumented completion) is a policy the consumer owns, and a
/// shared crate that picked one would be wrong for the other. What must not diverge is the variable
/// name and the parse, and those live here.
///
/// No test covers the absent case: `set_var` is unsafe in edition 2024 and this workspace denies
/// `unsafe_code`, so the variable cannot be manipulated from a test while other tests read it. The
/// consumers' tests cover the fallback path instead.
#[must_use]
pub fn ambient_trace_id() -> Option<uuid::Uuid> {
    std::env::var(TRACE_ENV)
        .ok()
        .and_then(|raw| uuid::Uuid::parse_str(raw.trim()).ok())
}

/// Appends one span as a single JSON line, creating the file and its directory when absent.
///
/// # Errors
///
/// Returns the I/O or serialisation error so a test can assert it. Callers should prefer
/// [`SpanGuard`], which swallows it: no caller of the flight recorder has a recovery to perform.
pub fn append_span(path: &Path, span: &TelemetrySpan) -> std::io::Result<()> {
    // A test run must not record into the log the suite is observed through (`GB-340`). The only spans in the
    // suite's log once were unit-test artifacts - a provider on a closed port, a deliberately absent API key -
    // and synthetic failures are worse than missing data: they teach a reader, human or model, that a clean run
    // is an error.
    //
    // The policy sits on this path and not on [`append_span_forced`], because a `SpanGuard` is constructed by a
    // caller that has already decided to record - including a test that is deliberately exercising emission.
    if !emission_enabled() {
        return Ok(());
    }
    append_span_forced(path, span)
}

/// Appends a span without consulting the emission policy.
///
/// For a caller that has already decided - `SpanGuard`, and tests of the write path itself. Nothing in production
/// should prefer this over [`append_span`]: the policy exists because a test run once wrote synthetic failures into
/// the only log the suite is observed through.
///
/// # Errors
///
/// [`std::io::Error`] when the span cannot be serialised, the directory cannot be created, or the append fails.
pub fn append_span_forced(path: &Path, span: &TelemetrySpan) -> std::io::Result<()> {
    let line = serde_json::to_string(span).map_err(std::io::Error::other)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    // One write call per line: the kernel keeps an `O_APPEND` write atomic against other appenders,
    // which is what lets several processes share the file without a lock.
    file.write_all(line.as_bytes())?;
    file.write_all(b"\n")
}

/// Milliseconds since the Unix epoch, or `0` if the clock is before it.
#[must_use]
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Writes its span on drop unless [`finish`](Self::finish) already did.
///
/// A guard rather than a pair of calls because spans wrap early returns: a `?` inside an instrumented
/// function used to lose the span entirely, and a lost span is worse than a late one.
#[derive(Debug)]
pub struct SpanGuard {
    path: PathBuf,
    span: Option<TelemetrySpan>,
}

impl SpanGuard {
    /// Opens a guard over an already-built span.
    ///
    /// The span is taken by value and returned only through the file, so there is no path where the
    /// same span is written twice.
    #[must_use]
    pub fn open(path: impl Into<PathBuf>, span: TelemetrySpan) -> Self {
        Self {
            path: path.into(),
            span: Some(span),
        }
    }

    /// The span being recorded, for a caller that needs to attach fields before it closes.
    #[must_use]
    pub fn span(&self) -> Option<&TelemetrySpan> {
        self.span.as_ref()
    }

    /// Mutable access to the span, so a caller can attach context without exposing the guard's state.
    pub fn span_mut(&mut self) -> Option<&mut TelemetrySpan> {
        self.span.as_mut()
    }

    /// Records the outcome and duration, then writes.
    ///
    /// Duration is measured here rather than passed in, so it is the guard's clock and every span is
    /// timed the same way.
    pub fn finish(mut self, status: SpanStatus) {
        if let Some(span) = self.span.as_mut() {
            span.status = status;
            span.duration_ms = Some(now_ms().saturating_sub(span.started_at_ms));
        }
        self.write();
    }

    fn write(&mut self) {
        if let Some(span) = self.span.take() {
            // Deliberately ignored: see the module docs. Telemetry is never a reason to fail.
            let _ = append_span_forced(&self.path, &span);
        }
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        self.write();
    }
}

/// Whether this process should record telemetry at all (`GB-340`).
///
/// Two reasons to stay quiet, both of which have put junk in a real log:
///
/// * `GAUGEBLOCK_NO_TELEMETRY` is set - an explicit opt-out for a scripted run that must not touch the log;
/// * the process **is a test binary**. Cargo builds them into `target/*/deps/<crate>-<hash>` and runs them from
///   there, and nextest does the same, so the executable path names the case without any test having to
///   cooperate. That matters because the setter cannot: this workspace denies `unsafe_code`, and `set_var` is
///   unsafe in edition 2024, so a test cannot turn emission off for itself.
#[must_use]
pub fn emission_enabled() -> bool {
    !disabled_by_env(std::env::var(NO_TELEMETRY_ENV).ok().as_deref())
        && !is_test_binary(&std::env::current_exe().unwrap_or_default())
}

/// Name of the opt-out variable.
pub const NO_TELEMETRY_ENV: &str = "GAUGEBLOCK_NO_TELEMETRY";

/// Whether the variable's value disables emission. Any value counts, including an empty one: a variable set to
/// nothing is still a deliberate act, and honouring it is cheaper than arguing about spellings.
#[must_use]
fn disabled_by_env(value: Option<&str>) -> bool {
    value.is_some()
}

/// Whether an executable path is a cargo-built test binary: `.../target/<profile>/deps/<name>-<hash>`.
///
/// Deliberately structural rather than name-based: a crate called `miter-tests` installed as a real binary lives
/// in `bin/`, not `deps/`, and a hash suffix is what cargo adds to a test artifact.
#[must_use]
fn is_test_binary(exe: &Path) -> bool {
    let in_deps = exe
        .parent()
        .and_then(|dir| dir.file_name())
        .is_some_and(|name| name == "deps");
    let hashed = exe
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.rsplit_once('-'));
    in_deps
        && hashed.is_some_and(|(_, hash)| {
            hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit())
        })
}

#[cfg(test)]
mod emission_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn an_installed_binary_is_not_a_test_binary() {
        assert!(!is_test_binary(Path::new("/Users/x/.cargo/bin/miter")));
        assert!(!is_test_binary(Path::new("/repo/target/release/storypole")));
    }

    #[test]
    fn a_cargo_test_artifact_is_recognised_by_its_directory_and_hash() {
        assert!(is_test_binary(Path::new(
            "/repo/target/debug/deps/storypole_core-9f8a1c2b3d4e5f60"
        )));
        // A name with a dash but no hash is not a cargo artifact.
        assert!(!is_test_binary(Path::new(
            "/repo/target/debug/deps/miter-tests"
        )));
        // A hashed name outside deps/ is something else entirely.
        assert!(!is_test_binary(Path::new(
            "/repo/target/debug/miter-9f8a1c2b3d4e5f60"
        )));
    }

    #[test]
    fn any_value_of_the_opt_out_variable_disables_emission() {
        assert!(disabled_by_env(Some("1")));
        assert!(disabled_by_env(Some("")));
        assert!(!disabled_by_env(None));
    }
}
