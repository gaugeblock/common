//! The suite-wide `.gaugeblock/config.toml` and its `.env` neighbour.
//!
//! Every tool used to parse this file itself: `outfeed`, `benchwork` and `joiner` each carried a
//! `[tool]`-table extraction and a credential lookup, and they disagreed at the edges - one trimmed
//! its value, one did not; one treated an empty variable as absent, one returned it. Loading stays
//! here and the *typed* interpretation stays with the tool, because this crate cannot depend on every
//! tool's schema without inverting the dependency graph.
//!
//! The file is read as a generic table on purpose. A typed struct per tool would put each tool's
//! config type in the foundation crate, which is the coupling the suite exists to avoid.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::WorkspaceError;
use crate::paths::GaugeblockPaths;

/// The unified suite configuration and its credentials.
#[derive(Clone, Debug, PartialEq)]
pub struct SuiteConfig {
    paths: GaugeblockPaths,
    table: toml::Table,
    env: BTreeMap<String, String>,
}

impl SuiteConfig {
    /// Loads `.gaugeblock/config.toml` and `.gaugeblock/.env` for a known suite root.
    ///
    /// A missing file is an empty configuration rather than an error: every tool must work in a fresh
    /// checkout that has never been configured, and refusing there would make `init` a prerequisite of
    /// `--help`.
    ///
    /// # Errors
    ///
    /// [`WorkspaceError::InvalidConfig`] when the file exists but is not valid TOML. Existence is not
    /// the caller's problem, but a malformed file is: silently defaulting would discard the user's
    /// configuration at the moment they most need to be told.
    pub fn load(suite_root: &Path) -> Result<Self, WorkspaceError> {
        let paths = GaugeblockPaths::at(suite_root);
        let table =
            match std::fs::read_to_string(paths.config()) {
                Ok(content) => content.parse::<toml::Table>().map_err(|source| {
                    WorkspaceError::InvalidConfig {
                        path: paths.config().display().to_string(),
                        message: source.to_string(),
                    }
                })?,
                Err(_) => toml::Table::new(),
            };
        let env = read_env_file(&paths.env_file());
        Ok(Self { paths, table, env })
    }

    /// The paths this configuration was loaded from.
    #[must_use]
    pub fn paths(&self) -> &GaugeblockPaths {
        &self.paths
    }

    /// One tool's table, e.g. `section("sawhorse")`.
    #[must_use]
    pub fn section(&self, name: &str) -> Option<&toml::Table> {
        self.table.get(name).and_then(|value| value.as_table())
    }

    /// A string inside a tool's table.
    #[must_use]
    pub fn string(&self, section: &str, key: &str) -> Option<&str> {
        self.section(section)
            .and_then(|table| table.get(key))
            .and_then(|value| value.as_str())
    }

    /// The names of every configured tool, sorted.
    #[must_use]
    pub fn sections(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.table.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Resolves a credential, preferring the process environment over `.gaugeblock/.env`.
    ///
    /// The environment wins because a shell export or a CI secret is a deliberate override, while
    /// `.env` is the committed-ish default a developer edits. A blank value counts as absent in either
    /// source - an empty `ANTHROPIC_API_KEY=` in a shell profile or a placeholder left in a committed
    /// `.env` is a mistake, not a credential, and returning `""` would hand the provider an empty
    /// token instead of reporting that none is configured.
    #[must_use]
    pub fn credential(&self, key: &str) -> Option<String> {
        self.credential_with(key, |key| std::env::var(key).ok())
    }

    /// [`SuiteConfig::credential`] with the environment supplied by the caller.
    ///
    /// The seam exists because `std::env::set_var` is unsafe under Rust 2024 and `unsafe_code` is
    /// denied suite-wide, so precedence cannot be exercised by mutating the real environment. Passing
    /// the lookup in keeps the rule testable instead of leaving it to production.
    #[must_use]
    pub fn credential_with<F>(&self, key: &str, env: F) -> Option<String>
    where
        F: FnOnce(&str) -> Option<String>,
    {
        if let Some(value) = env(key)
            && !value.trim().is_empty()
        {
            return Some(value);
        }
        // Same rule for the file: set-but-empty is not a credential.
        self.env
            .get(key)
            .filter(|value| !value.trim().is_empty())
            .cloned()
    }

    /// Whether the configuration is empty in every respect.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.table.is_empty() && self.env.is_empty()
    }
}

/// Reads `KEY=VALUE` lines, ignoring blanks and `#` comments.
///
/// Deliberately not a full dotenv parser: no interpolation, no `export` prefix, no multiline values.
/// A partial implementation that says so is safer than one that silently mis-reads the exotic syntax a
/// real parser supports.
fn read_env_file(path: &Path) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return vars;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        vars.insert(key.to_string(), value.to_string());
    }
    vars
}

/// The `.env` file's own path, for error messages that name it.
impl SuiteConfig {
    /// The dotenv path this configuration consulted.
    #[must_use]
    pub fn env_path(&self) -> PathBuf {
        self.paths.env_file()
    }
}
