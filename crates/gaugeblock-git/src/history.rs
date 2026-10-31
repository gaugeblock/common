//! Commit history: ranges, merge bases and ancestry.
//!
//! The range arithmetic that `GB-230` needs for mining Gold pairs from real history: a fix commit and
//! the one before it are a (wrong, right) pair, which is more honest than any fixture I could invent.
//! Parsing is line-based over a `%x1f` separator so a summary containing pipes cannot break it.

use super::error::GitError;
use super::repo::{Repository, run_git};

/// One commit, reduced to what the suite needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    /// Full sha.
    pub sha: String,
    /// First line of the message.
    pub summary: String,
}

impl Repository {
    /// The best common ancestor of two revisions.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the revisions do not share history - unrelated repositories have no
    /// merge base, and that is a real answer rather than an empty one.
    pub fn merge_base(&self, left: &str, right: &str) -> Result<String, GitError> {
        run_git(self.root(), &["merge-base", left, right])
    }

    /// True when `ancestor` is reachable from `descendant`.
    ///
    /// # Errors
    ///
    /// [`GitError`] only when git cannot run; "not an ancestor" exits non-zero by design and is
    /// reported as `false` rather than as a failure.
    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, GitError> {
        match run_git(
            self.root(),
            &["merge-base", "--is-ancestor", ancestor, descendant],
        ) {
            Ok(_) => Ok(true),
            Err(GitError::Failed { code: 1, .. }) => Ok(false),
            Err(other) => Err(other),
        }
    }

    /// Commits in `range` (any `rev-list` range, e.g. `trunk~2..trunk`), newest first.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the range is not valid.
    pub fn commits_between(&self, range: &str) -> Result<Vec<Commit>, GitError> {
        let output = run_git(
            self.root(),
            &[
                "rev-list",
                "--pretty=format:%H\x1f%s",
                "--no-commit-header",
                range,
            ],
        )?;
        Ok(parse_commits(&output))
    }
}

/// Parses `%H\x1f%s` lines into commits, skipping the blank separators `rev-list` emits.
/// How far a branch is ahead of and behind its upstream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AheadBehind {
    /// Commits the local branch has that the upstream does not.
    pub ahead: usize,
    /// Commits the upstream has that the local branch does not.
    pub behind: usize,
}

impl AheadBehind {
    /// True when the local branch is level with its upstream.
    #[must_use]
    pub fn is_level(&self) -> bool {
        self.ahead == 0 && self.behind == 0
    }
}

impl Repository {
    /// Nearest tag reachable from `HEAD`, if any.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when the repository has no tags - which is the answer rather than a
    /// fault, so a caller that treats "no release yet" as normal matches on the error.
    pub fn describe_tags(&self) -> Result<String, GitError> {
        run_git(self.root(), &["describe", "--tags", "--abbrev=0"])
    }

    /// Commits `branch` is ahead of and behind `upstream`.
    ///
    /// # Errors
    ///
    /// [`GitError::Failed`] when either ref is unknown, which is how a missing upstream surfaces.
    pub fn ahead_behind(&self, upstream: &str, branch: &str) -> Result<AheadBehind, GitError> {
        let range = format!("{upstream}...{branch}");
        let output = run_git(
            self.root(),
            &["rev-list", "--left-right", "--count", &range],
        )?;
        parse_ahead_behind(&output).ok_or_else(|| GitError::Failed {
            command: "rev-list --left-right --count".to_string(),
            code: 0,
            stderr: format!("unexpected counts: {output}"),
        })
    }
}

/// Parses the two numbers `rev-list --left-right --count` prints: behind then ahead.
fn parse_ahead_behind(output: &str) -> Option<AheadBehind> {
    let mut parts = output.split_whitespace();
    let behind = parts.next()?.parse().ok()?;
    let ahead = parts.next()?.parse().ok()?;
    Some(AheadBehind { ahead, behind })
}

fn parse_commits(output: &str) -> Vec<Commit> {
    output
        .lines()
        .filter_map(|line| {
            let (sha, summary) = line.split_once('\x1f')?;
            Some(Commit {
                sha: sha.trim().to_string(),
                summary: summary.trim().to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::parse_commits;

    #[test]
    fn test_unit_separated_lines_parse_into_commits() {
        let output = "abc123\x1ffix the parser\n\ndef456\x1fsecond commit\n";
        let commits = parse_commits(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].sha, "abc123");
        assert_eq!(commits[0].summary, "fix the parser");
        assert_eq!(commits[1].summary, "second commit");
    }

    #[test]
    fn test_a_summary_containing_pipes_or_colons_survives() {
        // The reason for a unit separator rather than a colon or a pipe.
        let output = "abc\x1fcmd: fix a | b\n";
        let commits = parse_commits(output);
        assert_eq!(commits[0].summary, "cmd: fix a | b");
    }

    #[test]
    fn test_empty_output_is_no_commits_rather_than_an_error() {
        assert!(parse_commits("").is_empty());
        assert!(parse_commits("\n\n").is_empty());
    }
}
