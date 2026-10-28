//! `storypole` wire domain: configuration, workspace membership, tasks and ledger events.

mod continuity;
mod events;
mod task;

pub use continuity::{AgentClaim, HandoffPacket, PlanStep, TaskPlan, VerificationReceipt};
pub use events::LedgerEvent;
pub use task::{
    DecisionStatus, EdgeKind, TaskContextBriefing, TaskDecision, TaskEdge, TaskKind, TaskNode,
    TaskScope, TaskStatus,
};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Per-repository or meta-harness configuration loaded from `.gaugeblock/config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorypoleConfig {
    /// Human-readable ticket prefix (e.g. `GB`) used to generate sequential keys.
    pub ticket_prefix: String,
    /// Allow-list of valid repository names, enforced when non-empty.
    pub allowed_repos: Vec<String>,
    /// Allow-list of valid crate or package names, enforced when non-empty.
    pub allowed_crates: Vec<String>,
    /// Default lease time-to-live in minutes before an in-progress claim expires.
    pub claim_ttl_minutes: u32,
    /// `(crate_or_repo_name, verification_command)` pairs.
    pub verification_commands: Vec<(String, String)>,
}

/// Configuration defining multi-repository workspace membership.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Member repository paths or names.
    pub members: Vec<String>,
    /// Default ticket prefix for workspace-level tasks.
    pub default_prefix: Option<String>,
}

/// A member repository discovered within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredMember {
    /// Member repository name.
    pub name: String,
    /// Absolute path of the member checkout.
    pub repo_path: PathBuf,
    /// Path of the member's ledger directory, when initialised.
    pub dot_dir: Option<PathBuf>,
    /// Ticket prefix configured for the member.
    pub ticket_prefix: Option<String>,
}
