//! Application adapter trait for the skill system.
//!
//! This trait abstracts application-specific functionality, allowing
//! the skill engine to remain domain-agnostic.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait that applications implement to provide context and tools to skills.
#[async_trait]
pub trait ApplicationAdapter: Send + Sync {
    /// Unique identifier for this application.
    fn app_id(&self) -> &str;

    /// List of context types this adapter can provide.
    fn available_context(&self) -> Vec<ContextType>;

    /// List of tools this adapter provides.
    fn available_tools(&self) -> Vec<ToolDefinition>;

    /// Fetch context values for the given keys.
    async fn get_context(&self, keys: &[&str]) -> Result<ContextBag, AdapterError>;

    /// Execute a tool call.
    async fn execute_tool(&self, call: ToolCall) -> Result<ToolResult, AdapterError>;

    /// Map skill output to application-specific format (optional).
    fn map_output(
        &self,
        output: crate::prompts::SkillOutput,
    ) -> Result<serde_json::Value, AdapterError> {
        Ok(serde_json::to_value(output)?)
    }
}

/// Description of a context type that an adapter can provide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextType {
    /// Unique key for this context (e.g., "schema", "query")
    pub key: String,

    /// Human-readable description
    pub description: String,

    /// Expected value type (for documentation)
    pub value_type: String,
}

impl ContextType {
    /// Create a new context type.
    pub fn new(key: impl Into<String>, description: impl Into<String>, value_type: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
            value_type: value_type.into(),
        }
    }
}

/// Definition of a tool that an adapter provides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (e.g., "execute_query")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// JSON Schema for parameters
    pub parameters: serde_json::Value,

    /// Whether the tool has side effects
    #[serde(default)]
    pub has_side_effects: bool,
}

impl ToolDefinition {
    /// Create a new tool definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
            has_side_effects: false,
        }
    }

    /// Set the parameters schema.
    pub fn with_parameters(mut self, schema: serde_json::Value) -> Self {
        self.parameters = schema;
        self
    }

    /// Mark as having side effects.
    pub fn with_side_effects(mut self) -> Self {
        self.has_side_effects = true;
        self
    }
}

/// Bag of context values retrieved from the adapter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextBag {
    /// The context values indexed by key
    pub values: HashMap<String, serde_json::Value>,
}

impl ContextBag {
    /// Create an empty context bag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a value into the context bag.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Serialize) -> Result<(), serde_json::Error> {
        self.values.insert(key.into(), serde_json::to_value(value)?);
        Ok(())
    }

    /// Get a value from the context bag.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.values.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get a raw JSON value from the context bag.
    pub fn get_raw(&self, key: &str) -> Option<&serde_json::Value> {
        self.values.get(key)
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Merge another context bag into this one.
    pub fn merge(&mut self, other: ContextBag) {
        self.values.extend(other.values);
    }
}

/// A tool call request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Name of the tool to call
    pub tool_name: String,

    /// Parameters for the tool
    pub parameters: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(tool_name: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            tool_name: tool_name.into(),
            parameters,
        }
    }
}

/// Result of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the call succeeded
    pub success: bool,

    /// The result data (if successful)
    pub data: Option<serde_json::Value>,

    /// Error message (if failed)
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful result.
    pub fn success(data: impl Serialize) -> Result<Self, serde_json::Error> {
        Ok(Self {
            success: true,
            data: Some(serde_json::to_value(data)?),
            error: None,
        })
    }

    /// Create a successful result with raw JSON.
    pub fn success_raw(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

/// Errors that can occur in adapter operations.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Context not available: {0}")]
    ContextNotAvailable(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Not connected to workspace")]
    NotConnected,
}
