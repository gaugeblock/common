# Design Decisions — gaugeblock/common

> **Scope:** why the foundation is shaped this way

---

## ADR-1: One contracts crate, not one per domain

**Context.** Six member repositories need overlapping wire types.
**Decision.** A single `gaugeblock-contracts` crate with per-domain submodules.
**Consequences.** One version to bump, one place to check against `CONTRACTS.md`; consumers that need
one domain still link a crate containing all of them (measured: ~2 s cold build). Splitting later is
additive, merging is not.

## ADR-2: Facade crate exists only to re-export

**Context.** Members should not need to know the sub-crate path.
**Decision.** `gaugeblock-common` re-exports `gaugeblock_contracts::*` and adds no logic.
**Consequences.** The reserved name `gaugeblock-common` is genuinely useful, not just a placeholder;
adding `gaugeblock-mcp` / `gaugeblock-git` later keeps the facade as the single import surface.

## ADR-3: `serde` only, no `serde_with` or schema generators

**Context.** Wire fidelity matters more than convenience.
**Decision.** Plain derives; optional-field omission handled with `skip_serializing_if`.
**Consequences.** Field names are literal and reviewable in one line each; no hidden attribute
machinery to debug when a consumer sees a different shape.

## ADR-4: Tests assert wire shape, not just round-trips

**Context.** A round-trip passes even if a field is renamed on both sides.
**Decision.** Every domain test asserts the presence of each documented JSON key and the exact
variant spelling of enums.
**Consequences.** Renaming a published field fails the suite before it reaches a consumer.
