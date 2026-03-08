use std::sync::Arc;

use rig::agent::Agent;
use rig::completion::{Chat, Prompt};
use rig::message::Message;
use rig::prelude::CompletionClient;
use rig::providers::{anthropic, openai};
use tokio::sync::broadcast;

use crate::config::AppConfig;
use crate::credential::CredentialStore;
use crate::tools::Tool;
use crate::{MesoError, Result};

// Only needed for resolve_agent(), which requires the full AppState
#[cfg(feature = "ai")]
use crate::gateway::state::AppState;

use super::adapter::{RigToolAdapter, ToolCallEvent};
use super::providers;

type OpenAIAgent = Agent<openai::completion::CompletionModel>;
type AnthropicAgent = Agent<anthropic::completion::CompletionModel>;

enum AgentInner {
    OpenAI(OpenAIAgent),
    Anthropic(AnthropicAgent),
}

/// MesoAgent wraps a rig-core Agent with provider abstraction.
/// Supports OpenAI-compatible and Anthropic providers.
// Debug can't be derived (Agent<M> doesn't impl Debug), use manual impl for test ergonomics.
pub struct MesoAgent {
    inner: AgentInner,
}

impl std::fmt::Debug for MesoAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variant = match &self.inner {
            AgentInner::OpenAI(_) => "OpenAI",
            AgentInner::Anthropic(_) => "Anthropic",
        };
        f.debug_struct("MesoAgent")
            .field("provider", &variant)
            .finish()
    }
}

impl MesoAgent {
    /// Build a new MesoAgent from config, credentials, and tools.
    pub async fn new(
        config: &AppConfig,
        credentials: &dyn CredentialStore,
        tools: &[Arc<dyn Tool>],
    ) -> Result<Self> {
        let api_key = providers::resolve_api_key(config, credentials).await?;
        let rig_tools = RigToolAdapter::from_tools(tools);

        let preamble = config
            .agent_system_prompt
            .as_deref()
            .unwrap_or("You are MesoClaw, a helpful AI assistant.");

        let inner = match config.provider_type.as_str() {
            "openai" | "custom" => {
                let client =
                    providers::build_openai_client(&api_key, config.provider_base_url.as_deref())?;
                let agent = client
                    .agent(&config.provider_model_id)
                    .preamble(preamble)
                    .max_tokens(config.agent_max_tokens as u64)
                    .default_max_turns(config.agent_max_turns)
                    .tools(rig_tools)
                    .build();
                AgentInner::OpenAI(agent)
            }
            "anthropic" => {
                let client = providers::build_anthropic_client(&api_key)?;
                let agent = client
                    .agent(&config.provider_model_id)
                    .preamble(preamble)
                    .max_tokens(config.agent_max_tokens as u64)
                    .default_max_turns(config.agent_max_turns)
                    .tools(rig_tools)
                    .build();
                AgentInner::Anthropic(agent)
            }
            other => {
                return Err(MesoError::Agent(format!(
                    "unsupported provider type: '{other}'. Supported: openai, anthropic, custom"
                )));
            }
        };

        Ok(Self { inner })
    }

    /// Build a new MesoAgent from provider details (for dynamic per-request agent building).
    ///
    /// Provider type is inferred at runtime: `provider_id == "anthropic"` uses the native
    /// Anthropic client, everything else uses the OpenAI-compatible client with `base_url`.
    ///
    /// If `preamble_override` is provided, it replaces the default system prompt.
    #[allow(clippy::too_many_arguments)]
    pub async fn from_provider(
        provider_id: &str,
        base_url: &str,
        model_id: &str,
        requires_api_key: bool,
        credentials: &dyn CredentialStore,
        tools: &[Arc<dyn Tool>],
        config: &AppConfig,
        preamble_override: Option<&str>,
    ) -> Result<Self> {
        let api_key =
            providers::resolve_api_key_for_provider(provider_id, requires_api_key, credentials)
                .await?;
        let rig_tools = RigToolAdapter::from_tools(tools);

        let preamble = preamble_override.unwrap_or_else(|| {
            config
                .agent_system_prompt
                .as_deref()
                .unwrap_or("You are MesoClaw, a helpful AI assistant.")
        });

        let inner = if provider_id == "anthropic" {
            let client = providers::build_anthropic_client(&api_key)?;
            let agent = client
                .agent(model_id)
                .preamble(preamble)
                .max_tokens(config.agent_max_tokens as u64)
                .default_max_turns(config.agent_max_turns)
                .tools(rig_tools)
                .build();
            AgentInner::Anthropic(agent)
        } else {
            let client = providers::build_openai_client(&api_key, Some(base_url))?;
            let agent = client
                .agent(model_id)
                .preamble(preamble)
                .max_tokens(config.agent_max_tokens as u64)
                .default_max_turns(config.agent_max_turns)
                .tools(rig_tools)
                .build();
            AgentInner::OpenAI(agent)
        };

        Ok(Self { inner })
    }

    /// Build a new MesoAgent from provider details with tool event broadcasting.
    ///
    /// If `preamble_override` is provided, it replaces the default system prompt.
    #[allow(clippy::too_many_arguments)]
    pub async fn from_provider_with_events(
        provider_id: &str,
        base_url: &str,
        model_id: &str,
        requires_api_key: bool,
        credentials: &dyn CredentialStore,
        tools: &[Arc<dyn Tool>],
        config: &AppConfig,
        tool_event_tx: broadcast::Sender<ToolCallEvent>,
        preamble_override: Option<&str>,
    ) -> Result<Self> {
        let api_key =
            providers::resolve_api_key_for_provider(provider_id, requires_api_key, credentials)
                .await?;
        let rig_tools = RigToolAdapter::from_tools_with_events(tools, tool_event_tx);

        let preamble = preamble_override.unwrap_or_else(|| {
            config
                .agent_system_prompt
                .as_deref()
                .unwrap_or("You are MesoClaw, a helpful AI assistant.")
        });

        let inner = if provider_id == "anthropic" {
            let client = providers::build_anthropic_client(&api_key)?;
            let agent = client
                .agent(model_id)
                .preamble(preamble)
                .max_tokens(config.agent_max_tokens as u64)
                .default_max_turns(config.agent_max_turns)
                .tools(rig_tools)
                .build();
            AgentInner::Anthropic(agent)
        } else {
            let client = providers::build_openai_client(&api_key, Some(base_url))?;
            let agent = client
                .agent(model_id)
                .preamble(preamble)
                .max_tokens(config.agent_max_tokens as u64)
                .default_max_turns(config.agent_max_turns)
                .tools(rig_tools)
                .build();
            AgentInner::OpenAI(agent)
        };

        Ok(Self { inner })
    }

    /// Send a simple prompt and get a response.
    pub async fn prompt(&self, input: &str) -> Result<String> {
        match &self.inner {
            AgentInner::OpenAI(agent) => agent
                .prompt(input)
                .await
                .map_err(|e| MesoError::Agent(format!("prompt failed: {e}"))),
            AgentInner::Anthropic(agent) => agent
                .prompt(input)
                .await
                .map_err(|e| MesoError::Agent(format!("prompt failed: {e}"))),
        }
    }

    /// Send a prompt with chat history and get a response.
    pub async fn chat(&self, input: &str, history: Vec<Message>) -> Result<String> {
        match &self.inner {
            AgentInner::OpenAI(agent) => agent
                .chat(input, history)
                .await
                .map_err(|e| MesoError::Agent(format!("chat failed: {e}"))),
            AgentInner::Anthropic(agent) => agent
                .chat(input, history)
                .await
                .map_err(|e| MesoError::Agent(format!("chat failed: {e}"))),
        }
    }
}

/// Resolve the agent to use for a chat request.
///
/// Resolution chain:
/// 1. If `requested_model` is Some ("provider_id:model_id") → build agent from provider registry
/// 2. `last_used_model` (session-persistent) from AppState
/// 3. Default model from `ProviderRegistry`
/// 4. Boot-time fallback agent
///
/// When `tool_event_tx` is provided, a fresh agent is always built with event-emitting adapters
/// so tool calls are visible to the caller.
///
/// If `preamble_override` is provided, it replaces the default system prompt.
#[cfg(feature = "ai")]
pub async fn resolve_agent(
    requested_model: Option<&str>,
    state: &AppState,
    tool_event_tx: Option<broadcast::Sender<ToolCallEvent>>,
    preamble_override: Option<&str>,
) -> Result<Arc<MesoAgent>> {
    // Try requested model first, then last_used, then default model
    let model_spec = if let Some(spec) = requested_model {
        // Explicit model — also update last_used_model
        {
            let mut last = state.last_used_model.write().await;
            *last = Some(spec.to_string());
        }
        Some(spec.to_string())
    } else {
        // Check last_used_model
        let last = state.last_used_model.read().await;
        if let Some(ref last_model) = *last {
            Some(last_model.clone())
        } else if let Some((pid, mid)) = state.provider_registry.get_default_model().await? {
            Some(format!("{pid}:{mid}"))
        } else {
            None
        }
    };

    if let Some(spec) = model_spec {
        let (provider_id, model_id) = spec.split_once(':').ok_or_else(|| {
            MesoError::Agent(format!(
                "invalid model format '{spec}': expected 'provider_id:model_id'"
            ))
        })?;

        let provider = state.provider_registry.get_provider(provider_id).await?;
        let tools = state.tools.to_vec();

        let agent = if let Some(tx) = tool_event_tx {
            MesoAgent::from_provider_with_events(
                provider_id,
                &provider.provider.base_url,
                model_id,
                provider.provider.requires_api_key,
                state.credentials.as_ref(),
                &tools,
                &state.config,
                tx,
                preamble_override,
            )
            .await?
        } else {
            MesoAgent::from_provider(
                provider_id,
                &provider.provider.base_url,
                model_id,
                provider.provider.requires_api_key,
                state.credentials.as_ref(),
                &tools,
                &state.config,
                preamble_override,
            )
            .await?
        };

        return Ok(Arc::new(agent));
    }

    // Fallback to boot-time agent
    state
        .agent
        .clone()
        .ok_or_else(|| MesoError::Agent("no agent configured".into()))
}

// Compile-time assertion: MesoAgent must be Send + Sync for use in AppState
#[cfg(test)]
const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    #[allow(dead_code)]
    fn check() {
        assert_send_sync::<MesoAgent>();
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::InMemoryCredentialStore;

    // 1.4.1 — agent new with mock (uses real client, no LLM call)
    #[tokio::test]
    async fn agent_new_with_openai_config() {
        let config = AppConfig {
            provider_type: "openai".into(),
            provider_model_id: "gpt-4o".into(),
            ..Default::default()
        };
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:openai", "sk-test-key").await.unwrap();

        let tools: Vec<Arc<dyn Tool>> = vec![];
        let agent = MesoAgent::new(&config, &creds, &tools).await;
        assert!(agent.is_ok());
    }

    // 1.4.1b — agent new with anthropic config
    #[tokio::test]
    async fn agent_new_with_anthropic_config() {
        let config = AppConfig {
            provider_name: "anthropic".into(),
            provider_type: "anthropic".into(),
            provider_model_id: "claude-sonnet-4-20250514".into(),
            ..Default::default()
        };
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:anthropic", "sk-ant-test").await.unwrap();

        let tools: Vec<Arc<dyn Tool>> = vec![];
        let agent = MesoAgent::new(&config, &creds, &tools).await;
        assert!(agent.is_ok());
    }

    // 1.4.1c — unknown provider type errors
    #[tokio::test]
    async fn agent_new_unknown_provider_errors() {
        let config = AppConfig {
            provider_name: "bad".into(),
            provider_type: "bad".into(),
            ..Default::default()
        };
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:bad", "sk-test").await.unwrap();

        let tools: Vec<Arc<dyn Tool>> = vec![];
        let result = MesoAgent::new(&config, &creds, &tools).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, MesoError::Agent(_)));
    }

    // from_provider: OpenAI-compatible provider
    #[tokio::test]
    async fn from_provider_openai_compatible() {
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:openai", "sk-test").await.unwrap();
        let config = AppConfig::default();
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = MesoAgent::from_provider(
            "openai",
            "https://api.openai.com/v1",
            "gpt-4o",
            true,
            &creds,
            &tools,
            &config,
            None,
        )
        .await;
        assert!(agent.is_ok());
    }

    // from_provider: Anthropic provider
    #[tokio::test]
    async fn from_provider_anthropic() {
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:anthropic", "sk-ant-test").await.unwrap();
        let config = AppConfig::default();
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = MesoAgent::from_provider(
            "anthropic",
            "https://api.anthropic.com",
            "claude-sonnet-4-20250514",
            true,
            &creds,
            &tools,
            &config,
            None,
        )
        .await;
        assert!(agent.is_ok());
    }

    // from_provider: OpenRouter with anthropic model uses OpenAI-compatible client
    #[tokio::test]
    async fn from_provider_openrouter_anthropic_model() {
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:openrouter", "sk-or-test").await.unwrap();
        let config = AppConfig::default();
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = MesoAgent::from_provider(
            "openrouter",
            "https://openrouter.ai/api/v1",
            "anthropic/claude-sonnet-4-20250514",
            true,
            &creds,
            &tools,
            &config,
            None,
        )
        .await;
        assert!(agent.is_ok());
    }

    // from_provider: no-key-required provider (ollama)
    #[tokio::test]
    async fn from_provider_no_key_required() {
        let creds = InMemoryCredentialStore::new();
        let config = AppConfig::default();
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let agent = MesoAgent::from_provider(
            "ollama",
            "http://localhost:11434/v1",
            "llama3",
            false,
            &creds,
            &tools,
            &config,
            None,
        )
        .await;
        assert!(agent.is_ok());
    }

    // from_provider: missing API key errors
    #[tokio::test]
    async fn from_provider_missing_key_errors() {
        let creds = InMemoryCredentialStore::new();
        let config = AppConfig::default();
        let tools: Vec<Arc<dyn Tool>> = vec![];

        let result = MesoAgent::from_provider(
            "openai",
            "https://api.openai.com/v1",
            "gpt-4o",
            true,
            &creds,
            &tools,
            &config,
            None,
        )
        .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::Credential(_)));
    }

    // 1.4.4 — agent respects max turns (config-level check)
    #[tokio::test]
    async fn agent_respects_config_max_turns() {
        let config = AppConfig {
            provider_type: "openai".into(),
            agent_max_turns: 1,
            agent_max_tokens: 100,
            ..Default::default()
        };
        let creds = InMemoryCredentialStore::new();
        creds.set("api_key:openai", "sk-test").await.unwrap();

        let tools: Vec<Arc<dyn Tool>> = vec![];
        let agent = MesoAgent::new(&config, &creds, &tools).await;
        assert!(agent.is_ok());
    }
}
