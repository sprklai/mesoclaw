use async_trait::async_trait;

use crate::{Result, ZeniiError};

use super::path::resolve_path;
use super::traits::{Tool, ToolResult};

pub struct FileSearchTool {
    max_results: usize,
    max_depth: usize,
    follow_symlinks: bool,
}

impl FileSearchTool {
    pub fn new(max_results: usize, max_depth: usize, follow_symlinks: bool) -> Self {
        Self {
            max_results,
            max_depth,
            follow_symlinks,
        }
    }
}

/// Check if a pattern contains glob metacharacters.
fn is_glob_pattern(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[') || pattern.contains('{')
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
        "Search for files by name pattern (glob or substring), respecting .gitignore. Supports named directories (Desktop, Downloads, Documents, Pictures, Videos, Music) and tilde paths."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "File name pattern (supports glob: *.rs, **/*.txt, report*) or substring match"
                },
                "directory": {
                    "type": "string",
                    "description": "Root directory (supports: ~/path, Desktop, Downloads, Documents, Pictures, Videos, Music, $HOME/projects, absolute paths)",
                    "default": "."
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case-insensitive matching",
                    "default": false
                },
                "file_type": {
                    "type": "string",
                    "description": "Filter by file extension (e.g., 'rs', 'txt', 'py')"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum directory depth to search"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ZeniiError::Tool("missing 'pattern' argument".into()))?;

        let directory = args
            .get("directory")
            .and_then(|v| v.as_str())
            .unwrap_or(".");
        let directory = resolve_path(directory);

        let case_insensitive = args
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let file_type = args
            .get("file_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let max_depth = args
            .get("max_depth")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(self.max_depth);

        // Build glob matcher if pattern has glob chars
        let glob_matcher = if is_glob_pattern(pattern) {
            Some(
                globset::GlobBuilder::new(pattern)
                    .case_insensitive(case_insensitive)
                    .build()
                    .map_err(|e| ZeniiError::Tool(format!("invalid glob pattern: {e}")))?
                    .compile_matcher(),
            )
        } else {
            None
        };

        let pattern = pattern.to_string();
        let max_results = self.max_results;
        let follow_links = self.follow_symlinks;

        let results = tokio::task::spawn_blocking(move || {
            let mut matches = Vec::new();
            let walker = ignore::WalkBuilder::new(&directory)
                .max_depth(Some(max_depth))
                .follow_links(follow_links)
                .build();

            for entry in walker {
                if matches.len() >= max_results {
                    break;
                }
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => continue,
                };

                // Apply file type filter
                if let Some(ref ext) = file_type {
                    match path.extension().and_then(|e| e.to_str()) {
                        Some(file_ext) if file_ext == ext.as_str() => {}
                        _ => continue,
                    }
                }

                // Match using glob or substring
                let matched = if let Some(ref glob) = glob_matcher {
                    glob.is_match(name)
                } else if case_insensitive {
                    name.to_lowercase().contains(&pattern.to_lowercase())
                } else {
                    name.contains(&pattern)
                };

                if matched {
                    matches.push(path.display().to_string());
                }
            }
            matches
        })
        .await
        .map_err(|e| ZeniiError::Tool(format!("search failed: {e}")))?;

        Ok(ToolResult::ok(results.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tool() -> FileSearchTool {
        FileSearchTool::new(100, 20, false)
    }

    #[tokio::test]
    async fn search_finds_matching_files() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("hello.rs"), "").unwrap();
        std::fs::write(dir.path().join("world.txt"), "").unwrap();
        let result = tool()
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
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(dir.path().join("ignored.txt"), "").unwrap();
        std::fs::write(dir.path().join("visible.txt"), "").unwrap();
        let result = tool()
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
        let tool = tool();
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["pattern"].is_object());
    }

    #[tokio::test]
    async fn missing_pattern_errors() {
        let result = tool().execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    // FS.1 — Tilde directory resolves
    #[tokio::test]
    async fn search_with_tilde_directory() {
        // Just verify it doesn't error — tilde resolves to home
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "nonexistent_file_xyz",
                "directory": "~"
            }))
            .await
            .unwrap();
        assert!(result.success);
    }

    // FS.2 — Named directory resolves
    #[tokio::test]
    async fn search_with_named_directory() {
        // Desktop may or may not exist, but shouldn't error
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "nonexistent_file_xyz",
                "directory": "Desktop"
            }))
            .await;
        // May fail if Desktop dir doesn't exist, that's fine
        assert!(result.is_ok());
    }

    // FS.3 — Glob pattern matching
    #[tokio::test]
    async fn search_glob_pattern() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("main.rs"), "").unwrap();
        std::fs::write(dir.path().join("lib.rs"), "").unwrap();
        std::fs::write(dir.path().join("readme.md"), "").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "*.rs",
                "directory": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("main.rs"));
        assert!(result.output.contains("lib.rs"));
        assert!(!result.output.contains("readme.md"));
    }

    // FS.4 — Case-insensitive search
    #[tokio::test]
    async fn search_case_insensitive() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("README.md"), "").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "readme",
                "directory": dir.path().to_str().unwrap(),
                "case_insensitive": true
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("README.md"));
    }

    // FS.5 — File type filter
    #[tokio::test]
    async fn search_file_type_filter() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("code.rs"), "").unwrap();
        std::fs::write(dir.path().join("notes.txt"), "").unwrap();
        std::fs::write(dir.path().join("data.txt"), "").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "",
                "directory": dir.path().to_str().unwrap(),
                "file_type": "txt"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("notes.txt"));
        assert!(result.output.contains("data.txt"));
        assert!(!result.output.contains("code.rs"));
    }

    // FS.6 — Max depth is respected
    #[tokio::test]
    async fn search_max_depth() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("top.txt"), "").unwrap();
        std::fs::create_dir_all(dir.path().join("a/b/c")).unwrap();
        std::fs::write(dir.path().join("a/b/c/deep.txt"), "").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": ".txt",
                "directory": dir.path().to_str().unwrap(),
                "max_depth": 1
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("top.txt"));
        assert!(!result.output.contains("deep.txt"));
    }
}
