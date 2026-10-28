//! Behaviour that travels with the contract types (option A of the GB-143 decision).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::{
    AgentClaim, GrammarFormat, InferenceRequest, ModelRole, ReasoningEffort, TypedHole,
};

#[test]
fn test_inference_request_builders_default_to_deterministic_output() {
    let request = InferenceRequest::new(ModelRole::HoleFiller, "fill the hole");
    assert_eq!(request.max_tokens, 1024);
    assert_eq!(request.temperature, 0.0, "deterministic by default");
    assert!(request.grammar_constraint.is_none());
    assert!(request.system_prompt.is_none());

    let request = request
        .with_system_prompt("emit code only")
        .with_reasoning_effort(ReasoningEffort::Low)
        .with_gbnf_grammar("root ::= expr");
    assert_eq!(request.system_prompt.as_deref(), Some("emit code only"));
    assert_eq!(request.reasoning_effort, Some(ReasoningEffort::Low));
    assert_eq!(request.grammar_format, Some(GrammarFormat::Gbnf));
    assert_eq!(request.grammar_constraint.as_deref(), Some("root ::= expr"));

    let lark = InferenceRequest::new(ModelRole::Planner, "plan").with_lark_grammar("start: expr");
    assert_eq!(lark.grammar_format, Some(GrammarFormat::Lark));
}

#[test]
fn test_typed_hole_constructor_defaults() {
    let hole = TypedHole::new("body", "Result<Json<T>, AppError>", "block");
    assert_eq!(hole.hole_id, "body");
    assert_eq!(hole.max_tokens, 64);
    assert!(hole.allow_literals);
}

#[test]
fn test_grammar_format_display_matches_wire_spelling() {
    for (format, expected) in [
        (GrammarFormat::Gbnf, "gbnf"),
        (GrammarFormat::Lark, "lark"),
        (GrammarFormat::Regex, "regex"),
    ] {
        assert_eq!(format.to_string(), expected);
        assert_eq!(
            serde_json::to_string(&format).expect("serialize"),
            format!("\"{expected}\"")
        );
    }
}

#[test]
fn test_agent_claim_expiry_is_inclusive_of_the_deadline() {
    let claim = AgentClaim {
        agent_id: "cline".to_string(),
        claimed_at_epoch_ms: 1_000,
        lease_expires_at_epoch_ms: 2_000,
        git_branch: None,
        base_commit_sha: None,
    };
    assert!(!claim.is_expired(1_999));
    assert!(claim.is_expired(2_000), "the deadline itself is expired");
    assert!(claim.is_expired(9_999));
}
