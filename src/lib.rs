//! # `gaugeblock-common`
//!
//! Foundation library shared by every `gaugeblock` repository.
//!
//! The facade re-exports [`gaugeblock_contracts`], the single code manifestation of the suite's
//! `CONTRACTS.md` wire types. Member repositories depend on this crate instead of transitively
//! linking another member's `-core` engine just to name a shared struct.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use gaugeblock_contracts::*;
