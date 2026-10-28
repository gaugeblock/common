# Product — gaugeblock/common

> **Users:** every member repository in the `gaugeblock` suite and anyone writing a compatible tool

---

## 1. What it provides

| Capability | Without `common` | With `common` |
|:---|:---|:---|
| Naming a shared wire type | Depend on a member's `-core` engine and inherit SQLite/reqwest/tree-sitter | Depend on a 2-second, pure-`serde` crate |
| Wire compatibility | Each tool re-declares structs and drifts silently | One definition, asserted against `CONTRACTS.md` |
| Interop with external tools | Reverse-engineer JSON from each binary | Import the published contract types |

## 2. Non-goals

* No orchestration logic — that stays in `outfeed-core`, `storypole-core`, `joiner-core`.
* No MCP transport — that becomes `gaugeblock-mcp` (`GB-130`).
* No git plumbing — that becomes `gaugeblock-git` (`GB-131`).

## 3. Success criteria

1. A downstream crate can drop a member `-core` dependency and still name every wire type it needs.
2. `cargo check` for a contracts-only consumer stays under three seconds.
3. Adding a contract type requires one file edit plus one wire-shape test.
