//! The resolved `.gaugeblock/` directory and the paths inside it.

use std::path::{Path, PathBuf};

use crate::WorkspaceError;

/// Name of the directory the suite keeps its state in.
const DIRECTORY: &str = ".gaugeblock";

/// The canonical `.gaugeblock/` directory for one repository.
///
/// Constructed by [`GaugeblockPaths::discover`] from a path inside a checkout, or by
/// [`GaugeblockPaths::at`] for a caller that already knows where the suite root is (a test fixture, or
/// a tool operating on a directory it was handed deliberately).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GaugeblockPaths {
    root: PathBuf,
}

impl GaugeblockPaths {
    /// Resolves the canonical directory from the git common dir of `start`.
    ///
    /// # Errors
    ///
    /// [`WorkspaceError::NotARepository`] when `start` is not inside a checkout. It is deliberately an
    /// error rather than a fallback: guessing a directory by walking up would recreate the ambiguity
    /// this type exists to remove.
    pub fn discover(start: &Path) -> Result<Self, WorkspaceError> {
        let repo = gaugeblock_git::Repository::discover(start).map_err(|source| {
            WorkspaceError::NotARepository {
                start: start.display().to_string(),
                source,
            }
        })?;
        let common_dir = repo
            .common_dir()
            .map_err(|source| WorkspaceError::NotARepository {
                start: start.display().to_string(),
                source,
            })?;
        // For a normal checkout the common dir is `<repo>/.git`, and for a linked worktree it is
        // still `<main repo>/.git` - which is exactly why both resolve to one `.gaugeblock/`.
        let suite_root = common_dir.parent().unwrap_or(&common_dir);
        Ok(Self {
            root: suite_root.join(DIRECTORY),
        })
    }

    /// The `.gaugeblock/` directory for a known suite root, without consulting git.
    #[must_use]
    pub fn at(suite_root: &Path) -> Self {
        Self {
            root: suite_root.join(DIRECTORY),
        }
    }

    /// The `.gaugeblock/` directory itself.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The repository the directory belongs to.
    #[must_use]
    pub fn suite_root(&self) -> &Path {
        self.root.parent().unwrap_or(&self.root)
    }

    /// Per-repository state inside `.gaugeblock/`, e.g. `.gaugeblock/storypole`.
    #[must_use]
    pub fn member(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// The redb index used by `miter`.
    #[must_use]
    pub fn index_db(&self) -> PathBuf {
        self.member("miter").join("miter.redb")
    }

    /// The directory telemetry envelopes are spooled into.
    #[must_use]
    pub fn spool_dir(&self) -> PathBuf {
        self.root.join("spool")
    }

    /// The telemetry outbox every producer appends to and `rabbet` integrates against.
    #[must_use]
    pub fn outbox(&self) -> PathBuf {
        self.spool_dir().join("outbox.jsonl")
    }

    /// The storypole ledger.
    #[must_use]
    pub fn ledger(&self) -> PathBuf {
        self.member("storypole").join("ledger.jsonl")
    }

    /// The directory holding unabridged task plans (`GB-11` law 11).
    #[must_use]
    pub fn plans_dir(&self) -> PathBuf {
        self.member("storypole").join("plans")
    }

    /// The flight-recorder directory, `.gaugeblock/telemetry` (`GB-243`).
    #[must_use]
    pub fn telemetry_dir(&self) -> PathBuf {
        self.root.join("telemetry")
    }

    /// The flight-recorder log, `.gaugeblock/telemetry/events.jsonl` (`GB-243`).
    ///
    /// One file for the whole suite, resolved through the git common dir like everything else here, so
    /// a worktree appends to the same log the main checkout reads. It is gitignored: spans are machine
    /// state, not source.
    #[must_use]
    pub fn telemetry_events(&self) -> PathBuf {
        self.telemetry_dir().join("events.jsonl")
    }

    /// The dotenv file holding secrets, resolved by `benchwork` and friends before the environment.
    ///
    /// It lives beside the config rather than inside a member directory because credentials are
    /// suite-wide: one `.env` answers for every tool that needs a provider key.
    #[must_use]
    pub fn env_file(&self) -> PathBuf {
        self.root.join(".env")
    }

    /// The suite configuration file.
    #[must_use]
    pub fn config(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// The per-tool configuration for `tool`, e.g. `.gaugeblock/miter/config.toml`.
    #[must_use]
    pub fn tool_config(&self, tool: &str) -> PathBuf {
        self.member(tool).join("config.toml")
    }
}
