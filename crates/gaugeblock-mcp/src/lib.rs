//! Shared JSON-RPC 2.0 plumbing for the suite's Model Context Protocol servers (`GB-130`).
//!
//! Seven repositories each grew a stdio server, and each one copied the same dispatch: read a line,
//! parse it, switch on `method`, answer `initialize` / `tools/list` / `tools/call`, write a line back.
//! The copies did not stay copies. Four servers build the MCP `content` envelope in the loop and three
//! build it inside `dispatch_tool`, so the same wire type has two homes and the tool layer has to know
//! about MCP in order to be called; two answered nothing at all when a `tools/call` arrived without
//! `params.name`, which a client cannot tell from a hang; and none of them replied to a line that
//! failed to parse, though JSON-RPC defines a code for exactly that.
//!
//! The extraction is therefore two things at once: the boilerplate moves here, and the behaviour is
//! settled here. A server now supplies a name, a tool array and a synchronous `call_tool`, and inherits
//! the wire rules that the copies disagreed about.

mod handler;
mod protocol;
mod stdio;

pub use handler::{McpHandler, dispatch};
pub use protocol::{PROTOCOL_VERSION, RpcRequest, RpcResponse, ToolOutcome, code};
pub use stdio::{handle_line, serve};

/// Failures from serving.
///
/// Deliberately small: a malformed request is a *response*, not an error, so almost everything a
/// client can do wrong is answered rather than propagated.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// The transport failed while reading or writing a line.
    #[error("MCP transport failed: {0}")]
    Transport(#[from] std::io::Error),
}
