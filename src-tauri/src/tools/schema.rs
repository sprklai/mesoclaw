//! Tool schema generation for LLM context.
//!
//! Provides functions to generate text descriptions of tools that can be
//! injected into the system prompt so the LLM knows what tools are available
//! and how to invoke them.

use super::profiles::ToolProfile;
use super::registry::ToolRegistry;

/// Generate a text description of tools for LLM context, filtered by profile.
///
/// This function creates a markdown-formatted section that describes all
/// available tools (filtered by the given profile) with their parameters
/// schemas in JSON format. The output is designed to be injected into the
/// system prompt so the LLM knows:
///
/// 1. What tools are available
/// 2. What each tool does
/// 3. What parameters each tool accepts
/// 4. How to format tool invocations
///
/// # Arguments
/// * `registry` - The tool registry containing all registered tools
/// * `profile` - The tool profile to filter tools by (e.g., Messaging for channels)
///
/// # Returns
/// A markdown-formatted string describing the available tools
pub fn generate_tool_schema_text(registry: &ToolRegistry, profile: ToolProfile) -> String {
    let mut output = String::from("# Available Tools\n\n");
    output += "You have access to the following tools. To use a tool, output a JSON block:\n";
    output += "```json\n{\"tool\": \"tool_name\", \"arguments\": {...}}\n```\n\n";
    output += "You can also use this format:\n";
    output += "```tool\nname: tool_name\narg1: value1\narg2: value2\n```\n\n";
    output += "---\n\n";

    let tools = registry.list_filtered(profile);

    if tools.is_empty() {
        output += "*No tools available for the current profile.*\n";
        return output;
    }

    for tool in tools {
        output += &format!("## {}\n\n", tool.name);
        output += &format!("{}\n\n", tool.description);
        output += "**Parameters:**\n```json\n";
        output += &serde_json::to_string_pretty(&tool.schema).unwrap_or_else(|e| {
            log::warn!("Failed to serialize schema for tool {}: {}", tool.name, e);
            format!("{{\"error\": \"{}\"}}", e)
        });
        output += "\n```\n\n---\n\n";
    }

    output
}

/// Generate a compact one-line description of available tools.
///
/// Useful for contexts where token usage is critical.
pub fn generate_tool_summary(registry: &ToolRegistry, profile: ToolProfile) -> String {
    let tools = registry.list_filtered(profile);
    if tools.is_empty() {
        return "No tools available.".to_string();
    }

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    format!("Available tools: {}", names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::traits::{Tool, ToolResult};
    use async_trait::async_trait;
    use serde_json::{Value, json};
    use std::sync::Arc;

    struct TestTool {
        name: &'static str,
        desc: &'static str,
        schema: Value,
    }

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            self.desc
        }
        fn parameters_schema(&self) -> Value {
            self.schema.clone()
        }
        async fn execute(&self, _args: Value) -> Result<ToolResult, String> {
            Ok(ToolResult::ok("done"))
        }
    }

    #[test]
    fn generates_schema_for_allowed_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool {
            name: "web_search",
            desc: "Search the web",
            schema: json!({"type": "object", "properties": {"query": {"type": "string"}}}),
        }));
        registry.register(Arc::new(TestTool {
            name: "shell",
            desc: "Execute shell commands",
            schema: json!({"type": "object", "properties": {"command": {"type": "string"}}}),
        }));

        // Messaging profile allows web tools but not shell
        let output = generate_tool_schema_text(&registry, ToolProfile::Messaging);

        assert!(output.contains("# Available Tools"));
        assert!(output.contains("web_search"));
        assert!(output.contains("Search the web"));
        assert!(!output.contains("shell")); // shell not allowed in Messaging profile
    }

    #[test]
    fn generates_empty_output_for_no_tools() {
        let registry = ToolRegistry::new();
        let output = generate_tool_schema_text(&registry, ToolProfile::Full);

        assert!(output.contains("No tools available"));
    }

    #[test]
    fn summary_lists_tool_names() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestTool {
            name: "web_search",
            desc: "Search",
            schema: json!({}),
        }));
        registry.register(Arc::new(TestTool {
            name: "web_fetch",
            desc: "Fetch",
            schema: json!({}),
        }));

        let summary = generate_tool_summary(&registry, ToolProfile::Messaging);
        assert!(summary.contains("web_search"));
        assert!(summary.contains("web_fetch"));
    }
}
