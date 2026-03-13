#[cfg(feature = "channels")]
pub mod channels;
pub mod channels_test;
pub mod chat;
pub mod config;
pub mod credentials;
pub mod embeddings;
pub mod health;
pub mod identity;
pub mod memory;
pub mod messages;
pub mod models;
pub mod permissions;
pub mod plugins;
pub mod providers;
#[cfg(feature = "scheduler")]
pub mod scheduler;
pub mod sessions;
pub mod skill_proposals;
pub mod skills;
pub mod system;
pub mod tools;
pub mod user;
pub mod ws;

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use tokio::sync::RwLock;

    use crate::config::AppConfig;
    use crate::credential::InMemoryCredentialStore;
    use crate::gateway::state::AppState;
    use crate::identity::SoulLoader;
    use crate::memory::in_memory_store::InMemoryStore;
    use crate::plugins::installer::PluginInstaller;
    use crate::plugins::registry::PluginRegistry;
    use crate::security::policy::SecurityPolicy;
    use crate::skills::SkillRegistry;
    use crate::user::UserLearner;

    pub async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&db_path).unwrap();
        crate::db::with_db(&pool, |conn| crate::db::run_migrations(conn))
            .await
            .unwrap();

        let config = AppConfig {
            gateway_cors_origins: vec!["http://localhost:18971".into()],
            ..Default::default()
        };

        let identity_dir = dir.path().join("identity");
        let skills_dir = dir.path().join("skills");

        let soul_loader = Arc::new(SoulLoader::new(&identity_dir).unwrap());
        let skill_registry =
            Arc::new(SkillRegistry::new(&skills_dir, config.skill_max_content_size).unwrap());
        let user_learner = Arc::new(UserLearner::new(pool.clone(), &config));

        let provider_registry = Arc::new(crate::ai::provider_registry::ProviderRegistry::new(
            pool.clone(),
        ));
        provider_registry.seed_builtin_providers().await.unwrap();

        #[cfg(feature = "channels")]
        let channel_registry = Arc::new(crate::channels::registry::ChannelRegistry::new());

        let config = Arc::new(config);
        let memory: Arc<dyn crate::memory::traits::Memory> = Arc::new(InMemoryStore::new());
        let session_manager = Arc::new(crate::ai::session::SessionManager::new(pool.clone()));

        let context_builder = Arc::new(crate::ai::context::ContextBuilder::new(
            session_manager.clone(),
            memory.clone(),
            soul_loader.clone(),
            user_learner.clone(),
            config.clone(),
        ));

        let plugins_dir = dir.path().join("plugins");
        let tool_registry = Arc::new(crate::tools::ToolRegistry::new());
        let plugin_registry = Arc::new(PluginRegistry::new(plugins_dir).unwrap());
        let plugin_installer = Arc::new(PluginInstaller::new(
            plugin_registry.clone(),
            tool_registry.clone(),
            skill_registry.clone(),
            60,
            3,
        ));

        let state = Arc::new(AppState {
            config: Arc::new(arc_swap::ArcSwap::from(config)),
            config_path: dir.path().join("config.toml"),
            config_write_lock: tokio::sync::Mutex::new(()),
            db: pool.clone(),
            event_bus: Arc::new(crate::event_bus::TokioBroadcastBus::new(16)),
            memory,
            credentials: Arc::new(InMemoryCredentialStore::new()),
            security: Arc::new(SecurityPolicy::default_policy()),
            tools: tool_registry,
            session_manager,
            agent: None,
            provider_registry,
            boot_context: crate::ai::context::BootContext::from_system(),
            last_used_model: Arc::new(RwLock::new(None)),
            context_builder,
            reasoning_engine: Arc::new(crate::ai::reasoning::ReasoningEngine::new(3)),
            prompt_strategy: {
                let compact_config = Arc::new(crate::config::AppConfig::default());
                Arc::new(crate::ai::prompt::CompactStrategy::new(
                    compact_config,
                    crate::ai::context::BootContext::from_system(),
                ))
            },
            context_injection_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            self_evolution_enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            soul_loader,
            skill_registry,
            user_learner,
            plugin_registry,
            plugin_installer,
            #[cfg(feature = "channels")]
            channel_registry,
            #[cfg(feature = "channels")]
            channel_router: None,
            #[cfg(feature = "scheduler")]
            scheduler: {
                let sched = crate::scheduler::TokioScheduler::new(
                    pool.clone(),
                    Arc::new(crate::event_bus::TokioBroadcastBus::new(16)),
                    &crate::config::AppConfig::default(),
                );
                Some(sched)
            },
            notification_router: None,
            embedding_model_available: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        });
        (dir, state)
    }
}
