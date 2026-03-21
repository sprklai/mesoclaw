use async_trait::async_trait;

use crate::{Result, ZeniiError};

use super::path::resolve_path;
use super::traits::{Tool, ToolResult};

pub struct ContentSearchTool {
    max_results: usize,
    max_file_size_bytes: usize,
    default_context_lines: usize,
    max_depth: usize,
}

impl ContentSearchTool {
    pub fn new(
        max_results: usize,
        max_file_size_bytes: usize,
        default_context_lines: usize,
        max_depth: usize,
    ) -> Self {
        Self {
            max_results,
            max_file_size_bytes,
            default_context_lines,
            max_depth,
        }
    }
}

#[async_trait]
impl Tool for ContentSearchTool {
    fn name(&self) -> &str {
        "content_search"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::High
    }

    fn description(&self) -> &str {
        "Search for text patterns inside files using regex, respecting .gitignore"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Text pattern to search for inside files (regex supported)"
                },
                "directory": {
                    "type": "string",
                    "description": "Root directory to search (supports: ~/path, Desktop, $HOME/projects, absolute paths)",
                    "default": "."
                },
                "file_pattern": {
                    "type": "string",
                    "description": "Glob to filter which files to search (e.g., '*.rs', '*.txt')"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case-insensitive matching",
                    "default": true
                },
                "context_lines": {
                    "type": "integer",
                    "description": "Lines of context around each match"
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

        let file_pattern = args
            .get("file_pattern")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let case_insensitive = args
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let context_lines = args
            .get("context_lines")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(self.default_context_lines);

        let regex = regex::RegexBuilder::new(pattern)
            .case_insensitive(case_insensitive)
            .build()
            .map_err(|e| ZeniiError::Tool(format!("invalid regex pattern: {e}")))?;

        let file_glob = if let Some(ref fp) = file_pattern {
            Some(
                globset::Glob::new(fp)
                    .map_err(|e| ZeniiError::Tool(format!("invalid file pattern: {e}")))?
                    .compile_matcher(),
            )
        } else {
            None
        };

        let max_results = self.max_results;
        let max_file_size = self.max_file_size_bytes;
        let max_depth = self.max_depth;

        let results = tokio::task::spawn_blocking(move || {
            let mut matches = Vec::new();
            let walker = ignore::WalkBuilder::new(&directory)
                .max_depth(Some(max_depth))
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
                if !path.is_file() {
                    continue;
                }

                // Apply file pattern filter
                if let Some(ref glob) = file_glob {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if !glob.is_match(name) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                // Skip files larger than max size
                if let Ok(meta) = path.metadata()
                    && meta.len() > max_file_size as u64
                {
                    continue;
                }

                // Read and search file
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue, // skip binary/unreadable files
                };

                let lines: Vec<&str> = content.lines().collect();
                for (line_idx, line) in lines.iter().enumerate() {
                    if matches.len() >= max_results {
                        break;
                    }
                    if regex.is_match(line) {
                        let mut match_block = String::new();
                        let start = line_idx.saturating_sub(context_lines);
                        let end = (line_idx + context_lines + 1).min(lines.len());

                        for (ctx_idx, line_content) in
                            lines[start..end].iter().enumerate()
                        {
                            let abs_idx = start + ctx_idx;
                            let prefix = if abs_idx == line_idx { ">" } else { " " };
                            match_block.push_str(&format!(
                                "{}{}:{}: {}\n",
                                prefix,
                                path.display(),
                                abs_idx + 1,
                                line_content
                            ));
                        }
                        matches.push(match_block);
                    }
                }
            }
            matches
        })
        .await
        .map_err(|e| ZeniiError::Tool(format!("search failed: {e}")))?;

        if results.is_empty() {
            Ok(ToolResult::ok("No matches found"))
        } else {
            Ok(ToolResult::ok(format!(
                "Found {} matches:\n\n{}",
                results.len(),
                results.join("\n")
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tool() -> ContentSearchTool {
        ContentSearchTool::new(50, 1024 * 1024, 2, 20)
    }

    // CS.1 — Basic text search finds content
    #[tokio::test]
    async fn content_search_finds_text() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hello world\ngoodbye world").unwrap();
        std::fs::write(dir.path().join("other.txt"), "no match here").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "hello",
                "directory": dir.path().to_str().unwrap(),
                "context_lines": 0
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello world"));
        assert!(!result.output.contains("no match here"));
    }

    // CS.2 — Regex pattern works
    #[tokio::test]
    async fn content_search_regex_pattern() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("data.txt"), "foo123bar\nbaz456qux").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "\\d{3}",
                "directory": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("foo123bar"));
    }

    // CS.3 — Case-insensitive matching
    #[tokio::test]
    async fn content_search_case_insensitive() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "Hello World").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "hello",
                "directory": dir.path().to_str().unwrap(),
                "case_insensitive": true
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("Hello World"));
    }

    // CS.4 — Context lines included
    #[tokio::test]
    async fn content_search_context_lines() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("ctx.txt"),
            "line1\nline2\ntarget\nline4\nline5",
        )
        .unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "target",
                "directory": dir.path().to_str().unwrap(),
                "context_lines": 1
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("line2"));
        assert!(result.output.contains("target"));
        assert!(result.output.contains("line4"));
    }

    // CS.5 — Respects .gitignore
    #[tokio::test]
    async fn content_search_respects_gitignore() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(dir.path().join("ignored.txt"), "secret data").unwrap();
        std::fs::write(dir.path().join("visible.txt"), "public data").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "data",
                "directory": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("visible.txt"));
        assert!(!result.output.contains("ignored.txt"));
    }

    // CS.6 — File pattern filter
    #[tokio::test]
    async fn content_search_file_pattern_filter() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("code.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "fn main description").unwrap();
        let result = tool()
            .execute(serde_json::json!({
                "pattern": "fn main",
                "directory": dir.path().to_str().unwrap(),
                "file_pattern": "*.rs"
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("code.rs"));
        assert!(!result.output.contains("readme.txt"));
    }

    // CS.7 — Skips files larger than max size
    #[tokio::test]
    async fn content_search_skips_large_files() {
        let dir = TempDir::new().unwrap();
        // Create a file larger than 100 bytes (our test limit)
        let big_content = "a".repeat(200);
        std::fs::write(dir.path().join("big.txt"), &big_content).unwrap();
        std::fs::write(dir.path().join("small.txt"), "findme").unwrap();
        let small_tool = ContentSearchTool::new(50, 100, 2, 20);
        let result = small_tool
            .execute(serde_json::json!({
                "pattern": "a+|findme",
                "directory": dir.path().to_str().unwrap()
            }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("small.txt"));
        assert!(!result.output.contains("big.txt"));
    }

    // CS.8 — Missing pattern errors
    #[tokio::test]
    async fn content_search_missing_pattern_errors() {
        let result = tool().execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
