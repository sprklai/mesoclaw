use async_trait::async_trait;

use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

pub struct FileSearchTool {
    max_results: usize,
}

impl FileSearchTool {
    pub fn new(max_results: usize) -> Self {
        Self { max_results }
    }
}

#[async_trait]
impl Tool for FileSearchTool {
    fn name(&self) -> &str {
        "file_search"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::High
    }

    fn description(&self) -> &str {
        "Search for files matching a pattern, respecting .gitignore"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "File name pattern to search" },
                "directory": { "type": "string", "description": "Root directory to search from", "default": "." }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'pattern' argument".into()))?;

        let directory = args
            .get("directory")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let pattern = pattern.to_string();
        let directory = directory.to_string();
        let max_results = self.max_results;

        let results = tokio::task::spawn_blocking(move || {
            let mut matches = Vec::new();
            let walker = ignore::WalkBuilder::new(&directory).build();

            for entry in walker {
                if matches.len() >= max_results {
                    break;
                }
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str())
                        && name.contains(&pattern)
                    {
                        matches.push(path.display().to_string());
                    }
                }
            }
            matches
        })
        .await
        .map_err(|e| MesoError::Tool(format!("search failed: {e}")))?;

        Ok(ToolResult::ok(results.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn search_finds_matching_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("hello.rs"), "").unwrap();
        std::fs::write(dir.path().join("world.txt"), "").unwrap();
        let tool = FileSearchTool::new(100);
        let result = tool
            .execute(serde_json::json!({
                "pattern": "hello",
                "directory": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello.rs"));
        assert!(!result.output.contains("world.txt"));
    }

    #[tokio::test]
    async fn respects_gitignore() {
        let dir = TempDir::new().unwrap();
        // ignore crate requires a .git dir to honor .gitignore
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(dir.path().join("ignored.txt"), "").unwrap();
        std::fs::write(dir.path().join("visible.txt"), "").unwrap();
        let tool = FileSearchTool::new(100);
        let result = tool
            .execute(serde_json::json!({
                "pattern": ".txt",
                "directory": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(result.output.contains("visible.txt"));
        assert!(!result.output.contains("ignored.txt"));
    }

    #[test]
    fn schema_is_valid() {
        let tool = FileSearchTool::new(100);
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["pattern"].is_object());
    }

    #[tokio::test]
    async fn missing_pattern_errors() {
        let tool = FileSearchTool::new(100);
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
