# Behavioral Specifications & Invariants — `gaugeblock/common`

> **Crates:** `gaugeblock-common`, `gaugeblock-contracts`  
> **Schema source:** meta-harness `CONTRACTS.md`

---

## 1. Invariants

### Invariant 1: Purity
Every type is a plain data carrier: no file, network or process access, and no dependency that compiles
C or C++. The crate must build on any platform the suite supports without a toolchain beyond rustc.

Behaviour travels with the type, but only *pure* behaviour: infallible builders (`InferenceRequest::new`,
`TypedHole::new`), formatting (`Display for GrammarFormat`) and value predicates
(`AgentClaim::is_expired`). No function here may perform I/O, spawn a process, or read a clock — anything
that needs the outside world belongs to the member crate that owns that concern.

### Invariant 2: Wire fidelity
Serialized field names, variant names and optional-field behaviour match `CONTRACTS.md` exactly.
Optional fields marked as omit-when-absent stay absent from the wire (`skip_serializing_if`).

### Invariant 3: Lossless round-trip
`serde_json::from_str(&serde_json::to_string(&value)?)?` reproduces the original value for every type in
the surface; this is asserted per domain in `crates/gaugeblock-contracts/tests/`.

### Invariant 4: Additive evolution
Adding a field or a variant is a pre-release patch bump. Removing or renaming a published field, or
changing its type, is breaking and bumps the minor base (`0.1.0-beta.1` -> `0.2.0-beta.1`).

### Invariant 5: No member may be required to build it
`gaugeblock-contracts` must never depend on a member repository. The dependency direction is always
member -> contracts.

---

## 2. Domain coverage

| Domain | Types | Status |
|:---|:---|:---|
| `outfeed` | `ReleasePlan`, `ReleaseStatus`, `CommitInfo`, `ConventionalKind`, `BumpType`, `ManifestDiff`, `SuiteLock`, `SuiteMetadata`, `SuiteToolEntry` | shipped (`GB-141`) |
| `miter` | `Language`, `SymbolKind`, `SymbolBinding`, `LexicalScope`, `VerificationReport`, `CompilerDiagnostic` | shipped (`GB-144`) |
| `storypole` | `StorypoleConfig`, `WorkspaceConfig`, `DiscoveredMember`, `TaskNode`, `TaskKind`, `TaskStatus`, `TaskScope`, `TaskEdge`, `EdgeKind`, `TaskDecision`, `DecisionStatus`, `TaskPlan`, `PlanStep`, `AgentClaim`, `HandoffPacket`, `VerificationReceipt`, `TaskContextBriefing`, `LedgerEvent` | shipped (`GB-145`) |
| `sawhorse` | `ExecutionMode`, `ModelRole`, `ReasoningEffort`, `InferenceRequest`, `InferenceResponse` | shipped (`GB-144`) |
| `dado` / `batten` | `TypedHole`, `DadoPattern`, `CompiledGrammarConstraint`, `GrammarFormat` | shipped (`GB-144`) |
| `telemetry` | `AnonymizedSynthesisTrace`, `TelemetryTier`, `SpoolEntry`, `SyncStatus` | shipped (`GB-182`) |

### Telemetry outbox record

One line of `<canonical .gaugeblock>/spool/outbox.jsonl` is one `SpoolEntry`. `storypole`, `joiner` and
`benchwork` all append this shape and `rabbet` integrates against it, so the envelope - not the bare
trace - is the boundary type.

1. `content_hash` is `SpoolEntry::hash_trace(&trace)`: SHA-256 over `serde_json::to_vec(&trace)`,
   lowercase hex. There is exactly one implementation of that rule, and every reader recomputes it.
2. `sync_status` serialises as `pending` | `synced` | `rejected` and defaults to `pending`. Because it
   is a snake_case string rather than a number or a struct, outboxes written before the type was
   shared still parse.
3. A line that parses as neither `SpoolEntry` nor a bare `AnonymizedSynthesisTrace` is preserved
   verbatim by migration rather than discarded.

`SpoolEntry::from_trace` is the only constructor that decides the hash; assembling the struct literal
by hand is how a writer and a reader disagree about a record.

