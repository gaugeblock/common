//! `TaskKind` wire shape: the one kind list, its spellings and its parse (`GB-343`).
//!
//! Split from `storypole_wire.rs` when that file passed the 250-line limit. Kinds have their own contract
//! now, so they earn their own file.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::TaskKind;

#[test]
fn the_docs_kind_round_trips_and_older_kinds_still_parse() {
    // GB-342: adding a kind must not disturb what is already on disk. A ledger written before `docs`
    // existed spells its kinds as snake_case strings, and those have to keep parsing byte-identically.
    assert_eq!(serde_json::to_string(&TaskKind::Docs).unwrap(), "\"docs\"");
    assert_eq!(
        serde_json::from_str::<TaskKind>("\"docs\"").unwrap(),
        TaskKind::Docs
    );
    for (wire, kind) in [
        ("epic", TaskKind::Epic),
        ("feature", TaskKind::Feature),
        ("tech_debt", TaskKind::TechDebt),
        ("chore", TaskKind::Chore),
    ] {
        assert_eq!(
            serde_json::from_str::<TaskKind>(&format!("\"{wire}\"")).unwrap(),
            kind,
            "{wire} still parses"
        );
    }
}

#[test]
fn every_kind_has_one_unique_wire_spelling_that_round_trips() {
    // GB-343: the list is the single source of truth, so it has to be internally consistent before
    // anything can derive from it. A duplicate spelling would silently make one kind unreachable.
    let mut spellings: Vec<&str> = TaskKind::ALL.iter().map(|k| k.as_str()).collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total, "spellings are unique");

    for kind in TaskKind::ALL {
        let wire = format!("\"{}\"", kind.as_str());
        assert_eq!(serde_json::from_str::<TaskKind>(&wire).unwrap(), kind);
        assert_eq!(serde_json::to_string(&kind).unwrap(), wire);
        assert_eq!(TaskKind::from_wire(kind.as_str()), Some(kind));
    }
}

#[test]
fn the_parse_tolerates_the_spellings_people_type() {
    assert_eq!(TaskKind::from_wire("TECH-DEBT"), Some(TaskKind::TechDebt));
    assert_eq!(TaskKind::from_wire("techdebt"), Some(TaskKind::TechDebt));
    assert_eq!(TaskKind::from_wire("Docs"), Some(TaskKind::Docs));
    assert_eq!(TaskKind::from_wire("perf"), Some(TaskKind::Perf));
    assert_eq!(
        TaskKind::from_wire("nonsense"),
        None,
        "unknown is None, never a default"
    );
}

#[test]
fn the_conventional_commit_types_law_7_names_all_have_a_kind() {
    // Law 7 allows ten commit types. Five map to kinds by another name; the other five are kinds in their
    // own right. This test is the reason GB-343 exists: work was being committed with types the tracker
    // could not express.
    for wire in [
        "feat", "fix", "refactor", "perf", "test", "docs", "style", "chore", "ci", "build",
    ] {
        let alias = match wire {
            "feat" => "feature",
            "fix" => "bug",
            other => other,
        };
        assert!(
            TaskKind::from_wire(alias).is_some(),
            "commit type '{wire}' has a kind via '{alias}'"
        );
    }
}
