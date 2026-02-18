//! Agent-callable tools backed by the memory subsystem.
//!
//! These three tools expose the [`Memory`] trait to the agent loop so that the
//! LLM can store, retrieve, and delete facts during a session.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::tools::{Tool, ToolResult};

use super::traits::{Memory, MemoryCategory};

// ─── MemoryStoreTool ─────────────────────────────────────────────────────────

/// Agent tool: store a fact in memory.
///
/// Parameters (JSON object):
/// - `key`      — namespaced lookup key, e.g. `"user:name"` (required)
/// - `content`  — text to store (required)
/// - `category` — one of `"core"`, `"daily"`, `"conversation"`, `"custom:<tag>"` (optional, default `"core"`)
pub struct MemoryStoreTool {
    memory: Arc<dyn Memory>,
}

impl MemoryStoreTool {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for MemoryStoreTool {
    fn name(&self) -> &str {
        "memory_store"
    }

    fn description(&self) -> &str {
        "Store a fact in the agent's persistent memory. \
         Use this to remember information across turns."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["key", "content"],
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Namespaced lookup key (e.g. 'user:name', 'project:goal')."
                },
                "content": {
                    "type": "string",
                    "description": "Text content to store."
                },
                "category": {
                    "type": "string",
                    "description": "Category: 'core', 'daily', 'conversation', or 'custom:<tag>'. Defaults to 'core'.",
                    "default": "core"
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("missing required parameter 'key'")?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("missing required parameter 'content'")?;
        let category = parse_category(args.get("category").and_then(|v| v.as_str()));

        self.memory.store(key, content, category).await?;
        Ok(ToolResult::ok(format!("Stored memory: {key}")))
    }
}

// ─── MemoryRecallTool ─────────────────────────────────────────────────────────

/// Agent tool: search memory by semantic query.
///
/// Parameters (JSON object):
/// - `query` — natural-language search query (required)
/// - `limit` — maximum number of results to return (optional, default 5)
pub struct MemoryRecallTool {
    memory: Arc<dyn Memory>,
}

impl MemoryRecallTool {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for MemoryRecallTool {
    fn name(&self) -> &str {
        "memory_recall"
    }

    fn description(&self) -> &str {
        "Search the agent's persistent memory for entries relevant to a query. \
         Returns the most relevant facts sorted by relevance score."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural-language search query."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return. Default: 5.",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 50
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or("missing required parameter 'query'")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        let entries = self.memory.recall(query, limit).await?;
        if entries.is_empty() {
            return Ok(ToolResult::ok("No matching memories found."));
        }

        let lines: Vec<String> = entries
            .iter()
            .map(|e| format!("[{:.2}] {} — {}", e.score, e.key, e.content))
            .collect();
        Ok(ToolResult::ok(lines.join("\n")))
    }
}

// ─── MemoryForgetTool ─────────────────────────────────────────────────────────

/// Agent tool: remove a memory entry by key.
///
/// Parameters (JSON object):
/// - `key` — lookup key of the entry to remove (required)
pub struct MemoryForgetTool {
    memory: Arc<dyn Memory>,
}

impl MemoryForgetTool {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for MemoryForgetTool {
    fn name(&self) -> &str {
        "memory_forget"
    }

    fn description(&self) -> &str {
        "Remove an entry from the agent's persistent memory by its key."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["key"],
            "properties": {
                "key": {
                    "type": "string",
                    "description": "The lookup key of the memory entry to remove."
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("missing required parameter 'key'")?;

        let found = self.memory.forget(key).await?;
        if found {
            Ok(ToolResult::ok(format!("Removed memory: {key}")))
        } else {
            Ok(ToolResult::ok(format!(
                "No memory entry found for key: {key}"
            )))
        }
    }
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn parse_category(s: Option<&str>) -> MemoryCategory {
    match s {
        None | Some("core") => MemoryCategory::Core,
        Some("daily") => MemoryCategory::Daily,
        Some("conversation") => MemoryCategory::Conversation,
        Some(other) => {
            if let Some(tag) = other.strip_prefix("custom:") {
                MemoryCategory::Custom(tag.to_owned())
            } else {
                MemoryCategory::Custom(other.to_owned())
            }
        }
    }
}
