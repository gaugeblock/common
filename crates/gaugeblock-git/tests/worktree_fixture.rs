//! Worktree fixtures: real repositories, real `git worktree` calls.
//!
//! A mock would prove nothing here. The behaviour being pinned is git's own bookkeeping - what appears
//! in the list after an add, what a refusal looks like for a dirty tree, what pruning does to an entry
//! whose directory is already gone - so the fixture creates genuine repositories in the temp directory.

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

/// A throwaway repository with one commit on `trunk`, plus a scratch directory for worktrees.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let base = std::env::temp_dir().join(format!("gb-wt-{tag}-{}", std::process::id()));
        let dir = base.join("repo");
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

    fn worktree_path(&self, name: &str) -> PathBuf {
        self.dir.parent().unwrap().join(name)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(base) = self.dir.parent() {
            std::fs::remove_dir_all(base).ok();
        }
    }
}

#[test]
fn test_a_new_repository_lists_only_its_main_checkout() {
    let fixture = Fixture::new("list-one");
    let repo = Repository::discover(&fixture.dir).unwrap();

    let worktrees = repo.worktrees().unwrap();
    assert_eq!(worktrees.len(), 1, "got {worktrees:?}");
    assert_eq!(worktrees[0].branch.as_deref(), Some("trunk"));
    assert!(!worktrees[0].detached);
}

#[test]
fn test_add_then_list_reports_the_new_tree_and_its_branch() {
    let fixture = Fixture::new("add");
    let repo = Repository::discover(&fixture.dir).unwrap();
    let path = fixture.worktree_path("agent-1");

    repo.add_worktree(&path, "agent/one", true).unwrap();

    let worktrees = repo.worktrees().unwrap();
    assert_eq!(worktrees.len(), 2, "got {worktrees:?}");
    let added = worktrees
        .iter()
        .find(|tree| tree.branch.as_deref() == Some("agent/one"))
        .expect("the new worktree is listed");
    assert_eq!(
        added.path.canonicalize().unwrap(),
        path.canonicalize().unwrap()
    );
    assert_eq!(added.label(), "agent/one");
    assert!(added.head.is_some(), "a fresh worktree has a HEAD");
}

#[test]
fn test_remove_takes_the_tree_out_of_the_list() {
    let fixture = Fixture::new("remove");
    let repo = Repository::discover(&fixture.dir).unwrap();
    let path = fixture.worktree_path("agent-2");
    repo.add_worktree(&path, "agent/two", true).unwrap();

    repo.remove_worktree(&path, false).unwrap();

    let worktrees = repo.worktrees().unwrap();
    assert_eq!(worktrees.len(), 1, "got {worktrees:?}");
    assert!(
        !worktrees
            .iter()
            .any(|tree| tree.branch.as_deref() == Some("agent/two"))
    );
}

#[test]
fn test_a_dirty_worktree_is_refused_without_force() {
    let fixture = Fixture::new("dirty");
    let repo = Repository::discover(&fixture.dir).unwrap();
    let path = fixture.worktree_path("agent-3");
    repo.add_worktree(&path, "agent/three", true).unwrap();
    std::fs::write(path.join("uncommitted.txt"), "work in progress\n").unwrap();

    let refused = repo.remove_worktree(&path, false);
    assert!(
        refused.is_err(),
        "an agent pool must not discard uncommitted work silently"
    );
    assert!(
        repo.worktrees().unwrap().len() == 2,
        "the tree is still there"
    );

    repo.remove_worktree(&path, true).unwrap();
    assert_eq!(repo.worktrees().unwrap().len(), 1, "force removes it");
}

#[test]
fn test_prune_forgets_a_worktree_whose_directory_vanished() {
    let fixture = Fixture::new("prune");
    let repo = Repository::discover(&fixture.dir).unwrap();
    let path = fixture.worktree_path("agent-4");
    repo.add_worktree(&path, "agent/four", true).unwrap();

    // What a crashed agent leaves behind: the directory is gone, the metadata is not.
    std::fs::remove_dir_all(&path).unwrap();
    repo.prune_worktrees().unwrap();

    let worktrees = repo.worktrees().unwrap();
    assert_eq!(
        worktrees.len(),
        1,
        "prune forgets the missing tree: {worktrees:?}"
    );
}
#[test]
fn test_adding_over_a_non_empty_directory_reports_gits_reason() {
    // An *empty* directory is fine to add into - git does it happily - so the fixture has to be
    // non-empty for this to be a refusal at all. Discovering that was the first version of this test
    // failing on an `Ok`, which is the useful kind of failure.
    let fixture = Fixture::new("conflict");
    let repo = Repository::discover(&fixture.dir).unwrap();
    let path = fixture.worktree_path("occupied");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("in-the-way.txt"), "x\n").unwrap();

    let error = repo
        .add_worktree(&path, "agent/conflict", true)
        .unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("not an empty directory") || message.contains("already"),
        "gits stderr must survive into the error, got: {message}"
    );
}

#[test]
fn test_a_branch_already_checked_out_cannot_be_added_twice() {
    let fixture = Fixture::new("dup");
    let repo = Repository::discover(&fixture.dir).unwrap();
    let first = fixture.worktree_path("agent-a");
    repo.add_worktree(&first, "agent/shared", true).unwrap();

    let second = fixture.worktree_path("agent-b");
    let error = repo
        .add_worktree(&second, "agent/shared", false)
        .unwrap_err();
    let message = error.to_string();
    // git words this "is already used by worktree at ...", which is also what makes the stderr worth
    // carrying: the message names the tree that holds the branch.
    assert!(
        message.contains("already used by worktree") || message.contains("already checked out"),
        "expected gits refusal, got: {message}"
    );
    assert!(
        message.contains("agent-a"),
        "the refusal should name the worktree holding the branch, got: {message}"
    );
}

#[test]
fn test_history_ranges_and_merge_bases_over_real_commits() {
    let fixture = Fixture::new("history");
    let repo = Repository::discover(&fixture.dir).unwrap();
    for (file, message) in [("b.txt", "second"), ("c.txt", "third")] {
        std::fs::write(fixture.dir.join(file), "x\n").unwrap();
        git(&fixture.dir, &["add", file]);
        git(&fixture.dir, &["commit", "-m", message]);
    }

    // A branch fork, so merge_base has something real to find.
    git(&fixture.dir, &["checkout", "-b", "side"]);
    std::fs::write(fixture.dir.join("d.txt"), "x\n").unwrap();
    git(&fixture.dir, &["add", "d.txt"]);
    git(&fixture.dir, &["commit", "-m", "side work"]);
    git(&fixture.dir, &["checkout", "trunk"]);

    let recent = repo.commits_between("trunk~2..trunk").unwrap();
    assert_eq!(recent.len(), 2, "got {recent:?}");
    assert_eq!(recent[0].summary, "third", "rev-list order is newest first");
    assert_eq!(recent[1].summary, "second");

    let base = repo.merge_base("trunk", "side").unwrap();
    // `trunk~0..trunk` is the empty range - `~0` is the tip itself - so this asks for the tip with `~1`.
    let trunk_head = repo.commits_between("trunk~1..trunk").unwrap();
    // `side` branched after the third commit, so the fork point *is* trunk's tip. Asserting otherwise
    // was my error, not git's - which is the useful kind of test to have written.
    assert_eq!(base, trunk_head[0].sha, "the fork point is trunk's tip");
    assert!(repo.is_ancestor(&base, "side").unwrap());
    assert!(
        !repo.is_ancestor("side", "trunk").unwrap(),
        "side is not an ancestor of trunk"
    );
}
