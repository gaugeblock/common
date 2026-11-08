#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use super::Membership;

fn dir(path: &Path) {
    fs::create_dir_all(path).expect("create dir");
}

fn member(suite_root: &Path, name: &str) {
    dir(&suite_root.join(name).join(".git"));
}

fn suite_root(root: &Path) {
    dir(&root.join(".gaugeblock"));
    let members: Vec<String> = ["miter", "storypole", "alpha", "beta"]
        .iter()
        .map(|m| format!("\"{m}\""))
        .collect();
    fs::write(
        root.join(".gaugeblock").join("config.toml"),
        format!("[suite]\nmembers = [{}]\n", members.join(", ")),
    )
    .expect("suite declaration");
}

#[test]
fn finds_the_suite_that_lists_this_repository() {
    let tmp = tempfile::tempdir().expect("tempdir");
    suite_root(tmp.path());
    member(tmp.path(), "miter");
    dir(&tmp.path().join("miter").join("crates").join("engine"));

    let found = Membership::containing(&tmp.path().join("miter").join("crates").join("engine"))
        .expect("miter is a member of the suite above it");

    assert_eq!(found.suite_root, tmp.path());
    assert_eq!(found.member.name, "miter");
}

#[test]
fn a_standalone_repository_belongs_to_no_suite() {
    let tmp = tempfile::tempdir().expect("tempdir");
    member(tmp.path(), "solo");
    dir(&tmp.path().join("solo").join("src"));

    assert!(
        Membership::containing(&tmp.path().join("solo").join("src")).is_none(),
        "no suite lists it, so it is not a member of one"
    );
}

#[test]
fn a_nested_checkout_is_not_adopted_by_the_suite_above_it() {
    // The GB-305 shape: a repository inside a member. The suite above does not list it, and looking
    // further up for a taker is how work lands in a ledger it does not belong to.
    let tmp = tempfile::tempdir().expect("tempdir");
    suite_root(tmp.path());
    member(tmp.path(), "miter");
    dir(&tmp.path().join("miter").join("vendor").join(".git"));

    assert!(
        Membership::containing(&tmp.path().join("miter").join("vendor")).is_none(),
        "a nested checkout is not the member `miter`, and not a member of its own"
    );
}

#[test]
fn a_suite_root_is_not_a_member_of_itself() {
    let tmp = tempfile::tempdir().expect("tempdir");
    suite_root(tmp.path());
    member(tmp.path(), "miter");

    assert!(Membership::containing(tmp.path()).is_none());
}
