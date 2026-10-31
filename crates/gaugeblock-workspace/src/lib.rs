//! Canonical `.gaugeblock/` paths, derived from the git common dir (`GB-189`).
//!
//! The suite had sixty-three sites each spelling `.gaugeblock` relative to whatever directory the
//! caller happened to be in. That is correct for the main checkout and wrong for a linked worktree,
//! where the working tree and the common dir differ - so a worktree grew a second ledger and a second
//! spool, and two agents working in parallel disagreed about which records existed.
//!
//! One rule replaces the sixty-three: **the canonical directory hangs off the git common dir, not off
//! the caller's directory.** Every worktree of a repository therefore resolves to the same
//! `.gaugeblock/`, which is what `GB-169` needs in order to isolate agents without splitting the
//! ledger.

mod paths;
mod suite_config;
mod topology;

pub use paths::GaugeblockPaths;
pub use suite_config::SuiteConfig;
pub use topology::{MemberRepo, WorkspaceTopology};

/// Failures from workspace resolution.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    /// The starting path is not inside a git repository, so no common dir exists to derive from.
    #[error(
        "`{start}` is not inside a git repository, so `.gaugeblock/` cannot be located: {source}"
    )]
    NotARepository {
        /// The path the caller started from.
        start: String,
        /// The git failure that established this.
        #[source]
        source: gaugeblock_git::GitError,
    },

    /// The unified configuration exists but is not valid TOML.
    #[error("`{path}` is not a valid TOML configuration: {message}")]
    InvalidConfig {
        /// The file that failed to parse.
        path: String,
        /// Why it failed, as reported by the parser.
        message: String,
    },
}
