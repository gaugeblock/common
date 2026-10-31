//! Fixture tests against real repositories: one main checkout and one linked worktree.
//!
//! A fake would prove nothing here. The behaviour being pinned is git's own - that a linked worktree
//! reports the same common dir as its parent, and that branch/sha/dirty answers match reality - so the
//! fixture creates genuine repositories in the temp directory and removes them afterwards.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_git::Repository;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string()
}

/// A throwaway repository with one commit on `trunk`.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("gb-git-{tag}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-b", "trunk"]);
        git(&dir, &["config", "user.email", "fixture@gaugeblock.dev"]);
        git(&dir, &["config", "user.name", "Fixture"]);
        git(&dir, &["config", "commit.gpgsign", "false"]);
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        git(&dir, &["add", "a.txt"]);
        git(&dir, &["commit", "-m", "one"]);
        Self { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

#[test]
fn test_discover_resolves_the_root_from_a_subdirectory() {
    let fixture = Fixture::new("discover");
    let nested = fixture.path().join("src/deep");
    std::fs::create_dir_all(&nested).unwrap();

    let repo = Repository::discover(&nested).unwrap();
    assert_eq!(
        repo.root().canonicalize().unwrap(),
        fixture.path().canonicalize().unwrap(),
        "a nested path resolves to the checkout root"
    );
}

#[test]
fn test_branch_sha_and_dirty_state() {
    let fixture = Fixture::new("state");
    let repo = Repository::discover(fixture.path()).unwrap();

    assert_eq!(repo.current_branch().unwrap().as_deref(), Some("trunk"));
    assert_eq!(
        repo.head_sha().unwrap().len(),
        40,
        "full sha, not abbreviated"
    );
    assert!(!repo.is_dirty().unwrap(), "fresh commit is clean");

    std::fs::write(fixture.path().join("untracked.txt"), "x\n").unwrap();
    assert!(
        repo.is_dirty().unwrap(),
        "an untracked file counts as dirty"
    );
}

#[test]
fn test_detached_head_has_no_branch() {
    let fixture = Fixture::new("detached");
    let repo = Repository::discover(fixture.path()).unwrap();
    git(fixture.path(), &["checkout", "--detach"]);

    assert_eq!(
        repo.current_branch().unwrap(),
        None,
        "detached HEAD is reported as no branch rather than the literal `HEAD`"
    );
}

#[test]
fn test_a_linked_worktree_shares_one_common_dir() {
    let fixture = Fixture::new("worktree");
    let main = Repository::discover(fixture.path()).unwrap();
    let linked = fixture
        .path()
        .join("..")
        .join(format!("gb-git-wt-{}", std::process::id()));
    std::fs::remove_dir_all(&linked).ok();
    git(
        fixture.path(),
        &["worktree", "add", linked.to_str().unwrap(), "-b", "side"],
    );

    let worktree = Repository::discover(&linked).unwrap();
    assert!(
        !main.is_worktree().unwrap(),
        "the main checkout is not a worktree"
    );
    assert!(
        worktree.is_worktree().unwrap(),
        "a linked checkout reports itself as a worktree"
    );
    assert_eq!(
        main.common_dir().unwrap().canonicalize().unwrap(),
        worktree.common_dir().unwrap().canonicalize().unwrap(),
        "both checkouts resolve to the same common dir, which is what .gaugeblock discovery keys on"
    );
    assert_eq!(worktree.current_branch().unwrap().as_deref(), Some("side"));

    std::fs::remove_dir_all(&linked).ok();
}

#[test]
fn test_an_unborn_head_reports_its_branch() {
    // A repository that has never been committed to has a branch but no HEAD commit. `rev-parse`
    // refuses; `symbolic-ref` answers. Getting this wrong made a release check report a git failure
    // where the real answer was "you are on the wrong branch".
    let dir = std::env::temp_dir().join(format!("gb-git-unborn-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-b", "feat/unreleased"]);

    let repo = Repository::discover(&dir).unwrap();
    assert_eq!(
        repo.current_branch().unwrap().as_deref(),
        Some("feat/unreleased")
    );
    assert!(
        repo.head_sha().is_err(),
        "there is no commit to resolve, and saying so is correct"
    );

    std::fs::remove_dir_all(&dir).ok();
}
