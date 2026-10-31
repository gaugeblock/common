//! The wire rules the seven hand-written servers disagreed about.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_mcp::{McpHandler, ToolOutcome, code, handle_line, serve};
use serde_json::{Value, json};

/// A handler with one tool that always succeeds and one that always fails.
struct Fixture {
    server: String,
}

impl McpHandler for Fixture {
    fn server_name(&self) -> &str {
        &self.server
    }

    fn list_tools(&self) -> Value {
        json!([{ "name": "echo", "description": "Echo the arguments" }])
    }

    fn call_tool(&self, name: &str, arguments: &Value) -> ToolOutcome {
        match name {
            "echo" => ToolOutcome::Json(arguments.clone()),
            "boom" => ToolOutcome::Failed("deliberate failure".to_string()),
            other => ToolOutcome::Failed(format!("unknown tool: {other}")),
        }
    }
}

fn fixture() -> Fixture {
    Fixture {
        server: "fixture-mcp".to_string(),
    }
}

fn reply(line: &str) -> Value {
    handle_line(line, &fixture())
        .expect("a reply was expected")
        .into_value()
}

#[test]
fn test_initialize_announces_the_protocol_and_server() {
    let value = reply(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
    assert_eq!(value["jsonrpc"], json!("2.0"));
    assert_eq!(value["id"], json!(1));
    assert_eq!(
        value["result"]["protocolVersion"],
        json!(gaugeblock_mcp::PROTOCOL_VERSION)
    );
    assert_eq!(value["result"]["serverInfo"]["name"], json!("fixture-mcp"));
    assert!(value["result"]["serverInfo"]["version"].is_string());
}

#[test]
fn test_tool_call_returns_the_content_envelope_not_raw_json() {
    // Batten, joiner and benchwork built this envelope inside their dispatcher, so the tool layer knew
    // what MCP looks like on the wire. Here the tool returns data and the transport owns the shape.
    let value = reply(
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"echo","arguments":{"n":1}}}"#,
    );
    assert_eq!(value["result"]["content"][0]["type"], json!("text"));
    let text = value["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, r#"{"n":1}"#);
    assert!(
        value["result"].get("isError").is_none(),
        "a successful call must not set isError"
    );
}

#[test]
fn test_a_failed_tool_is_a_result_with_is_error_not_a_json_rpc_error() {
    // MCP reserves `error` for protocol faults: a tool that ran and failed is a successful call.
    let value = reply(r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"boom"}}"#);
    assert!(value.get("error").is_none(), "got {value}");
    assert_eq!(value["result"]["isError"], json!(true));
    assert!(
        value["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("deliberate failure")
    );
}

#[test]
fn test_tool_call_without_a_tool_name_is_an_invalid_params_error() {
    // One server answered nothing at all here, which from the client's side is a hang.
    let value = reply(r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{}}"#);
    assert_eq!(value["error"]["code"], json!(code::INVALID_PARAMS));
    assert_eq!(value["id"], json!(9));
}

#[test]
fn test_notifications_are_never_answered() {
    for line in [
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","method":"no/such/method"}"#,
    ] {
        assert!(
            handle_line(line, &fixture()).is_none(),
            "answered a notification: {line}"
        );
    }
}

#[test]
fn test_unknown_method_with_an_id_is_method_not_found() {
    let value = reply(r#"{"jsonrpc":"2.0","id":3,"method":"no/such/method"}"#);
    assert_eq!(value["error"]["code"], json!(code::METHOD_NOT_FOUND));
    assert_eq!(value["id"], json!(3));
}

#[test]
fn test_a_malformed_line_is_answered_with_parse_error() {
    // Every one of the seven logged this and dropped it, leaving the client waiting.
    let value = reply("{not json");
    assert_eq!(value["error"]["code"], json!(code::PARSE_ERROR));
    assert_eq!(value["id"], json!(null), "id unknown, so null");
}

#[test]
fn test_serve_answers_each_line_and_survives_a_bad_one() {
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n",
        "garbage\n",
        "\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n",
    );
    let mut output = Vec::new();
    serve(input.as_bytes(), &mut output, &fixture()).expect("serve");

    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    // One reply per request, none for the blank line, and a parse error rather than a dropped line.
    assert_eq!(lines.len(), 3, "got {text}");
    for line in &lines {
        let value: Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["jsonrpc"], json!("2.0"));
    }
    let codes: Vec<Value> = lines
        .iter()
        .map(|line| serde_json::from_str::<Value>(line).unwrap()["error"]["code"].clone())
        .collect();
    assert_eq!(codes[1], json!(code::PARSE_ERROR));
}

#[test]
fn test_tool_arguments_default_to_an_empty_object() {
    // A client may omit `arguments` for a tool that takes none.
    let value = reply(r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"echo"}}"#);
    assert_eq!(value["result"]["content"][0]["text"], json!("{}"));
}

struct LegacyHandler;

impl McpHandler for LegacyHandler {
    fn server_name(&self) -> &str {
        "legacy-mcp"
    }

    fn list_tools(&self) -> Value {
        json!([])
    }

    fn call_tool(&self, _name: &str, _arguments: &Value) -> ToolOutcome {
        // A dispatcher that still builds the envelope itself is passed through unchanged.
        ToolOutcome::Raw(json!({ "content": [{ "type": "text", "text": "legacy" }] }))
    }
}

#[test]
fn test_a_legacy_dispatcher_result_is_passed_through_unchanged() {
    let value = handle_line(
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"any"}}"#,
        &LegacyHandler,
    )
    .unwrap()
    .into_value();
    assert_eq!(value["result"]["content"][0]["text"], json!("legacy"));
}

/// A server whose dispatch is asynchronous, as three of the seven are.
struct AsyncFixture;

impl gaugeblock_mcp::AsyncMcpHandler for AsyncFixture {
    fn server_name(&self) -> &str {
        "async-mcp"
    }

    fn list_tools(&self) -> Value {
        json!([])
    }

    async fn call_tool(&self, name: &str, arguments: &Value) -> ToolOutcome {
        // A real await point, so this is genuinely driven rather than trivially ready.
        std::future::ready(()).await;
        ToolOutcome::Json(json!({ "tool": name, "arguments": arguments }))
    }
}

/// A one-poll executor, sufficient for futures that never pend on I/O.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }
}

#[test]
fn test_the_async_loop_agrees_with_the_synchronous_one() {
    let input = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n",
        "not-json\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"t\",\"arguments\":{\"a\":1}}}\n",
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
    );
    let mut output = Vec::new();
    block_on(gaugeblock_mcp::serve_async(
        input.as_bytes(),
        &mut output,
        &AsyncFixture,
    ))
    .expect("serve_async");

    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "initialize, parse error, tool call - not the notification"
    );

    let init: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], json!("async-mcp"));

    let parse: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(parse["error"]["code"], json!(code::PARSE_ERROR));

    let call: Value = serde_json::from_str(lines[2]).unwrap();
    let text = call["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains(r#""tool":"t""#), "got {text}");
}
