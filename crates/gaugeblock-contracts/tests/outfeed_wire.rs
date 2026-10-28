//! Wire-shape regression tests for the `outfeed` contract domain.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::{
    BumpType, CommitInfo, ConventionalKind, ManifestDiff, ReleasePlan, ReleaseStatus, SuiteLock,
    SuiteMetadata, SuiteToolEntry,
};

fn sample_commit() -> CommitInfo {
    CommitInfo {
        sha: "dd42b57".to_string(),
        summary: "feat(outfeed-core): add adaptive disk headroom guard".to_string(),
        body: Some("Task: GB-122".to_string()),
        kind: ConventionalKind::Feat,
        scope: Some("outfeed-core".to_string()),
        is_breaking: false,
        breaking_description: None,
        referenced_tickets: vec!["GB-122".to_string()],
    }
}

fn sample_plan() -> ReleasePlan {
    ReleasePlan {
        repo_name: "outfeed".to_string(),
        current_version: "0.1.0-beta.7".to_string(),
        target_version: "0.1.0-beta.8".to_string(),
        bump_type: BumpType::PreReleasePatch,
        bump_rationale: "pre-release patch sequence".to_string(),
        commits: vec![sample_commit()],
        referenced_tickets: vec!["GB-122".to_string()],
        changelog_markdown: "## [0.1.0-beta.8]\n".to_string(),
        manifest_diffs: vec![ManifestDiff {
            relative_path: "crates/outfeed-core/Cargo.toml".to_string(),
            original_content: "version = \"0.1.0-beta.7\"".to_string(),
            proposed_content: "version = \"0.1.0-beta.8\"".to_string(),
        }],
    }
}

#[test]
fn test_release_plan_roundtrips_through_json() {
    let plan = sample_plan();
    let json = serde_json::to_string(&plan).expect("serialize");
    let parsed: ReleasePlan = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(plan, parsed);
}

#[test]
fn test_release_plan_field_names_match_contracts() {
    let json = serde_json::to_value(sample_plan()).expect("value");
    for key in [
        "repo_name",
        "current_version",
        "target_version",
        "bump_type",
        "bump_rationale",
        "commits",
        "referenced_tickets",
        "changelog_markdown",
        "manifest_diffs",
    ] {
        assert!(json.get(key).is_some(), "missing ReleasePlan.{key}");
    }
    let commit = &json["commits"][0];
    for key in [
        "sha",
        "summary",
        "body",
        "kind",
        "scope",
        "is_breaking",
        "breaking_description",
        "referenced_tickets",
    ] {
        assert!(commit.get(key).is_some(), "missing CommitInfo.{key}");
    }
}

#[test]
fn test_bump_type_uses_explicit_variant_names() {
    let json = serde_json::to_string(&BumpType::PreReleasePatch).expect("serialize");
    assert_eq!(json, "\"PreReleasePatch\"");
    assert_eq!(
        serde_json::to_string(&BumpType::Minor).expect("serialize"),
        "\"Minor\""
    );
}

#[test]
fn test_conventional_kind_carries_unknown_prefixes() {
    let kind = ConventionalKind::Other("wip".to_string());
    let json = serde_json::to_string(&kind).expect("serialize");
    let parsed: ConventionalKind = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, kind);
}

#[test]
fn test_suite_lock_roundtrip_and_optional_tag() {
    let lock = SuiteLock {
        suite: SuiteMetadata {
            version: "0.1.0-beta.3".to_string(),
            created_at: "2026-10-28T00:00:00Z".to_string(),
            meta_harness: "gaugeblock".to_string(),
        },
        tools: vec![
            SuiteToolEntry {
                name: "miter".to_string(),
                version: "0.1.0-beta.4".to_string(),
                commit: "abc1234".to_string(),
                tag: Some("v0.1.0-beta.4".to_string()),
            },
            SuiteToolEntry {
                name: "common".to_string(),
                version: "0.1.0-beta.1".to_string(),
                commit: "def5678".to_string(),
                tag: None,
            },
        ],
    };

    let json = serde_json::to_value(&lock).expect("value");
    assert!(
        json["tools"][1].get("tag").is_none(),
        "an absent tag must be omitted from the wire shape"
    );

    let parsed: SuiteLock = serde_json::from_value(json).expect("deserialize");
    assert_eq!(lock, parsed);
}

#[test]
fn test_release_status_roundtrip() {
    let status = ReleaseStatus {
        repo_name: "storypole".to_string(),
        current_version: "0.1.0-beta.5".to_string(),
        latest_tag: Some("v0.1.0-beta.5".to_string()),
        unreleased_commits: 0,
        is_clean: true,
    };
    let json = serde_json::to_string(&status).expect("serialize");
    let parsed: ReleaseStatus = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(status, parsed);
}
