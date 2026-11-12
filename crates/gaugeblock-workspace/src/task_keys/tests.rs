//! The rules that decide whether an ambient key is attributable (`GB-356`).
//!
//! These test [`verdict`] rather than [`ambient`] on purpose: `set_var` is unsafe in edition 2024, this
//! workspace denies `unsafe_code`, and a test that plants an environment variable would race every other test
//! reading the environment. The judgement is pure, so every rule is testable without touching the environment -
//! and the rules are the part that decides whether a corpus gets a wrong fact.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// A prefix set shaped like a real answer: the harness plus one repository.
fn allowed() -> Vec<String> {
    vec!["GB".to_string(), "SP".to_string()]
}

const NOW: u64 = 1_000_000;

#[test]
fn a_fresh_lease_attributes_the_key() {
    assert_eq!(
        verdict("GB-356", Some(NOW + 60_000), NOW, &allowed()),
        Attribution::Attributed("GB-356".to_string())
    );
}

#[test]
fn an_expired_lease_refuses_and_that_is_the_leftover_case() {
    // The concern this design exists for: a shell still holding yesterday's key. One millisecond past the lease
    // is enough - the variable outlived the claim that justified it, so it is not a fact about this run.
    assert_eq!(
        verdict("GB-355", Some(NOW - 1), NOW, &allowed()),
        Attribution::Refused(Refusal::Expired)
    );
    assert_eq!(
        verdict("GB-355", Some(NOW), NOW, &allowed()),
        Attribution::Refused(Refusal::Expired),
        "the boundary is inclusive: a lease that expires now has expired"
    );
}

#[test]
fn a_key_with_no_lease_is_refused_rather_than_trusted() {
    // `export GAUGEBLOCK_TASK_KEY=GB-355` and nothing else is precisely the hand-set variable that never
    // expires. Accepting it would make the leftover case unbounded instead of merely likely.
    assert_eq!(
        verdict("GB-355", None, NOW, &allowed()),
        Attribution::Refused(Refusal::NoExpiry)
    );
}

#[test]
fn junk_is_refused() {
    for junk in [
        "",
        "GB",
        "GB-",
        "-12",
        "gb-355",
        "GB-12a",
        "GAUGEBLOCK-1",
        "GB-3 5",
    ] {
        assert_eq!(
            verdict(junk, Some(NOW + 60_000), NOW, &allowed()),
            Attribution::Refused(Refusal::Malformed),
            "{junk:?} must not be recorded as a key"
        );
    }
}

#[test]
fn a_key_from_another_repository_is_refused() {
    // A real key, a live lease, and still wrong: `MTR-9` has no business in a repository whose prefix is SP,
    // because that is a stale export or a copy-paste, and either one is a false fact in the corpus.
    assert_eq!(
        verdict("MTR-9", Some(NOW + 60_000), NOW, &allowed()),
        Attribution::Refused(Refusal::WrongRepository)
    );
}

#[test]
fn the_harness_prefix_is_allowed_inside_a_member() {
    // Not a loophole: harness tickets are worked on in members constantly, which is what the harness is for.
    // The rule is "this repository or the suite", never "this repository only".
    assert_eq!(
        verdict("GB-305", Some(NOW + 60_000), NOW, &allowed()),
        Attribution::Attributed("GB-305".to_string())
    );
}

#[test]
fn an_unknown_prefix_set_skips_the_check_rather_than_refusing() {
    // Configurations that cannot be read mean the question was not answerable, not that the answer is no.
    // Refusing here would discard attribution over a missing file; the degradation is visible because the
    // caller passes an empty list only when it could not look.
    assert_eq!(
        verdict("MTR-9", Some(NOW + 60_000), NOW, &[]),
        Attribution::Attributed("MTR-9".to_string())
    );
}

#[test]
fn the_absent_case_records_nothing() {
    assert_eq!(Attribution::Absent.key(), None);
    assert_eq!(
        Attribution::Refused(Refusal::Expired).key(),
        None,
        "a refusal must leave the field empty rather than carry the key it refused"
    );
    assert_eq!(Attribution::Attributed("GB-1".into()).key(), Some("GB-1"));
}

#[test]
fn every_refusal_has_a_reason_a_human_can_act_on() {
    for refusal in [
        Refusal::NoExpiry,
        Refusal::Expired,
        Refusal::Malformed,
        Refusal::WrongRepository,
    ] {
        let reason = refusal.reason();
        assert!(reason.len() > 20, "a reason must explain itself: {reason}");
        assert!(
            reason
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_lowercase()),
            "the reason is prose for a human, not a variant name: {reason}"
        );
    }
}
