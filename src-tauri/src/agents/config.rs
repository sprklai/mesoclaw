//! Agent configuration management.
//!
//! Provides CRUD operations for agent configurations with JSON file persistence.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{AgentError, AgentId, AgentStatus, ThinkingLevel, VerboseLevel};
use crate::tools::ToolProfile;

/// Model configuration for an agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConfig {
    /// Primary LLM provider (e.g., "anthropic", "openai").
    pub provider: String,
    /// Primary model ID (e.g., "claude-sonnet-4-20250514").
    pub model_id: String,
    /// Fallback models to try if primary fails.
    #[serde(default)]
    pub fallbacks: Vec<ModelFallback>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
            fallbacks: vec![],
        }
    }
}

/// Fallback model configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFallback {
    pub provider: String,
    pub model_id: String,
}

/// Concurrency settings for an agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConcurrencyConfig {
    /// Maximum parallel executions for this agent.
    #[serde(default = "default_max_parallel")]
    pub max_parallel: u32,
    /// Maximum concurrent runs per agent instance.
    #[serde(default = "default_per_agent")]
    pub per_agent: u32,
}

fn default_max_parallel() -> u32 {
    3
}

fn default_per_agent() -> u32 {
    1
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_parallel: default_max_parallel(),
            per_agent: default_per_agent(),
        }
    }
}

/// Complete agent configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    /// Unique agent identifier.
    pub id: AgentId,
    /// Human-readable name.
    pub name: String,
    /// Agent description.
    #[serde(default)]
    pub description: String,
    /// Path to the agent's workspace directory.
    pub workspace: PathBuf,
    /// Model configuration.
    #[serde(default)]
    pub model: ModelConfig,
    /// Default thinking level.
    #[serde(default)]
    pub thinking_level: ThinkingLevel,
    /// Default verbose level.
    #[serde(default)]
    pub verbose_level: VerboseLevel,
    /// Default timeout in seconds (0 = no timeout).
    #[serde(default)]
    pub timeout_seconds: u64,
    /// Concurrency settings.
    #[serde(default)]
    pub concurrency: ConcurrencyConfig,
    /// Tool access profile for this agent.
    #[serde(default)]
    pub tool_profile: ToolProfile,
    /// Whether this agent is user-defined.
    #[serde(default)]
    pub is_user_defined: bool,
    /// Creation timestamp (ISO 8601).
    pub created_at: String,
    /// Last update timestamp (ISO 8601).
    pub updated_at: String,
    /// Current status.
    #[serde(default)]
    pub status: AgentStatus,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentConfig {
    /// Create a new agent configuration with defaults.
    pub fn new(id: impl Into<String>, name: impl Into<String>, workspace: PathBuf) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: AgentId::new(id),
            name: name.into(),
            description: String::new(),
            workspace,
            model: ModelConfig::default(),
            thinking_level: ThinkingLevel::default(),
            verbose_level: VerboseLevel::default(),
            timeout_seconds: 300, // 5 minutes default
            concurrency: ConcurrencyConfig::default(),
            tool_profile: ToolProfile::default(),
            is_user_defined: true,
            created_at: now.clone(),
            updated_at: now,
            status: AgentStatus::default(),
            metadata: HashMap::new(),
        }
    }

    /// Update the configuration with partial changes.
    pub fn update(&mut self, changes: AgentConfigUpdate) {
        if let Some(name) = changes.name {
            self.name = name;
        }
        if let Some(description) = changes.description {
            self.description = description;
        }
        if let Some(model) = changes.model {
            self.model = model;
        }
        if let Some(thinking_level) = changes.thinking_level {
            self.thinking_level = thinking_level;
        }
        if let Some(verbose_level) = changes.verbose_level {
            self.verbose_level = verbose_level;
        }
        if let Some(timeout_seconds) = changes.timeout_seconds {
            self.timeout_seconds = timeout_seconds;
        }
        if let Some(concurrency) = changes.concurrency {
            self.concurrency = concurrency;
        }
        if let Some(tool_profile) = changes.tool_profile {
            self.tool_profile = tool_profile;
        }
        if let Some(status) = changes.status {
            self.status = status;
        }
        if let Some(metadata) = changes.metadata {
            self.metadata = metadata;
        }
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

/// Partial update for agent configuration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfigUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub model: Option<ModelConfig>,
    pub thinking_level: Option<ThinkingLevel>,
    pub verbose_level: Option<VerboseLevel>,
    pub timeout_seconds: Option<u64>,
    pub concurrency: Option<ConcurrencyConfig>,
    pub tool_profile: Option<ToolProfile>,
    pub status: Option<AgentStatus>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Manager for agent configurations.
pub struct AgentConfigManager {
    configs: RwLock<HashMap<String, AgentConfig>>,
    config_dir: PathBuf,
}

impl AgentConfigManager {
    /// Create a new configuration manager.
    pub fn new(config_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            configs: RwLock::new(HashMap::new()),
            config_dir,
        })
    }

    /// Load all agent configurations from disk.
    pub async fn load_all(&self) -> Result<(), AgentError> {
        let mut configs = self.configs.write().await;
        configs.clear();

        if !self.config_dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(&self.config_dir)
            .map_err(|e| AgentError::Config(format!("Failed to read config directory: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let config_file = path.join("agent.json");
            if !config_file.exists() {
                continue;
            }

            match self.load_config_file(&config_file) {
                Ok(config) => {
                    configs.insert(config.id.0.clone(), config);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to load agent config from {}: {}",
                        config_file.display(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Load a single configuration file.
    fn load_config_file(&self, path: &PathBuf) -> Result<AgentConfig, AgentError> {
        let content = std::fs::read_to_string(path)?;
        let config: AgentConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save a configuration to disk.
    async fn save_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        let agent_dir = self.config_dir.join(&config.id.0);
        std::fs::create_dir_all(&agent_dir)?;

        let config_file = agent_dir.join("agent.json");
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&config_file, content)?;

        Ok(())
    }

    /// Create a new agent configuration.
    pub async fn create_agent(
        &self,
        id: impl Into<String>,
        name: impl Into<String>,
        workspace: Option<PathBuf>,
    ) -> Result<AgentConfig, AgentError> {
        let id = id.into();
        let name = name.into();

        // Check if agent already exists
        {
            let configs = self.configs.read().await;
            if configs.contains_key(&id) {
                return Err(AgentError::AlreadyExists(id));
            }
        }

        // Determine workspace path
        let workspace = workspace.unwrap_or_else(|| self.config_dir.join(&id).join("workspace"));

        let config = AgentConfig::new(&id, &name, workspace);

        // Save to disk
        self.save_config(&config).await?;

        // Add to in-memory cache
        {
            let mut configs = self.configs.write().await;
            configs.insert(id.clone(), config.clone());
        }

        Ok(config)
    }

    /// List all agent configurations.
    pub async fn list_agents(&self) -> Vec<AgentConfig> {
        self.configs.read().await.values().cloned().collect()
    }

    /// Get a specific agent configuration.
    pub async fn get_agent(&self, id: &str) -> Result<AgentConfig, AgentError> {
        self.configs
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| AgentError::NotFound(id.to_string()))
    }

    /// Update an agent configuration.
    pub async fn update_agent(
        &self,
        id: &str,
        changes: AgentConfigUpdate,
    ) -> Result<AgentConfig, AgentError> {
        let mut configs = self.configs.write().await;

        let config = configs
            .get_mut(id)
            .ok_or_else(|| AgentError::NotFound(id.to_string()))?;

        config.update(changes);

        // Save to disk
        let config_clone = config.clone();
        drop(configs); // Release lock before I/O

        self.save_config(&config_clone).await?;

        // Update in-memory
        {
            let mut configs = self.configs.write().await;
            configs.insert(id.to_string(), config_clone.clone());
        }

        Ok(config_clone)
    }

    /// Delete an agent configuration.
    pub async fn delete_agent(&self, id: &str) -> Result<(), AgentError> {
        // Check existence
        {
            let configs = self.configs.read().await;
            if !configs.contains_key(id) {
                return Err(AgentError::NotFound(id.to_string()));
            }
        }

        // Remove from disk
        let agent_dir = self.config_dir.join(id);
        if agent_dir.exists() {
            std::fs::remove_dir_all(&agent_dir)?;
        }

        // Remove from in-memory
        {
            let mut configs = self.configs.write().await;
            configs.remove(id);
        }

        Ok(())
    }

    /// Check if an agent exists.
    pub async fn exists(&self, id: &str) -> bool {
        self.configs.read().await.contains_key(id)
    }

    /// Count total agents.
    pub async fn count(&self) -> usize {
        self.configs.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_agent_config_creation() {
        let workspace = PathBuf::from("/tmp/test-workspace");
        let config = AgentConfig::new("test-agent", "Test Agent", workspace.clone());

        assert_eq!(config.id.0, "test-agent");
        assert_eq!(config.name, "Test Agent");
        assert_eq!(config.workspace, workspace);
        assert!(config.is_user_defined);
        assert_eq!(config.timeout_seconds, 300);
    }

    #[test]
    fn test_agent_config_update() {
        let mut config = AgentConfig::new("test", "Test", PathBuf::from("/tmp"));

        let update = AgentConfigUpdate {
            name: Some("Updated Name".to_string()),
            timeout_seconds: Some(600),
            ..Default::default()
        };

        config.update(update);

        assert_eq!(config.name, "Updated Name");
        assert_eq!(config.timeout_seconds, 600);
    }

    #[tokio::test]
    async fn test_config_manager_create_agent() {
        let temp_dir = tempdir().unwrap();
        let manager = AgentConfigManager::new(temp_dir.path().to_path_buf());

        let config = manager
            .create_agent("test-agent", "Test Agent", None)
            .await
            .unwrap();

        assert_eq!(config.id.0, "test-agent");
        assert!(manager.exists("test-agent").await);
    }

    #[tokio::test]
    async fn test_config_manager_list_agents() {
        let temp_dir = tempdir().unwrap();
        let manager = AgentConfigManager::new(temp_dir.path().to_path_buf());

        manager
            .create_agent("agent-1", "Agent 1", None)
            .await
            .unwrap();
        manager
            .create_agent("agent-2", "Agent 2", None)
            .await
            .unwrap();

        let agents = manager.list_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_config_manager_delete_agent() {
        let temp_dir = tempdir().unwrap();
        let manager = AgentConfigManager::new(temp_dir.path().to_path_buf());

        manager
            .create_agent("to-delete", "To Delete", None)
            .await
            .unwrap();

        manager.delete_agent("to-delete").await.unwrap();
        assert!(!manager.exists("to-delete").await);
    }

    #[tokio::test]
    async fn test_config_manager_update_agent() {
        let temp_dir = tempdir().unwrap();
        let manager = AgentConfigManager::new(temp_dir.path().to_path_buf());

        manager.create_agent("test", "Test", None).await.unwrap();

        let update = AgentConfigUpdate {
            name: Some("Updated Name".to_string()),
            ..Default::default()
        };

        let updated = manager.update_agent("test", update).await.unwrap();
        assert_eq!(updated.name, "Updated Name");
    }
}
