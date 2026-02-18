use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::security::{RiskLevel, SecurityPolicy, ValidationResult};

use super::traits::{Tool, ToolResult};

// ─── FileReadTool ────────────────────────────────────────────────────────────

pub struct FileReadTool {
    policy: Arc<SecurityPolicy>,
}

impl FileReadTool {
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file.  Use max_lines to limit output for large files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute path to the file." },
                "max_lines": {
                    "type": "integer",
                    "description": "Maximum number of lines to return (default: all).",
                    "minimum": 1
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let path = required_path(&args, "path")?;

        match self.policy.validate_path(&path) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Err("path access requires user approval".into());
            }
            ValidationResult::Denied(reason) => {
                return Err(format!("path denied: {reason}"));
            }
        }

        self.policy.log_action(
            self.name(),
            args.clone(),
            RiskLevel::Low,
            "allowed",
            None,
        );

        let max_lines = args.get("max_lines").and_then(Value::as_u64).map(|n| n as usize);

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read '{}': {e}", path.display()))?;

        let output = match max_lines {
            Some(n) => contents.lines().take(n).collect::<Vec<_>>().join("\n"),
            None => contents,
        };

        Ok(ToolResult::ok(output)
            .with_metadata(json!({ "path": path.display().to_string() })))
    }
}

// ─── FileWriteTool ────────────────────────────────────────────────────────────

pub struct FileWriteTool {
    policy: Arc<SecurityPolicy>,
}

impl FileWriteTool {
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file, creating it if it does not exist and overwriting it if it does."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path":    { "type": "string", "description": "Absolute path to write." },
                "content": { "type": "string", "description": "Content to write." }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let path = required_path(&args, "path")?;
        let content = args
            .get("content")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'content'")?
            .to_string();

        match self.policy.validate_path(&path) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Err("path access requires user approval".into());
            }
            ValidationResult::Denied(reason) => {
                return Err(format!("path denied: {reason}"));
            }
        }

        self.policy.log_action(
            self.name(),
            args.clone(),
            RiskLevel::Medium,
            "allowed",
            None,
        );

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create parent dirs: {e}"))?;
        }

        let bytes = content.len();
        fs::write(&path, &content)
            .map_err(|e| format!("failed to write '{}': {e}", path.display()))?;

        Ok(ToolResult::ok(format!(
            "wrote {} bytes to '{}'",
            bytes,
            path.display()
        ))
        .with_metadata(json!({ "path": path.display().to_string(), "bytes": bytes })))
    }
}

// ─── FileListTool ─────────────────────────────────────────────────────────────

pub struct FileListTool {
    policy: Arc<SecurityPolicy>,
}

impl FileListTool {
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl Tool for FileListTool {
    fn name(&self) -> &str {
        "file_list"
    }

    fn description(&self) -> &str {
        "List the contents of a directory, optionally recursively."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute path of the directory." },
                "recursive": {
                    "type": "boolean",
                    "description": "If true, list all files recursively (default: false)."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let path = required_path(&args, "path")?;
        let recursive = args
            .get("recursive")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        match self.policy.validate_path(&path) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Err("path access requires user approval".into());
            }
            ValidationResult::Denied(reason) => {
                return Err(format!("path denied: {reason}"));
            }
        }

        self.policy.log_action(
            self.name(),
            args.clone(),
            RiskLevel::Low,
            "allowed",
            None,
        );

        let entries = collect_entries(&path, recursive)
            .map_err(|e| format!("failed to list '{}': {e}", path.display()))?;

        Ok(ToolResult::ok(entries.join("\n"))
            .with_metadata(json!({ "count": entries.len() })))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn required_path(args: &Value, key: &str) -> Result<PathBuf, String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument '{key}'"))
}

fn collect_entries(dir: &Path, recursive: bool) -> std::io::Result<Vec<String>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let display = path.display().to_string();
        result.push(display.clone());
        if recursive && path.is_dir() {
            let sub = collect_entries(&path, true)?;
            result.extend(sub);
        }
    }
    result.sort();
    Ok(result)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::AutonomyLevel;
    use tempfile::TempDir;

    fn full_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(
            AutonomyLevel::Full,
            None,
            vec![],
            3600,
            100,
        ))
    }

    // ── FileReadTool ────────────────────────────────────────────────────

    #[tokio::test]
    async fn read_existing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("hello.txt");
        fs::write(&file, "hello world").unwrap();

        let tool = FileReadTool::new(full_policy());
        let r = tool.execute(json!({"path": file.display().to_string()})).await.unwrap();
        assert!(r.success);
        assert_eq!(r.output.trim(), "hello world");
    }

    #[tokio::test]
    async fn read_respects_max_lines() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("lines.txt");
        fs::write(&file, "a\nb\nc\nd\ne").unwrap();

        let tool = FileReadTool::new(full_policy());
        let r = tool
            .execute(json!({"path": file.display().to_string(), "max_lines": 3}))
            .await
            .unwrap();
        assert_eq!(r.output.lines().count(), 3);
    }

    #[tokio::test]
    async fn read_missing_file_errors() {
        let tool = FileReadTool::new(full_policy());
        let r = tool.execute(json!({"path": "/tmp/__nonexistent_file_xyz__"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn read_missing_path_arg_errors() {
        let tool = FileReadTool::new(full_policy());
        let r = tool.execute(json!({})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn read_blocked_by_policy() {
        let dir = TempDir::new().unwrap();
        let blocked = dir.path().to_path_buf();
        let policy = Arc::new(SecurityPolicy::new(
            AutonomyLevel::Full,
            None,
            vec![blocked.clone()],
            60,
            100,
        ));
        let file = blocked.join("secret.txt");
        fs::write(&file, "secret").unwrap();

        let tool = FileReadTool::new(policy);
        let r = tool.execute(json!({"path": file.display().to_string()})).await;
        assert!(r.is_err());
    }

    // ── FileWriteTool ───────────────────────────────────────────────────

    #[tokio::test]
    async fn write_creates_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("output.txt");

        let tool = FileWriteTool::new(full_policy());
        let r = tool
            .execute(json!({"path": file.display().to_string(), "content": "written"}))
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(fs::read_to_string(&file).unwrap(), "written");
    }

    #[tokio::test]
    async fn write_missing_content_errors() {
        let tool = FileWriteTool::new(full_policy());
        let r = tool.execute(json!({"path": "/tmp/x"})).await;
        assert!(r.is_err());
    }

    // ── FileListTool ────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_directory() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "").unwrap();
        fs::write(dir.path().join("b.txt"), "").unwrap();

        let tool = FileListTool::new(full_policy());
        let r = tool
            .execute(json!({"path": dir.path().display().to_string()}))
            .await
            .unwrap();
        assert!(r.success);
        assert!(r.output.contains("a.txt"));
        assert!(r.output.contains("b.txt"));
    }

    #[tokio::test]
    async fn list_recursive() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("nested.txt"), "").unwrap();

        let tool = FileListTool::new(full_policy());
        let r = tool
            .execute(json!({"path": dir.path().display().to_string(), "recursive": true}))
            .await
            .unwrap();
        assert!(r.output.contains("nested.txt"));
    }

    #[tokio::test]
    async fn list_schema_correct() {
        let tool = FileListTool::new(full_policy());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["properties"]["recursive"].is_object());
    }
}
