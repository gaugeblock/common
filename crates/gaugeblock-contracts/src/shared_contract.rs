//! The neutral wire schema `chalkline` locks so `dado` can emit matching Rust and TypeScript types.
//!
//! These lived in `chalkline-core` and were imported by `dado` and `benchwork`, which made two lateral
//! edges out of a plain data type whose own doc comment already said it was a wire schema (`GB-204`).
//! A contract that three products exchange belongs with the other contracts.

use serde::{Deserialize, Serialize};

/// Neutral wire schema locked by `chalkline` so `dado` can emit matching Rust + TS types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedTypeContract {
    /// Name of the contract type (e.g., "UserSession", "TaskReceipt").
    pub name: String,
    /// Typed fields comprising the contract.
    pub fields: Vec<ContractField>,
}

/// A single field inside a [`SharedTypeContract`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractField {
    /// Field identifier.
    pub name: String,
    /// Canonical neutral type: "uuid", "string", "i64", "vec<string>", "option<i32>".
    pub canonical_type: String,
    /// Whether the field is mandatory.
    pub required: bool,
}
