# Task Plan: GB-189 — `gaugeblock-workspace`

> **Scope:** `common/crates/gaugeblock-workspace` **Parent epic:** `GB-128` **Status:** Planned

## 1. Why this crate exists

Every tool answers "which repository am I in, and where is `.gaugeblock/`?" differently:

| Tool | Today |
|:--|:--|
| `storypole` | `StorypoleStore::open` walks up from `--dir` for `.storypole`; `telemetry::spool_path` takes `.parent()` of the dot dir and hopes |
| `outfeed` | takes a positional root and guesses members from a hard-coded list |
| `chalkline` | `-d` / `--workspace` heuristics |
| `benchwork` | enumerates repos from its own config |
| `joiner` | `JoinerConfig::default_spool_dir` re-implements the spool rule |

That is five answers to one question, and the worktree case (`.git` is a *file* pointing at the real
common dir) breaks all of them — which is precisely what `GB-169` needs to stop happening.

## 2. Boundary (frozen — see `common/ARCHITECTURE.md` §4)

**Owns:** `GaugeblockPaths` (canonical `.gaugeblock/` resolution), `WorkspaceTopology` (meta-harness
root, 11 member repositories, their `Cargo.toml` members), `SuiteConfig` (`.gaugeblock/config.toml`,
`.env`).

**Does not own:** any `git` subprocess. Worktree detection, `common_dir`, branches and commits are
`gaugeblock-git` (`GB-131`). This crate may call it, never the reverse.

## 3. Data contracts

```rust
pub struct GaugeblockPaths {
    pub repo_root: PathBuf,   // the checkout the caller asked about
    pub common_dir: PathBuf,  // git common dir: shared by every linked worktree
    pub dot_dir: PathBuf,     // <common_dir parent>/.gaugeblock — one per repository, not per worktree
}

impl GaugeblockPaths {
    pub fn resolve(start: &Path) -> Result<Self, WorkspaceError>;
    pub fn storypole_dir(&self) -> PathBuf;
    pub fn spool_outbox(&self) -> PathBuf;
}

pub struct WorkspaceTopology {
    pub root: PathBuf,
    pub members: Vec<MemberRepo>, // name, path, Cargo.toml packages
}

pub struct SuiteConfig { /* .gaugeblock/config.toml + .env, typed, no globals */ }
```

Invariants:

1. `dot_dir` is derived from the git common dir, so a linked worktree resolves to the *same*
   `.gaugeblock/` as its main checkout. A test must create a real linked worktree and assert it.
2. `WorkspaceTopology::discover` reads manifests; it never runs `cargo`.
3. `resolve` on a path outside any repository returns a typed error, never a fallback to
   `current_dir` — silent fallbacks are how split-brain state starts.

## 4. Steps

| # | Step | Gate |
|:--|:--|:--|
| 1 | `paths` module + linked-worktree fixture | `cargo nextest -p gaugeblock-workspace` |
| 2 | `topology` module: root discovery + 11 members + package names from `Cargo.toml` | fixture tree with two members |
| 3 | `config` module: typed `SuiteConfig` with `toml` + `.env` | fixture config + missing-file default |
| 4 | Re-point `storypole` (store + `spool_path`), `joiner` (`default_spool_dir`), `chalkline --workspace` | `cargo nextest --workspace` per repo |
| 5 | `outfeed` + `benchwork` member enumeration | Tier 3 |

## 5. What this buys `GB-169` and `GB-180`

* `GB-169` — a claimed worktree and the main checkout write one ledger and one spool because
  `GaugeblockPaths` says so; `storypole claim --worktree` becomes a `gaugeblock-git` worktree call plus
  a path value, not new path logic.
* `GB-180` — Panel 1 (multi-repo Docs Studio) needs the member list and per-repo `.gaugeblock/`
  resolution; it consumes `WorkspaceTopology` instead of hard-coding eleven names.
