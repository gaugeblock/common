//! Wire representation of `ReasoningEffort` (GB-221).
//!
//! The enum is serialised into inference envelopes, so its string form is a contract, not an
//! implementation detail. `XHigh` is the one tier whose canonical upstream token (`xhigh`) differs from
//! what `snake_case` would produce, which is why it carries an explicit rename.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::ReasoningEffort;

#[test]
fn test_every_tier_round_trips_through_serde() {
    for (effort, wire) in [
        (ReasoningEffort::None, "none"),
        (ReasoningEffort::Low, "low"),
        (ReasoningEffort::Medium, "medium"),
        (ReasoningEffort::High, "high"),
        (ReasoningEffort::XHigh, "xhigh"),
        (ReasoningEffort::Max, "max"),
    ] {
        let encoded = serde_json::to_string(&effort).expect("serialize");
        assert_eq!(encoded, format!("\"{wire}\""));
        let decoded: ReasoningEffort = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded, effort);
    }
}

#[test]
fn test_xhigh_uses_the_canonical_wire_string_not_snake_case() {
    assert_eq!(
        serde_json::to_string(&ReasoningEffort::XHigh).expect("serialize"),
        "\"xhigh\"",
        "`x_high` would not round-trip against a provider that speaks xhigh"
    );
    assert!(
        serde_json::from_str::<ReasoningEffort>("\"x_high\"").is_err(),
        "the snake_case spelling must not silently parse as a different tier"
    );
}

#[test]
fn test_unknown_upstream_tiers_are_rejected_rather_than_guessed() {
    // The set is closed at six. A provider token the catalog cannot map is dropped there, so the
    // contract must not invent a tier here either.
    assert!(serde_json::from_str::<ReasoningEffort>("\"extreme\"").is_err());
    assert!(serde_json::from_str::<ReasoningEffort>("\"minimal\"").is_err());
}
