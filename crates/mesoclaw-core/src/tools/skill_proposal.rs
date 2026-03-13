use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use serde_json::json;

use crate::db::{self, DbPool};
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

const VALID_ACTIONS: &[&str] = &["create", "update", "delete"];

/// Agent tool for proposing skill changes (create/update/delete).
/// Proposals are stored in the database and require user approval.
pub struct SkillProposalTool {
    db: DbPool,
    enabled: Arc<AtomicBool>,
}

impl SkillProposalTool {
    pub fn new(db: DbPool, enabled: Arc<AtomicBool>) -> Self {
        Self { db, enabled }
    }
}

#[async_trait]
impl Tool for SkillProposalTool {
    fn name(&self) -> &str {
        "propose_skill_change"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Propose creating, updating, or deleting a skill. Requires user approval before execution."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": VALID_ACTIONS,
                    "description": "Action to perform: create, update, or delete"
                },
                "skill_name": {
                    "type": "string",
                    "description": "Name of the skill"
                },
                "content": {
                    "type": "string",
                    "description": "Skill content (required for create/update, optional for delete)"
                },
                "rationale": {
                    "type": "string",
                    "description": "Why this change is proposed"
                }
            },
            "required": ["action", "skill_name", "rationale"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(ToolResult::err(
                "Self-evolution is disabled. Enable `self_evolution_enabled` in config.",
            ));
        }

        let action = args["action"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'action' field".into()))?;
        let skill_name = args["skill_name"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'skill_name' field".into()))?;
        let content = args["content"].as_str().map(|s| s.to_string());
        let rationale = args["rationale"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'rationale' field".into()))?;

        // Validate action
        if !VALID_ACTIONS.contains(&action) {
            return Ok(ToolResult::err(format!(
                "Invalid action '{}'. Valid: {}",
                action,
                VALID_ACTIONS.join(", ")
            )));
        }

        // Create/update requires content
        if (action == "create" || action == "update") && content.is_none() {
            return Ok(ToolResult::err(format!(
                "'{action}' action requires 'content' field"
            )));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let proposal_id = id.clone();
        let action = action.to_string();
        let skill_name = skill_name.to_string();
        let rationale = rationale.to_string();

        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO skill_proposals (id, action, skill_name, content, rationale)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![proposal_id, action, skill_name, content, rationale],
            )
            .map_err(crate::MesoError::from)?;
            Ok(())
        })
        .await?;

        Ok(ToolResult::ok(format!(
            "Skill proposal created (id: {id}). Awaiting user approval."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, SkillProposalTool) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let tool = SkillProposalTool::new(pool, Arc::new(AtomicBool::new(true)));
        (dir, tool)
    }

    // 15.3.18 — proposal tool creates pending proposal
    #[tokio::test]
    async fn proposal_tool_creates_pending_proposal() {
        let (_dir, tool) = setup().await;

        let result = tool
            .execute(json!({
                "action": "create",
                "skill_name": "greeting",
                "content": "# Greeting Skill\nSay hello warmly.",
                "rationale": "User frequently asks for greetings"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("proposal created"));
        assert!(result.output.contains("Awaiting user approval"));
    }

    // 15.3.19 — proposal tool rejects when disabled
    #[tokio::test]
    async fn proposal_tool_rejects_when_disabled() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let tool = SkillProposalTool::new(pool, Arc::new(AtomicBool::new(false)));

        let result = tool
            .execute(json!({
                "action": "create",
                "skill_name": "test",
                "content": "content",
                "rationale": "reason"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("disabled"));
    }

    // 15.3.20 — proposal tool validates action
    #[tokio::test]
    async fn proposal_tool_validates_action() {
        let (_dir, tool) = setup().await;

        let result = tool
            .execute(json!({
                "action": "invalid_action",
                "skill_name": "test",
                "rationale": "reason"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Invalid action"));
    }

    // 15.3.21 — proposal tool requires content for create
    #[tokio::test]
    async fn proposal_tool_requires_content_for_create() {
        let (_dir, tool) = setup().await;

        let result = tool
            .execute(json!({
                "action": "create",
                "skill_name": "test",
                "rationale": "reason"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("requires 'content'"));
    }

    // 15.3.22 — proposal tool definition matches schema
    #[test]
    fn proposal_tool_definition_matches_schema() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let tool = SkillProposalTool::new(pool, Arc::new(AtomicBool::new(true)));

        assert_eq!(tool.name(), "propose_skill_change");
        assert!(tool.description().contains("skill"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
        assert!(required.contains(&json!("skill_name")));
        assert!(required.contains(&json!("rationale")));
    }
}
