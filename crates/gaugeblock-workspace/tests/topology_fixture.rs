//! Discovery rules for `WorkspaceTopology` (`GB-189`).
//!
//! The default rule is `.git`, which is right for this suite. `discover_with` exists because
//! `storypole` legitimately accepts a member that is not a checkout, and a shared model must not
//! impose one product's policy on another.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use gaugeblock_workspace::WorkspaceTopology;
use tempfile::TempDir;

fn member_dir(root: &Path, name: &str) -> std::path::PathBuf {
    let path = root.join(name);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn default_discovery_requires_a_git_marker() {
    let temp = TempDir::new().unwrap();
    member_dir(temp.path(), "checkout");
    fs::create_dir_all(temp.path().join("checkout").join(".git")).unwrap();
    member_dir(temp.path(), "plain-directory");

    let topology = WorkspaceTopology::discover(temp.path());

    assert_eq!(topology.members().len(), 1);
    assert_eq!(
        topology.member("checkout").map(|m| m.name.as_str()),
        Some("checkout")
    );
    assert!(topology.member("plain-directory").is_none());
}

#[test]
fn discover_with_applies_a_caller_supplied_rule() {
    let temp = TempDir::new().unwrap();
    member_dir(temp.path(), "has-state");
    fs::create_dir_all(temp.path().join("has-state").join(".storypole")).unwrap();
    member_dir(temp.path(), "has-nothing");

    let topology =
        WorkspaceTopology::discover_with(temp.path(), |path| path.join(".storypole").is_dir());

    assert_eq!(topology.members().len(), 1);
    assert_eq!(topology.members()[0].name, "has-state");
}

#[test]
fn discover_with_still_skips_hidden_directories() {
    let temp = TempDir::new().unwrap();
    member_dir(temp.path(), ".gaugeblock");
    fs::create_dir_all(temp.path().join(".gaugeblock").join(".git")).unwrap();

    // Dot directories hold tool state, never a member, whatever the rule says.
    let topology = WorkspaceTopology::discover_with(temp.path(), |_| true);

    assert!(topology.members().is_empty());
}

#[test]
fn members_are_sorted_and_expected_members_are_reported_missing() {
    let temp = TempDir::new().unwrap();
    for name in ["zeta", "alpha"] {
        member_dir(temp.path(), name);
        fs::create_dir_all(temp.path().join(name).join(".git")).unwrap();
    }

    let topology = WorkspaceTopology::discover(temp.path());

    let names: Vec<&str> = topology.members().iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "zeta"]);
    // `miter` is expected and absent here.
    assert!(topology.missing_members().contains(&"miter"));
}
