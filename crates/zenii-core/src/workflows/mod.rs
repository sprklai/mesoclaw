pub mod definition;
pub mod executor;
pub mod generator;
pub mod runtime;
pub mod templates;

pub use definition::{
    FailurePolicy, NodePosition, RetryConfig, StepOutput, StepType, Workflow, WorkflowRun,
    WorkflowRunStatus, WorkflowStep,
};

use std::path::PathBuf;

use dashmap::DashMap;
use tracing::warn;

use crate::{Result, ZeniiError};

pub struct WorkflowRegistry {
    workflows: DashMap<String, Workflow>,
    directory: PathBuf,
}

impl WorkflowRegistry {
    pub fn new(directory: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&directory)?;
        let registry = Self {
            workflows: DashMap::new(),
            directory,
        };
        registry.load_all()?;
        Ok(registry)
    }

    fn load_all(&self) -> Result<()> {
        if !self.directory.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(&self.directory)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "toml") {
                let content = std::fs::read_to_string(&path)?;
                match toml::from_str::<Workflow>(&content) {
                    Ok(wf) => {
                        // Check that the workflow id matches the file stem to avoid ghost files.
                        let stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");
                        if wf.id != stem {
                            warn!(
                                "Skipping workflow {:?}: id '{}' does not match filename stem '{stem}'",
                                path, wf.id
                            );
                            continue;
                        }
                        self.workflows.insert(wf.id.clone(), wf);
                    }
                    Err(e) => {
                        warn!("Failed to parse workflow {:?}: {e}", path);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<Workflow> {
        self.workflows.get(id).map(|r| r.value().clone())
    }

    pub fn list(&self) -> Vec<Workflow> {
        self.workflows.iter().map(|r| r.value().clone()).collect()
    }

    pub fn save(&self, workflow: Workflow) -> Result<()> {
        // Validate before writing to disk
        workflow.validate()?;

        // Sanity check: the file we're about to write must match the workflow id.
        // This is always true when id is valid (validate() ensures that), but we
        // assert explicitly to guard against future refactors.
        let intended_stem = workflow.id.clone();
        let path = self.directory.join(format!("{}.toml", workflow.id));
        assert_eq!(
            path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
            intended_stem,
            "save() invariant violated: file stem does not match workflow id"
        );

        let content = toml::to_string_pretty(&workflow)
            .map_err(|e| ZeniiError::Workflow(format!("serialize error: {e}")))?;
        std::fs::write(&path, content)?;
        self.workflows.insert(workflow.id.clone(), workflow);
        Ok(())
    }

    pub fn get_raw_toml(&self, id: &str) -> Option<String> {
        let path = self.directory.join(format!("{id}.toml"));
        std::fs::read_to_string(path).ok()
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        let path = self.directory.join(format!("{id}.toml"));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(self.workflows.remove(id).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_workflow(id: &str, name: &str) -> Workflow {
        Workflow {
            id: id.into(),
            name: name.into(),
            description: format!("{name} description"),
            schedule: None,
            steps: vec![WorkflowStep {
                name: "step1".into(),
                step_type: StepType::Delay { seconds: 1 },
                depends_on: vec![],
                retry: None,
                failure_policy: FailurePolicy::Stop,
                timeout_secs: None,
            }],
            layout: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    // 5.19
    #[test]
    fn registry_save_and_get() {
        let dir = tempfile::TempDir::new().unwrap();
        let registry = WorkflowRegistry::new(dir.path().to_path_buf()).unwrap();

        let wf = test_workflow("wf1", "Test Workflow");
        registry.save(wf).unwrap();

        let found = registry.get("wf1");
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, "wf1");
        assert_eq!(found.name, "Test Workflow");
        assert_eq!(found.steps.len(), 1);
    }

    // 5.20
    #[test]
    fn registry_list() {
        let dir = tempfile::TempDir::new().unwrap();
        let registry = WorkflowRegistry::new(dir.path().to_path_buf()).unwrap();

        registry.save(test_workflow("wf1", "First")).unwrap();
        registry.save(test_workflow("wf2", "Second")).unwrap();

        let list = registry.list();
        assert_eq!(list.len(), 2);
        let ids: Vec<&str> = list.iter().map(|w| w.id.as_str()).collect();
        assert!(ids.contains(&"wf1"));
        assert!(ids.contains(&"wf2"));
    }

    // 5.21
    #[test]
    fn registry_delete() {
        let dir = tempfile::TempDir::new().unwrap();
        let registry = WorkflowRegistry::new(dir.path().to_path_buf()).unwrap();

        registry.save(test_workflow("wf1", "ToDelete")).unwrap();
        assert!(registry.get("wf1").is_some());

        let deleted = registry.delete("wf1").unwrap();
        assert!(deleted);
        assert!(registry.get("wf1").is_none());

        // File should be gone
        let path = dir.path().join("wf1.toml");
        assert!(!path.exists());
    }

    // 5.22
    #[test]
    fn registry_load_from_disk() {
        let dir = tempfile::TempDir::new().unwrap();

        // Write a TOML file directly to disk
        let toml_content = r#"
            id = "disk-wf"
            name = "From Disk"
            description = "Loaded from disk"

            [[steps]]
            name = "s1"
            type = "delay"
            seconds = 5
        "#;
        std::fs::write(dir.path().join("disk-wf.toml"), toml_content).unwrap();

        // Create a new registry — it should load the file
        let registry = WorkflowRegistry::new(dir.path().to_path_buf()).unwrap();
        let found = registry.get("disk-wf");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "From Disk");
    }

    // 5.23
    #[test]
    fn registry_save_writes_toml_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let registry = WorkflowRegistry::new(dir.path().to_path_buf()).unwrap();

        registry
            .save(test_workflow("file-check", "File Check"))
            .unwrap();

        let path = dir.path().join("file-check.toml");
        assert!(path.exists());

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("file-check"));
        assert!(content.contains("File Check"));
    }

    // 5.24
    #[test]
    fn registry_get_nonexistent() {
        let dir = tempfile::TempDir::new().unwrap();
        let registry = WorkflowRegistry::new(dir.path().to_path_buf()).unwrap();
        assert!(registry.get("missing").is_none());
    }

    // registry_load_skips_mismatched_id — file named foo.toml but contains id = "bar" is skipped
    #[test]
    fn registry_load_skips_mismatched_id() {
        let dir = tempfile::TempDir::new().unwrap();

        // Write foo.toml with id = "bar" (mismatch)
        let toml_content = r#"
            id = "bar"
            name = "Mismatch"
            description = "ID does not match filename"

            [[steps]]
            name = "s1"
            type = "delay"
            seconds = 1
        "#;
        std::fs::write(dir.path().join("foo.toml"), toml_content).unwrap();

        let registry = WorkflowRegistry::new(dir.path().to_path_buf()).unwrap();

        // Neither "foo" nor "bar" should be loaded
        assert!(registry.get("foo").is_none(), "foo should not be loaded");
        assert!(registry.get("bar").is_none(), "bar should not be loaded (filename mismatch)");
        assert_eq!(registry.list().len(), 0);
    }

    // registry_save_rejects_invalid_id — save() with invalid id returns Validation error
    #[test]
    fn registry_save_rejects_invalid_id() {
        let dir = tempfile::TempDir::new().unwrap();
        let registry = WorkflowRegistry::new(dir.path().to_path_buf()).unwrap();

        let mut wf = test_workflow("valid-id", "Valid");
        wf.id = "../traversal".into();
        let err = registry.save(wf).unwrap_err();
        assert!(err.to_string().contains("invalid"), "{err}");
    }
}
