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
//!
//! # Hardening notes
//! - Individual malformed entries are **skipped** (not propagated as errors) so
//!   that one bad call never silently discards all other valid calls.
//! - Unicode content in tool arguments is preserved correctly.
//! - Extremely long responses are handled without additional allocation beyond
//!   the normal slice operations.
//! - Use [`has_partial_tool_call`] to detect incomplete streaming payloads.

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
///
/// Individual malformed entries are skipped rather than causing the whole
/// parse to fail, so a single bad call never silently discards valid ones.
pub fn parse_tool_calls(content: &str) -> Vec<ParsedToolCall> {
    // Prefer the JSON format (explicit contract).
    if let Some(calls) = try_parse_json(content)
        && !calls.is_empty()
    {
        return calls;
    }
    // Fall back to XML scanning.
    parse_xml(content)
}

/// Return `true` when `content` contains an *unclosed* `<tool_call>` tag.
///
/// Useful during streaming to detect that a tool call has started but not yet
/// been fully received.
pub fn has_partial_tool_call(content: &str) -> bool {
    const OPEN: &str = "<tool_call>";
    const CLOSE: &str = "</tool_call>";
    let open_count = content.matches(OPEN).count();
    let close_count = content.matches(CLOSE).count();
    open_count > close_count
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
        // Use `let Some(...) else { continue }` so that one malformed entry
        // never silently discards all subsequent valid entries.
        let Some(func) = entry.get("function") else {
            continue;
        };
        let Some(name_str) = func.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let name = name_str.to_string();
        let call_id = entry.get("id").and_then(|v| v.as_str()).map(str::to_string);

        // `arguments` may be a JSON string (OpenAI) or an object (some providers).
        let arguments = match func.get("arguments") {
            Some(Value::String(s)) => {
                serde_json::from_str(s).unwrap_or(Value::Object(Default::default()))
            }
            Some(v) => v.clone(),
            None => Value::Object(Default::default()),
        };

        result.push(ParsedToolCall {
            name,
            arguments,
            call_id,
        });
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
    Some(ParsedToolCall {
        name,
        arguments,
        call_id: None,
    })
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

    // ── Edge cases ────────────────────────────────────────────────────────────

    #[test]
    fn json_skips_entry_missing_function_field() {
        // First entry is malformed (no `function` key); second is valid.
        // The fix ensures the second entry is NOT lost.
        let content = r#"{"tool_calls": [
            {"id": "bad", "type": "function"},
            {"id": "good", "type": "function",
             "function": {"name": "valid_tool", "arguments": "{}"}}
        ]}"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1, "valid entry should still be parsed");
        assert_eq!(calls[0].name, "valid_tool");
    }

    #[test]
    fn json_skips_entry_missing_name_field() {
        // Entry with a `function` object but no `name` key.
        let content = r#"{"tool_calls": [
            {"type": "function", "function": {"arguments": "{}"}},
            {"type": "function", "function": {"name": "ok_tool", "arguments": "{}"}}
        ]}"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "ok_tool");
    }

    #[test]
    fn json_null_tool_calls_falls_back_to_xml() {
        // `tool_calls: null` is not an array → falls back to XML scan.
        let content = r#"{"tool_calls": null}
<tool_call>{"name": "xml_tool", "arguments": {}}</tool_call>"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "xml_tool");
    }

    #[test]
    fn xml_unicode_in_arguments() {
        let content = "<tool_call>{\"name\": \"translate\", \"arguments\": {\"text\": \"こんにちは世界\"}}</tool_call>";
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].arguments["text"], "こんにちは世界");
    }

    #[test]
    fn xml_emoji_in_tool_name() {
        // Unusual but should not crash.
        let content = "<tool_call>{\"name\": \"search_🔍\", \"arguments\": {}}</tool_call>";
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "search_🔍");
    }

    #[test]
    fn xml_arguments_contain_angle_brackets() {
        // Arguments with HTML-like strings should not confuse the parser since
        // we search for the exact `</tool_call>` close sequence.
        let content =
            r#"<tool_call>{"name": "echo", "arguments": {"msg": "<b>bold</b>"}}</tool_call>"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].arguments["msg"], "<b>bold</b>");
    }

    #[test]
    fn xml_mixed_valid_and_invalid_entries() {
        let content = r#"
<tool_call>{"name": "first", "arguments": {"n": 1}}</tool_call>
<tool_call>not json at all!!!</tool_call>
<tool_call>{"name": "third", "arguments": {"n": 3}}</tool_call>
"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "first");
        assert_eq!(calls[1].name, "third");
    }

    #[test]
    fn xml_whitespace_only_content() {
        let content = "   \n\t  ";
        assert!(parse_tool_calls(content).is_empty());
    }

    #[test]
    fn xml_very_long_argument_string() {
        // Fuzzing: 10 000-char string in an argument should not panic.
        let big = "A".repeat(10_000);
        let content = format!(
            r#"<tool_call>{{"name": "big_tool", "arguments": {{"data": "{}"}}}}</tool_call>"#,
            big
        );
        let calls = parse_tool_calls(&content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].arguments["data"].as_str().unwrap().len(), 10_000);
    }

    #[test]
    fn xml_many_tool_calls_does_not_panic() {
        // Fuzzing: 500 tool calls in one response.
        let single = r#"<tool_call>{"name": "t", "arguments": {}}</tool_call>"#;
        let content = single.repeat(500);
        let calls = parse_tool_calls(&content);
        assert_eq!(calls.len(), 500);
    }

    #[test]
    fn xml_nested_open_tag_in_arguments() {
        // A literal `<tool_call>` string inside an argument value must not
        // confuse the nesting — we match CLOSE strictly.
        let content = r#"<tool_call>{"name": "doc", "arguments": {"snippet": "<tool_call> text"}}</tool_call>"#;
        let calls = parse_tool_calls(content);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].arguments["snippet"], "<tool_call> text");
    }

    // ── Streaming / partial tool call detection ───────────────────────────────

    #[test]
    fn partial_tool_call_detected() {
        let partial = r#"Let me search. <tool_call>{"name": "search""#;
        assert!(has_partial_tool_call(partial));
    }

    #[test]
    fn complete_tool_call_not_partial() {
        let complete = r#"<tool_call>{"name": "search", "arguments": {}}</tool_call>"#;
        assert!(!has_partial_tool_call(complete));
    }

    #[test]
    fn no_tool_call_not_partial() {
        assert!(!has_partial_tool_call("Just some text."));
    }

    #[test]
    fn multiple_complete_calls_not_partial() {
        let content = r#"
<tool_call>{"name": "a", "arguments": {}}</tool_call>
<tool_call>{"name": "b", "arguments": {}}</tool_call>
"#;
        assert!(!has_partial_tool_call(content));
    }

    #[test]
    fn one_complete_one_partial() {
        let content = r#"<tool_call>{"name": "a", "arguments": {}}</tool_call>
<tool_call>{"name": "b""#;
        assert!(has_partial_tool_call(content));
    }
}
