//! multi-repo: a suite with two members in scope, so the spool question can be asked from a member - the only
//! position from which the two answers differ (`GB-304`, `GB-305`).
//!
//! `GaugeblockPaths::discover` resolves through the git common dir, so from a member checkout it returns that
//! member's `.gaugeblock` and never looks up. `outbox_path` walks up to the declared suite first. Both answers
//! are correct for their own question; using the first for the second is what made `rabbet ingest` read an empty
//! member spool while the suite held the traces.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use gaugeblock_workspace::{GaugeblockPaths, outbox_path, sink_path};

/// A member checkout. `real_git` builds a genuine repository, which `GaugeblockPaths::discover` requires -
/// it asks git where the common dir is rather than looking for a marker.
fn member(root: &Path, name: &str, real_git: bool) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("src")).expect("member source");
    if real_git {
        let output = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&dir)
            .output()
            .expect("git spawns");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        fs::create_dir_all(dir.join(".git")).expect("member checkout");
    }
    dir
}

/// A suite root that declares `members`.
fn suite(members: &[&str]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join(".gaugeblock")).expect("suite state");
    let declared: Vec<String> = members.iter().map(|name| format!("\"{name}\"")).collect();
    fs::write(
        root.path().join(".gaugeblock").join("config.toml"),
        format!("[suite]\nmembers = [{}]\n", declared.join(", ")),
    )
    .expect("suite declaration");
    root
}

/// Both sides canonicalised: the resolver canonicalises before walking ancestors (`GB-327`), and on macOS a
/// tempdir is reached through `/var`, which is a symlink to `/private/var`.
fn canon(path: PathBuf) -> PathBuf {
    // The file itself does not exist, and `canonicalize` fails silently for a missing path - so the parent is
    // canonicalised and the file name re-attached.
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => match std::fs::canonicalize(parent) {
            Ok(dir) => dir.join(name),
            Err(_) => path,
        },
        _ => path,
    }
}

#[test]
fn from_a_member_the_outbox_is_the_suite_outbox() {
    let root = suite(&["alpha", "beta"]);
    let alpha = member(root.path(), "alpha", true);
    member(root.path(), "beta", false);

    fs::create_dir_all(root.path().join(".gaugeblock").join("spool")).expect("suite spool");
    fs::create_dir_all(alpha.join(".gaugeblock").join("spool")).expect("member spool");
    let expected = GaugeblockPaths::at(root.path()).outbox();
    assert_eq!(
        canon(outbox_path(&alpha.join("src")).expect("a member resolves")),
        canon(expected),
        "a member writes traces into the suite spool, because a member's own spool is never filled"
    );
}

#[test]
fn the_repository_local_answer_stays_the_member() {
    // Not a bug being preserved - a boundary being pinned. `discover` answers "which checkout am I in", and
    // callers that want the suite must say so. When these two agreed, nothing revealed which one a caller used.
    let root = suite(&["alpha", "beta"]);
    let alpha = member(root.path(), "alpha", true);
    member(root.path(), "beta", false);

    fs::create_dir_all(alpha.join(".gaugeblock").join("spool")).expect("member spool");
    let local = GaugeblockPaths::discover(&alpha.join("src")).expect("a checkout resolves");
    assert_eq!(
        canon(local.outbox()),
        canon(alpha.join(".gaugeblock").join("spool").join("outbox.jsonl"))
    );
    assert_ne!(
        local.outbox(),
        canon(outbox_path(&alpha.join("src")).expect("a member resolves")),
        "if these agreed the distinction could not be tested, so a caller could not be caught using the wrong one"
    );
}

#[test]
fn a_lone_repository_keeps_its_own_spool() {
    // No declared suite above it: there is nothing to centralise into, and refusing to record would be worse
    // than recording locally.
    let lone = tempfile::tempdir().expect("tempdir");
    let repo = member(lone.path(), "solo", true);
    fs::create_dir_all(repo.join(".gaugeblock").join("spool")).expect("member spool");
    assert_eq!(
        canon(outbox_path(&repo.join("src")).expect("a checkout resolves")),
        canon(repo.join(".gaugeblock").join("spool").join("outbox.jsonl"))
    );
}

#[test]
fn the_two_derived_artefacts_resolve_to_the_same_root() {
    // The span log and the trace outbox are both derived and both centralise, so a process that writes one and
    // reads the other must not straddle two roots.
    let root = suite(&["alpha", "beta"]);
    let alpha = member(root.path(), "alpha", true);
    member(root.path(), "beta", false);

    let sink = sink_path(&alpha.join("src")).expect("a member resolves");
    let outbox = outbox_path(&alpha.join("src")).expect("a member resolves");
    assert_eq!(
        sink.parent().and_then(|p| p.parent()),
        outbox.parent().and_then(|p| p.parent()),
        "sink {} and outbox {} must share the .gaugeblock root",
        sink.display(),
        outbox.display()
    );
}
