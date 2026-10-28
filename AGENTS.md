# AGENTS.md — gaugeblock/common

Operational directives for autonomous agents working in this repository. The suite-wide laws live in
the meta-harness `AGENTS.md`; this file records what is specific to `common`.

## 1. Non-negotiables

1. **Contracts are data only.** No I/O, no `reqwest`, no `rusqlite`, no tree-sitter, no async runtime
   in any crate here. A dependency that compiles C code does not belong in this workspace.
2. **`CONTRACTS.md` is canonical.** Every type here mirrors a block in the meta-harness
   `CONTRACTS.md`. If the two disagree, fix the crate — never "fix" the doc to match accidental code.
3. **250-line ceiling per file**, enforced by review and `miter audit`. Split into submodules early.
4. **`unwrap_used` / `expect_used` are denied** at workspace level; tests opt out with a module-level
   `#![allow(clippy::unwrap_used, clippy::expect_used)]`.

## 2. Tiered Verification Matrix

* **Tier 1 (inner loop):** `cargo check -p gaugeblock-contracts` while iterating.
* **Tier 2 (atomic commit gate):** `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo nextest run --workspace`.
* **Tier 3 (task completion gate):** Tier 2 plus `chalkline docs check -d common` and
  `miter contracts` run from the meta-harness root.
* **Tier 4 (release gate):** the full suite matrix across all workspace targets before tagging.

## 3. Workflow

* Claim the ticket in Storypole before touching code, work on a branch, and defer anything out of
  scope with `storypole defer` instead of fixing it inline.
* Conventional Commits with the crate scope; documentation-only changes use `docs:`; put the task key
  in the footer (`Task: GB-141`).

## 4. Versioning

Reservations publish at `0.1.0-alpha.1`; the workspace is at `0.1.0-beta.1` because it is dogfood-ready.
Removing or renaming a published field is a breaking change and bumps the minor base.
