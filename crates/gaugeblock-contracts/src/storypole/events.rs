//! Append-only ledger events written to `ledger.jsonl`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::continuity::{AgentClaim, HandoffPacket, VerificationReceipt};
use super::task::{TaskDecision, TaskEdge, TaskNode, TaskScope, TaskStatus};

/// Immutable append-only event written to the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LedgerEvent {
    /// A task was created.
    TaskCreated {
        /// The new task.
        task: TaskNode,
    },
    /// Mutable fields of a task were edited.
    TaskUpdated {
        /// Task that changed.
        task_id: Uuid,
        /// New title, when changed.
        title: Option<String>,
        /// New description, when changed.
        description: Option<String>,
        /// New priority, when changed.
        priority: Option<u8>,
        /// New scope, when changed.
        scope: Option<TaskScope>,
        /// New acceptance criteria, when changed.
        acceptance_criteria: Option<Vec<String>>,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// A task moved between lifecycle states.
    StatusChanged {
        /// Task that moved.
        task_id: Uuid,
        /// Previous state.
        from: TaskStatus,
        /// New state.
        to: TaskStatus,
        /// Why the state changed.
        reason: Option<String>,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// An edge was added to the DAG.
    EdgeAdded {
        /// Source task.
        task_id: Uuid,
        /// Edge that was added.
        edge: TaskEdge,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// An edge was removed from the DAG.
    EdgeRemoved {
        /// Source task.
        task_id: Uuid,
        /// Edge that was removed.
        edge: TaskEdge,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// A decision was attached to a task.
    DecisionRaised {
        /// Owning task.
        task_id: Uuid,
        /// The raised decision.
        decision: TaskDecision,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// A decision was answered.
    DecisionResolved {
        /// Owning task.
        task_id: Uuid,
        /// Decision that was answered.
        decision_id: Uuid,
        /// Selected option.
        chosen_option: String,
        /// Justification.
        rationale: String,
        /// Agent or human who answered.
        resolved_by: String,
        /// Follow-up decision spawned by the answer.
        spawned_followup: Option<TaskDecision>,
        /// Acceptance criterion the answer appended.
        appended_criterion: Option<String>,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// A decision was replaced by a newer one.
    DecisionSuperseded {
        /// Owning task.
        task_id: Uuid,
        /// Decision that was superseded.
        decision_id: Uuid,
        /// Why it was superseded.
        reason: String,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// An agent acquired a lease on a task.
    TaskClaimed {
        /// Claimed task.
        task_id: Uuid,
        /// The lease.
        claim: AgentClaim,
    },
    /// A lease was released without completing the task.
    TaskReleased {
        /// Released task.
        task_id: Uuid,
        /// Agent that released it.
        agent_id: String,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// A continuity packet was recorded.
    HandoffRecorded {
        /// Task handed off.
        task_id: Uuid,
        /// The packet.
        handoff: HandoffPacket,
        /// State the task moved to.
        new_status: TaskStatus,
    },
    /// A task was split into children.
    TaskSplit {
        /// Parent task.
        parent_task_id: Uuid,
        /// Children created by the split.
        spawned_children: Vec<TaskNode>,
        /// Mutation timestamp in epoch milliseconds.
        timestamp_ms: u64,
    },
    /// A task was verified.
    TaskVerified {
        /// Verified task.
        task_id: Uuid,
        /// Verification evidence.
        receipt: VerificationReceipt,
    },
    /// An out-of-band issue was captured.
    DeferredItemCaptured {
        /// The captured task.
        task: TaskNode,
        /// Agent that captured it.
        captured_by_agent: String,
        /// Task it was discovered from, when applicable.
        discovered_from_task_id: Option<Uuid>,
    },
}
