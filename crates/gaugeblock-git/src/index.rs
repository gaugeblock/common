//! The index: staging a named set of paths and committing them.
//!
//! Scoped staging exists because `git add .` is how a commit acquires files nobody meant to include -
//! `GB-150` recorded a whole `git add` aborting silently, and `GB-180` needs Panel 1 to commit docs it
//! named rather than whatever happens to be dirty. This module refuses to be a wrapper around
//! "stage everything": an empty path list is an error, not a shortcut.

use super::error::GitError;
use super::repo::{Repository, run_git};
use std::path::Path;

/// What a commit produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitSummary {
    /// Full sha of the new commit.
    pub sha: String,
    /// First line of its message.
    pub summary: String,
}

impl Repository {
    /// Stages exactly `paths`, returning how many paths were given.
    ///
    /// # Errors
    ///
    /// [`GitError::EmptyScope`] when `paths` is empty - the caller asked for nothing specific, which
    /// is the behaviour this function exists to prevent - and [`GitError::Failed`] when git rejects a
    /// path, carrying stderr.
    pub fn stage_paths(&self, paths: &[&Path]) -> Result<usize, GitError> {
        if paths.is_empty() {
            return Err(GitError::EmptyScope);
        }
        let mut args: Vec<String> = vec!["add".to_string(), "--".to_string()];
        args.extend(paths.iter().map(|path| path.to_string_lossy().into_owned()));
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        run_git(self.root(), &borrowed)?;
        Ok(paths.len())
    }

    /// True when something is staged and waiting for a commit.
    ///
    /// `git diff --cached --quiet` exits 1 to mean "differences exist", which is a fact rather than a
    /// failure, so that code is mapped to `true` instead of surfacing as an error.
    ///
    /// # Errors
    ///
    /// [`GitError`] only when git cannot run at all.
    pub fn has_staged_changes(&self) -> Result<bool, GitError> {
        match run_git(self.root(), &["diff", "--cached", "--quiet"]) {
            Ok(_) => Ok(false),
            Err(GitError::Failed { code: 1, .. }) => Ok(true),
            Err(other) => Err(other),
        }
    }

    /// Commits what is staged, returning the new commit.
    ///
    /// The message is passed as a single `-m` argument, so a multi-line message stays one message
    /// instead of becoming a sequence of `-m` flags that git concatenates with blank lines.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when there is nothing to commit or a hook rejects it.
    pub fn commit(&self, message: &str) -> Result<CommitSummary, GitError> {
        run_git(self.root(), &["commit", "-m", message])?;
        let line = run_git(self.root(), &["log", "-1", "--pretty=format:%H\x1f%s"])?;
        let (sha, summary) = line.split_once('\x1f').ok_or_else(|| GitError::Failed {
            command: "log -1".to_string(),
            code: 0,
            stderr: format!("unexpected log output: {line}"),
        })?;
        Ok(CommitSummary {
            sha: sha.trim().to_string(),
            summary: summary.trim().to_string(),
        })
    }
}

/// Whether `path` is tracked by the index.
///
/// # Errors
///
/// [`GitError`] when `git ls-files` fails.
pub fn is_tracked(repo: &Repository, path: &Path) -> Result<bool, GitError> {
    let output = run_git(
        repo.root(),
        &["ls-files", "--error-unmatch", &path.to_string_lossy()],
    );
    match output {
        Ok(_) => Ok(true),
        Err(GitError::Failed { code: 1, .. }) => Ok(false),
        Err(other) => Err(other),
    }
}
