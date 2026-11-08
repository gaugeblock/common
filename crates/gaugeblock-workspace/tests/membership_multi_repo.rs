//! multi-repo: a suite with two member checkouts in scope, which is the only way to see that membership
//! resolves each one to the suite above it rather than to itself.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use gaugeblock_workspace::Membership;

fn member(root: &Path, name: &str) {
    fs::create_dir_all(root.join(name).join(".git")).expect("member checkout");
    fs::create_dir_all(root.join(name).join("src")).expect("member source");
}

#[test]
fn each_member_of_a_suite_resolves_to_the_suite_and_to_itself() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join(".gaugeblock")).expect("suite state");
    member(tmp.path(), "alpha");
    member(tmp.path(), "beta");

    // One member does not prove the answer: a resolution that returned "the nearest directory with a
    // checkout" passes with one and fails with two.
    for name in ["alpha", "beta"] {
        let found =
            Membership::containing(&tmp.path().join(name).join("src")).expect("a member resolves");
        assert_eq!(
            found.suite_root,
            tmp.path(),
            "{name} belongs to the suite above it"
        );
        assert_eq!(found.member.name, name, "and to itself within it");
    }
}

#[test]
fn a_checkout_outside_the_suite_resolves_to_nothing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join(".gaugeblock")).expect("suite state");
    member(tmp.path(), "alpha");

    let other = tempfile::tempdir().expect("second tempdir");
    let outsider = other.path().join("elsewhere");
    fs::create_dir_all(outsider.join(".git")).expect("outsider checkout");
    fs::create_dir_all(outsider.join("src")).expect("outsider source");

    assert!(
        Membership::containing(&outsider.join("src")).is_none(),
        "a directory the suite does not list is not one of its members"
    );
}
