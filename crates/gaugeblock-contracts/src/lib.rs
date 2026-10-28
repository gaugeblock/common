//! Wire contracts shared across the `gaugeblock` neurosymbolic suite.
//!
//! Every type here is a **pure serde data carrier**: no I/O, no C dependencies, no async runtime.
//! The definitions mirror the canonical blocks in the meta-harness `CONTRACTS.md`; when the two
//! disagree, `CONTRACTS.md` wins and this crate is corrected to match it.
//!
//! Domains live in submodules so a consumer only pays for the surface it names. Cross-domain
//! references resolve inside this crate (`DadoPattern::language` is [`miter::Language`], not a
//! member engine's type), which is what lets a consumer drop its member `-core` dependency.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod dado_batten;
pub mod miter;
pub mod outfeed;
pub mod sawhorse;
pub mod storypole;

pub use dado_batten::*;
pub use miter::*;
pub use outfeed::*;
pub use sawhorse::*;
pub use storypole::*;
