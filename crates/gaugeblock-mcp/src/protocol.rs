//! JSON-RPC 2.0 wire types for Model Context Protocol servers.
//!
//! The suite had seven copies of this and they did not agree. Who builds a `tools/call` result is the
//! disagreement that mattered: four servers wrap it in the MCP `content` envelope inside the loop and
//! three wrap it inside `dispatch_tool`, so a wire type lived in a tool layer that should never have
//! known about MCP. Two of them answered nothing at all to a `tools/call` with no `params.name`, which
//! from the client's side is indistinguishable from a hang.
//!
//! So the rules below are the specification's, stated once, and every server follows them because
//! there is nowhere else to put them.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The MCP protocol revision every server in the suite announces.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// JSON-RPC 2.0 and MCP error codes used by the suite.
pub mod code {
    /// The payload could not be parsed as JSON.
    pub const PARSE_ERROR: i64 = -32700;
    /// The method is not implemented by this server.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// The method exists but its parameters are unusable.
    pub const INVALID_PARAMS: i64 = -32602;
    /// The server failed for a reason that is not the request's fault.
    pub const INTERNAL_ERROR: i64 = -32603;
}

/// A request or notification received from a client.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RpcRequest {
    /// Always `"2.0"`; not validated, because a client that gets this wrong still wants an answer.
    #[serde(default)]
    pub jsonrpc: Option<String>,
    /// Absent for notifications, which must not be answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    /// The method name.
    pub method: String,
    /// Method-specific parameters.
    #[serde(default)]
    pub params: Option<Value>,
}

impl RpcRequest {
    /// Whether this request is a notification, which takes no reply.
    #[must_use]
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// The `params.name` of a `tools/call`, when present.
    #[must_use]
    pub fn tool_name(&self) -> Option<&str> {
        self.params
            .as_ref()
            .and_then(|params| params.get("name"))
            .and_then(Value::as_str)
    }

    /// The `params.arguments` of a `tools/call`, defaulting to an empty object.
    ///
    /// A tool with no parameters is common, and a client is entitled to omit the field.
    #[must_use]
    pub fn tool_arguments(&self) -> Value {
        self.params
            .as_ref()
            .and_then(|params| params.get("arguments"))
            .cloned()
            .unwrap_or_else(|| json!({}))
    }
}

/// How a tool call ended.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolOutcome {
    /// Serialized as compact JSON text inside the `content` envelope.
    Json(Value),
    /// Passed through verbatim, for a server that formats its own output.
    Text(String),
    /// The tool ran and failed. This is a *successful* JSON-RPC call: MCP reserves the `error` field
    /// for protocol faults, so a tool failure travels in `content` with `isError` set.
    Failed(String),
}

impl ToolOutcome {
    /// The `tools/call` result value.
    #[must_use]
    pub fn into_result(self) -> Value {
        match self {
            Self::Json(value) => Self::text(value.to_string(), false),
            Self::Text(text) => Self::text(text, false),
            Self::Failed(message) => Self::text(format!("Error: {message}"), true),
        }
    }

    fn text(text: String, is_error: bool) -> Value {
        let mut result = json!({ "content": [{ "type": "text", "text": text }] });
        if is_error && let Some(object) = result.as_object_mut() {
            object.insert("isError".to_string(), Value::Bool(true));
        }
        result
    }
}

/// A JSON-RPC response.
///
/// Built rather than serialized from a struct so a caller can see, at the call site, that exactly one
/// of `result` and `error` is present - the property the hand-written `json!` copies kept breaking.
#[derive(Clone, Debug, PartialEq)]
pub struct RpcResponse(Value);

impl RpcResponse {
    /// A successful response.
    #[must_use]
    pub fn result(id: Option<Value>, result: Value) -> Self {
        Self(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
    }

    /// An error response.
    #[must_use]
    pub fn error(id: Option<Value>, code: i64, message: impl Into<String>) -> Self {
        Self(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message.into() }
        }))
    }

    /// The `initialize` result for a named server.
    #[must_use]
    pub fn initialize(id: Option<Value>, server_name: &str, version: &str) -> Self {
        Self::result(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": server_name, "version": version }
            }),
        )
    }

    /// The `tools/list` result for a pre-built tool array.
    #[must_use]
    pub fn tools(id: Option<Value>, tools: Value) -> Self {
        Self::result(id, json!({ "tools": tools }))
    }

    /// The wire value.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.0
    }

    /// Consumes the response, yielding the wire value.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.0
    }
}

impl std::fmt::Display for RpcResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
