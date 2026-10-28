//! Wire-shape regression tests for the `storypole` contract domain.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::{
    AgentClaim, DecisionStatus, EdgeKind, HandoffPacket, LedgerEvent, TaskContextBriefing,
    TaskDecision, TaskEdge, TaskKind, TaskNode, TaskScope, TaskStatus, VerificationReceipt,
};
use uuid::Uuid;

fn uuid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn sample_task() -> TaskNode {
    let task_id = uuid(1);
    TaskNode {
        id: task_id,
        ticket_key: "GB-122".to_string(),
        slug: "gb-122".to_string(),
        title: "Adaptive disk headroom guard".to_string(),
        description: "Guard disk headroom during suite releases".to_string(),
        kind: TaskKind::Feature,
        status: TaskStatus::InProgress,
        priority: 1,
        scope: TaskScope {
            repo: Some("outfeed".to_string()),
            crate_name: Some("outfeed-core".to_string()),
            target_files: vec!["crates/outfeed-core/src/headroom/mod.rs".to_string()],
            ..Default::default()
        },
        edges: vec![TaskEdge {
            kind: EdgeKind::ParentEpic,
            target_id: uuid(2),
            target_ticket_key: Some("GB-121".to_string()),
            target_repo: None,
        }],
        decisions: vec![TaskDecision {
            decision_id: uuid(3),
            parent_decision_id: None,
            status: DecisionStatus::Resolved,
            category: "cache_policy".to_string(),
            question: "Retain the cache between repos?".to_string(),
            proposed_options: vec!["yes".to_string(), "no".to_string()],
            chosen_option: Some("yes".to_string()),
            rationale: Some("cheaper and safe with a guard".to_string()),
            resolved_by: Some("owner".to_string()),
            updated_at_epoch_ms: 1,
        }],
        acceptance_criteria: vec!["headroom is enforced".to_string()],
        plan: None,
        active_claim: Some(AgentClaim {
            agent_id: "cline".to_string(),
            claimed_at_epoch_ms: 1,
            lease_expires_at_epoch_ms: 2,
            git_branch: Some("feat/outfeed-adaptive-headroom".to_string()),
            base_commit_sha: Some("eed7824".to_string()),
        }),
        latest_handoff: Some(HandoffPacket {
            recorded_by_agent: "cline".to_string(),
            timestamp_ms: 3,
            completed_items: vec!["guard".to_string()],
            remaining_items: vec!["tier 3".to_string()],
            last_error_or_blocker: None,
            git_ref: Some("dd42b57".to_string()),
            dirty_files: Vec::new(),
        }),
        verification_receipt: Some(VerificationReceipt {
            verifier: "cargo-nextest".to_string(),
            command: "cargo nextest run -p outfeed-core".to_string(),
            exit_code: Some(0),
            diagnostic_count: None,
            summary: "tier 2 green".to_string(),
            verified_at_epoch_ms: 4,
            git_branch: Some("feat/outfeed-adaptive-headroom".to_string()),
            base_commit_sha: Some("eed7824".to_string()),
            head_commit_sha: Some("dd42b57".to_string()),
            commit_shas: vec!["dd42b57".to_string()],
        }),
        created_at_epoch_ms: 1,
        updated_at_epoch_ms: 5,
    }
}

#[test]
fn test_task_node_roundtrips_with_full_continuity_state() {
    let task = sample_task();
    let json = serde_json::to_string(&task).expect("serialize");
    let parsed: TaskNode = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(task, parsed);
}

#[test]
fn test_task_node_field_names_and_snake_case_enums() {
    let value = serde_json::to_value(sample_task()).expect("value");
    for key in [
        "id",
        "ticket_key",
        "slug",
        "title",
        "description",
        "kind",
        "status",
        "priority",
        "scope",
        "edges",
        "decisions",
        "acceptance_criteria",
        "plan",
        "active_claim",
        "latest_handoff",
        "verification_receipt",
        "created_at_epoch_ms",
        "updated_at_epoch_ms",
    ] {
        assert!(value.get(key).is_some(), "missing TaskNode.{key}");
    }
    assert_eq!(value["kind"], "feature");
    assert_eq!(value["status"], "in_progress");
    assert_eq!(value["edges"][0]["kind"], "parent_epic");
    assert_eq!(value["decisions"][0]["status"], "resolved");
}

#[test]
fn test_task_edge_omits_absent_metadata() {
    let edge = TaskEdge {
        kind: EdgeKind::BlockedBy,
        target_id: uuid(9),
        target_ticket_key: None,
        target_repo: None,
    };
    let value = serde_json::to_value(&edge).expect("value");
    assert!(value.get("target_ticket_key").is_none());
    assert!(value.get("target_repo").is_none());
    assert_eq!(value["kind"], "blocked_by");
}

#[test]
fn test_ledger_event_is_tagged_on_event() {
    let task = sample_task();
    let receipt = task.verification_receipt.clone().expect("receipt");
    let event = LedgerEvent::TaskVerified {
        task_id: task.id,
        receipt,
    };

    let value = serde_json::to_value(&event).expect("value");
    assert_eq!(value["event"], "task_verified");
    assert!(value.get("task_id").is_some());

    let parsed: LedgerEvent = serde_json::from_value(value).expect("deserialize");
    assert_eq!(parsed, event);
}

#[test]
fn test_deferred_capture_event_roundtrips() {
    let event = LedgerEvent::DeferredItemCaptured {
        task: sample_task(),
        captured_by_agent: "cline".to_string(),
        discovered_from_task_id: Some(uuid(7)),
    };
    let value = serde_json::to_value(&event).expect("value");
    assert_eq!(value["event"], "deferred_item_captured");
    let parsed: LedgerEvent = serde_json::from_value(value).expect("deserialize");
    assert_eq!(parsed, event);
}

#[test]
fn test_briefing_carries_bidirectional_link_summary() {
    let briefing = TaskContextBriefing {
        task: sample_task(),
        verification_command: Some("cargo nextest run -p outfeed-core".to_string()),
        related_tasks_summary: vec![
            (
                "GB-121".to_string(),
                "Phase 1 epic".to_string(),
                TaskStatus::Verified,
            ),
            (
                "GB-123".to_string(),
                "Wave parallel".to_string(),
                TaskStatus::Ready,
            ),
        ],
        inherited_decisions: Vec::new(),
        file_collision_warnings: Vec::new(),
        known_deferred_issues_in_scope: Vec::new(),
    };

    let value = serde_json::to_value(&briefing).expect("value");
    assert_eq!(value["related_tasks_summary"][0][0], "GB-121");
    assert_eq!(value["related_tasks_summary"][0][2], "verified");

    let parsed: TaskContextBriefing = serde_json::from_value(value).expect("deserialize");
    assert_eq!(briefing, parsed);
}
