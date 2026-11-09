//! Where spans are written (`GB-338`).
//!
//! Telemetry is derived and untracked, so it centralises at the suite root: there is nothing to merge, and every
//! worktree of a member already shares it through the common dir. The alternative - following whichever repository
//! ran the command - is what left a panel tailing the root with nothing to show.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use gaugeblock_workspace::sink_path;

/// A suite root declaring `alpha` as a member, both canonicalised.
fn suite(root: &Path) -> std::path::PathBuf {
    fs::create_dir_all(root.join(".gaugeblock")).unwrap();
    fs::write(
        root.join(".gaugeblock/config.toml"),
        "[suite]\nmembers = [\"alpha\"]\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("alpha/.git")).unwrap();
    std::fs::canonicalize(root).unwrap()
}

#[test]
fn a_run_inside_a_member_writes_to_the_suite_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = suite(temp.path());

    let sink = sink_path(&root.join("alpha")).expect("a suite member has a sink");

    assert_eq!(
        sink,
        root.join(".gaugeblock/telemetry/events.jsonl"),
        "one log for the suite, not one per member"
    );
}

#[test]
fn a_lone_checkout_writes_to_its_own_directory() {
    // No suite lists it, so there is nothing to centralise into - and a user with several unrelated repositories
    // has no suite at all.
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("solo");
    fs::create_dir_all(&repo).unwrap();
    // A real repository: `Repository::discover` asks git, and an empty `.git` directory is not one.
    assert!(
        std::process::Command::new("git")
            .current_dir(&repo)
            .args(["init", "-q"])
            .status()
            .unwrap()
            .success()
    );
    let canonical = std::fs::canonicalize(&repo).unwrap();

    let sink = sink_path(&canonical).expect("a checkout has a sink");

    assert_eq!(sink, canonical.join(".gaugeblock/telemetry/events.jsonl"));
}

#[test]
fn outside_a_checkout_there_is_nowhere_to_write() {
    let temp = tempfile::tempdir().unwrap();
    assert!(sink_path(temp.path()).is_none());
}
