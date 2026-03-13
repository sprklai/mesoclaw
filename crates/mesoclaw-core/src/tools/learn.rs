use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use serde_json::json;

use crate::user::UserLearner;
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

const VALID_CATEGORIES: &[&str] = &["preference", "style", "workflow", "project", "personal"];

/// Agent tool for silently recording user observations during conversations.
pub struct LearnTool {
    learner: Arc<UserLearner>,
    enabled: Arc<AtomicBool>,
}

impl LearnTool {
    pub fn new(learner: Arc<UserLearner>, enabled: Arc<AtomicBool>) -> Self {
        Self { learner, enabled }
    }
}

#[async_trait]
impl Tool for LearnTool {
    fn name(&self) -> &str {
        "learn"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Silently record an observation about the user. Do not announce to the user. Use when you notice preferences, habits, or patterns."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "enum": VALID_CATEGORIES,
                    "description": "Category of observation"
                },
                "key": {
                    "type": "string",
                    "description": "Unique key for this observation"
                },
                "value": {
                    "type": "string",
                    "description": "The observed value or preference"
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "description": "Confidence level (0.0 to 1.0)"
                }
            },
            "required": ["category", "key", "value", "confidence"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(ToolResult::err(
                "Self-evolution is disabled. Enable `self_evolution_enabled` in config.",
            ));
        }

        let category = args["category"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'category' field".into()))?;
        let key = args["key"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'key' field".into()))?;
        let value = args["value"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'value' field".into()))?;
        let confidence = args["confidence"]
            .as_f64()
            .ok_or_else(|| MesoError::Validation("missing 'confidence' field".into()))?;

        // Validate category
        if !VALID_CATEGORIES.contains(&category) {
            return Ok(ToolResult::err(format!(
                "Invalid category '{}'. Valid: {}",
                category,
                VALID_CATEGORIES.join(", ")
            )));
        }

        // Validate confidence range
        if !(0.0..=1.0).contains(&confidence) {
            return Ok(ToolResult::err("Confidence must be between 0.0 and 1.0"));
        }

        match self
            .learner
            .observe(category, key, value, confidence as f32)
            .await
        {
            Ok(()) => Ok(ToolResult::ok("Observation recorded.")),
            Err(e) => Ok(ToolResult::err(format!("Failed to record: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::db;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, LearnTool) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = AppConfig::default();
        let learner = Arc::new(UserLearner::new(pool, &config));
        let tool = LearnTool::new(learner, Arc::new(AtomicBool::new(true)));
        (dir, tool)
    }

    // 15.3.13 — learn tool records observation
    #[tokio::test]
    async fn learn_tool_records_observation() {
        let (_dir, tool) = setup().await;

        let result = tool
            .execute(json!({
                "category": "preference",
                "key": "editor",
                "value": "vim",
                "confidence": 0.9
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("recorded"));
    }

    // 15.3.14 — learn tool rejects when disabled
    #[tokio::test]
    async fn learn_tool_rejects_when_disabled() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = AppConfig::default();
        let learner = Arc::new(UserLearner::new(pool, &config));
        let tool = LearnTool::new(learner, Arc::new(AtomicBool::new(false)));

        let result = tool
            .execute(json!({
                "category": "preference",
                "key": "editor",
                "value": "vim",
                "confidence": 0.9
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("disabled"));
    }

    // 15.3.15 — learn tool validates category
    #[tokio::test]
    async fn learn_tool_validates_category() {
        let (_dir, tool) = setup().await;

        let result = tool
            .execute(json!({
                "category": "invalid_category",
                "key": "test",
                "value": "test",
                "confidence": 0.5
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Invalid category"));
    }

    // 15.3.16 — learn tool validates confidence range
    #[tokio::test]
    async fn learn_tool_validates_confidence_range() {
        let (_dir, tool) = setup().await;

        let result = tool
            .execute(json!({
                "category": "preference",
                "key": "test",
                "value": "test",
                "confidence": 1.5
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("between 0.0 and 1.0"));
    }

    // 15.3.17 — learn tool definition matches schema
    #[test]
    fn learn_tool_definition_matches_schema() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let config = AppConfig::default();
        let learner = Arc::new(UserLearner::new(pool, &config));
        let tool = LearnTool::new(learner, Arc::new(AtomicBool::new(true)));

        assert_eq!(tool.name(), "learn");
        assert!(tool.description().contains("observation"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("category")));
        assert!(required.contains(&json!("key")));
        assert!(required.contains(&json!("value")));
        assert!(required.contains(&json!("confidence")));
    }
}
