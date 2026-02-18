//! Dual-format tool-call parser for LLM responses.
//!
//! Supports two formats that the LLM may use to express a tool call:
//!
//! # Format 1 — OpenAI JSON
//! The response content is a JSON object containing a `tool_calls` array:
//!
//! ```json
//! {
//!   "tool_calls": [
//!     { "id": "call_abc", "type": "function",
//!       "function": { "name": "read_file", "arguments": "{\"path\": \"/tmp/x\"}" } }
//!   ]
//! }
//! ```
//!
//! # Format 2 — XML inline
//! The response is free-form text containing one or more `<tool_call>` tags:
//!
//! ```xml
//! I'll read the file now.
//! <tool_call>{"name": "read_file", "arguments": {"path": "/tmp/x"}}</tool_call>
//! ```

use serde_json::Value;

// ─── ParsedToolCall ───────────────────────────────────────────────────────────

/// A tool invocation extracted from an LLM response.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedToolCall {
    /// Tool name as registered in the `ToolRegistry`.
    pub name: String,
    /// Arguments to pass to the tool.
    pub arguments: Value,
    /// Optional call ID (present in OpenAI JSON format).
    pub call_id: Option<String>,
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Parse all tool calls from an LLM response string.
///
/// Tries the JSON format first; if the content is not a valid JSON object with
/// `tool_calls`, falls back to scanning for XML `<tool_call>` tags.
///
/// Returns an empty `Vec` when no tool calls are found.
pub fn parse_tool_calls(content: &str) -> Vec<ParsedToolCall> {
    // Prefer the JSON format (explicit contract).
    if let Some(calls) = try_parse_json(content) {
        if !calls.is_empty() {
            return calls;
        }
    }
    // Fall back to XML scanning.
    parse_xml(content)
}

// ─── JSON parser ──────────────────────────────────────────────────────────────

fn try_parse_json(content: &str) -> Option<Vec<ParsedToolCall>> {
    let trimmed = content.trim();
    // The entire content must be a valid JSON object.
    let obj: Value = serde_json::from_str(trimmed).ok()?;
    let tool_calls = obj.get("tool_calls")?.as_array()?;

    let mut result = Vec::new();
    for entry in tool_calls {
        // OpenAI format: { "id": "...", "type": "function", "function": { "name": "...", "arguments": "..." } }
        let func = entry.get("function")?;
        let name = func.get("name")?.as_str()?.to_string();
        let call_id = entry.get("id").and_then(|v| v.as_str()).map(str::to_string);

        // `arguments` may be a JSON string (OpenAI) or an object (some providers).
        let arguments = match func.get("arguments") {
            Some(Value::String(s)) => {
                serde_json::from_str(s).unwrap_or(Value::Object(Default::default()))
            }
            Some(v) => v.clone(),
            None => Value::Object(Default::default()),
        };

        result.push(ParsedToolCall { name, arguments, call_id });
    }
    Some(result)
}

// ─── XML parser ───────────────────────────────────────────────────────────────

fn parse_xml(content: &str) -> Vec<ParsedToolCall> {
    const OPEN: &str = "<tool_call>";
    const CLOSE: &str = "</tool_call>";

    let mut result = Vec::new();
    let mut remaining = content;

    while let Some(start) = remaining.find(OPEN) {
        remaining = &remaining[start + OPEN.len()..];
        let end = match remaining.find(CLOSE) {
            Some(i) => i,
            None => break,
        };
        let body = remaining[..end].trim();
        remaining = &remaining[end + CLOSE.len()..];

        if let Some(call) = parse_xml_body(body) {
            result.push(call);
        }
    }
    result
}

/// Parse the JSON body of a single `<tool_call>` tag.
///
/// Expected shape:
/// ```json
/// {"name": "tool_name", "arguments": { … }}
/// ```
fn parse_xml_body(body: &str) -> Option<ParsedToolCall> {
    let obj: Value = serde_json::from_str(body).ok()?;
    let name = obj.get("name")?.as_str()?.to_string();
    let arguments = obj
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    Some(ParsedToolCall { name, arguments, call_id: None })
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── XML format ────────────────────────────────────────────────────────────

    #[test]
    fn xml_single_tool_call() {
        let content = r#"Let me search for that.
<tool_call>{"name": "web_search", "arguments": {"query": "rust async"}}</tool_call>
I found something."#;

        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "web_search");
        assert_eq!(calls[0].arguments["query"], "rust async");
        assert!(calls[0].call_id.is_none());
    }

    #[test]
    fn xml_multiple_tool_calls() {
        let content = r#"<tool_call>{"name": "read_file", "arguments": {"path": "/a"}}</tool_call>
<tool_call>{"name": "read_file", "arguments": {"path": "/b"}}</tool_call>"#;

        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].arguments["path"], "/a");
        assert_eq!(calls[1].arguments["path"], "/b");
    }

    #[test]
    fn xml_no_tool_calls() {
        let content = "The answer is 42. No tools needed.";
        let calls = parse_tool_calls(content);
        assert!(calls.is_empty());
    }

    #[test]
    fn xml_malformed_json_skipped() {
        // Malformed body should be ignored; valid ones still returned.
        let content = r#"<tool_call>not json</tool_call>
<tool_call>{"name": "valid_tool", "arguments": {}}</tool_call>"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "valid_tool");
    }

    #[test]
    fn xml_empty_arguments_defaults_to_empty_object() {
        let content = r#"<tool_call>{"name": "ping"}</tool_call>"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert!(calls[0].arguments.is_object());
        assert!(calls[0].arguments.as_object().unwrap().is_empty());
    }

    #[test]
    fn xml_unclosed_tag_stops_parsing() {
        let content = r#"<tool_call>{"name": "orphan""#;
        let calls = parse_tool_calls(content);
        assert!(calls.is_empty());
    }

    // ── JSON format ───────────────────────────────────────────────────────────

    #[test]
    fn json_single_tool_call() {
        let content = r#"{
  "tool_calls": [
    { "id": "call_xyz", "type": "function",
      "function": { "name": "get_weather", "arguments": "{\"city\": \"Toronto\"}" } }
  ]
}"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "get_weather");
        assert_eq!(calls[0].arguments["city"], "Toronto");
        assert_eq!(calls[0].call_id.as_deref(), Some("call_xyz"));
    }

    #[test]
    fn json_multiple_tool_calls() {
        let content = r#"{"tool_calls": [
            {"id": "c1", "type": "function",
             "function": {"name": "tool_a", "arguments": "{}"}},
            {"id": "c2", "type": "function",
             "function": {"name": "tool_b", "arguments": "{\"x\": 1}"}}
        ]}"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "tool_a");
        assert_eq!(calls[1].arguments["x"], 1);
    }

    #[test]
    fn json_arguments_as_object_not_string() {
        // Some providers return arguments as a JSON object, not a string.
        let content = r#"{"tool_calls": [
            {"type": "function",
             "function": {"name": "my_tool", "arguments": {"key": "value"}}}
        ]}"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].arguments["key"], "value");
    }

    #[test]
    fn json_empty_tool_calls_falls_back_to_xml_scan() {
        // JSON with empty tool_calls → no JSON calls → XML scan.
        let content = r#"{"tool_calls": []}
<tool_call>{"name": "fallback", "arguments": {}}</tool_call>"#;
        let calls = parse_tool_calls(content);
        // Empty JSON tool_calls → falls back to XML
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "fallback");
    }

    #[test]
    fn plain_text_returns_empty() {
        let content = "I don't need any tools for this.";
        assert!(parse_tool_calls(content).is_empty());
    }
}
