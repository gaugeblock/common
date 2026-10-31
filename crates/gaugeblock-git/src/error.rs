//! Failure modes of driving `git` as a subprocess.
//!
//! Stderr is carried on every variant. Callers used to lose it by unwrapping `status.success()`, so a
//! failing `git push` surfaced as "command failed" with the actual reason discarded; keeping stderr
//! in the error is the reason this type exists rather than a `String`.

/// Anything that can go wrong while running git.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// `git` could not be started at all (missing from `PATH`, or not executable).
    #[error("could not run git: {source}")]
    Spawn {
        /// Underlying spawn failure.
        #[source]
        source: std::io::Error,
    },
    /// `git` ran and exited non-zero.
    #[error("`git {command}` failed with exit code {code}: {stderr}")]
    Failed {
        /// The subcommand and arguments, joined for display.
        command: String,
        /// Process exit code, or `-1` when terminated by a signal.
        code: i32,
        /// Captured stderr, trimmed.
        stderr: String,
    },
    /// A staging request named no paths.
    ///
    /// Refused rather than treated as "everything": scoped staging exists so a commit cannot acquire
    /// files nobody asked for (`GB-150`, `GB-180`).
    #[error("staging was given no paths; refusing to stage everything")]
    EmptyScope,

    /// `git` succeeded but printed something that is not UTF-8.
    #[error("`git {command}` printed output that is not valid UTF-8")]
    NonUtf8 {
        /// The subcommand and arguments, joined for display.
        command: String,
    },
}
