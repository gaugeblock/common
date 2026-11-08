//! Which suite contains a member repository (`GB-317`).
//!
//! `GaugeblockPaths::discover` answers a *downward* question - given a directory, where is the nearest
//! canonical root - and through git's own repository discovery that answer from inside a member is the
//! member itself. `WorkspaceTopology::discover` takes a root as an argument, so it cannot find one.
//!
//! Between them the suite could describe every member's layout and never answer *which suite contains
//! this member*, which is why a member repository could not reach the suite's backlog, and why
//! `storypole verify` run inside `miter/` reported "task not found" against an empty local store.

use std::path::PathBuf;

use crate::paths::GaugeblockPaths;
use crate::topology::{MemberRepo, WorkspaceTopology};

/// The suite a directory belongs to, and the member it sits within.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    /// Root of the suite that lists this member.
    pub suite_root: PathBuf,
    /// The member this directory belongs to.
    pub member: MemberRepo,
}

impl Membership {
    /// Resolves the suite that contains `start`, if the directory sits inside one of its members.
    ///
    /// The **nearest** suite wins. A directory holding `.gaugeblock` is not automatically a suite for
    /// this question - it must also *list* the containing directory as a member - because adopting any
    /// parent with tool state is the defect that let a nested repository record its work in an
    /// unrelated parent's ledger (`GB-305`).
    ///
    /// `None` means this directory is not inside a member of any suite: a standalone repository, the
    /// suite root itself, or a checkout nested in something that is not a member.
    #[must_use]
    pub fn containing(start: &std::path::Path) -> Option<Self> {
        let start = if start.is_dir() {
            start.to_path_buf()
        } else {
            start.parent()?.to_path_buf()
        };
        // `skip(1)` starts at the parent: a suite root is not a member of itself.
        for ancestor in start.ancestors().skip(1) {
            if !GaugeblockPaths::at(ancestor).root().is_dir() {
                continue;
            }
            // The nearest suite is the only candidate. If it does not list this path as a member, the
            // answer is "not a member" rather than "look further up", because looking further up is
            // exactly how work lands in a ledger it does not belong to.
            let relative = start.strip_prefix(ancestor).ok()?;
            let first = relative
                .components()
                .next()?
                .as_os_str()
                .to_string_lossy()
                .to_string();
            let topology = WorkspaceTopology::discover(ancestor);
            let resolved = topology.member(&first).map(|member| Self {
                suite_root: ancestor.to_path_buf(),
                member: member.clone(),
            })?;
            // Membership is at repository granularity: a checkout nested *inside* a member is a
            // repository of its own, and the suite does not list it. Without this, `miter/vendor`
            // resolves to member `miter` and the nested repository's work lands in the suite ledger it
            // never joined - the defect `GB-305` guarded against, arriving by another door.
            if git_between(&start, &resolved.member.path) {
                return None;
            }
            return Some(resolved);
        }
        None
    }
}

/// Whether a `.git` sits strictly between `start` and `stop`, making `start` part of another repository.
fn git_between(start: &std::path::Path, stop: &std::path::Path) -> bool {
    start
        .ancestors()
        .take_while(|ancestor| *ancestor != stop)
        .any(|ancestor| ancestor.join(".git").exists())
}

#[cfg(test)]
mod tests;
