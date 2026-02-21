//! Apply unified diff patches to files.
//!
//! This tool uses the `diffy` crate to parse and apply unified diff patches.
//! It supports conflict detection and reports success/failure with details.

use std::{
    path::PathBuf,
    sync::Arc,
};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::security::{RiskLevel, SecurityPolicy, ValidationResult};

use super::traits::{Tool, ToolResult};

/// Applies unified diff patches to files.
pub struct PatchTool {
    policy: Arc<SecurityPolicy>,
}

impl PatchTool {
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl Tool for PatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a unified diff patch to a file. Validates path access via security policy \
         and reports conflicts if the patch cannot be applied cleanly."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file to patch."
                },
                "diff_content": {
                    "type": "string",
                    "description": "Unified diff content to apply to the file."
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "If true, validate the patch without applying it (default: false)."
                }
            },
            "required": ["file_path", "diff_content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let path = args
            .get("file_path")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .ok_or("missing required argument 'file_path'")?;

        let diff_content = args
            .get("diff_content")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'diff_content'")?
            .to_string();

        let dry_run = args
            .get("dry_run")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // Security gate: validate path access.
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

        // Run patch application in a blocking thread.
        let path_clone = path.clone();
        let result = tokio::task::spawn_blocking(move || {
            apply_patch_blocking(&path_clone, &diff_content, dry_run)
        })
        .await
        .map_err(|e| format!("blocking task panicked: {e}"))??;

        Ok(result)
    }
}

/// Apply a patch in a blocking context.
fn apply_patch_blocking(
    path: &PathBuf,
    diff_content: &str,
    dry_run: bool,
) -> Result<ToolResult, String> {
    use std::fs;

    // Read the original file.
    let original = fs::read_to_string(path)
        .map_err(|e| format!("failed to read '{}': {e}", path.display()))?;

    // Parse the unified diff.
    let patch = diffy::Patch::from_str(diff_content)
        .map_err(|e| format!("failed to parse diff: {e}"))?;

    // Try to apply the patch.
    let result = diffy::apply(&original, &patch);

    match result {
        Ok(patched) => {
            if dry_run {
                return Ok(ToolResult::ok(format!(
                    "Patch would apply successfully to '{}' (dry run)",
                    path.display()
                ))
                .with_metadata(json!({
                    "path": path.display().to_string(),
                    "dry_run": true,
                    "success": true
                })));
            }

            // Write the patched content back.
            fs::write(path, &patched)
                .map_err(|e| format!("failed to write '{}': {e}", path.display()))?;

            Ok(ToolResult::ok(format!(
                "Patch applied successfully to '{}'",
                path.display()
            ))
            .with_metadata(json!({
                "path": path.display().to_string(),
                "success": true,
                "conflicts": false
            })))
        }
        Err(e) => {
            // Patch failed to apply (conflict or other error).
            let error_msg = format!("Patch conflict in '{}': {}", path.display(), e);
            Ok(ToolResult::err(error_msg.clone()).with_metadata(json!({
                "path": path.display().to_string(),
                "success": false,
                "conflicts": true,
                "error": error_msg
            })))
        }
    }
}

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

    #[tokio::test]
    async fn apply_simple_patch() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello world\n").unwrap();

        let diff = r#"--- test.txt
+++ test.txt
@@ -1 +1 @@
-hello world
+hello universe
"#;

        let tool = PatchTool::new(full_policy());
        let r = tool
            .execute(json!({
                "file_path": file.display().to_string(),
                "diff_content": diff
            }))
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello universe\n");
    }

    #[tokio::test]
    async fn dry_run_does_not_modify() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "original\n").unwrap();

        let diff = r#"--- test.txt
+++ test.txt
@@ -1 +1 @@
-original
+modified
"#;

        let tool = PatchTool::new(full_policy());
        let r = tool
            .execute(json!({
                "file_path": file.display().to_string(),
                "diff_content": diff,
                "dry_run": true
            }))
            .await
            .unwrap();
        assert!(r.success);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "original\n");
    }

    #[tokio::test]
    async fn conflict_returns_error() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        // Content doesn't match what the patch expects.
        std::fs::write(&file, "different content\n").unwrap();

        let diff = r#"--- test.txt
+++ test.txt
@@ -1 +1 @@
-hello world
+hello universe
"#;

        let tool = PatchTool::new(full_policy());
        let r = tool
            .execute(json!({
                "file_path": file.display().to_string(),
                "diff_content": diff
            }))
            .await
            .unwrap();
        assert!(!r.success);
        assert!(r.output.contains("conflict"));
    }

    #[tokio::test]
    async fn missing_file_path_errors() {
        let tool = PatchTool::new(full_policy());
        let r = tool.execute(json!({"diff_content": "x"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn invalid_diff_returns_error_result() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "content\n").unwrap();

        let tool = PatchTool::new(full_policy());
        // Use a properly formatted but mismatched diff that will fail to apply.
        // This tests the conflict handling path.
        let diff = r#"--- test.txt
+++ test.txt
@@ -1 +1 @@
-wrong content
+new content
"#;
        let r = tool
            .execute(json!({
                "file_path": file.display().to_string(),
                "diff_content": diff
            }))
            .await
            .unwrap();
        // Should return a conflict error (success=false).
        assert!(!r.success);
        assert!(r.output.contains("conflict"));
    }
}
