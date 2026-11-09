//! Path fixtures: the main checkout and a linked worktree must resolve to one directory.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_workspace::{GaugeblockPaths, WorkspaceError};
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
        let base = std::env::temp_dir().join(format!("gb-ws-{tag}-{}", std::process::id()));
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
        // A real suite has the directory; creating it here also lets the assertions canonicalise,
        // which matters because macOS resolves /var through a symlink to /private/var.
        std::fs::create_dir_all(repo_dir.join(".gaugeblock")).unwrap();
        Self { base, repo_dir }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.base).ok();
    }
}

#[test]
fn test_the_main_checkout_resolves_to_its_own_gaugeblock_directory() {
    let fixture = Fixture::new("main");
    let paths = GaugeblockPaths::discover(&fixture.repo_dir).unwrap();

    assert_eq!(
        paths.root(),
        fixture.repo_dir.canonicalize().unwrap().join(".gaugeblock"),
        "the directory hangs off the checkout root"
    );
    assert_eq!(paths.outbox(), paths.root().join("spool/outbox.jsonl"));
    assert_eq!(paths.ledger(), paths.root().join("storypole/ledger.jsonl"));
}

#[test]
fn test_a_linked_worktree_resolves_to_the_same_directory_as_its_parent() {
    // The entire reason this type exists. A worktree that resolved to its own `.gaugeblock/` would
    // grow a second ledger and a second spool, and two agents would disagree about which records
    // exist.
    let fixture = Fixture::new("worktree");
    let main = GaugeblockPaths::discover(&fixture.repo_dir).unwrap();
    let worktree = fixture.base.join("agent-1");
    git(
        &fixture.repo_dir,
        &[
            "worktree",
            "add",
            "-b",
            "agent/one",
            worktree.to_str().unwrap(),
        ],
    );

    let from_worktree = GaugeblockPaths::discover(&worktree).unwrap();

    assert_eq!(
        from_worktree.root().canonicalize().unwrap(),
        main.root().canonicalize().unwrap(),
        "a worktree shares its parent's .gaugeblock directory"
    );
    assert_eq!(
        from_worktree.suite_root().canonicalize().unwrap(),
        fixture.repo_dir.canonicalize().unwrap(),
        "and reports the main checkout as its suite root, not the worktree"
    );
}

#[test]
fn test_a_subdirectory_resolves_to_the_checkout_root() {
    let fixture = Fixture::new("nested");
    let nested = fixture.repo_dir.join("src/deep");
    std::fs::create_dir_all(&nested).unwrap();

    let paths = GaugeblockPaths::discover(&nested).unwrap();
    assert_eq!(
        paths.root(),
        fixture.repo_dir.canonicalize().unwrap().join(".gaugeblock")
    );
}

#[test]
fn test_a_directory_outside_a_repository_is_refused_rather_than_guessed() {
    // Walking up to find "some" .gaugeblock is the ambiguity this type removes, so an unresolvable
    // path is an error instead of a fallback.
    let outside = std::env::temp_dir().join(format!("gb-ws-none-{}", std::process::id()));
    std::fs::create_dir_all(&outside).unwrap();

    let error = GaugeblockPaths::discover(&outside).unwrap_err();
    assert!(
        matches!(error, WorkspaceError::NotARepository { .. }),
        "{error:?}"
    );
    assert!(error.to_string().contains("not inside a git repository"));

    std::fs::remove_dir_all(&outside).ok();
}

#[test]
fn test_explicit_construction_does_not_consult_git() {
    // A fixture or a tool handed a directory deliberately should not need a repository.
    let paths = GaugeblockPaths::at(Path::new("/tmp/suite"));
    assert_eq!(paths.root(), Path::new("/tmp/suite/.gaugeblock"));
    assert_eq!(
        paths.tool_config("miter"),
        Path::new("/tmp/suite/.gaugeblock/miter/config.toml")
    );
}

#[test]
fn test_topology_discovers_git_repositories_and_reports_missing_expected_members() {
    use gaugeblock_workspace::WorkspaceTopology;

    let base = std::env::temp_dir().join(format!("gb-ws-topo-{}", std::process::id()));
    std::fs::remove_dir_all(&base).ok();
    for name in ["miter", "storypole", "not-a-repo"] {
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        if name != "not-a-repo" {
            std::fs::create_dir_all(dir.join(".git")).unwrap();
        }
    }

    let topology = WorkspaceTopology::discover(&base);
    let names: Vec<&str> = topology.members().iter().map(|m| m.name.as_str()).collect();

    assert_eq!(
        names,
        vec!["miter", "storypole"],
        "sorted, and only repositories"
    );
    assert_eq!(
        topology.member("miter").unwrap().gaugeblock_dir(),
        base.join("miter/.gaugeblock/miter")
    );
    // The expected list is what makes an empty tree distinguishable from a healthy one.
    let missing = topology.missing_members();
    assert!(missing.contains(&"benchwork"), "got {missing:?}");
    assert!(!missing.contains(&"miter"));
    assert_eq!(missing.len(), 9, "2 present of 11 expected");

    std::fs::remove_dir_all(&base).ok();
}

#[test]
fn the_ticket_directory_sits_beside_the_legacy_ledger() {
    // GB-348: both paths are offered, and the legacy one says so, so a consumer has to choose rather than
    // copying whichever filename it found first.
    let paths = gaugeblock_workspace::GaugeblockPaths::at(std::path::Path::new("/repo"));
    assert_eq!(
        paths.storypole_tickets(),
        paths.member("storypole").join("tickets")
    );
    assert!(paths.ledger().ends_with("storypole/ledger.jsonl"));
}
