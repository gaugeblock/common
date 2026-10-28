# `gaugeblock-common` — Shared Foundation for the gaugeblock Suite

[![Version](https://img.shields.io/badge/version-0.1.0--beta.1-blue.svg)](https://gaugeblock.dev)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0.svg)](LICENSE)
[![suite: gaugeblock](https://img.shields.io/badge/suite-gaugeblock-orange.svg)](https://github.com/gaugeblock)

Part of the **[`gaugeblock`](https://gaugeblock.dev)** neurosymbolic code synthesis ecosystem.

> A *common* is the shared ground between two pieces of joinery: the reference surface both parts are
> cut to. This repository is that reference for the suite — the one place every tool agrees on the
> shape of a `ReleasePlan`, a `Task` or an `InferenceRequest`.

---

## 1. Why it exists

Downstream repositories used to link a member's `-core` engine just to name a shared struct, which
pulled bundled SQLite, `reqwest`/`rustls` and tree-sitter grammars into every build and publish cycle.
`gaugeblock-contracts` carries the wire types alone: pure `serde`, no C dependencies, compiling in
under two seconds.

## 2. Crate topology

| Crate | Path | Responsibility |
|:---|:---|:---|
| `gaugeblock-common` | `.` | Facade re-exporting every contract domain |
| `gaugeblock-contracts` | `crates/gaugeblock-contracts` | Pure wire types mirroring `CONTRACTS.md` |

## 3. Usage

```rust
use gaugeblock_common::{BumpType, ReleasePlan};
```

## 4. Status

- `gaugeblock-common` / `gaugeblock-contracts` names reserved on crates.io at **`0.1.0-alpha.1`**.
- This workspace is the dogfood-ready **`0.1.0-beta.1`** line: reservations stay `alpha.1`, real
  crates ship `beta.1`.
- Contract surface so far: the `outfeed` release/`suite.lock` domain (`GB-141`). Miter, storypole,
  sawhorse, dado and batten domains follow in `GB-142`; downstream repointing follows in `GB-143`.

## 5. Verification

```bash
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
chalkline docs check -d common
```
