# Changelog

All notable changes to `gaugeblock/common` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the suite uses semantic versioning.

## [Unreleased]

### Added
- `gaugeblock-contracts`: `outfeed` wire domain — `ReleasePlan`, `ReleaseStatus`, `CommitInfo`,
  `ConventionalKind`, `BumpType`, `ManifestDiff`, `SuiteLock`, `SuiteMetadata`, `SuiteToolEntry`.
  Wire-shape and round-trip tests cover every type. [GB-141]
 - `gaugeblock-common`: facade crate re-exporting the contracts surface. [GB-141]
- `gaugeblock-contracts`: `miter` domain (`Language`, `SymbolKind`, `SymbolBinding`, `LexicalScope`,
  `VerificationReport`, `CompilerDiagnostic`), `dado`/`batten` domain (`TypedHole`, `DadoPattern`,
  `GrammarFormat`, `CompiledGrammarConstraint`) and `sawhorse` domain (`ExecutionMode`, `ModelRole`,
  `ReasoningEffort`, `InferenceRequest`, `InferenceResponse`). Cross-domain references resolve inside
  the crate, and the internally tagged `ExecutionMode` / snake_case enums are covered by wire tests.
  [GB-144]

## [v0.1.0-alpha.1] — 2026-10-28

### Maintenance
- Reserved the `gaugeblock-common` and `gaugeblock-contracts` names on crates.io with placeholder
  crates. [GB-127]
