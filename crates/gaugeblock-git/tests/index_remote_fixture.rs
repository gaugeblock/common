//! Index and remote fixtures: scoped staging, typed commits, annotated tags, atomic push.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_git::{GitError, Repository};
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

fn git_dir(dir: &Path, args: &[&str]) -> String {
    let mut full = vec!["--git-dir".to_string(), dir.to_string_lossy().into_owned()];
    full.extend(args.iter().map(|arg| (*arg).to_string()));
    let borrowed: Vec<&str> = full.iter().map(String::as_str).collect();
    let output = Command::new("git").args(&borrowed).output().unwrap();
    assert!(output.status.success(), "git {args:?} failed");
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
        let base = std::env::temp_dir().join(format!("gb-idx-{tag}-{}", std::process::id()));
        let repo_dir = base.join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        git(&repo_dir, &["init", "-b", "trunk"]);
        git(
            &repo_dir,
            &["config", "user.email", "fixture@gaugeblock.dev"],
        );
        git(&repo_dir, &["config", "user.name", "Fixture"]);
        git(&repo_dir, &["config", "commit.gpgsign", "false"]);
        std::fs::write(repo_dir.join("base.txt"), "one\n").unwrap();
        git(&repo_dir, &["add", "base.txt"]);
        git(&repo_dir, &["commit", "-m", "one"]);
        Self { base, repo_dir }
    }

    fn repo(&self) -> Repository {
        Repository::discover(&self.repo_dir).unwrap()
    }

    fn bare_remote(&self) -> PathBuf {
        let bare = self.base.join("origin.git");
        std::fs::create_dir_all(&bare).unwrap();
        git(&bare, &["init", "--bare", "-b", "trunk"]);
        git(
            &self.repo_dir,
            &["remote", "add", "origin", bare.to_str().unwrap()],
        );
        bare
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.base).ok();
    }
}

#[test]
fn test_staging_is_scoped_to_the_named_paths() {
    let fixture = Fixture::new("scoped");
    let repo = fixture.repo();
    std::fs::write(fixture.repo_dir.join("wanted.txt"), "x\n").unwrap();
    std::fs::write(fixture.repo_dir.join("stray.txt"), "y\n").unwrap();

    repo.stage_paths(&[Path::new("wanted.txt")]).unwrap();
    let commit = repo.commit("add wanted").unwrap();

    // The commit must contain the named file and nothing else - the whole point of scoped staging.
    let files = git(
        &fixture.repo_dir,
        &["show", "--name-only", "--pretty=format:", &commit.sha],
    );
    assert!(files.contains("wanted.txt"), "got {files:?}");
    assert!(
        !files.contains("stray.txt"),
        "the stray file leaked in: {files:?}"
    );
}

#[test]
fn test_an_empty_scope_is_refused_rather_than_staging_everything() {
    let fixture = Fixture::new("empty");
    let repo = fixture.repo();
    std::fs::write(fixture.repo_dir.join("dirty.txt"), "x\n").unwrap();

    let error = repo.stage_paths(&[]).unwrap_err();
    assert!(matches!(error, GitError::EmptyScope), "got {error:?}");
    assert!(!repo.has_staged_changes().unwrap(), "nothing was staged");
}

#[test]
fn test_commit_returns_its_own_sha_and_first_line() {
    let fixture = Fixture::new("summary");
    let repo = fixture.repo();
    std::fs::write(fixture.repo_dir.join("a.txt"), "x\n").unwrap();
    repo.stage_paths(&[Path::new("a.txt")]).unwrap();

    let commit = repo
        .commit("subject line\n\nbody line that must not become the summary")
        .unwrap();

    assert_eq!(commit.sha.len(), 40, "full sha, not abbreviated");
    assert_eq!(commit.summary, "subject line");
    assert_eq!(commit.sha, repo.head_sha().unwrap());
}

#[test]
fn test_staged_state_is_visible_and_clears_after_commit() {
    let fixture = Fixture::new("staged");
    let repo = fixture.repo();
    assert!(!repo.has_staged_changes().unwrap());

    std::fs::write(fixture.repo_dir.join("b.txt"), "x\n").unwrap();
    repo.stage_paths(&[Path::new("b.txt")]).unwrap();
    assert!(repo.has_staged_changes().unwrap(), "staged work is visible");

    repo.commit("add b").unwrap();
    assert!(
        !repo.has_staged_changes().unwrap(),
        "a commit clears the index"
    );
}

#[test]
fn test_annotated_and_lightweight_tags_are_distinguishable() {
    let fixture = Fixture::new("tags");
    let repo = fixture.repo();

    git(&fixture.repo_dir, &["tag", "lightweight"]);
    assert!(
        !repo.has_annotated_tag("lightweight").unwrap(),
        "a lightweight tag carries no tagger or message"
    );

    repo.tag_annotated("v0.1.0", "release v0.1.0").unwrap();
    assert!(repo.has_annotated_tag("v0.1.0").unwrap());
    assert!(!repo.has_annotated_tag("no-such-tag").unwrap());
}

#[test]
fn test_the_branch_and_its_tag_reach_a_bare_remote_together() {
    let fixture = Fixture::new("push");
    let repo = fixture.repo();
    // A real bare remote: the only way to prove a push landed is to look at the other side.
    let bare = fixture.bare_remote();

    repo.tag_annotated("v0.1.0", "release v0.1.0").unwrap();
    repo.push_branch_and_tag("origin", "trunk", "v0.1.0")
        .unwrap();

    let remote_tags = git_dir(&bare, &["tag"]);
    assert!(
        remote_tags.contains("v0.1.0"),
        "the tag reached the remote: {remote_tags:?}"
    );

    let local_head = repo.head_sha().unwrap();
    let remote_tag_target = git_dir(&bare, &["rev-parse", "v0.1.0^{commit}"]);
    assert_eq!(
        remote_tag_target, local_head,
        "the remote tag points at the commit that was pushed"
    );
    assert_eq!(
        git_dir(&bare, &["rev-parse", "trunk"]),
        local_head,
        "the branch landed too"
    );
}

#[test]
fn test_fetching_a_tag_the_remote_does_not_have_is_an_error() {
    let fixture = Fixture::new("fetch-missing");
    let repo = fixture.repo();
    fixture.bare_remote();

    let error = repo.fetch_tag("origin", "v9.9.9").unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("couldn't find remote ref") || message.contains("not found"),
        "verification must report a missing tag rather than assume it: {message}"
    );
}
