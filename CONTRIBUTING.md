# Contributing to gaugeblock/common

## 1. Adding a contract type

1. Find the canonical block in the meta-harness `CONTRACTS.md` (or add it there first, with a ticket).
2. Add the type to the matching domain module (`src/<domain>.rs`) with doc comments on the type and
   every field.
3. Re-export it from `src/lib.rs` (`pub use <domain>::*;`).
4. Extend `tests/<domain>_wire.rs` with a round-trip **and** a field-name assertion.
5. Run the gate:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
```

## 2. Changing a published type

Removing or renaming a field is a breaking change: bump the minor base and note it under
`### Breaking` in `CHANGELOG.md`. Additive fields are pre-release patch bumps.

## 3. Commits

Conventional Commits with the crate scope (`feat(gaugeblock-contracts): ...`) and the task key in the
footer. One logical change per commit.

## 4. Documentation

All nine canonical documents are maintained here. If a change alters behaviour, update `SPEC.md` in the
same commit.
