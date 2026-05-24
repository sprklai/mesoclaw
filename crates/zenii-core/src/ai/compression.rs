use crate::config::AppConfig;

/// Compresses tool output strings to reduce token usage in LLM context windows.
///
/// Applies tool-specific compression rules (line truncation, result limiting,
/// snippet truncation) and a hard character ceiling. Disabled outputs and
/// failed tool calls pass through unchanged.
pub struct ToolOutputCompressor {
    enabled: bool,
    max_output_chars: usize,
    web_search_results: usize,
    file_max_lines: usize,
    shell_max_lines: usize,
}

impl Default for ToolOutputCompressor {
    fn default() -> Self {
        Self {
            enabled: true,
            max_output_chars: 8000,
            web_search_results: 3,
            file_max_lines: 200,
            shell_max_lines: 100,
        }
    }
}

impl ToolOutputCompressor {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            enabled: config.compression_enabled,
            max_output_chars: config.compression_max_output_chars,
            web_search_results: config.compression_web_search_results,
            file_max_lines: config.compression_file_max_lines,
            shell_max_lines: config.compression_shell_max_lines,
        }
    }

    /// Compress tool output string. Returns compressed string (may be identical if no rule applies).
    /// Always passes through unchanged if `success` is `false` or compression is disabled.
    pub fn compress(&self, tool_name: &str, output: &str, success: bool) -> String {
        if !self.enabled || !success {
            return output.to_string();
        }

        let compressed = match tool_name {
            "web_search" => self.compress_web_search(output),
            "file_read" => self.compress_lines(output, self.file_max_lines),
            "shell" => self.compress_lines(output, self.shell_max_lines),
            _ => output.to_string(),
        };

        self.apply_hard_ceiling(compressed)
    }

    fn compress_web_search(&self, output: &str) -> String {
        // Try to parse as JSON array; keep top-N results; truncate long snippets (>300 chars).
        // If not parseable as JSON array, fall through to ceiling only.
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(output) {
            let kept: Vec<serde_json::Value> = arr
                .into_iter()
                .take(self.web_search_results)
                .map(|mut v| {
                    if let Some(obj) = v.as_object_mut() {
                        let content_fields =
                            ["snippet", "body", "content", "description", "summary"];
                        for key in &content_fields {
                            if let Some(val) = obj.get_mut(*key)
                                && let Some(s) = val.as_str()
                                && s.len() > 300
                            {
                                let cutoff = char_boundary_at_or_before(s, 300);
                                *val = serde_json::Value::String(format!("{}...", &s[..cutoff]));
                            }
                        }
                    }
                    v
                })
                .collect();
            serde_json::to_string(&kept).unwrap_or_else(|_| output.to_string())
        } else {
            output.to_string()
        }
    }

    fn compress_lines(&self, output: &str, max_lines: usize) -> String {
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() <= max_lines {
            return output.to_string();
        }
        let kept = lines[..max_lines].join("\n");
        format!("{kept}\n...[{} more lines]", lines.len() - max_lines)
    }

    fn apply_hard_ceiling(&self, s: String) -> String {
        if s.len() <= self.max_output_chars {
            return s;
        }
        let cutoff = char_boundary_at_or_before(&s, self.max_output_chars);
        format!(
            "{}...[truncated at {} chars]",
            &s[..cutoff], self.max_output_chars
        )
    }
}

/// Find the largest char boundary index <= `max`.
fn char_boundary_at_or_before(s: &str, max: usize) -> usize {
    if max >= s.len() {
        return s.len();
    }
    // Walk back from max until we land on a char boundary.
    let mut idx = max;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compressor() -> ToolOutputCompressor {
        ToolOutputCompressor {
            enabled: true,
            max_output_chars: 100,
            web_search_results: 2,
            file_max_lines: 5,
            shell_max_lines: 3,
        }
    }

    // TJ.1 — web_search keeps top-N results
    #[test]
    fn web_search_keeps_top_n_results() {
        let c = compressor();
        let input = serde_json::json!([
            {"title": "A", "url": "http://a.com", "snippet": "a"},
            {"title": "B", "url": "http://b.com", "snippet": "b"},
            {"title": "C", "url": "http://c.com", "snippet": "c"},
            {"title": "D", "url": "http://d.com", "snippet": "d"},
            {"title": "E", "url": "http://e.com", "snippet": "e"},
        ])
        .to_string();

        let result = c.compress("web_search", &input, true);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["title"], "A");
        assert_eq!(parsed[1]["title"], "B");
    }

    // TJ.2 — web_search truncates long snippets
    #[test]
    fn web_search_truncates_long_snippets() {
        // Use a large ceiling so the hard ceiling doesn't interfere with the
        // snippet-truncation check.
        let c = ToolOutputCompressor {
            enabled: true,
            max_output_chars: 10_000,
            web_search_results: 2,
            file_max_lines: 5,
            shell_max_lines: 3,
        };
        let long_snippet = "x".repeat(400);
        let input = serde_json::json!([
            {"title": "A", "snippet": long_snippet},
        ])
        .to_string();

        let result = c.compress("web_search", &input, true);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        let snippet = parsed[0]["snippet"].as_str().unwrap();
        // 300 chars + "..."
        assert!(snippet.ends_with("..."), "should end with '...'");
        assert!(
            snippet.len() <= 303,
            "snippet should be at most 303 chars, got {}",
            snippet.len()
        );
    }

    // TJ.3 — file_read truncates at max_lines with marker
    #[test]
    fn file_read_truncates_at_max_lines_with_marker() {
        let c = compressor();
        let input: String = (1..=10).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");

        let result = c.compress("file_read", &input, true);
        let lines: Vec<&str> = result.lines().collect();
        // 5 content lines + 1 marker line
        assert_eq!(lines.len(), 6, "should have 5 kept lines + 1 marker");
        assert!(
            lines.last().unwrap().contains("5 more lines"),
            "marker should say '5 more lines', got: {}",
            lines.last().unwrap()
        );
    }

    // TJ.4 — shell truncates at max_lines
    #[test]
    fn shell_truncates_at_max_lines() {
        let c = compressor();
        let input: String = (1..=5).map(|i| format!("out {i}")).collect::<Vec<_>>().join("\n");

        let result = c.compress("shell", &input, true);
        let lines: Vec<&str> = result.lines().collect();
        // 3 content lines + 1 marker line
        assert_eq!(lines.len(), 4, "should have 3 kept lines + 1 marker");
        assert!(
            lines.last().unwrap().contains("2 more lines"),
            "marker should say '2 more lines', got: {}",
            lines.last().unwrap()
        );
    }

    // TJ.5 — hard ceiling applied when over max_chars
    #[test]
    fn hard_ceiling_applied_when_over_max_chars() {
        let c = compressor();
        // Use system_info so no line-level rule fires, only ceiling
        let input = "a".repeat(200);
        let result = c.compress("system_info", &input, true);
        assert!(
            result.len() < 200,
            "result should be shorter than input after ceiling"
        );
        assert!(
            result.contains("truncated at 100 chars"),
            "should contain truncation marker, got: {result}"
        );
    }

    // TJ.6 — failed tool output not compressed
    #[test]
    fn failed_tool_output_not_compressed() {
        let c = compressor();
        let input = "a".repeat(200);
        let result = c.compress("shell", &input, false);
        assert_eq!(result, input, "failed output should pass through unchanged");
    }

    // TJ.7 — disabled compressor passes through unchanged
    #[test]
    fn disabled_compressor_passthrough() {
        let c = ToolOutputCompressor {
            enabled: false,
            ..compressor()
        };
        let input = "a".repeat(200);
        let result = c.compress("shell", &input, true);
        assert_eq!(result, input, "disabled compressor should pass through");
    }

    // TJ.8 — unknown tool name: only ceiling applied
    #[test]
    fn unknown_tool_name_only_ceiling_applied() {
        let c = compressor();
        // Short output — ceiling not hit, no line rule for unknown tool
        let input = "hello world";
        let result = c.compress("system_info", input, true);
        assert_eq!(result, input, "short unknown-tool output should be unchanged");
    }
}
