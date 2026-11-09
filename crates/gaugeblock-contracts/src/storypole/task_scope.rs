//! The coordinates a task claims to touch (`GB-343`).
//!
//! Lifted out of `task.rs` when the kind registry pushed that file past the 250-line limit. The split is
//! mechanical and the type is re-exported, so no caller path changed.

use serde::{Deserialize, Serialize};

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
