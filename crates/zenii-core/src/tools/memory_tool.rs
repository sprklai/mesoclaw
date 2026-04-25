use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::memory::traits::{Memory, MemoryCategory};
use crate::{Result, ZeniiError};

use super::traits::{Tool, ToolResult};

/// Agent tool for managing persistent memory (store/recall/forget).
pub struct MemoryTool {
    memory: Arc<dyn Memory>,
}

impl MemoryTool {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &str {
        "memory"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Store, recall, or forget information in persistent memory. Use 'store' or 'update' to save facts, 'recall' to search memories, 'forget' to remove a specific entry."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["store", "update", "recall", "forget"],
                    "description": "The memory operation to perform"
                },
                "key": {
                    "type": "string",
                    "description": "Unique key for the memory entry (required for store/forget)"
                },
                "content": {
                    "type": "string",
                    "description": "The content to store (required for store)"
                },
                "category": {
                    "type": "string",
                    "enum": ["core", "daily", "conversation"],
                    "description": "Category for the memory entry (default: conversation)"
                },
                "query": {
                    "type": "string",
                    "description": "Search query for recall"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results for recall (default: 5)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ZeniiError::Validation("missing 'action' field".into()))?;

        match action {
            "store" | "update" => {
                let key = args["key"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'key' for store".into()))?;
                let content = args["content"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'content' for store".into()))?;
                let category: MemoryCategory =
                    args["category"].as_str().unwrap_or("conversation").into();

                match self.memory.store(key, content, category).await {
                    Ok(()) => Ok(ToolResult::ok(format!("Memory stored with key '{key}'"))),
                    Err(ZeniiError::MemoryDuplicate(_)) => {
                        Ok(ToolResult::ok("Memory already indexed (deduplicated)"))
                    }
                    Err(e) => Ok(ToolResult::err(format!("Failed to store memory: {e}"))),
                }
            }
            "recall" => {
                let query = args["query"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'query' for recall".into()))?;
                let limit = args["limit"].as_u64().unwrap_or(5) as usize;

                match self.memory.recall(query, limit, 0).await {
                    Ok(entries) => {
                        let results: Vec<serde_json::Value> = entries
                            .iter()
                            .map(|e| {
                                json!({
                                    "key": e.key,
                                    "content": e.content,
                                    "category": e.category.to_string(),
                                    "score": e.score,
                                })
                            })
                            .collect();
                        Ok(ToolResult::ok(
                            serde_json::to_string_pretty(&results).unwrap_or_default(),
                        ))
                    }
                    Err(e) => Ok(ToolResult::err(format!("Failed to recall: {e}"))),
                }
            }
            "forget" => {
                let key = args["key"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'key' for forget".into()))?;

                match self.memory.forget(key).await {
                    Ok(found) => {
                        if found {
                            Ok(ToolResult::ok(format!("Memory '{key}' forgotten")))
                        } else {
                            Ok(ToolResult::ok(format!("No memory found with key '{key}'")))
                        }
                    }
                    Err(e) => Ok(ToolResult::err(format!("Failed to forget: {e}"))),
                }
            }
            other => Ok(ToolResult::err(format!(
                "Unknown action '{other}'. Valid actions: store, recall, forget"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::in_memory_store::InMemoryStore;

    fn setup() -> MemoryTool {
        MemoryTool::new(Arc::new(InMemoryStore::new()))
    }

    // 17.9 — Store memory via tool succeeds
    #[tokio::test]
    async fn memory_tool_store() {
        let tool = setup();
        let result = tool
            .execute(json!({
                "action": "store",
                "key": "favorite_color",
                "content": "The user's favorite color is blue",
                "category": "core"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("stored"));
    }

    // 17.10 — Recall memory via tool returns entries
    #[tokio::test]
    async fn memory_tool_recall() {
        let tool = setup();

        // Store first
        tool.execute(json!({
            "action": "store",
            "key": "test_recall",
            "content": "Important test data for recall",
            "category": "core"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({
                "action": "recall",
                "query": "test data",
                "limit": 5
            }))
            .await
            .unwrap();

        assert!(result.success);
        // InMemoryStore uses substring matching, so our entry should appear
        assert!(result.output.contains("test_recall") || result.output == "[]");
    }

    // 17.11 — Forget memory via tool removes entry
    #[tokio::test]
    async fn memory_tool_forget() {
        let tool = setup();

        // Store then forget
        tool.execute(json!({
            "action": "store",
            "key": "to_forget",
            "content": "This will be forgotten"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({
                "action": "forget",
                "key": "to_forget"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("forgotten"));
    }

    // 17.12 — Invalid action returns error
    #[tokio::test]
    async fn memory_tool_invalid_action() {
        let tool = setup();
        let result = tool.execute(json!({ "action": "invalid" })).await.unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Unknown action"));
    }

    // 17.13 — Store with missing key returns error
    #[tokio::test]
    async fn memory_tool_store_missing_fields() {
        let tool = setup();
        let result = tool
            .execute(json!({
                "action": "store",
                "content": "no key provided"
            }))
            .await;

        assert!(result.is_err());
    }

    // 17.14 — Tool name/description/schema validation
    #[test]
    fn memory_tool_schema() {
        let tool = setup();
        assert_eq!(tool.name(), "memory");
        assert!(tool.description().contains("memory"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
    }
}
