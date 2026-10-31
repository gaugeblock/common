//! Repository identity: where a checkout lives and what state it is in.
//!
//! Every answer comes from asking git, never from walking parent directories or string-matching
//! `.git` paths. Two callers that disagree about whether a directory is a worktree have caused real
//! bugs, and `rev-parse` is the only authority that can settle it.

use super::error::GitError;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs `git <args>` in `dir` and returns stdout with the trailing newline removed.
///
/// # Errors
///
/// [`GitError::Spawn`] when git cannot start, [`GitError::Failed`] carrying stderr when it exits
/// non-zero, and [`GitError::NonUtf8`] when the output cannot be read as text.
pub fn run_git(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|source| GitError::Spawn { source })?;

    let command = args.join(" ");
    if !output.status.success() {
        return Err(GitError::Failed {
            command,
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    String::from_utf8(output.stdout)
        .map(|text| text.trim_end().to_string())
        .map_err(|_| GitError::NonUtf8 { command })
}

/// A checkout, resolved by asking git rather than by walking parents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    root: PathBuf,
}

impl Repository {
    /// Discovers the checkout containing `path`.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when `path` is not inside a repository, which is how callers detect "not a
    /// repository" without a separate probe.
    pub fn discover(path: &Path) -> Result<Self, GitError> {
        let root = run_git(path, &["rev-parse", "--show-toplevel"])?;
        Ok(Self {
            root: PathBuf::from(root),
        })
    }

    /// Clones `url` into `target` and opens it.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the clone fails - an unreachable remote, an occupied target - and
    /// [`GitError::Spawn`] when git is missing.
    pub fn clone(url: &str, target: &Path) -> Result<Self, GitError> {
        run_git(Path::new("."), &["clone", url, &target.to_string_lossy()])?;
        Self::discover(target)
    }

    /// Root of the working tree.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The shared `.git` directory, identical for every worktree of one repository.
    ///
    /// # Errors
    ///
    /// [`GitError`] when `git rev-parse --git-common-dir` fails.
    pub fn common_dir(&self) -> Result<PathBuf, GitError> {
        let raw = run_git(&self.root, &["rev-parse", "--git-common-dir"])?;
        Ok(self.absolutise(&raw))
    }

    /// True when this checkout is a linked worktree rather than the main one.
    ///
    /// # Errors
    ///
    /// [`GitError`] when either `rev-parse` probe fails.
    pub fn is_worktree(&self) -> Result<bool, GitError> {
        let git_dir = run_git(&self.root, &["rev-parse", "--git-dir"])?;
        Ok(self.absolutise(&git_dir) != self.common_dir()?)
    }

    /// Full sha of `HEAD`.
    ///
    /// # Errors
    ///
    /// [`GitError`] when `HEAD` cannot be resolved, which includes a repository with no commits.
    pub fn head_sha(&self) -> Result<String, GitError> {
        run_git(&self.root, &["rev-parse", "HEAD"])
    }

    /// Current branch, or `None` when `HEAD` is detached.
    ///
    /// `rev-parse --abbrev-ref HEAD` answers the literal `HEAD` in a detached state, so callers that
    /// switched on the string had to special-case it in six places; the type carries it instead.
    ///
    /// # Errors
    ///
    /// [`GitError`] when `rev-parse` fails.
    pub fn current_branch(&self) -> Result<Option<String>, GitError> {
        match run_git(&self.root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
            Ok(name) if name != "HEAD" => Ok(Some(name)),
            // A detached head reports the literal `HEAD`, and an unborn one - a repository that has
            // never been committed to - makes `rev-parse` fail outright. Both are answered by asking
            // what HEAD points at, which is how a freshly `git init -b main` repository reports main.
            // Without this a release check reported a git failure where the real answer was "you are
            // on the wrong branch".
            Ok(_) | Err(_) => match run_git(&self.root, &["symbolic-ref", "--short", "HEAD"]) {
                Ok(name) if !name.is_empty() => Ok(Some(name)),
                Ok(_) => Ok(None),
                Err(GitError::Failed { .. }) => Ok(None),
                Err(other) => Err(other),
            },
        }
    }

    /// True when the working tree or index differs from `HEAD`, untracked files included.
    ///
    /// # Errors
    ///
    /// [`GitError`] when `git status` fails.
    pub fn is_dirty(&self) -> Result<bool, GitError> {
        let porcelain = run_git(&self.root, &["status", "--porcelain"])?;
        Ok(!porcelain.is_empty())
    }

    /// Resolves a git-reported path against the checkout root.
    ///
    /// `rev-parse` answers `--git-common-dir` and `--git-dir` relative to the working directory it ran
    /// in, so an unanchored comparison compares `.git` against `/repo/.git` and calls every worktree
    /// dirty. Anchoring once, here, is why that class of bug cannot come back.
    fn absolutise(&self, raw: &str) -> PathBuf {
        let path = Path::new(raw);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }
}
