pub mod coordinator;
pub mod sub_agent;
pub mod task;

pub use coordinator::Coordinator;
pub use task::{DelegationResult, DelegationTask, TaskResult, TaskStatus};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationConfig {
    pub max_sub_agents: usize,
    pub per_agent_token_budget: usize,
    pub per_agent_timeout_secs: u64,
    pub decomposition_model: Option<String>,
}

impl Default for DelegationConfig {
    fn default() -> Self {
        Self {
            max_sub_agents: 4,
            per_agent_token_budget: 4000,
            per_agent_timeout_secs: 120,
            decomposition_model: None,
        }
    }
}

impl DelegationConfig {
    pub fn from_app_config(config: &crate::config::AppConfig) -> Self {
        Self {
            max_sub_agents: config.delegation_max_sub_agents,
            per_agent_token_budget: config.delegation_per_agent_token_budget,
            per_agent_timeout_secs: config.delegation_per_agent_timeout_secs,
            decomposition_model: config.delegation_decomposition_model.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 7.6
    #[test]
    fn delegation_config_defaults() {
        let config = DelegationConfig::default();
        assert_eq!(config.max_sub_agents, 4);
        assert_eq!(config.per_agent_token_budget, 4000);
        assert_eq!(config.per_agent_timeout_secs, 120);
        assert!(config.decomposition_model.is_none());
    }

    // 7.7
    #[test]
    fn delegation_config_serde() {
        let config = DelegationConfig {
            max_sub_agents: 6,
            per_agent_token_budget: 8000,
            per_agent_timeout_secs: 300,
            decomposition_model: Some("openai:gpt-4o".into()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: DelegationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_sub_agents, 6);
        assert_eq!(back.per_agent_token_budget, 8000);
        assert_eq!(back.per_agent_timeout_secs, 300);
        assert_eq!(back.decomposition_model.as_deref(), Some("openai:gpt-4o"));
    }
}
