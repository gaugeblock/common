# gaugeblock-contracts

Wire contracts shared across the gaugeblock neurosymbolic suite.

Every type is a pure `serde` data carrier mirroring the canonical blocks in the meta-harness
`CONTRACTS.md`; there are no C dependencies, no I/O and no async runtime.

```rust
use gaugeblock_contracts::{BumpType, ReleasePlan};
```

See [gaugeblock.dev](https://gaugeblock.dev) for suite documentation.
