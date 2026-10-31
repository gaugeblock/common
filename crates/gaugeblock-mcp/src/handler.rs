//! What a server has to provide, and the dispatch that turns one message into one reply.

use serde_json::Value;

use crate::protocol::{RpcRequest, RpcResponse, ToolOutcome, code};

/// One tool server's behaviour, minus the wire.
///
/// Synchronous on purpose. Two of the seven servers dispatch through `async` functions and the rest do
/// not, so a synchronous trait is the only shape that all of them satisfy without making the others
/// adopt a runtime they never needed. A server with async work blocks inside `call_tool`, which is
/// what those servers already did at the top of their own loops.
pub trait McpHandler {
    /// The `serverInfo.name` reported at `initialize`.
    fn server_name(&self) -> &str;

    /// The tool array reported at `tools/list`.
    fn list_tools(&self) -> Value;

    /// Runs one tool. Implementations are expected to report a tool failure as
    /// [`ToolOutcome::Failed`] rather than panicking or returning an error.
    fn call_tool(&self, name: &str, arguments: &Value) -> ToolOutcome;
}

/// Dispatches one parsed request, returning the reply to send (if any).
///
/// The three "no reply" cases are deliberate and each was a bug in at least one hand-written copy:
/// a notification takes no answer; and a `tools/call` that is missing its tool name gets
/// [`code::INVALID_PARAMS`] rather than silence, because silence is indistinguishable from a hang.
#[must_use]
pub fn dispatch<H: McpHandler>(handler: &H, request: &RpcRequest) -> Option<RpcResponse> {
    let id = request.id.clone();
    let response = match request.method.as_str() {
        "initialize" => {
            RpcResponse::initialize(id, handler.server_name(), env!("CARGO_PKG_VERSION"))
        }
        "notifications/initialized" | "initialized" => return None,
        "tools/list" => RpcResponse::tools(id, handler.list_tools()),
        "tools/call" => match request.tool_name() {
            Some(name) => RpcResponse::result(
                id,
                handler
                    .call_tool(name, &request.tool_arguments())
                    .into_result(),
            ),
            None => RpcResponse::error(id, code::INVALID_PARAMS, "tools/call requires params.name"),
        },
        // `ping` is part of the specification and expecting it costs nothing to support.
        "ping" => RpcResponse::result(id, serde_json::json!({})),
        other => RpcResponse::error(
            id,
            code::METHOD_NOT_FOUND,
            format!("Method not found: {other}"),
        ),
    };

    // A notification is never answered, whatever the method - JSON-RPC forbids it, and a client that
    // sent one is not reading for a reply.
    if request.is_notification() {
        return None;
    }
    Some(response)
}
