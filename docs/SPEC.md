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

### Invariant 6: One process boundary for `git`
`gaugeblock-git` is the only crate in the suite that spawns `git`. It resolves no `.gaugeblock/` path,
chooses no repository and knows no suite vocabulary, so the edge `gaugeblock-workspace` ->
`gaugeblock-git` stays one-way and the crate graph stays acyclic (`GB-131`, `GB-189`).

### Invariant 7: One answer for `.gaugeblock/`
`GaugeblockPaths` is the only resolver of the canonical `.gaugeblock/` directory, and it derives that
directory from the git *common* dir rather than by walking up from the caller's directory — which is
what stops a linked worktree from growing a second ledger and a second spool (`GB-189`, `GB-169`).

---

## 2. Domain coverage

| Domain | Types | Status |
|:---|:---|:---|
| `outfeed` | `ReleasePlan`, `ReleaseStatus`, `CommitInfo`, `ConventionalKind`, `BumpType`, `ManifestDiff`, `SuiteLock`, `SuiteMetadata`, `SuiteToolEntry` | shipped (`GB-141`) |
| `miter` | `Language`, `SymbolKind`, `SymbolBinding`, `LexicalScope`, `VerificationReport`, `CompilerDiagnostic` | shipped (`GB-144`) |
| `storypole` | `StorypoleConfig`, `WorkspaceConfig`, `DiscoveredMember`, `TaskNode`, `TaskKind`, `TaskStatus`, `TaskScope`, `TaskEdge`, `EdgeKind`, `TaskDecision`, `DecisionStatus`, `TaskPlan`, `PlanStep`, `AgentClaim`, `HandoffPacket`, `VerificationReceipt`, `TaskContextBriefing`, `LedgerEvent` | shipped (`GB-145`) |
| `sawhorse` | `ExecutionMode`, `ModelRole`, `ReasoningEffort`, `InferenceRequest`, `InferenceResponse` | shipped (`GB-144`) |
| `dado` / `batten` | `TypedHole`, `DadoPattern`, `CompiledGrammarConstraint`, `GrammarFormat` | shipped (`GB-144`) |
| `telemetry` | `AnonymizedSynthesisTrace`, `TelemetryTier`, `SpoolEntry`, `SyncStatus`, `TelemetrySpan`, `SpanStatus` | shipped (`GB-182`, `GB-243`) |
| `git` | `Repository`, `WorktreeInfo`, `Commit`, `CommitSummary`, `AheadBehind`, `GitError` | shipped (`GB-131`) |
| `workspace` | `GaugeblockPaths`, `WorkspaceTopology`, `MemberRepo`, `SuiteConfig`, `WorkspaceError` | shipped (`GB-189`) |
| `mcp` | `RpcRequest`, `RpcResponse`, `ToolOutcome`, `Route`, `McpHandler`, `AsyncMcpHandler`, `McpError` | shipped (`GB-130`) |
| `inventory` | `WorkspaceInventory`, `CrateInventory`, `CliCommandInventory`, `CliArgInventory`, `PublicTypeInventory`, `GitInventory`, `ToolchainInventory`, `InventoryError` | shipped (`GB-204`) |

### Planned crates

| Domain | Types | Status |
|:---|:---|:---|

### MCP wire rules

`gaugeblock-mcp` is the single answer to what a Model Context Protocol server writes on stdout. Its
boundary rules are the ones the seven hand-written copies disagreed about:

1. **`parse` failure produces a response, not an error.** `McpError::Transport` covers only I/O: a
   malformed line is answered with `-32700` and `id: null`, then the session continues.
2. **A notification is never answered.** Notifications are classified before the method is looked at, so
   a notification for an unknown method is silent rather than `-32601` - JSON-RPC forbids a reply.
3. **A missing tool name is `-32602`, not silence.** Two servers answered nothing at all here, which a
   client cannot distinguish from a hang.
4. **A tool failure is a result.** MCP reserves `error` for protocol faults; a tool that ran and failed
   returns `content` with `isError: true`, and `error` stays for routing faults only.

### Syntax vocabulary and diagnostics

`gaugeblock-syntax` answers one question for the suite: does this source parse, and where exactly is it
malformed. It reads the shared wire enum `gaugeblock_contracts::Language` rather than defining a second
language set, because a second enum is a second table that can disagree with the first (`GB-246`).

`grammar::SyntaxError` is the boundary type:

| Variant | Meaning | Consumer behaviour |
| --- | --- | --- |
| `Unsupported { language }` | A declarative format (SQL, HTML, CSS, Markdown, Docker, HCL, Kubernetes) has no syntax tree to walk | Callers that validate before writing must treat this as "nothing to check", not as a failure |
| `Grammar { language, message }` | The compiled grammar could not be loaded or the parser rejected it | A build defect, not an input defect |
| `Parse { language }` | `tree-sitter` returned no tree | Malformed input, reported like a diagnostic |

`syntax::SyntaxDiagnostic` positions each malformed span for a human (`line`, `column`) and for a fixer
(`start_byte`, `end_byte`), with the snippet sliced on character boundaries so a diagnostic never panics
on multi-byte source. An empty diagnostic vector means the grammar accepted the code: absence of findings
is the success signal, and it is the only one.

### Workspace resolution

`GaugeblockPaths` is the single answer to "where is `.gaugeblock/`" and `WorkspaceError` is the one
way it refuses to answer:

1. **The common dir decides, not the caller.** `discover` derives the directory from the git common
dir, so a linked worktree and its parent resolve to the same `.gaugeblock/`. A worktree that resolved
to its own directory would grow a second ledger and a second spool.
2. **`NotARepository` is a refusal, not a gap.** Walking up to find "some" `.gaugeblock` is the
ambiguity this type removes, so a path outside a repository is an error that names the path and
carries the git failure that established it.
3. **Two constructors, two meanings.** `discover` infers from a directory; `at` accepts a root the
caller already knows. A tool handed a workspace root uses `at` and does not pretend to infer it.

`SuiteConfig` loads `.gaugeblock/config.toml` and its `.env` neighbour and nothing more: the file is a
generic table because a typed struct per tool would put every tool's config type in the foundation
crate, inverting the dependency graph. Its three rules are the ones the per-tool loaders disagreed on:

1. **Absent is empty, malformed is an error.** A checkout that was never configured still runs every
tool; a file that exists but does not parse must never be silently discarded, because that throws away
the user's configuration exactly when they need to be told.
2. **The environment outranks the file.** A shell export or CI secret is a deliberate override; `.env`
is the default a developer edits.
3. **Blank is absent in either source.** An empty `ANTHROPIC_API_KEY=` in a profile or a placeholder
left in a `.env` is a mistake. Returning `""` would hand a provider an empty token instead of
reporting that no credential is configured.

`WorkspaceTopology` keeps discovery and expectation apart: `discover` reports what is on disk and
`expected_members` reports the eleven the suite should contain, so a missing checkout is reportable
rather than invisible - an empty tree and a healthy workspace otherwise look identical.

### Git process boundary

Every `gaugeblock-git` fallible call returns `GitError`, which is the crate's only error type and its
boundary with the rest of the suite. Three rules make it useful rather than decorative:

1. **Stderr is carried.** `Failed` holds the exit code and git's own stderr, trimmed. A caller that
   reports "git failed" without those has thrown away the only part that says which of the plausible
   causes it was - a dirty worktree, a branch checked out elsewhere, a rejected push.
2. **Facts are not faults.** `is_ancestor`, `has_staged_changes` and `is_tracked` map git's exit code
   1 to `Ok(false)`. "Not an ancestor" and "nothing staged" are answers, and surfacing them as
   errors would force every caller to unwrap a non-error.
3. **Refusals are typed where the reason is structural.** `EmptyScope` exists because an empty path
   list is the request that means "stage everything", which is the `GB-150` hazard; the variant
   names the refusal rather than leaving it to a message string.

A caller never inspects `Failed.code` to decide policy. If a distinction matters - present versus
absent, tracked versus untracked - the crate exposes a method that answers it as a boolean or an
`Option`, so the exit-code convention lives in one place.

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

