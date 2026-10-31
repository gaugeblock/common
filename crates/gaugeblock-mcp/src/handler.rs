//! What a server has to provide, and the routing that turns one request into one reply.
//!
//! Routing is factored out of both loops - synchronous and asynchronous - so the seven servers agree on
//! it regardless of whether their tool dispatch is `async`. Without this the async ones would keep a
//! second copy of the decisions, which is how the first seven copies diverged.

use serde_json::Value;

use crate::protocol::{RpcRequest, ToolOutcome, code};

/// What a request asks for.
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    /// `initialize`.
    Initialize,
    /// `tools/list`.
    ListTools,
    /// `tools/call` with its tool name and arguments.
    CallTool {
        /// The requested tool.
        name: String,
        /// Its arguments, defaulted to `{}`.
        arguments: Value,
    },
    /// `ping`, which the specification defines and costs nothing to honour.
    Ping,
    /// A method this server does not implement.
    Unknown(String),
    /// `tools/call` with no tool name, which is a parameter error rather than an unknown method.
    MissingToolName,
    /// A notification, which takes no reply whatever the method.
    Silent,
}

/// Classifies a request.
///
/// Notifications are classified first and win: JSON-RPC forbids answering one, so a notification for an
/// unknown method is silent rather than `-32601` - a client that sent it is not reading for a reply.
#[must_use]
pub fn route(request: &RpcRequest) -> Route {
    if request.is_notification() {
        return Route::Silent;
    }
    match request.method.as_str() {
        "initialize" => Route::Initialize,
        "notifications/initialized" | "initialized" => Route::Silent,
        "tools/list" => Route::ListTools,
        "ping" => Route::Ping,
        "tools/call" => match request.tool_name() {
            Some(name) => Route::CallTool {
                name: name.to_string(),
                arguments: request.tool_arguments(),
            },
            // The name is required. Two servers used to answer nothing here, which a client cannot
            // tell from a hang, so it is an error code rather than silence.
            None => Route::MissingToolName,
        },
        other => Route::Unknown(other.to_string()),
    }
}

/// The `-32602` message for a `tools/call` with no tool name.
pub const MISSING_TOOL_NAME: &str = "tools/call requires params.name";

/// One tool server's behaviour, minus the wire.
///
/// Synchronous on purpose: five of the seven servers dispatch synchronously. A server whose dispatch is
/// genuinely `async` implements [`AsyncMcpHandler`] instead and gets the same routing through
/// [`crate::serve_async`].
pub trait McpHandler {
    /// The `serverInfo.name` reported at `initialize`.
    fn server_name(&self) -> &str;

    /// The tool array reported at `tools/list`.
    fn list_tools(&self) -> Value;

    /// Runs one tool. Implementations report a tool failure as [`ToolOutcome::Failed`] rather than
    /// panicking or returning an error.
    fn call_tool(&self, name: &str, arguments: &Value) -> ToolOutcome;
}

/// A server whose tool dispatch is asynchronous.
pub trait AsyncMcpHandler {
    /// The `serverInfo.name` reported at `initialize`.
    fn server_name(&self) -> &str;

    /// The tool array reported at `tools/list`.
    fn list_tools(&self) -> Value;

    /// Runs one tool.
    fn call_tool(
        &self,
        name: &str,
        arguments: &Value,
    ) -> impl std::future::Future<Output = ToolOutcome>;
}

/// The reply for a route that needs no tool execution, or `None` when the tool must be called.
///
/// Shared by both loops so `initialize`, `tools/list`, `ping` and the two error cases are built in
/// exactly one place.
#[must_use]
pub fn reply_without_tool<N, T>(
    routed: &Route,
    id: Option<Value>,
    server_name: N,
    list_tools: T,
) -> Option<crate::RpcResponse>
where
    N: FnOnce() -> String,
    T: FnOnce() -> Value,
{
    use crate::RpcResponse;
    match routed {
        Route::Initialize => Some(RpcResponse::initialize(
            id,
            &server_name(),
            env!("CARGO_PKG_VERSION"),
        )),
        Route::ListTools => Some(RpcResponse::tools(id, list_tools())),
        Route::Ping => Some(RpcResponse::result(id, serde_json::json!({}))),
        Route::Unknown(method) => Some(RpcResponse::error(
            id,
            code::METHOD_NOT_FOUND,
            format!("Method not found: {method}"),
        )),
        Route::MissingToolName => Some(RpcResponse::error(
            id,
            code::INVALID_PARAMS,
            MISSING_TOOL_NAME,
        )),
        Route::Silent => None,
        Route::CallTool { .. } => None,
    }
}
