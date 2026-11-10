# Task Plan: GB-182 — One spool file, one record shape

> **Key:** `GB-182` **Scope:** `common/crates/gaugeblock-contracts`, `storypole-core`, `storypole-cli`,
> `joiner-core`, `rabbet-core` **Status:** Verified

## 1. Symptom

A single `<canonical .gaugeblock>/spool/outbox.jsonl` held records from two writers, and the same file
could not be read by one parser:

| Writer | Line it appended | Readable by `rabbet`? |
|:--|:--|:--|
| `storypole verify` | bare `AnonymizedSynthesisTrace` | no - no envelope, no hash |
| `joiner fill` | `{"trace":…,"content_hash":…,"sync_status":…,"recorded_at_ms":…}` | yes |
| `rabbet` model | its own `SpoolEntry` with `sync_status: String` | accepted any status verbatim |

The live outbox at `GB-179` close-out held exactly this mix: 2 bare traces (GB-165, GB-167) and 2
`joiner` envelopes.

## 2. Decisions

1. **One owner.** `SpoolEntry` and `SyncStatus` move to `gaugeblock-contracts`, beside the trace they
   wrap. Everything else re-exports; no path breaks.
2. **The envelope is the wire type.** A bare trace is not a record: a reader cannot recompute a hash
   the writer never recorded.
3. **One hash implementation.** `SpoolEntry::hash_trace` (SHA-256 over `serde_json` bytes, lowercase
   hex) is what writers record and what the `rabbet` gate recomputes. `joiner`'s `compute_trace_hash`
   and `rabbet`'s `compute_canonical_trace_hash` delegate to it; both keep their signature.
4. **`sync_status` stays a snake_case string.** `pending` | `synced` | `rejected` as an enum keeps
   today's outboxes parseable, so migration is additive rather than a rewrite of every file.
5. **Migration is idempotent and non-destructive.** `storypole spool migrate` rewrites bare-trace
   lines, leaves envelopes byte-for-byte, preserves unparsable lines verbatim, and writes only when
   something changed - a second run reports 0.

## 3. Steps and gates

| # | Step | Gate |
|:--|:--|:--|
| 3.1 | `gaugeblock-contracts`: own `SpoolEntry`/`SyncStatus` + `from_trace`/`hash_trace` | `cargo nextest -p gaugeblock-contracts` |
| 3.2 | `storypole-core`: write envelopes, add `migrate_spool`/`normalize_spool_text` | `cargo nextest -p storypole-core` |
| 3.3 | `storypole-cli`: `spool migrate [--dry-run]`; split `args.rs` under 250 LOC | `storypole spool --help` |
| 3.4 | `joiner-core`: re-export shared types, delegate the hash, drop `sha2` | `cargo nextest -p joiner-core` |
| 3.5 | `rabbet-core`: consume shared types, delegate the gate hash | `cargo nextest -p rabbet-core` |
| 3.6 | Docs: `CONTRACTS.md` §6, `common/SPEC.md`, `storypole/README.md` | `chalkline docs check` 0 drift |

Tier 2 for every repo: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo nextest run --workspace`, `cargo test --doc --workspace`.

## 4. Evidence (run, not asserted)

* Gate matrix green in `common`, `storypole`, `joiner`, `rabbet` (fmt/clippy/nextest/doc all exit 0).
* `storypole spool migrate --dry-run` -> `2 legacy record(s) would be rewritten`; `migrate` -> `2`
  rewritten; second run -> `no legacy records to migrate (already canonical)`.
* Both `joiner` envelopes kept their hashes across the migration (`e9750e36…`, `b2f96682…`).
* Ingest proof over the live outbox via `POST /api/v1/traces` on `rabbet serve`:
  `{"accepted":5,"duplicates":0,"rejected":0}` - 4 migrated/legacy records plus the GB-182 trace
  `storypole verify` wrote through the new envelope writer.

## 5. Deferred, not fixed here

* `benchwork` owns a third writer path; it now builds against the shared type but has no
  outbox-migration verb of its own.
* Outbox compaction (rewriting `sync_status` in place) is still `joiner mark_synced` only.
