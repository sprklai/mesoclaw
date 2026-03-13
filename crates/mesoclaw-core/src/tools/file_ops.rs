use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use crate::security::policy::{SecurityPolicy, ValidationResult};
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

// ---------------------------------------------------------------------------
// Smart path resolution (inspired by Tauri BaseDirectory)
// ---------------------------------------------------------------------------

fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
        .or_else(|| {
            directories::UserDirs::new().map(|u| u.home_dir().to_string_lossy().into_owned())
        })
}

/// Resolve a path string to an absolute path.
/// Handles: tilde (~), named directories (Desktop, Downloads, Documents, Home),
/// and absolute paths (passthrough).
fn resolve_path(raw: &str) -> String {
    let trimmed = raw.trim();

    // 1. Already absolute — pass through
    if trimmed.starts_with('/') || (trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':') {
        return trimmed.to_string();
    }

    // 2. Tilde expansion
    if (trimmed == "~" || trimmed.starts_with("~/"))
        && let Some(home) = home_dir()
    {
        return if trimmed == "~" {
            home
        } else {
            format!("{}{}", home, &trimmed[1..])
        };
    }

    // 3. Named directory resolution (BaseDirectory-inspired)
    let (first_segment, rest) = match trimmed.find('/') {
        Some(i) => (&trimmed[..i], Some(&trimmed[i..])),
        None => (trimmed, None),
    };

    let resolved_base = match first_segment.to_lowercase().as_str() {
        "desktop" => directories::UserDirs::new()
            .and_then(|u| u.desktop_dir().map(|p| p.to_string_lossy().into_owned())),
        "downloads" | "download" => directories::UserDirs::new()
            .and_then(|u| u.download_dir().map(|p| p.to_string_lossy().into_owned())),
        "documents" | "document" => directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(|p| p.to_string_lossy().into_owned())),
        "home" => home_dir(),
        _ => None,
    };

    if let Some(base) = resolved_base {
        return match rest {
            Some(suffix) => format!("{base}{suffix}"),
            None => base,
        };
    }

    // 4. Fallback: return as-is (relative path)
    trimmed.to_string()
}

// ---------------------------------------------------------------------------
// FileReadTool
// ---------------------------------------------------------------------------

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

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::High
    }

    fn description(&self) -> &str {
        "Read the contents of a file"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file. Accepts: absolute paths (/home/user/file.txt), tilde paths (~/file.txt), or named directories (Desktop/file.txt, Downloads/file.txt, Documents/file.txt)." },
                "max_lines": { "type": "integer", "description": "Maximum number of lines to return" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let raw_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'path' argument".into()))?;
        let path = resolve_path(raw_path);

        match self.policy.validate_path(Path::new(&path)) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Ok(ToolResult::err(format!("Path needs approval: {path}")));
            }
            ValidationResult::Denied(reason) => {
                return Ok(ToolResult::err(format!("Denied: {reason}")));
            }
        }

        let max_lines = args
            .get("max_lines")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize);

        let content = tokio::task::spawn_blocking(move || {
            std::fs::read_to_string(&path)
                .map_err(|e| MesoError::Tool(format!("failed to read file: {e}")))
        })
        .await
        .map_err(|e| MesoError::Tool(format!("spawn_blocking error: {e}")))??;

        let output = match max_lines {
            Some(limit) => content.lines().take(limit).collect::<Vec<_>>().join("\n"),
            None => content,
        };

        Ok(ToolResult::ok(output))
    }
}

// ---------------------------------------------------------------------------
// FileWriteTool
// ---------------------------------------------------------------------------

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

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::High
    }

    fn description(&self) -> &str {
        "Write content to a file"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file. Accepts: absolute paths, tilde paths (~/file.txt), or named directories (Desktop/file.txt, Downloads/file.txt, Documents/file.txt)." },
                "content": { "type": "string", "description": "Content to write to the file" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let raw_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'path' argument".into()))?;
        let path = resolve_path(raw_path);

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'content' argument".into()))?;

        match self.policy.validate_path(Path::new(&path)) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Ok(ToolResult::err(format!("Path needs approval: {path}")));
            }
            ValidationResult::Denied(reason) => {
                return Ok(ToolResult::err(format!("Denied: {reason}")));
            }
        }

        let content = content.to_string();

        tokio::task::spawn_blocking(move || {
            // Create parent directories if they don't exist
            if let Some(parent) = std::path::Path::new(&path).parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| MesoError::Tool(format!("failed to create directories: {e}")))?;
            }
            std::fs::write(&path, &content)
                .map_err(|e| MesoError::Tool(format!("failed to write file: {e}")))?;
            Ok(ToolResult::ok("File written successfully"))
        })
        .await
        .map_err(|e| MesoError::Tool(format!("spawn_blocking error: {e}")))?
    }
}

// ---------------------------------------------------------------------------
// FileListTool
// ---------------------------------------------------------------------------

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

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::High
    }

    fn description(&self) -> &str {
        "List the contents of a directory"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the directory. Accepts: absolute paths (/home/user/Desktop), tilde paths (~/Desktop), or named directories (Desktop, Downloads, Documents, Home)." },
                "recursive": { "type": "boolean", "description": "Whether to list recursively", "default": false }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let raw_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'path' argument".into()))?;
        let path = resolve_path(raw_path);

        match self.policy.validate_path(Path::new(&path)) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Ok(ToolResult::err(format!("Path needs approval: {path}")));
            }
            ValidationResult::Denied(reason) => {
                return Ok(ToolResult::err(format!("Denied: {reason}")));
            }
        }

        let recursive = args
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let entries = tokio::task::spawn_blocking(move || {
            let mut results = Vec::new();
            if recursive {
                collect_recursive(std::path::Path::new(&path), &mut results)?;
            } else {
                let dir = std::fs::read_dir(&path)
                    .map_err(|e| MesoError::Tool(format!("failed to read directory: {e}")))?;
                for entry in dir {
                    let entry = entry
                        .map_err(|e| MesoError::Tool(format!("directory entry error: {e}")))?;
                    results.push(entry.path().display().to_string());
                }
            }
            results.sort();
            Ok::<_, MesoError>(results)
        })
        .await
        .map_err(|e| MesoError::Tool(format!("spawn_blocking error: {e}")))??;

        Ok(ToolResult::ok(entries.join("\n")))
    }
}

fn collect_recursive(
    dir: &std::path::Path,
    results: &mut Vec<String>,
) -> std::result::Result<(), MesoError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| MesoError::Tool(format!("failed to read directory: {e}")))?;
    for entry in entries {
        let entry = entry.map_err(|e| MesoError::Tool(format!("directory entry error: {e}")))?;
        let path = entry.path();
        results.push(path.display().to_string());
        if path.is_dir() {
            collect_recursive(&path, results)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::policy::AutonomyLevel;
    use tempfile::TempDir;

    fn policy(level: AutonomyLevel) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(level, None, vec![], 60, 60, 100))
    }

    #[tokio::test]
    async fn read_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "hello world").unwrap();
        let tool = FileReadTool::new(policy(AutonomyLevel::Full));
        let result = tool
            .execute(serde_json::json!({"path": path.to_str().unwrap()}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello world"));
    }

    #[tokio::test]
    async fn read_respects_max_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("multi.txt");
        std::fs::write(&path, "line1\nline2\nline3\nline4\nline5").unwrap();
        let tool = FileReadTool::new(policy(AutonomyLevel::Full));
        let result = tool
            .execute(serde_json::json!({"path": path.to_str().unwrap(), "max_lines": 2}))
            .await
            .unwrap();
        assert!(result.success);
        let lines: Vec<&str> = result.output.lines().collect();
        assert!(lines.len() <= 2);
    }

    #[tokio::test]
    async fn read_missing_file_errors() {
        let tool = FileReadTool::new(policy(AutonomyLevel::Full));
        let result = tool
            .execute(serde_json::json!({"path": "/tmp/nonexistent_file_xyz.txt"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_blocked_by_policy() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "secret").unwrap();
        let blocked_dir = dir.path().to_path_buf();
        let pol = Arc::new(SecurityPolicy::new(
            AutonomyLevel::Full,
            None,
            vec![blocked_dir],
            60,
            60,
            100,
        ));
        let tool = FileReadTool::new(pol);
        let result = tool
            .execute(serde_json::json!({"path": path.to_str().unwrap()}))
            .await
            .unwrap();
        assert!(!result.success);
    }

    #[tokio::test]
    async fn write_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("output.txt");
        let pol = Arc::new(SecurityPolicy::new(
            AutonomyLevel::Full,
            Some(dir.path().to_path_buf()),
            vec![],
            60,
            60,
            100,
        ));
        let tool = FileWriteTool::new(pol);
        let result = tool
            .execute(serde_json::json!({"path": path.to_str().unwrap(), "content": "written"}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "written");
    }

    #[tokio::test]
    async fn write_missing_content_errors() {
        let tool = FileWriteTool::new(policy(AutonomyLevel::Full));
        let result = tool
            .execute(serde_json::json!({"path": "/tmp/test.txt"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_directory() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();
        let tool = FileListTool::new(policy(AutonomyLevel::Full));
        let result = tool
            .execute(serde_json::json!({"path": dir.path().to_str().unwrap()}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("a.txt"));
        assert!(result.output.contains("b.txt"));
    }

    #[tokio::test]
    async fn list_recursive() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("nested.txt"), "").unwrap();
        let tool = FileListTool::new(policy(AutonomyLevel::Full));
        let result = tool
            .execute(serde_json::json!({"path": dir.path().to_str().unwrap(), "recursive": true}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("nested.txt"));
    }

    // --- resolve_path tests ---

    #[test]
    fn resolve_path_tilde_desktop() {
        let resolved = resolve_path("~/Desktop");
        assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
        assert!(
            resolved.ends_with("/Desktop"),
            "should end with /Desktop: {resolved}"
        );
        assert!(
            !resolved.contains('~'),
            "should not contain tilde: {resolved}"
        );
    }

    #[test]
    fn resolve_path_named_desktop() {
        let resolved = resolve_path("Desktop");
        // On systems with XDG user dirs, resolves to /home/user/Desktop.
        // In CI or headless environments, UserDirs::new() returns None and
        // the function correctly falls back to returning the input as-is.
        if directories::UserDirs::new()
            .and_then(|u| u.desktop_dir().map(|_| ()))
            .is_some()
        {
            assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
            assert!(
                resolved.contains("Desktop") || resolved.contains("desktop"),
                "should resolve Desktop: {resolved}"
            );
        } else {
            assert_eq!(resolved, "Desktop", "fallback to passthrough: {resolved}");
        }
    }

    #[test]
    fn resolve_path_named_downloads_with_file() {
        let resolved = resolve_path("Downloads/file.txt");
        if directories::UserDirs::new()
            .and_then(|u| u.download_dir().map(|_| ()))
            .is_some()
        {
            assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
            assert!(
                resolved.ends_with("/file.txt"),
                "should keep suffix: {resolved}"
            );
        } else {
            assert_eq!(
                resolved, "Downloads/file.txt",
                "fallback to passthrough: {resolved}"
            );
        }
    }

    #[test]
    fn resolve_path_absolute_passthrough() {
        let resolved = resolve_path("/absolute/path");
        assert_eq!(resolved, "/absolute/path");
    }

    #[test]
    fn resolve_path_tilde_only() {
        let resolved = resolve_path("~");
        assert!(resolved.starts_with('/'), "should be absolute: {resolved}");
        assert!(
            !resolved.contains('~'),
            "should not contain tilde: {resolved}"
        );
    }

    #[test]
    fn resolve_path_unknown_relative_passthrough() {
        let resolved = resolve_path("unknown_relative");
        assert_eq!(resolved, "unknown_relative");
    }
}
