//! E19 (AJ-E1905): the bridge as the Claude Code plugin's MCP server
//! (`juggler-agent --mcp --plugin`).
//!
//! - In a session AI Juggler started with its agent tools, the launch
//!   already added the `juggler` server (D-30). The plugin's copy then
//!   offers nothing (no tools, no resources) and says why, so the model
//!   never sees the tools twice.
//! - Anywhere else it serves the usual tools; with no key each call
//!   answers in words: this session wasn't started by AI Juggler with its
//!   agent tools, or AI Juggler isn't running (ADR-0020: a key comes only
//!   with a launch).

use serde_json::{Value, json};

use crate::mcp::{Backend, handle_line as serve_line};

/// Said when the launch already gave this session its tools.
pub const ALREADY_GIVEN: &str = "AI Juggler already gave this session its pool tools (the juggler server it added at launch); this plugin's copy stays empty so they aren't listed twice.";

/// Answers one line as the plugin's server: `already_given` when the
/// session holds AI Juggler's key (its launch added the tools itself).
pub fn handle_line(line: &str, backend: &mut dyn Backend, already_given: bool) -> Option<String> {
    if !already_given {
        return serve_line(line, backend);
    }
    let request: Value = serde_json::from_str(line.trim()).unwrap_or(Value::Null);
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let empty = match method {
        "tools/list" => Some(json!({"tools": []})),
        "resources/list" => Some(json!({"resources": []})),
        "tools/call" => {
            Some(json!({"content": [{"type": "text", "text": ALREADY_GIVEN}], "isError": true}))
        }
        _ => None,
    };
    if let (Some(result), Some(id)) = (empty, id) {
        return Some(json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string());
    }
    let reply = serve_line(line, backend)?;
    if !matches!(method, "initialize" | "server/discover") {
        return Some(reply);
    }
    let mut reply: Value = serde_json::from_str(&reply).ok()?;
    if let Some(result) = reply.get_mut("result") {
        result["instructions"] = json!(ALREADY_GIVEN);
    }
    Some(reply.to_string())
}

#[cfg(test)]
mod tests {
    use super::{ALREADY_GIVEN, handle_line};
    use crate::client::CallError;
    use crate::mcp::Backend;
    use crate::wire::{Args, Call};
    use serde_json::Value;

    struct NoKey(usize);
    impl Backend for NoKey {
        fn call(&mut self, _call: Call, _args: Args) -> Result<Value, CallError> {
            self.0 += 1;
            Err(CallError::NoKey)
        }
    }

    fn ask(line: &str, given: bool, backend: &mut NoKey) -> Value {
        serde_json::from_str(&handle_line(line, backend, given).unwrap()).unwrap()
    }

    #[test]
    fn in_an_ai_juggler_session_it_lists_nothing_and_says_why() {
        let mut backend = NoKey(0);
        let list = ask(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
            true,
            &mut backend,
        );
        assert_eq!(list["result"]["tools"], serde_json::json!([]));
        let hello = ask(
            r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
            true,
            &mut backend,
        );
        assert_eq!(hello["result"]["instructions"], ALREADY_GIVEN);
        let call = ask(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tasks_list","arguments":{}}}"#,
            true,
            &mut backend,
        );
        assert_eq!(call["result"]["isError"], true);
        assert_eq!(backend.0, 0, "nothing reaches the app");
    }

    #[test]
    fn elsewhere_it_serves_the_tools_and_each_call_says_why_not() {
        let mut backend = NoKey(0);
        let list = ask(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
            false,
            &mut backend,
        );
        assert_eq!(
            list["result"]["tools"].as_array().unwrap().len(),
            crate::wire::TOOLS.len()
        );
        let call = ask(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tasks_list","arguments":{}}}"#,
            false,
            &mut backend,
        );
        assert_eq!(call["result"]["isError"], true);
        assert!(
            call["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("wasn't started by AI Juggler")
        );
    }
}
