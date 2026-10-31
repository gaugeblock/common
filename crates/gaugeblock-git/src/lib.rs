//! Git plumbing for the gaugeblock suite (`GB-131`).
//!
//! Six places used to spawn `git` with `format!`-built command lines and parse stdout by hand:
//! `storypole-core::git`, `outfeed-core::git`, the benchwork copy, `miter deps --since`, the rabbet
//! backfill, and a handful of shell scripts. This crate is the single owner of that process boundary.
//!
//! **Owns:** repository detection (`common_dir`, worktree versus main checkout), head sha, branch and
//! dirty state today; `worktree add/remove/prune/list`, history ranges, scoped staging and annotated
//! tags plus a single-round-trip push follow in later steps of the same plan.
//!
//! **Does not own:** which repository to act on, `.gaugeblock/` resolution, workspace configuration,
//! or any suite type. A caller hands it a `Path` and typed arguments; it returns values, or an error
//! carrying stderr. It never depends on `gaugeblock-workspace`: the edge runs one way, `GB-189` to
//! `GB-131`.

mod error;
mod history;
mod index;
mod remote;
mod repo;
mod worktree;

pub use error::GitError;
pub use history::Commit;
pub use index::{CommitSummary, is_tracked};
pub use repo::{Repository, run_git};
pub use worktree::WorktreeInfo;
