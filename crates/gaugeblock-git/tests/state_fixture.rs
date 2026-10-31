//! State and history fixtures: tags, ahead/behind, checkout, restore, pull, clone.

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

struct Fixture {
    base: PathBuf,
    repo_dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let base = std::env::temp_dir().join(format!("gb-state-{tag}-{}", std::process::id()));
        let repo_dir = base.join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        git(&repo_dir, &["init", "-b", "trunk"]);
        git(
            &repo_dir,
            &["config", "user.email", "fixture@gaugeblock.dev"],
        );
        git(&repo_dir, &["config", "user.name", "Fixture"]);
        git(&repo_dir, &["config", "commit.gpgsign", "false"]);
        std::fs::write(repo_dir.join("a.txt"), "one\n").unwrap();
        git(&repo_dir, &["add", "a.txt"]);
        git(&repo_dir, &["commit", "-m", "one"]);
        Self { base, repo_dir }
    }

    fn repo(&self) -> Repository {
        Repository::discover(&self.repo_dir).unwrap()
    }

    fn commit_file(&self, name: &str, message: &str) {
        std::fs::write(self.repo_dir.join(name), "x\n").unwrap();
        git(&self.repo_dir, &["add", name]);
        git(&self.repo_dir, &["commit", "-m", message]);
    }

    fn bare_remote(&self) -> PathBuf {
        let bare = self.base.join("origin.git");
        std::fs::create_dir_all(&bare).unwrap();
        git(&bare, &["init", "--bare", "-b", "trunk"]);
        git(
            &self.repo_dir,
            &["remote", "add", "origin", bare.to_str().unwrap()],
        );
        git(&self.repo_dir, &["push", "-u", "origin", "trunk"]);
        bare
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.base).ok();
    }
}

#[test]
fn test_describe_reports_the_nearest_tag_and_errors_when_there_is_none() {
    let fixture = Fixture::new("describe");
    let repo = fixture.repo();
    assert!(
        repo.describe_tags().is_err(),
        "no tags yet is an error, not an empty string"
    );

    repo.tag_annotated("v0.1.0", "first").unwrap();
    fixture.commit_file("b.txt", "two");
    repo.tag_annotated("v0.2.0", "second").unwrap();

    assert_eq!(
        repo.describe_tags().unwrap(),
        "v0.2.0",
        "nearest reachable tag"
    );
}

#[test]
fn test_ahead_behind_counts_in_the_right_directions() {
    let fixture = Fixture::new("ahead");
    let repo = fixture.repo();
    fixture.bare_remote();

    // Level after the initial push.
    assert!(
        repo.ahead_behind("origin/trunk", "trunk")
            .unwrap()
            .is_level()
    );

    fixture.commit_file("b.txt", "two");
    let counts = repo.ahead_behind("origin/trunk", "trunk").unwrap();
    assert_eq!(counts.ahead, 1, "one commit the upstream lacks");
    assert_eq!(counts.behind, 0);
}

#[test]
fn test_checkout_moves_head_to_a_named_commit() {
    let fixture = Fixture::new("checkout");
    let repo = fixture.repo();
    let first = repo.head_sha().unwrap();
    fixture.commit_file("b.txt", "two");

    repo.checkout(&first).unwrap();

    assert_eq!(repo.head_sha().unwrap(), first);
    assert_eq!(
        repo.current_branch().unwrap(),
        None,
        "checking out a sha detaches HEAD"
    );
}

#[test]
fn test_restore_discards_local_edits_to_a_tracked_path() {
    let fixture = Fixture::new("restore");
    let repo = fixture.repo();
    std::fs::write(fixture.repo_dir.join("a.txt"), "mangled\n").unwrap();
    assert!(repo.is_dirty().unwrap());

    repo.restore_path(Path::new("a.txt")).unwrap();

    assert!(
        !repo.is_dirty().unwrap(),
        "the file is back to its committed content"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.repo_dir.join("a.txt")).unwrap(),
        "one\n"
    );
}

#[test]
fn test_restore_of_an_untracked_path_is_refused() {
    let fixture = Fixture::new("restore-missing");
    let repo = fixture.repo();
    // There is nothing in HEAD to restore, so succeeding here would hide a caller naming the
    // wrong file.
    assert!(repo.restore_path(Path::new("never-existed.txt")).is_err());
}

#[test]
fn test_pull_ff_only_advances_a_clone_and_refuses_divergence() {
    let fixture = Fixture::new("pull");
    let bare = fixture.bare_remote();
    let clone_dir = fixture.base.join("clone");
    let clone = Repository::clone(bare.to_str().unwrap(), &clone_dir).unwrap();

    // A new commit upstream fast-forwards the clone.
    fixture.commit_file("c.txt", "three");
    git(&fixture.repo_dir, &["push", "origin", "trunk"]);
    clone.pull_ff_only("origin", "trunk").unwrap();
    assert_eq!(
        clone.head_sha().unwrap(),
        fixture.repo().head_sha().unwrap(),
        "the clone caught up"
    );

    // A local commit on the clone makes the histories diverge, and --ff-only must refuse.
    std::fs::write(clone_dir.join("local.txt"), "x\n").unwrap();
    git(&clone_dir, &["add", "local.txt"]);
    git(&clone_dir, &["commit", "-m", "local"]);
    fixture.commit_file("d.txt", "four");
    git(&fixture.repo_dir, &["push", "origin", "trunk"]);

    let error = clone.pull_ff_only("origin", "trunk").unwrap_err();
    assert!(
        error.to_string().contains("diverge") || error.to_string().contains("fast-forward"),
        "a release tool must not invent a merge commit: {error}"
    );
}

#[test]
fn test_push_branch_and_remotes_round_trip() {
    let fixture = Fixture::new("push-branch");
    let repo = fixture.repo();
    let bare = fixture.bare_remote();
    assert_eq!(repo.remotes().unwrap(), vec!["origin".to_string()]);

    fixture.commit_file("e.txt", "five");
    repo.push_branch("origin", "trunk").unwrap();

    let remote_head = Command::new("git")
        .args(["--git-dir", bare.to_str().unwrap(), "rev-parse", "trunk"])
        .output()
        .unwrap();
    let remote_head = String::from_utf8_lossy(&remote_head.stdout)
        .trim()
        .to_string();
    assert_eq!(
        remote_head,
        repo.head_sha().unwrap(),
        "the branch reached the remote"
    );
}
