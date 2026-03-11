use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use serde_json::json;

use crate::db::{self, DbPool};
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

/// Agent tool for managing its own behavioral notes (self-evolving prompt).
/// Notes are stored in the database and automatically injected into context
/// when relevant domains are detected.
pub struct AgentSelfTool {
    db: DbPool,
    self_evolution_enabled: Arc<AtomicBool>,
}

impl AgentSelfTool {
    pub fn new(db: DbPool, self_evolution_enabled: Arc<AtomicBool>) -> Self {
        Self {
            db,
            self_evolution_enabled,
        }
    }
}

#[async_trait]
impl Tool for AgentSelfTool {
    fn name(&self) -> &str {
        "agent_notes"
    }

    fn description(&self) -> &str {
        "Manage your own behavioral notes. Notes you store are automatically available \
         in your context for future conversations when relevant. \
         Use 'learn' to record a discovery, preference, or pattern you want to remember. \
         Use 'rules' to review what you currently know. Use 'forget' to remove outdated notes. \
         Categories: general, channel, scheduling, user_preference, tool_usage. \
         Only store genuinely useful patterns — not ephemeral facts."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["learn", "rules", "forget"],
                    "description": "The operation to perform"
                },
                "content": {
                    "type": "string",
                    "description": "Rule/note content (required for learn)"
                },
                "category": {
                    "type": "string",
                    "description": "Category: general, channel, scheduling, user_preference, tool_usage (default: general)"
                },
                "id": {
                    "type": "integer",
                    "description": "Rule ID to forget (required for forget)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        if !self.self_evolution_enabled.load(Ordering::Relaxed) {
            return Ok(ToolResult::err(
                "Self-evolution is disabled. Enable it via config to use agent_notes.".to_string(),
            ));
        }

        let action = args["action"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'action' field".into()))?;

        match action {
            "learn" => self.learn(&args).await,
            "rules" => self.list_rules(&args).await,
            "forget" => self.forget(&args).await,
            other => Ok(ToolResult::err(format!(
                "Unknown action '{other}'. Valid actions: learn, rules, forget"
            ))),
        }
    }
}

const VALID_CATEGORIES: &[&str] = &[
    "general",
    "channel",
    "scheduling",
    "user_preference",
    "tool_usage",
];

impl AgentSelfTool {
    async fn learn(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let content = args["content"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'content' for learn".into()))?;

        if content.trim().is_empty() {
            return Ok(ToolResult::err("Content cannot be empty".to_string()));
        }

        let category = args["category"]
            .as_str()
            .unwrap_or("general")
            .to_string();

        if !VALID_CATEGORIES.contains(&category.as_str()) {
            return Ok(ToolResult::err(format!(
                "Invalid category '{category}'. Valid: {}",
                VALID_CATEGORIES.join(", ")
            )));
        }

        let content = content.to_string();
        let cat = category.clone();
        let pool = self.db.clone();

        let id: i64 = db::with_db(&pool, move |conn| {
            conn.execute(
                "INSERT INTO agent_rules (content, category) VALUES (?1, ?2)",
                rusqlite::params![content, cat],
            )
            .map_err(crate::MesoError::from)?;
            Ok(conn.last_insert_rowid())
        })
        .await?;

        Ok(ToolResult::ok(format!(
            "Learned rule #{id} (category: {category})"
        )))
    }

    async fn list_rules(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let category_filter = args["category"].as_str().map(|s| s.to_string());
        let pool = self.db.clone();

        let rules: Vec<serde_json::Value> = db::with_db(&pool, move |conn| {
            let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
                if let Some(ref cat) = category_filter {
                    (
                        "SELECT id, content, category, created_at FROM agent_rules \
                         WHERE active = 1 AND category = ?1 ORDER BY created_at"
                            .to_string(),
                        vec![Box::new(cat.clone())],
                    )
                } else {
                    (
                        "SELECT id, content, category, created_at FROM agent_rules \
                         WHERE active = 1 ORDER BY created_at"
                            .to_string(),
                        vec![],
                    )
                };

            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::types::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(param_refs.as_slice(), |row| {
                Ok(json!({
                    "id": row.get::<_, i64>(0)?,
                    "content": row.get::<_, String>(1)?,
                    "category": row.get::<_, String>(2)?,
                    "created_at": row.get::<_, String>(3)?,
                }))
            })?;

            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .await?;

        Ok(ToolResult::ok(
            serde_json::to_string_pretty(&rules).unwrap_or_default(),
        ))
    }

    async fn forget(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let id = args["id"]
            .as_i64()
            .ok_or_else(|| MesoError::Validation("missing 'id' for forget".into()))?;

        let pool = self.db.clone();
        let affected = db::with_db(&pool, move |conn| {
            let count = conn.execute(
                "UPDATE agent_rules SET active = 0 WHERE id = ?1 AND active = 1",
                rusqlite::params![id],
            )?;
            Ok(count)
        })
        .await?;

        if affected > 0 {
            Ok(ToolResult::ok(format!("Rule #{id} forgotten")))
        } else {
            Ok(ToolResult::err(format!(
                "Rule #{id} not found or already forgotten"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    async fn setup() -> (tempfile::TempDir, AgentSelfTool) {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let enabled = Arc::new(AtomicBool::new(true));
        let tool = AgentSelfTool::new(pool, enabled);
        (dir, tool)
    }

    #[test]
    fn tool_name_and_schema() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let enabled = Arc::new(AtomicBool::new(true));
        let tool = AgentSelfTool::new(pool, enabled);

        assert_eq!(tool.name(), "agent_notes");
        assert!(tool.description().contains("behavioral notes"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
    }

    #[tokio::test]
    async fn learn_stores_rule() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "learn",
                "content": "User prefers dark mode",
                "category": "user_preference"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("Learned rule #1"));
        assert!(result.output.contains("user_preference"));
    }

    #[tokio::test]
    async fn learn_defaults_to_general_category() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "learn",
                "content": "Some general note"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("general"));
    }

    #[tokio::test]
    async fn learn_rejects_invalid_category() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "learn",
                "content": "test",
                "category": "invalid_cat"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Invalid category"));
    }

    #[tokio::test]
    async fn learn_rejects_empty_content() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "learn",
                "content": "  "
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("empty"));
    }

    #[tokio::test]
    async fn rules_lists_stored_rules() {
        let (_dir, tool) = setup().await;

        // Store two rules
        tool.execute(json!({
            "action": "learn",
            "content": "Rule 1",
            "category": "channel"
        }))
        .await
        .unwrap();
        tool.execute(json!({
            "action": "learn",
            "content": "Rule 2",
            "category": "general"
        }))
        .await
        .unwrap();

        // List all
        let result = tool.execute(json!({ "action": "rules" })).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("Rule 1"));
        assert!(result.output.contains("Rule 2"));
    }

    #[tokio::test]
    async fn rules_filters_by_category() {
        let (_dir, tool) = setup().await;

        tool.execute(json!({
            "action": "learn",
            "content": "Channel rule",
            "category": "channel"
        }))
        .await
        .unwrap();
        tool.execute(json!({
            "action": "learn",
            "content": "General rule",
            "category": "general"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({ "action": "rules", "category": "channel" }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("Channel rule"));
        assert!(!result.output.contains("General rule"));
    }

    #[tokio::test]
    async fn forget_deactivates_rule() {
        let (_dir, tool) = setup().await;

        tool.execute(json!({
            "action": "learn",
            "content": "Forget me"
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({ "action": "forget", "id": 1 }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("forgotten"));

        // Should no longer appear in list
        let list = tool.execute(json!({ "action": "rules" })).await.unwrap();
        assert!(!list.output.contains("Forget me"));
    }

    #[tokio::test]
    async fn forget_nonexistent_returns_error() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({ "action": "forget", "id": 999 }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("not found"));
    }

    #[tokio::test]
    async fn disabled_self_evolution_blocks_all_actions() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let enabled = Arc::new(AtomicBool::new(false));
        let tool = AgentSelfTool::new(pool, enabled);

        let result = tool
            .execute(json!({
                "action": "learn",
                "content": "test"
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("disabled"));
    }

    #[tokio::test]
    async fn invalid_action_returns_error() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({ "action": "invalid" }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Unknown action"));
    }
}
