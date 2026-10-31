//! Worktree lifecycle: adding, listing, removing and pruning working trees.
//!
//! This is the half of `GB-131` that other tickets are waiting on. `gaugeblock-workspace` resolves
//! `.gaugeblock/` from the common dir, and a worktree is precisely the case where the working tree and
//! the common dir differ - so the workspace crate cannot be finished before this exists (`GB-189`).
//! `GB-169` and `GB-170` need `add`/`remove`/`prune` to isolate agents.
//!
//! Everything is parsed from `--porcelain` output rather than human output, and every path stays as git
//! reported it: a worktree list is data, and reformatting it would quietly change what callers key on.

use super::error::GitError;
use super::repo::{Repository, run_git};
use std::path::{Path, PathBuf};

/// One working tree of a repository, as git reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// Absolute path of the working tree.
    pub path: PathBuf,
    /// `HEAD` commit, if the worktree has one.
    pub head: Option<String>,
    /// Checked-out branch, without the `refs/heads/` prefix.
    pub branch: Option<String>,
    /// True when `HEAD` is detached.
    pub detached: bool,
    /// True for the bare entry of a bare repository.
    pub bare: bool,
    /// True when the worktree is locked (a portable disk, an agent lease, a manual lock).
    pub locked: bool,
}

impl WorktreeInfo {
    /// A readable label: the branch if there is one, else the directory name.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.branch {
            Some(branch) => branch.clone(),
            None => self.path.file_name().map_or_else(
                || self.path.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            ),
        }
    }
}

impl Repository {
    /// Every working tree of this repository, main checkout first as git orders them.
    ///
    /// # Errors
    ///
    /// [`GitError`] when `git worktree list` fails.
    pub fn worktrees(&self) -> Result<Vec<WorktreeInfo>, GitError> {
        let output = run_git(self.root(), &["worktree", "list", "--porcelain"])?;
        Ok(parse_worktree_list(&output))
    }

    /// Adds a working tree at `path`, optionally creating `branch` first.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when git refuses - an existing directory, a branch already checked out, or
    /// a path outside the repository. The stderr is carried, which is what tells those apart.
    pub fn add_worktree(
        &self,
        path: &Path,
        branch: &str,
        create_branch: bool,
    ) -> Result<(), GitError> {
        let path = path.to_string_lossy().into_owned();
        if create_branch {
            run_git(self.root(), &["worktree", "add", "-b", branch, &path])?;
        } else {
            run_git(self.root(), &["worktree", "add", &path, branch])?;
        }
        Ok(())
    }

    /// Adds a detached working tree at `path` on `rev`.
    ///
    /// Detached because a sandbox is not a branch: an agent that verified code at a commit must not
    /// create a named branch for the attempt, or a failed run leaves branches behind.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the path is occupied or `rev` is unknown.
    pub fn add_worktree_detached(&self, path: &Path, rev: &str) -> Result<(), GitError> {
        run_git(
            self.root(),
            &["worktree", "add", "--detach", &path.to_string_lossy(), rev],
        )?;
        Ok(())
    }

    /// Removes the working tree at `path`.
    ///
    /// `force` is required when the tree is dirty; refusing by default is deliberate, because an agent
    /// pool that discards uncommitted work silently is worse than one that stops.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the worktree is dirty and `force` is false, or the path is not a
    /// worktree of this repository.
    pub fn remove_worktree(&self, path: &Path, force: bool) -> Result<(), GitError> {
        let path = path.to_string_lossy().into_owned();
        if force {
            run_git(self.root(), &["worktree", "remove", "--force", &path])?;
        } else {
            run_git(self.root(), &["worktree", "remove", &path])?;
        }
        Ok(())
    }

    /// Prunes administrative files for worktrees whose directories are gone.
    ///
    /// # Errors
    ///
    /// [`GitError`] when `git worktree prune` fails.
    pub fn prune_worktrees(&self) -> Result<(), GitError> {
        run_git(self.root(), &["worktree", "prune"])?;
        Ok(())
    }
}

/// Parses `git worktree list --porcelain`.
///
/// Blocks are separated by a blank line; each carries `worktree <path>` first and then optional
/// `HEAD`, `branch`, `detached`, `bare` and `locked` lines. Unknown keys are ignored on purpose: git
/// adds them over time, and a parser that refused them would break on an upgrade.
fn parse_worktree_list(output: &str) -> Vec<WorktreeInfo> {
    let mut worktrees = Vec::new();
    let mut current: Option<WorktreeInfo> = None;

    for line in output.lines() {
        if line.trim().is_empty() {
            if let Some(info) = current.take() {
                worktrees.push(info);
            }
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(info) = current.take() {
                worktrees.push(info);
            }
            current = Some(WorktreeInfo {
                path: PathBuf::from(path),
                head: None,
                branch: None,
                detached: false,
                bare: false,
                locked: false,
            });
            continue;
        }
        let Some(info) = current.as_mut() else {
            continue;
        };
        if let Some(head) = line.strip_prefix("HEAD ") {
            info.head = Some(head.to_string());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            info.branch = Some(branch.trim_start_matches("refs/heads/").to_string());
        } else if line == "detached" {
            info.detached = true;
        } else if line == "bare" {
            info.bare = true;
        } else if line.starts_with("locked") {
            info.locked = true;
        }
    }
    if let Some(info) = current {
        worktrees.push(info);
    }
    worktrees
}
