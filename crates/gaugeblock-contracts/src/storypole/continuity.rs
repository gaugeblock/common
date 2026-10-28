//! Continuity types: plans, leases, handoff packets and verification receipts.

use serde::{Deserialize, Serialize};

/// One execution step inside an attached plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step identifier within the plan.
    pub step_id: String,
    /// What the step does.
    pub title: String,
    /// Files the step expects to touch.
    pub target_files: Vec<String>,
    /// Command that proves the step is complete.
    pub verification_gate: Option<String>,
    /// Whether the step has been completed.
    pub completed: bool,
}

/// Attached architectural plan and execution blueprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlan {
    /// One-line plan summary.
    pub summary: String,
    /// Full design markdown, normally persisted under the plans directory.
    pub design_markdown: String,
    /// Ordered execution steps.
    pub steps: Vec<PlanStep>,
    /// Repository-relative path of the persisted plan file.
    pub plan_file_path: Option<String>,
    /// Last mutation timestamp in epoch milliseconds.
    pub updated_at_epoch_ms: u64,
}

/// Active agent claim lease, preventing parallel agents from colliding on one task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentClaim {
    /// Identifier of the claiming agent.
    pub agent_id: String,
    /// Claim timestamp in epoch milliseconds.
    pub claimed_at_epoch_ms: u64,
    /// Lease expiry in epoch milliseconds.
    pub lease_expires_at_epoch_ms: u64,
    /// Branch the agent works on.
    pub git_branch: Option<String>,
    /// Commit the branch started from.
    pub base_commit_sha: Option<String>,
}

/// Structured continuity checkpoint recorded when an agent pauses, yields or fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffPacket {
    /// Agent that recorded the packet.
    pub recorded_by_agent: String,
    /// Recording timestamp in epoch milliseconds.
    pub timestamp_ms: u64,
    /// Work finished in this session.
    pub completed_items: Vec<String>,
    /// Work still outstanding.
    pub remaining_items: Vec<String>,
    /// Last error or blocker encountered.
    pub last_error_or_blocker: Option<String>,
    /// Git reference the handoff was taken at.
    pub git_ref: Option<String>,
    /// Files left uncommitted.
    pub dirty_files: Vec<String>,
}

/// Evidence required to transition a task into `TaskStatus::Verified`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReceipt {
    /// Tool or system that ran verification (e.g. `miter-engine`, `cargo-test`, `human`).
    pub verifier: String,
    /// Exact command or suite executed.
    pub command: String,
    /// Process exit code; `0` indicates success.
    pub exit_code: Option<i32>,
    /// Count of compiler warnings or diagnostics emitted.
    pub diagnostic_count: Option<u32>,
    /// Execution output summary or stdout/stderr snippet.
    pub summary: String,
    /// Verification timestamp in epoch milliseconds.
    pub verified_at_epoch_ms: u64,
    /// Branch verification ran on.
    #[serde(default)]
    pub git_branch: Option<String>,
    /// Commit the branch started from.
    #[serde(default)]
    pub base_commit_sha: Option<String>,
    /// Certified head commit.
    #[serde(default)]
    pub head_commit_sha: Option<String>,
    /// Ordered commits covering the branch delta.
    #[serde(default)]
    pub commit_shas: Vec<String>,
}

impl AgentClaim {
    /// Returns true once the lease has expired at `current_epoch_ms`.
    pub fn is_expired(&self, current_epoch_ms: u64) -> bool {
        current_epoch_ms >= self.lease_expires_at_epoch_ms
    }
}
