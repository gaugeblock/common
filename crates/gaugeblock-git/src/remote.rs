//! Remotes: annotated tags and a push that lands the branch and its tag together.
//!
//! A release is two refs that must agree. Pushing them in one round trip with `--atomic` means either
//! both land or neither does, which is the difference between a tag pointing at a pushed commit and a
//! tag pointing at something the remote never received.

use super::error::GitError;
use super::repo::{Repository, run_git};

impl Repository {
    /// Creates an annotated tag at `HEAD`.
    ///
    /// Annotated rather than lightweight: a lightweight tag is a bare ref with no tagger, no date and no
    /// message, and `outfeed release` needs the message to carry the version bump.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the tag already exists or the message is empty.
    pub fn tag_annotated(&self, name: &str, message: &str) -> Result<(), GitError> {
        run_git(self.root(), &["tag", "-a", name, "-m", message])?;
        Ok(())
    }

    /// True when `name` exists locally as an annotated tag.
    ///
    /// # Errors
    ///
    /// [`GitError`] only when git cannot run.
    pub fn has_annotated_tag(&self, name: &str) -> Result<bool, GitError> {
        match run_git(
            self.root(),
            &["cat-file", "-t", &format!("refs/tags/{name}")],
        ) {
            Ok(kind) => Ok(kind.trim() == "tag"),
            Err(GitError::Failed { code: 128, .. }) => Ok(false),
            Err(other) => Err(other),
        }
    }

    /// Pushes `branch` and `refs/tags/<tag>` to `remote` in a single atomic round trip.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the push is rejected or the remote does not support `--atomic`; the
    /// stderr names which ref failed, which is the reason the error carries it.
    pub fn push_branch_and_tag(
        &self,
        remote: &str,
        branch: &str,
        tag: &str,
    ) -> Result<(), GitError> {
        let tag_ref = format!("refs/tags/{tag}");
        run_git(self.root(), &["push", "--atomic", remote, branch, &tag_ref])?;
        Ok(())
    }

    /// Fetches `refs/tags/<tag>` from `remote` into the local tag namespace, for verification.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the remote does not have that tag - which is exactly what a
    /// verification step should report rather than assume.
    pub fn fetch_tag(&self, remote: &str, tag: &str) -> Result<(), GitError> {
        let refspec = format!("refs/tags/{tag}:refs/tags/{tag}");
        run_git(self.root(), &["fetch", remote, &refspec])?;
        Ok(())
    }
}
