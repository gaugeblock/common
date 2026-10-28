# Behavioral Specifications & Invariants — `gaugeblock/common`

> **Crates:** `gaugeblock-common`, `gaugeblock-contracts`  
> **Schema source:** meta-harness `CONTRACTS.md`

---

## 1. Invariants

### Invariant 1: Purity
Every type is a plain data carrier: no file, network or process access, and no dependency that compiles
C or C++. The crate must build on any platform the suite supports without a toolchain beyond rustc.

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
| `storypole` | `TaskNode`, `TaskKind`, `TaskStatus`, `TaskScope`, `TaskEdge`, `TaskDecision`, `StorypoleConfig`, `TaskPlan`, `LedgerEvent` and friends | `GB-145` |
| `sawhorse` | `ExecutionMode`, `ModelRole`, `ReasoningEffort`, `InferenceRequest`, `InferenceResponse` | shipped (`GB-144`) |
| `dado` / `batten` | `TypedHole`, `DadoPattern`, `CompiledGrammarConstraint`, `GrammarFormat` | shipped (`GB-144`) |
