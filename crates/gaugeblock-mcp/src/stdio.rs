//! The stdio transports: one JSON object per line in, one reply per line out.
//!
//! Seven servers wrote this loop. They agreed on the happy path and differed on every failure: a line
//! that could not be parsed was logged and dropped by all of them, so a client waited for a reply that
//! JSON-RPC has a code for. The loops here answer it.

use std::io::{BufRead, Write};

use crate::handler::{AsyncMcpHandler, McpHandler, Route, reply_without_tool, route};
use crate::protocol::{RpcRequest, RpcResponse, code};

/// Answers one parsed request, returning the reply to send (if any).
#[must_use]
pub fn dispatch<H: McpHandler>(handler: &H, request: &RpcRequest) -> Option<RpcResponse> {
    let routed = route(request);
    if let Some(response) = reply_without_tool(
        &routed,
        request.id.clone(),
        || handler.server_name().to_string(),
        || handler.list_tools(),
    ) {
        return Some(response);
    }
    if let Route::CallTool { name, arguments } = &routed {
        return Some(RpcResponse::result(
            request.id.clone(),
            handler.call_tool(name, arguments).into_result(),
        ));
    }
    None
}

/// [`dispatch`] for a server whose tool dispatch is asynchronous.
///
/// Exists so an async server can keep a message-shaped entry point for its own tests without rebuilding
/// the routing beside it.
pub async fn dispatch_async<H: AsyncMcpHandler>(
    handler: &H,
    request: &RpcRequest,
) -> Option<RpcResponse> {
    let routed = route(request);
    if let Some(response) = reply_without_tool(
        &routed,
        request.id.clone(),
        || handler.server_name().to_string(),
        || handler.list_tools(),
    ) {
        return Some(response);
    }
    match &routed {
        Route::CallTool { name, arguments } => Some(RpcResponse::result(
            request.id.clone(),
            handler.call_tool(name, arguments).await.into_result(),
        )),
        _ => None,
    }
}

/// Answers one line, or `None` when the message takes no reply.
///
/// A line that is not valid JSON is answered with [`code::PARSE_ERROR`] rather than an `Err`: one
/// malformed message must not end a session, and the client is entitled to be told.
#[must_use]
pub fn handle_line<H: McpHandler>(line: &str, handler: &H) -> Option<RpcResponse> {
    match parse(line) {
        Ok(request) => dispatch(handler, &request),
        Err(response) => Some(response),
    }
}

/// Runs the synchronous server over a reader and a writer, one request per line.
///
/// # Errors
///
/// The I/O failure when the reader cannot be read or the writer written.
pub fn serve<R, W, H>(reader: R, writer: &mut W, handler: &H) -> std::io::Result<()>
where
    R: BufRead,
    W: Write,
    H: McpHandler,
{
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(response) = handle_line(trimmed, handler) {
            write_response(writer, &response)?;
        }
    }
    Ok(())
}

/// Runs an asynchronous server over a reader and a writer, one request per line.
///
/// The crate does not depend on an executor: this is a plain `async fn`, so the caller's runtime drives
/// it. Reading a line blocks the calling thread, which is what every server in the suite already did.
///
/// # Errors
///
/// The I/O failure when the reader cannot be read or the writer written.
pub async fn serve_async<R, W, H>(reader: R, writer: &mut W, handler: &H) -> std::io::Result<()>
where
    R: BufRead,
    W: Write,
    H: AsyncMcpHandler,
{
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let response = match parse(trimmed) {
            Err(response) => Some(response),
            Ok(request) => dispatch_async(handler, &request).await,
        };
        if let Some(response) = response {
            write_response(writer, &response)?;
        }
    }
    Ok(())
}

/// Parses a line, or returns the parse-error response to send.
fn parse(line: &str) -> Result<RpcRequest, RpcResponse> {
    serde_json::from_str(line).map_err(|source| {
        tracing::error!(error = %source, "Unparseable JSON-RPC message");
        // `id: null` is what JSON-RPC prescribes when the id could not be recovered.
        RpcResponse::error(None, code::PARSE_ERROR, format!("Parse error: {source}"))
    })
}

fn write_response<W: Write>(writer: &mut W, response: &RpcResponse) -> std::io::Result<()> {
    let line = serde_json::to_string(response.value()).map_err(std::io::Error::other)?;
    writeln!(writer, "{line}")?;
    writer.flush()
}
