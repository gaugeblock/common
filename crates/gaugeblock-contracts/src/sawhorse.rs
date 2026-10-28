//! `sawhorse` wire domain: execution modes, model roles and inference messages.

use serde::{Deserialize, Serialize};

use crate::dado_batten::GrammarFormat;

/// How a model is executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Mode 1: specialised `gaugeblock` micro-weights running natively via `candle`.
    InHouseCandle {
        /// Model identifier.
        model_id: String,
        /// Path to the local weights file.
        weights_path: String,
    },
    /// Mode 2: local GGUF or quantised open weights via `mistral.rs` or `llama.cpp`.
    LocalRuntime {
        /// Model identifier.
        model_id: String,
        /// Endpoint URL or local model path.
        endpoint_or_path: String,
    },
    /// Mode 3: any OpenAI-compatible HTTP endpoint.
    OpenAiCompatible {
        /// Base URL of the endpoint.
        base_url: String,
        /// Environment variable holding the API key.
        api_key_env: String,
        /// Provider-side model name.
        model_name: String,
    },
}

/// The job a model is being asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    /// Spec planning (`chalkline`).
    Planner,
    /// Fast pattern routing that selects a `dado` pattern id and structural mixins.
    System1Router,
    /// Grammar-masked expression generation inside `batten` constraints.
    HoleFiller,
    /// Unconstrained or lightly constrained fallback for novel algorithms.
    Oracle,
}

/// Reasoning or thinking effort requested from the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    /// Explicitly disable the reasoning trace.
    None,
    /// Minimal reasoning effort.
    Low,
    /// Standard reasoning effort.
    Medium,
    /// Exhaustive reasoning effort.
    High,
}

/// A single inference request handed to a provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// Job the model is being asked to do.
    pub role: ModelRole,
    /// User prompt.
    pub prompt: String,
    /// Optional system prompt.
    pub system_prompt: Option<String>,
    /// Raw grammar string produced by `batten`.
    pub grammar_constraint: Option<String>,
    /// Dialect of `grammar_constraint`.
    pub grammar_format: Option<GrammarFormat>,
    /// Maximum completion tokens.
    pub max_tokens: u16,
    /// Sampling temperature.
    pub temperature: f32,
    /// Optional reasoning effort level.
    pub reasoning_effort: Option<ReasoningEffort>,
}

/// A provider response carrying text, usage and optional reasoning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferenceResponse {
    /// Completion text.
    pub text: String,
    /// Prompt tokens billed.
    pub prompt_tokens: u32,
    /// Completion tokens billed.
    pub completion_tokens: u32,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
    /// Provider-side model name that answered.
    pub model_used: String,
    /// Optional chain-of-thought trace.
    pub reasoning_trace: Option<String>,
    /// Tokens consumed by reasoning, when reported.
    pub reasoning_tokens: Option<u32>,
}

impl InferenceRequest {
    /// Builds a deterministic prompt request for `role` (temperature `0.0`, 1024 tokens).
    pub fn new(role: ModelRole, prompt: impl Into<String>) -> Self {
        Self {
            role,
            prompt: prompt.into(),
            system_prompt: None,
            grammar_constraint: None,
            grammar_format: None,
            max_tokens: 1024,
            temperature: 0.0,
            reasoning_effort: None,
        }
    }

    /// Sets the reasoning effort level.
    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }

    /// Attaches a GBNF grammar constraint.
    pub fn with_gbnf_grammar(mut self, grammar: impl Into<String>) -> Self {
        self.grammar_constraint = Some(grammar.into());
        self.grammar_format = Some(GrammarFormat::Gbnf);
        self
    }

    /// Attaches a Lark grammar constraint.
    pub fn with_lark_grammar(mut self, grammar: impl Into<String>) -> Self {
        self.grammar_constraint = Some(grammar.into());
        self.grammar_format = Some(GrammarFormat::Lark);
        self
    }

    /// Attaches an optional system prompt.
    pub fn with_system_prompt(mut self, system: impl Into<String>) -> Self {
        self.system_prompt = Some(system.into());
        self
    }
}
