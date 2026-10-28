//! Wire-shape regression tests for the miter, dado/batten and sawhorse domains.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_contracts::{
    CompiledGrammarConstraint, CompilerDiagnostic, DadoPattern, ExecutionMode, GrammarFormat,
    InferenceRequest, InferenceResponse, Language, LexicalScope, ModelRole, ReasoningEffort,
    SymbolBinding, SymbolKind, TypedHole, VerificationReport,
};

#[test]
fn test_lexical_scope_roundtrip_and_snake_case_kinds() {
    let scope = LexicalScope {
        repo_relative_path: "crates/outfeed-core/src/lib.rs".to_string(),
        language: Language::Rust,
        enclosing_symbol: Some("release_repo".to_string()),
        bindings: vec![SymbolBinding {
            name: "plan".to_string(),
            ty: "ReleasePlan".to_string(),
            is_mutable: false,
            kind: SymbolKind::LocalVar,
        }],
        available_imports: vec!["use std::path::Path;".to_string()],
    };

    let json = serde_json::to_string(&scope).expect("serialize");
    let parsed: LexicalScope = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(scope, parsed);

    let value = serde_json::to_value(&scope).expect("value");
    assert_eq!(value["language"], "Rust");
    assert_eq!(value["bindings"][0]["kind"], "local_var");
    for key in [
        "repo_relative_path",
        "language",
        "enclosing_symbol",
        "bindings",
        "available_imports",
    ] {
        assert!(value.get(key).is_some(), "missing LexicalScope.{key}");
    }
}

#[test]
fn test_verification_report_carries_diagnostics() {
    let report = VerificationReport {
        ast_valid: true,
        typecheck_passed: false,
        tests_passed: None,
        diagnostics: vec![CompilerDiagnostic {
            file: "src/lib.rs".to_string(),
            line: 12,
            column: 5,
            code: Some("E0308".to_string()),
            message: "mismatched types".to_string(),
            suggestion: Some("expect `u32`".to_string()),
        }],
        duration_ms: 42,
    };

    let json = serde_json::to_string(&report).expect("serialize");
    let parsed: VerificationReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, parsed);
    assert_eq!(
        parsed.tests_passed, None,
        "unrun tests stay absent, not false"
    );
}

#[test]
fn test_dado_pattern_reuses_the_contract_language_enum() {
    let pattern = DadoPattern {
        pattern_id: "rust.axum.handler.sqlx_tx.v1".to_string(),
        language: Language::Rust,
        description: "axum handler wrapped in a sqlx transaction".to_string(),
        template_source: "async fn {handler}(...) { ... }".to_string(),
        required_imports: vec!["use sqlx::PgPool;".to_string()],
        holes: vec![TypedHole {
            hole_id: "body".to_string(),
            expected_type: "Result<Json<T>, AppError>".to_string(),
            ast_node_kind: "block".to_string(),
            max_tokens: 256,
            allow_literals: false,
        }],
    };

    let json = serde_json::to_string(&pattern).expect("serialize");
    let parsed: DadoPattern = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(pattern, parsed);
}

#[test]
fn test_grammar_constraint_serialises_snake_case_format() {
    let constraint = CompiledGrammarConstraint {
        hole_id: "body".to_string(),
        format: GrammarFormat::Gbnf,
        grammar_definition: "root ::= expr".to_string(),
        allowed_identifiers: vec!["pool".to_string()],
    };
    let value = serde_json::to_value(&constraint).expect("value");
    assert_eq!(value["format"], "gbnf");
}

#[test]
fn test_execution_mode_is_internally_tagged() {
    let mode = ExecutionMode::OpenAiCompatible {
        base_url: "https://api.example.com".to_string(),
        api_key_env: "GAUGEBLOCK_API_KEY".to_string(),
        model_name: "router-small".to_string(),
    };
    let value = serde_json::to_value(&mode).expect("value");
    assert_eq!(value["mode"], "open_ai_compatible");
    assert_eq!(value["base_url"], "https://api.example.com");

    let parsed: ExecutionMode = serde_json::from_value(value).expect("deserialize");
    assert_eq!(parsed, mode);
}

#[test]
fn test_model_role_and_reasoning_effort_snake_case() {
    assert_eq!(
        serde_json::to_string(&ModelRole::System1Router).expect("serialize"),
        "\"system1_router\""
    );
    assert_eq!(
        serde_json::to_string(&ModelRole::HoleFiller).expect("serialize"),
        "\"hole_filler\""
    );
    assert_eq!(
        serde_json::to_string(&ReasoningEffort::None).expect("serialize"),
        "\"none\""
    );
}

#[test]
fn test_inference_messages_roundtrip() {
    let request = InferenceRequest {
        role: ModelRole::HoleFiller,
        prompt: "fill the hole".to_string(),
        system_prompt: Some("you emit code only".to_string()),
        grammar_constraint: Some("root ::= expr".to_string()),
        grammar_format: Some(GrammarFormat::Gbnf),
        max_tokens: 512,
        temperature: 0.2,
        reasoning_effort: Some(ReasoningEffort::Low),
    };
    let json = serde_json::to_string(&request).expect("serialize");
    let parsed: InferenceRequest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(request, parsed);

    let response = InferenceResponse {
        text: "let x = 1;".to_string(),
        prompt_tokens: 120,
        completion_tokens: 8,
        latency_ms: 240,
        model_used: "router-small".to_string(),
        reasoning_trace: None,
        reasoning_tokens: None,
    };
    let json = serde_json::to_string(&response).expect("serialize");
    let parsed: InferenceResponse = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(response, parsed);
}
