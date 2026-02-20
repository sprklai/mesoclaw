//! Agent workspace management.
//!
//! Provides workspace isolation and file operations for agents.

use std::path::PathBuf;
use std::sync::Arc;

use super::{AgentError, AgentId};

/// Bootstrap file names for agent workspace initialization.
const BOOTSTRAP_FILES: &[&str] = &[
    "SOUL.md",
    "AGENTS.md",
    "TOOLS.md",
    "IDENTITY.md",
    "MEMORY.md",
    "HEARTBEAT.md",
];

/// Agent workspace representation.
#[derive(Debug, Clone)]
pub struct AgentWorkspace {
    /// Agent ID.
    pub agent_id: AgentId,
    /// Root directory of the workspace.
    pub root: PathBuf,
}

impl AgentWorkspace {
    /// Create a new workspace representation.
    pub fn new(agent_id: AgentId, root: PathBuf) -> Self {
        Self { agent_id, root }
    }

    /// Get path to a file in the workspace.
    pub fn path(&self, filename: &str) -> PathBuf {
        self.root.join(filename)
    }

    /// Get the memory directory path.
    pub fn memory_dir(&self) -> PathBuf {
        self.root.join("memory")
    }

    /// Get the skills directory path.
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    /// Check if the workspace exists.
    pub fn exists(&self) -> bool {
        self.root.exists() && self.root.is_dir()
    }
}

/// Manager for agent workspaces.
pub struct WorkspaceManager {
    base_dir: PathBuf,
}

impl WorkspaceManager {
    /// Create a new workspace manager.
    pub fn new(base_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self { base_dir })
    }

    /// Get the workspace for an agent.
    pub fn get_workspace(&self, agent_id: &AgentId) -> AgentWorkspace {
        let workspace_path = self.base_dir.join("agents").join(&agent_id.0);
        AgentWorkspace::new(agent_id.clone(), workspace_path)
    }

    /// Ensure an agent's workspace exists with bootstrap files.
    pub async fn ensure_agent_workspace(
        &self,
        agent_id: &AgentId,
        ensure_bootstrap_files: bool,
    ) -> Result<AgentWorkspace, AgentError> {
        let workspace = self.get_workspace(agent_id);

        // Create workspace directory
        std::fs::create_dir_all(&workspace.root)?;

        // Create memory directory
        let memory_dir = workspace.memory_dir();
        if !memory_dir.exists() {
            std::fs::create_dir_all(&memory_dir)?;
        }

        // Create skills directory
        let skills_dir = workspace.skills_dir();
        if !skills_dir.exists() {
            std::fs::create_dir_all(&skills_dir)?;
        }

        // Create bootstrap files if requested
        if ensure_bootstrap_files {
            self.create_bootstrap_files(&workspace).await?;
        }

        Ok(workspace)
    }

    /// Create bootstrap files from embedded templates.
    async fn create_bootstrap_files(&self, workspace: &AgentWorkspace) -> Result<(), AgentError> {
        for filename in BOOTSTRAP_FILES {
            let file_path = workspace.path(filename);
            if !file_path.exists() {
                let content = self.get_bootstrap_template(filename);
                std::fs::write(&file_path, content)?;
            }
        }
        Ok(())
    }

    /// Get bootstrap template content for a file.
    fn get_bootstrap_template(&self, filename: &str) -> &'static str {
        match filename {
            "SOUL.md" => include_str!("bootstrap/SOUL.md"),
            "AGENTS.md" => include_str!("bootstrap/AGENTS.md"),
            "TOOLS.md" => include_str!("bootstrap/TOOLS.md"),
            "IDENTITY.md" => include_str!("bootstrap/IDENTITY.md"),
            "MEMORY.md" => include_str!("bootstrap/MEMORY.md"),
            "HEARTBEAT.md" => include_str!("bootstrap/HEARTBEAT.md"),
            _ => "",
        }
    }

    /// Read a file from an agent's workspace.
    pub async fn read_workspace_file(
        &self,
        agent_id: &AgentId,
        filename: &str,
    ) -> Result<String, AgentError> {
        let workspace = self.get_workspace(agent_id);
        let file_path = workspace.path(filename);

        if !file_path.exists() {
            return Err(AgentError::Workspace(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", filename),
            )));
        }

        let content = std::fs::read_to_string(&file_path)?;
        Ok(content)
    }

    /// Write a file to an agent's workspace.
    pub async fn write_workspace_file(
        &self,
        agent_id: &AgentId,
        filename: &str,
        content: &str,
    ) -> Result<(), AgentError> {
        let workspace = self.get_workspace(agent_id);

        // Ensure workspace exists
        if !workspace.exists() {
            self.ensure_agent_workspace(agent_id, false).await?;
        }

        let file_path = workspace.path(filename);
        std::fs::write(&file_path, content)?;

        Ok(())
    }

    /// List all files in an agent's workspace.
    pub async fn list_workspace_files(
        &self,
        agent_id: &AgentId,
    ) -> Result<Vec<String>, AgentError> {
        let workspace = self.get_workspace(agent_id);

        if !workspace.exists() {
            return Ok(vec![]);
        }

        let mut files = Vec::new();
        collect_files_recursive(&workspace.root, &workspace.root, &mut files)?;

        Ok(files)
    }

    /// Delete an agent's workspace.

    /// Delete an agent's workspace.
    pub async fn delete_workspace(&self, agent_id: &AgentId) -> Result<(), AgentError> {
        let workspace = self.get_workspace(agent_id);

        if workspace.exists() {
            std::fs::remove_dir_all(&workspace.root)?;
        }

        Ok(())
    }

    /// Check if a workspace exists.
    pub fn workspace_exists(&self, agent_id: &AgentId) -> bool {
        let workspace = self.get_workspace(agent_id);
        workspace.exists()
    }
}

/// Get the default agents directory path.
pub fn get_default_agents_dir() -> Result<PathBuf, AgentError> {
    let home = dirs::home_dir()
        .ok_or_else(|| AgentError::Config("Could not find home directory".to_string()))?;
    Ok(home.join(".config").join("MesoClaw").join("agents"))
}

/// Recursively collect files from a directory.
fn collect_files_recursive(
    current: &PathBuf,
    base: &PathBuf,
    files: &mut Vec<String>,
) -> Result<(), AgentError> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_files_recursive(&path, base, files)?;
        } else {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            files.push(relative.to_string_lossy().to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_agent_workspace_path() {
        let workspace = AgentWorkspace::new(
            AgentId::new("test-agent"),
            PathBuf::from("/tmp/agents/test-agent"),
        );

        assert_eq!(
            workspace.path("SOUL.md"),
            PathBuf::from("/tmp/agents/test-agent/SOUL.md")
        );
        assert_eq!(
            workspace.memory_dir(),
            PathBuf::from("/tmp/agents/test-agent/memory")
        );
        assert_eq!(
            workspace.skills_dir(),
            PathBuf::from("/tmp/agents/test-agent/skills")
        );
    }

    #[tokio::test]
    async fn test_ensure_workspace() {
        let temp_dir = tempdir().unwrap();
        let manager = WorkspaceManager::new(temp_dir.path().to_path_buf());

        let agent_id = AgentId::new("test-agent");
        let workspace = manager
            .ensure_agent_workspace(&agent_id, true)
            .await
            .unwrap();

        assert!(workspace.exists());

        // Check bootstrap files were created
        for filename in BOOTSTRAP_FILES {
            assert!(workspace.path(filename).exists(), "Missing: {}", filename);
        }
    }

    #[tokio::test]
    async fn test_read_write_file() {
        let temp_dir = tempdir().unwrap();
        let manager = WorkspaceManager::new(temp_dir.path().to_path_buf());

        let agent_id = AgentId::new("test-agent");
        manager
            .ensure_agent_workspace(&agent_id, false)
            .await
            .unwrap();

        // Write file
        manager
            .write_workspace_file(&agent_id, "test.txt", "Hello, world!")
            .await
            .unwrap();

        // Read file
        let content = manager
            .read_workspace_file(&agent_id, "test.txt")
            .await
            .unwrap();

        assert_eq!(content, "Hello, world!");
    }

    #[tokio::test]
    async fn test_list_files() {
        let temp_dir = tempdir().unwrap();
        let manager = WorkspaceManager::new(temp_dir.path().to_path_buf());

        let agent_id = AgentId::new("test-agent");
        manager
            .ensure_agent_workspace(&agent_id, true)
            .await
            .unwrap();

        let files = manager.list_workspace_files(&agent_id).await.unwrap();

        // Should have at least the bootstrap files
        assert!(!files.is_empty());

        // Check that SOUL.md is in the list
        assert!(files.iter().any(|f| f == "SOUL.md"));
    }

    #[tokio::test]
    async fn test_delete_workspace() {
        let temp_dir = tempdir().unwrap();
        let manager = WorkspaceManager::new(temp_dir.path().to_path_buf());

        let agent_id = AgentId::new("test-agent");
        manager
            .ensure_agent_workspace(&agent_id, true)
            .await
            .unwrap();

        assert!(manager.workspace_exists(&agent_id));

        manager.delete_workspace(&agent_id).await.unwrap();

        assert!(!manager.workspace_exists(&agent_id));
    }
}
