//! Which repositories the suite contains, and where they live.
//!
//! Discovery answers "what is on disk here" and the expected list answers "what should be". They are
//! deliberately separate: a member that is missing from disk is a fact worth reporting, and a
//! directory that looks like a member but is not in the list is a fact worth reporting too. Merging
//! them would hide whichever one the caller needed.

use std::path::{Path, PathBuf};

/// One repository in the suite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberRepo {
    /// Directory name, which is also the member name (`miter`, `storypole`, ...).
    pub name: String,
    /// Root of the checkout.
    pub path: PathBuf,
}

impl MemberRepo {
    /// The member's `.gaugeblock` subdirectory, e.g. `.gaugeblock/storypole`.
    #[must_use]
    pub fn gaugeblock_dir(&self) -> PathBuf {
        self.path.join(".gaugeblock").join(&self.name)
    }

    /// The member's own dot directory (`.storypole`, `.miter`, ...) if it uses one.
    #[must_use]
    pub fn dot_dir(&self, tool: &str) -> PathBuf {
        self.path.join(format!(".{tool}"))
    }
}

/// The suite's member list: what is present, and what is expected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceTopology {
    suite_root: PathBuf,
    members: Vec<MemberRepo>,
}

impl WorkspaceTopology {
    /// Discovers members by looking for directories that are git repositories.
    ///
    /// A directory qualifies when it contains `.git` as a directory or a file - the file form is a
    /// linked worktree, and skipping it would make a worktree-based workspace look empty.
    #[must_use]
    pub fn discover(suite_root: &Path) -> Self {
        Self::discover_with(suite_root, |path| path.join(".git").exists())
    }

    /// Discovers members by a caller-supplied qualification rule.
    ///
    /// `.git` is the right rule for this suite and the wrong rule for a consumer whose workspaces are
    /// plain directories: `storypole` accepts a member that holds `.storypole` or `Cargo.toml` even
    /// when it is not a checkout, and narrowing that would break its standalone use. The rule is a
    /// parameter so the shared model - the suite root, the sorted member list, the expected-versus-
    /// present distinction - is reused without imposing one product's discovery policy on another
    /// (`GB-189`).
    ///
    /// The predicate receives each immediate child directory that is not hidden; hidden directories
    /// are skipped because every dot directory in a suite root is tool state, never a member.
    #[must_use]
    pub fn discover_with(suite_root: &Path, qualifies: impl Fn(&Path) -> bool) -> Self {
        let mut members = Vec::new();
        if let Ok(entries) = std::fs::read_dir(suite_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.starts_with('.') || !qualifies(&path) {
                    continue;
                }
                members.push(MemberRepo {
                    name: name.to_string(),
                    path,
                });
            }
        }
        members.sort_by(|a, b| a.name.cmp(&b.name));
        Self {
            suite_root: suite_root.to_path_buf(),
            members,
        }
    }

    /// Builds a topology from an explicit member list, for a caller that keeps its own configuration.
    #[must_use]
    pub fn from_members(suite_root: &Path, members: Vec<MemberRepo>) -> Self {
        Self {
            suite_root: suite_root.to_path_buf(),
            members,
        }
    }

    /// The root every member lives under.
    #[must_use]
    pub fn suite_root(&self) -> &Path {
        &self.suite_root
    }

    /// Members present on disk, sorted by name.
    #[must_use]
    pub fn members(&self) -> &[MemberRepo] {
        &self.members
    }

    /// Looks a member up by name.
    #[must_use]
    pub fn member(&self, name: &str) -> Option<&MemberRepo> {
        self.members.iter().find(|member| member.name == name)
    }

    /// The suites a repository is expected to contain at this lifecycle.
    ///
    /// Kept as a constant rather than discovered so a missing checkout can be reported: an empty
    /// directory tree discovering zero members looks identical to a healthy workspace with nothing to
    /// do, and only the expected list can tell them apart.
    #[must_use]
    pub fn expected_members() -> &'static [&'static str] {
        &[
            "batten",
            "benchwork",
            "chalkline",
            "common",
            "dado",
            "joiner",
            "miter",
            "outfeed",
            "rabbet",
            "sawhorse",
            "storypole",
        ]
    }

    /// Expected members that are not present on disk, sorted.
    #[must_use]
    pub fn missing_members(&self) -> Vec<&'static str> {
        Self::expected_members()
            .iter()
            .copied()
            .filter(|name| self.member(name).is_none())
            .collect()
    }
}
