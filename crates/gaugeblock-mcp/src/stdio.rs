//! The stdio transport: one JSON object per line in, one reply per line out.
//!
//! Seven servers wrote this loop. They agreed on the happy path and differed on every failure: a line
//! that could not be parsed was logged and dropped by all of them, so a client waited for a reply that
//! JSON-RPC has a code for. The loop here answers it.

use std::io::{BufRead, Write};

use crate::handler::{McpHandler, dispatch};
use crate::protocol::{RpcRequest, RpcResponse, code};

/// Runs the server over a reader and a writer, one request per line.
///
/// Generic over the streams so it can be driven from a test without a subprocess, which is the only
/// way to pin the failure cases: a server that drops a malformed line looks identical to a working one
/// unless something is watching what it wrote.
///
/// # Errors
///
/// Returns the I/O failure when stdin cannot be read or stdout cannot be written. A line that is not
/// valid JSON is *not* an error - it is answered with [`code::PARSE_ERROR`] and the loop continues,
/// because one malformed message must not end a session.
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

        if let Some(response) = handle_line(trimmed, handler)? {
            writeln!(
                writer,
                "{}",
                serde_json::to_string(response.value()).map_err(std::io::Error::other)?
            )?;
            writer.flush()?;
        }
    }
    Ok(())
}

/// Answers one line, or `None` when the message takes no reply.
///
/// # Errors
///
/// Only a response that cannot be serialized, which cannot happen for a value built from JSON.
pub fn handle_line<H: McpHandler>(line: &str, handler: &H) -> std::io::Result<Option<RpcResponse>> {
    let request: RpcRequest = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(source) => {
            tracing::error!(error = %source, "Unparseable JSON-RPC message");
            // `id: null` is what JSON-RPC prescribes when the id could not be recovered.
            return Ok(Some(RpcResponse::error(
                None,
                code::PARSE_ERROR,
                format!("Parse error: {source}"),
            )));
        }
    };
    Ok(dispatch(handler, &request))
}
