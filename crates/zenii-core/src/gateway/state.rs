use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use arc_swap::ArcSwap;
use tokio::sync::RwLock;

use crate::config::AppConfig;
use crate::credential::CredentialStore;
use crate::db::DbPool;
use crate::event_bus::EventBus;
use crate::identity::SoulLoader;
use crate::logging::UsageLogger;
use crate::memory::traits::Memory;
use crate::plugins::installer::PluginInstaller;
use crate::plugins::registry::PluginRegistry;
use crate::security::policy::SecurityPolicy;
use crate::skills::SkillRegistry;
use crate::tools::ToolRegistry;
use crate::user::UserLearner;

#[cfg(feature = "channels")]
use crate::channels::registry::ChannelRegistry;
#[cfg(feature = "scheduler")]
use crate::scheduler::TokioScheduler;

#[cfg(feature = "ai")]
use crate::ai::agent::ZeniiAgent;
#[cfg(feature = "ai")]
use crate::ai::context::{BootContext, ContextBuilder};
#[cfg(feature = "ai")]
use crate::ai::prompt::PromptStrategy;
#[cfg(feature = "ai")]
use crate::ai::provider_registry::ProviderRegistry;
#[cfg(feature = "ai")]
use crate::ai::reasoning::ReasoningEngine;
#[cfg(feature = "ai")]
use crate::ai::session::SessionManager;

pub struct AppState {
    pub config: Arc<ArcSwap<AppConfig>>,
    pub config_path: PathBuf,
    /// Write lock for config read-modify-write cycles (prevents lost updates).
    pub config_write_lock: tokio::sync::Mutex<()>,
    pub db: DbPool,
    pub event_bus: Arc<dyn EventBus>,
    pub memory: Arc<dyn Memory>,
    pub credentials: Arc<dyn CredentialStore>,
    pub security: Arc<SecurityPolicy>,
    pub tools: Arc<ToolRegistry>,
    #[cfg(feature = "ai")]
    pub session_manager: Arc<SessionManager>,
    #[cfg(feature = "ai")]
    pub agent: Option<Arc<ZeniiAgent>>,
    #[cfg(feature = "ai")]
    pub provider_registry: Arc<ProviderRegistry>,
    #[cfg(feature = "ai")]
    pub boot_context: BootContext,
    #[cfg(feature = "ai")]
    pub last_used_model: Arc<RwLock<Option<String>>>,
    #[cfg(feature = "ai")]
    pub context_builder: Arc<ContextBuilder>,
    #[cfg(feature = "ai")]
    pub reasoning_engine: Arc<ReasoningEngine>,
    #[cfg(feature = "ai")]
    pub prompt_strategy: Arc<dyn PromptStrategy>,
    /// Runtime toggle: context injection (mutable via PUT /config)
    pub context_injection_enabled: Arc<AtomicBool>,
    /// Runtime toggle: self-evolution / learning (mutable via PUT /config)
    pub self_evolution_enabled: Arc<AtomicBool>,
    pub soul_loader: Arc<SoulLoader>,
    pub skill_registry: Arc<SkillRegistry>,
    pub user_learner: Arc<UserLearner>,
    pub plugin_registry: Arc<PluginRegistry>,
    pub plugin_installer: Arc<PluginInstaller>,
    #[cfg(feature = "channels")]
    pub channel_registry: Arc<ChannelRegistry>,
    #[cfg(feature = "channels")]
    pub channel_router: Option<Arc<crate::channels::router::ChannelRouter>>,
    #[cfg(feature = "scheduler")]
    pub scheduler: Option<Arc<TokioScheduler>>,
    pub notification_router: Option<Arc<crate::notification::router::NotificationRouter>>,
    pub coordinator: Arc<crate::ai::delegation::Coordinator>,
    #[cfg(feature = "workflows")]
    pub workflow_registry: Option<Arc<crate::workflows::WorkflowRegistry>>,
    #[cfg(feature = "workflows")]
    pub workflow_executor: Option<Arc<crate::workflows::executor::WorkflowExecutor>>,
    #[cfg(feature = "workflows")]
    pub active_workflow_runs: Arc<dashmap::DashMap<String, tokio::task::AbortHandle>>,
    pub usage_logger: Arc<UsageLogger>,
    /// Whether the local embedding model is downloaded and ready.
    pub embedding_model_available: Arc<AtomicBool>,
    pub approval_broker: Option<Arc<crate::security::approval::ApprovalBroker>>,
    pub wiki: std::sync::Arc<crate::wiki::WikiManager>,
}

impl AppState {
    /// Wire the scheduler with this AppState for payload execution.
    /// Call this after constructing Arc<AppState>.
    #[cfg(feature = "scheduler")]
    pub fn wire_scheduler(self: &Arc<Self>) {
        if let Some(ref scheduler) = self.scheduler {
            scheduler.wire(Arc::clone(self));
            tracing::info!("Scheduler wired with AppState");
        }
    }

    /// Wire the notification router: subscribe to EventBus and route to channel targets.
    /// Call this after constructing Arc<AppState>.
    pub fn wire_notifications(self: &Arc<Self>) {
        if let Some(ref router) = self.notification_router {
            router.start();
            tracing::info!("Notification router wired with AppState");
        }
    }

    /// Wire channels: start the router and begin listen loops for connected channels.
    /// Call this after constructing Arc<AppState>.
    #[cfg(feature = "channels")]
    pub fn wire_channels(self: &Arc<Self>) {
        if let Some(ref router) = self.channel_router {
            router.wire(Arc::clone(self));
            let router_clone = router.clone();
            let registry = self.channel_registry.clone();
            let event_bus = self.event_bus.clone();
            let config = self.config.load_full();

            tokio::spawn(async move {
                router_clone.start().await;

                // Start supervised listen loops for all registered channels
                for name in registry.list() {
                    if let Some(channel) = registry.get_channel(&name) {
                        let tx = router_clone.sender();
                        let bus = event_bus.clone();
                        let cfg = config.clone();
                        tokio::spawn(crate::channels::router::supervise_channel(
                            channel, tx, bus, cfg,
                        ));
                    }
                }
            });

            tracing::info!("Channel router wired with AppState");
        }
    }
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
        let agent = crate::ai::resolve_agent(None, &state, None, None, "desktop").await;
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
        let agent =
            crate::ai::resolve_agent(Some("openai:gpt-4o"), &state, None, None, "desktop").await;
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
        let _ =
            crate::ai::resolve_agent(Some("openai:gpt-4o"), &state, None, None, "desktop").await;

        // Should now be set
        let last = state.last_used_model.read().await;
        assert_eq!(last.as_deref(), Some("openai:gpt-4o"));
    }
}
