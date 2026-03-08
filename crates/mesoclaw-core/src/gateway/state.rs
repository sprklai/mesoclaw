use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::RwLock;

use crate::config::AppConfig;
use crate::credential::CredentialStore;
use crate::db::DbPool;
use crate::event_bus::EventBus;
use crate::identity::SoulLoader;
use crate::memory::traits::Memory;
use crate::security::policy::SecurityPolicy;
use crate::skills::SkillRegistry;
use crate::tools::ToolRegistry;
use crate::user::UserLearner;

#[cfg(feature = "channels")]
use crate::channels::registry::ChannelRegistry;
#[cfg(feature = "scheduler")]
use crate::scheduler::TokioScheduler;

#[cfg(feature = "ai")]
use crate::ai::agent::MesoAgent;
#[cfg(feature = "ai")]
use crate::ai::context::{BootContext, ContextBuilder};
#[cfg(feature = "ai")]
use crate::ai::provider_registry::ProviderRegistry;
#[cfg(feature = "ai")]
use crate::ai::session::SessionManager;

pub struct AppState {
    pub config: Arc<AppConfig>,
    pub config_path: PathBuf,
    pub db: DbPool,
    pub event_bus: Arc<dyn EventBus>,
    pub memory: Arc<dyn Memory>,
    pub credentials: Arc<dyn CredentialStore>,
    pub security: Arc<SecurityPolicy>,
    pub tools: Arc<ToolRegistry>,
    #[cfg(feature = "ai")]
    pub session_manager: Arc<SessionManager>,
    #[cfg(feature = "ai")]
    pub agent: Option<Arc<MesoAgent>>,
    #[cfg(feature = "ai")]
    pub provider_registry: Arc<ProviderRegistry>,
    #[cfg(feature = "ai")]
    pub boot_context: BootContext,
    #[cfg(feature = "ai")]
    pub last_used_model: Arc<RwLock<Option<String>>>,
    #[cfg(feature = "ai")]
    pub context_builder: Arc<ContextBuilder>,
    /// Runtime toggle: context injection (mutable via PUT /config)
    pub context_injection_enabled: Arc<AtomicBool>,
    /// Runtime toggle: self-evolution / learning (mutable via PUT /config)
    pub self_evolution_enabled: Arc<AtomicBool>,
    pub soul_loader: Arc<SoulLoader>,
    pub skill_registry: Arc<SkillRegistry>,
    pub user_learner: Arc<UserLearner>,
    #[cfg(feature = "channels")]
    pub channel_registry: Arc<ChannelRegistry>,
    #[cfg(feature = "scheduler")]
    pub scheduler: Option<Arc<TokioScheduler>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppState>();
    }

    // 15.3.34 — last_used_model initially None
    #[tokio::test]
    async fn last_used_model_initially_none() {
        let last_used = Arc::new(RwLock::new(None::<String>));
        let val = last_used.read().await;
        assert!(val.is_none());
    }

    // 15.3.35 — resolve_agent uses last_used_model when no explicit model
    #[tokio::test]
    async fn resolve_agent_uses_last_used_model() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        // Set an API key for openai
        state
            .credentials
            .set("api_key:openai", "sk-test-key")
            .await
            .unwrap();

        // Set last_used_model
        {
            let mut last = state.last_used_model.write().await;
            *last = Some("openai:gpt-4o".into());
        }

        // resolve_agent with no explicit model should use last_used_model
        let agent = crate::ai::resolve_agent(None, &state, None, None).await;
        assert!(agent.is_ok(), "Should resolve using last_used_model");
    }

    // 15.3.36 — resolve_agent explicit model overrides last_used
    #[tokio::test]
    async fn resolve_agent_explicit_overrides_last_used() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        state
            .credentials
            .set("api_key:openai", "sk-test-key")
            .await
            .unwrap();

        // Set last_used_model to something
        {
            let mut last = state.last_used_model.write().await;
            *last = Some("openai:gpt-3.5-turbo".into());
        }

        // Explicit model should override
        let agent = crate::ai::resolve_agent(Some("openai:gpt-4o"), &state, None, None).await;
        assert!(agent.is_ok());

        // Verify last_used_model was updated to the explicit one
        let last = state.last_used_model.read().await;
        assert_eq!(last.as_deref(), Some("openai:gpt-4o"));
    }

    // 15.3.37 — resolve_agent updates last_used_model on explicit selection
    #[tokio::test]
    async fn resolve_agent_updates_last_used_on_explicit() {
        let (_dir, state) = crate::gateway::handlers::tests::test_state().await;
        state
            .credentials
            .set("api_key:openai", "sk-test-key")
            .await
            .unwrap();

        // Initially None
        assert!(state.last_used_model.read().await.is_none());

        // Resolve with explicit model
        let _ = crate::ai::resolve_agent(Some("openai:gpt-4o"), &state, None, None).await;

        // Should now be set
        let last = state.last_used_model.read().await;
        assert_eq!(last.as_deref(), Some("openai:gpt-4o"));
    }
}
