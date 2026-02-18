use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The result of executing a [`Tool`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Human-readable output (stdout, file contents, directory listing, etc.).
    pub output: String,
    /// Whether the tool considered the execution successful.
    pub success: bool,
    /// Optional structured metadata for machine consumption.
    pub metadata: Option<Value>,
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
            metadata: None,
        }
    }

    pub fn err(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: false,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// A capability that the agent loop can invoke.
///
/// Implementations must be `Send + Sync` so they can be held in a shared
/// `Arc<dyn Tool>` registry.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Stable machine identifier (snake_case), e.g. `"shell"`.
    fn name(&self) -> &str;

    /// Human-readable description used in LLM `tool_use` payloads.
    fn description(&self) -> &str;

    /// JSON Schema object describing the tool's accepted parameters.
    fn parameters_schema(&self) -> Value;

    /// Execute the tool with the given arguments (validated against the schema
    /// by the caller when invoking from the agent loop).
    async fn execute(&self, args: Value) -> Result<ToolResult, String>;
}

/// Summary of a registered tool, suitable for inclusion in LLM API requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub schema: Value,
}
