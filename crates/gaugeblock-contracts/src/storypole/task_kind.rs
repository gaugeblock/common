//! `TaskKind` helpers: the one list of kinds, their wire spellings, and the tolerant parse (`GB-343`).
//!
//! Adding `docs` by hand required five enumerations to agree - the contract enum, the CLI value list, two
//! parsers and the MCP schema - and three were found only by searching. So the list lives here, and every
//! consumer derives from it.

use super::TaskKind;

impl TaskKind {
    /// Every kind, in the order they are advertised to callers.
    pub const ALL: [Self; 13] = [
        Self::Epic,
        Self::Feature,
        Self::Refactor,
        Self::Bug,
        Self::TechDebt,
        Self::Spike,
        Self::Chore,
        Self::Docs,
        Self::Perf,
        Self::Test,
        Self::Style,
        Self::Ci,
        Self::Build,
    ];

    /// The wire spelling, as serialised into every ledger and advertised in every schema.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Epic => "epic",
            Self::Feature => "feature",
            Self::Refactor => "refactor",
            Self::Bug => "bug",
            Self::TechDebt => "tech_debt",
            Self::Spike => "spike",
            Self::Chore => "chore",
            Self::Docs => "docs",
            Self::Perf => "perf",
            Self::Test => "test",
            Self::Style => "style",
            Self::Ci => "ci",
            Self::Build => "build",
        }
    }

    /// A kind from its wire spelling, tolerating the spellings people type.
    ///
    /// `None` rather than a default: interpreting a misspelled kind as `Feature` is what silently filed
    /// `--kind perf` as a feature (`GB-196`), so the caller decides what an unknown value means.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        let normalised = value.to_lowercase().replace('-', "_");
        let normalised = if normalised == "techdebt" {
            "tech_debt".to_string()
        } else {
            normalised
        };
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == normalised)
    }
}
