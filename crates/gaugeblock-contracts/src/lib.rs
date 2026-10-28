//! Wire contracts shared across the `gaugeblock` neurosymbolic suite.
//!
//! Every type here is a **pure serde data carrier**: no I/O, no C dependencies, no async runtime.
//! The definitions mirror the canonical blocks in the meta-harness `CONTRACTS.md`; when the two
//! disagree, `CONTRACTS.md` wins and this crate is corrected to match it.
//!
//! Domains live in submodules so a consumer only pays for the surface it names.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod outfeed;

pub use outfeed::*;
