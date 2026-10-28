# Roadmap — gaugeblock/common

> **Suite context:** meta-harness epic `GB-128`

---

## Milestone 1 — Contracts bootstrap (current)

| Item | Ticket | Status |
|:---|:---|:---|
| Scaffold the repository and land the `outfeed` wire domain | `GB-141` | shipped |
| Add the `miter`, `storypole`, `sawhorse`, `dado` and `batten` domains | `GB-142` | next |
| Repoint downstream crates and delete the duplicated definitions | `GB-143` | blocked on `GB-142` |

## Milestone 2 — Transport and plumbing

| Item | Ticket | Notes |
|:---|:---|:---|
| `gaugeblock-mcp`: `McpToolHandler` + generic JSON-RPC stdio loop | `GB-130` | replaces seven hand-rolled servers |
| `gaugeblock-git`: porcelain + single-round-trip push | `GB-131` | consumes the `GB-126` helper |

## Milestone 3 — Publication

1. Publish `gaugeblock-contracts` and `gaugeblock-common` at `0.1.0-beta.1` once the surface is complete.
2. Repoint members, then flip each member's dependency from a member `-core` crate to the contracts
   crate in its own release.
3. Cut the meta-harness snapshot tag once every member builds against the published contracts.
