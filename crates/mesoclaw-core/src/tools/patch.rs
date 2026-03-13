use async_trait::async_trait;

use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

pub struct PatchTool;

impl PatchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PatchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PatchTool {
    fn name(&self) -> &str {
        "patch"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::High
    }

    fn description(&self) -> &str {
        "Apply a unified diff patch to a file"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string", "description": "Path to the file to patch" },
                "diff": { "type": "string", "description": "Unified diff to apply" },
                "dry_run": { "type": "boolean", "description": "If true, only check if patch applies cleanly", "default": false }
            },
            "required": ["file_path", "diff"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'file_path' argument".into()))?;

        let diff = args
            .get("diff")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'diff' argument".into()))?;

        let dry_run = args
            .get("dry_run")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let file_path = file_path.to_string();
        let diff = diff.to_string();

        tokio::task::spawn_blocking(move || {
            let original = std::fs::read_to_string(&file_path)
                .map_err(|e| MesoError::Tool(format!("failed to read file: {e}")))?;

            let patch = diffy::Patch::from_str(&diff)
                .map_err(|e| MesoError::Tool(format!("invalid diff: {e}")))?;

            let patched = diffy::apply(&original, &patch)
                .map_err(|e| MesoError::Tool(format!("patch conflict: {e}")))?;

            if dry_run {
                Ok(ToolResult::ok("Patch applies cleanly"))
            } else {
                std::fs::write(&file_path, &patched)
                    .map_err(|e| MesoError::Tool(format!("failed to write file: {e}")))?;
                Ok(ToolResult::ok("Patch applied successfully"))
            }
        })
        .await
        .map_err(|e| MesoError::Tool(format!("spawn_blocking error: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_diff(original: &str, modified: &str) -> String {
        diffy::create_patch(original, modified).to_string()
    }

    #[tokio::test]
    async fn apply_simple_patch() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello\nworld\n").unwrap();
        let diff = make_diff("hello\nworld\n", "hello\nrust\n");
        let tool = PatchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": path.to_str().unwrap(),
                "diff": diff
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\nrust\n");
    }

    #[tokio::test]
    async fn dry_run_does_not_modify() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello\nworld\n").unwrap();
        let diff = make_diff("hello\nworld\n", "hello\nrust\n");
        let tool = PatchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": path.to_str().unwrap(),
                "diff": diff,
                "dry_run": true
            }))
            .await
            .unwrap();
        assert!(result.success);
        // File should be unchanged
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\nworld\n");
    }

    #[tokio::test]
    async fn conflict_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "completely different content\n").unwrap();
        // This diff expects "hello\nworld\n" but file has different content
        let diff = make_diff("hello\nworld\n", "hello\nrust\n");
        let tool = PatchTool::new();
        let result = tool
            .execute(serde_json::json!({
                "file_path": path.to_str().unwrap(),
                "diff": diff
            }))
            .await;
        // Should fail either at the tool level or return error result
        match result {
            Ok(r) => assert!(!r.success),
            Err(_) => {} // Also acceptable
        }
    }

    #[tokio::test]
    async fn missing_file_path_errors() {
        let tool = PatchTool::new();
        let result = tool.execute(serde_json::json!({"diff": "something"})).await;
        assert!(result.is_err());
    }

    #[test]
    fn schema_is_valid() {
        let tool = PatchTool::new();
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["file_path"].is_object());
        assert!(schema["properties"]["diff"].is_object());
    }
}
