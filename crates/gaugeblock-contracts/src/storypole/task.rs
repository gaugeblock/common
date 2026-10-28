//! Task graph types: kinds, statuses, edges, decisions, scope and the task node itself.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::continuity::{AgentClaim, HandoffPacket, TaskPlan, VerificationReceipt};

/// Category of backlog work item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Grouping task that owns child work packages.
    Epic,
    /// New capability.
    Feature,
    /// Behaviour-preserving restructuring.
    Refactor,
    /// Defect repair.
    Bug,
    /// Technical debt or maintenance.
    TechDebt,
    /// Time-boxed investigation with an open-ended outcome.
    Spike,
}

/// Lifecycle state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Captured via `storypole defer`; awaits triage before entering `Ready`.
    Triage,
    /// Held by unverified `blocked_by` tasks or unresolved decisions.
    Blocked,
    /// Every hard dependency is verified; eligible for `storypole next`.
    Ready,
    /// Claimed under an active agent lease.
    InProgress,
    /// Active pattern or hole synthesis via `joiner`.
    Synthesizing,
    /// Compiler or test verification via `miter`.
    Verifying,
    /// Passed verification with a recorded receipt.
    Verified,
    /// Intentionally parked for a future milestone.
    Deferred,
    /// Completed and archived out of active backlog views.
    Archived,
    /// Retry budget exhausted or verification failed.
    Failed,
}

/// Semantic relationship between two tasks in the DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Hard topological lock: the target cannot become `Ready` first.
    BlockedBy,
    /// Soft bidirectional association.
    RelatedTo,
    /// Provenance link recorded when an agent defers an issue.
    DiscoveredFrom,
    /// Hierarchical grouping under a parent epic.
    ParentEpic,
}

/// A directed edge to another task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEdge {
    /// Relationship type.
    pub kind: EdgeKind,
    /// Stable identifier of the target task.
    pub target_id: Uuid,
    /// Cached ticket key for cross-repository rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ticket_key: Option<String>,
    /// Cached repository name for cross-repository rendering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_repo: Option<String>,
}

/// State of a decision node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    /// Awaiting an answer; blocks the owning task.
    Pending,
    /// Answered with a chosen option and rationale.
    Resolved,
    /// Replaced by a newer decision.
    Superseded,
}

/// A decision node attached to a task, supporting chained follow-ups and reversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDecision {
    /// Stable decision identifier.
    pub decision_id: Uuid,
    /// Decision this one elaborates on, when chained.
    pub parent_decision_id: Option<Uuid>,
    /// Current decision state.
    pub status: DecisionStatus,
    /// Decision category (e.g. `error_strategy`, `auth_boundary`).
    pub category: String,
    /// The question an owner must answer.
    pub question: String,
    /// Candidate answers presented to the owner.
    pub proposed_options: Vec<String>,
    /// Selected option, once resolved.
    pub chosen_option: Option<String>,
    /// Justification for the selection.
    pub rationale: Option<String>,
    /// Agent or human who resolved it.
    pub resolved_by: Option<String>,
    /// Last mutation timestamp in epoch milliseconds.
    pub updated_at_epoch_ms: u64,
}

/// Multi-dimensional scope coordinates for filtering across repos, crates and symbols.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskScope {
    /// Target repository name.
    pub repo: Option<String>,
    /// Target crate or package name.
    pub crate_name: Option<String>,
    /// Target module path.
    pub module_path: Option<String>,
    /// Files the task expects to touch, used for collision warnings.
    pub target_files: Vec<String>,
    /// Symbols the task expects to change, used for collision warnings.
    pub target_symbols: Vec<String>,
    /// Free-form labels.
    pub labels: Vec<String>,
}

/// A single backlog task node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNode {
    /// Stable task identifier.
    pub id: Uuid,
    /// Human-friendly sequential key (e.g. `GB-3`).
    pub ticket_key: String,
    /// URL-friendly unique slug.
    pub slug: String,
    /// Short title.
    pub title: String,
    /// Full description or briefing.
    pub description: String,
    /// Work item category.
    pub kind: TaskKind,
    /// Current lifecycle state.
    pub status: TaskStatus,
    /// Priority from 1 (highest) to 5 (lowest).
    pub priority: u8,
    /// Scope coordinates.
    pub scope: TaskScope,
    /// Outgoing DAG edges.
    pub edges: Vec<TaskEdge>,
    /// Attached decision nodes.
    pub decisions: Vec<TaskDecision>,
    /// Acceptance criteria that must hold before verification.
    pub acceptance_criteria: Vec<String>,
    /// Attached execution plan.
    pub plan: Option<TaskPlan>,
    /// Active agent lease, when claimed.
    pub active_claim: Option<AgentClaim>,
    /// Most recent continuity handoff.
    pub latest_handoff: Option<HandoffPacket>,
    /// Verification evidence, once certified.
    pub verification_receipt: Option<VerificationReceipt>,
    /// Creation timestamp in epoch milliseconds.
    pub created_at_epoch_ms: u64,
    /// Last mutation timestamp in epoch milliseconds.
    pub updated_at_epoch_ms: u64,
}

/// Assembled context payload returned by `storypole next` / `claim` and the MCP tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskContextBriefing {
    /// The task being briefed.
    pub task: TaskNode,
    /// Verification command configured for the task's crate or repository.
    pub verification_command: Option<String>,
    /// `(ticket_key, title, status)` for every task linked by `RelatedTo`, `BlockedBy` or
    /// `ParentEpic` in **either** direction. Outgoing edges cover dependencies and the parent epic;
    /// the inbound pass surfaces an epic's children, the tasks this one blocks, and siblings that
    /// recorded the link on their own side. Mutual links appear once.
    pub related_tasks_summary: Vec<(String, String, TaskStatus)>,
    /// Resolved decisions inherited from linked tasks.
    pub inherited_decisions: Vec<TaskDecision>,
    /// Warnings about files or symbols already claimed by another active task.
    pub file_collision_warnings: Vec<String>,
    /// Deferred issues whose scope overlaps this task.
    pub known_deferred_issues_in_scope: Vec<(String, String)>,
}
