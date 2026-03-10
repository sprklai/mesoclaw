use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use tokio::sync::RwLock;
use tracing::info;

use crate::Result;
use crate::config::AppConfig;
use crate::credential::CredentialStore;
use crate::db::{self, DbPool};
use crate::event_bus::TokioBroadcastBus;
use crate::identity::SoulLoader;
use crate::memory::in_memory_store::InMemoryStore;
use crate::security::policy::SecurityPolicy;
use crate::skills::SkillRegistry;
use crate::tools::ToolRegistry;
use crate::user::UserLearner;

#[cfg(feature = "channels")]
use crate::channels::registry::ChannelRegistry;

#[cfg(feature = "scheduler")]
use crate::scheduler::{TokioScheduler, traits::Scheduler};

#[cfg(feature = "ai")]
use crate::ai::{
    agent::MesoAgent,
    context::{BootContext, ContextBuilder},
    provider_registry::ProviderRegistry,
    reasoning::{ReasoningEngine, continuation::ContinuationStrategy},
    session::SessionManager,
};

#[cfg(feature = "gateway")]
use crate::gateway::state::AppState;

/// Initialized services bundle for use without the gateway feature.
pub struct Services {
    pub config: Arc<AppConfig>,
    pub config_path: PathBuf,
    pub db: DbPool,
    pub event_bus: Arc<TokioBroadcastBus>,
    pub memory: Arc<InMemoryStore>,
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
    #[cfg(feature = "ai")]
    pub reasoning_engine: Arc<ReasoningEngine>,
    pub context_injection_enabled: Arc<AtomicBool>,
    pub self_evolution_enabled: Arc<AtomicBool>,
    pub soul_loader: Arc<SoulLoader>,
    pub skill_registry: Arc<SkillRegistry>,
    pub user_learner: Arc<UserLearner>,
    #[cfg(feature = "channels")]
    pub channel_registry: Arc<ChannelRegistry>,
    #[cfg(feature = "channels")]
    pub channel_router: Option<Arc<crate::channels::router::ChannelRouter>>,
    #[cfg(feature = "scheduler")]
    pub scheduler: Option<Arc<TokioScheduler>>,
}

/// Initialize all services from config.
pub async fn init_services(config: AppConfig) -> Result<Services> {
    let config = Arc::new(config);

    // 1. Database
    let db_path = config
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config::default_data_dir().join("mesoclaw.db"));

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pool = db::init_pool(&db_path)?;
    db::with_db(&pool, db::run_migrations).await?;
    info!("Database initialized at {}", db_path.display());

    // 2. Event bus
    let event_bus = Arc::new(TokioBroadcastBus::new(256));

    // 3. Memory
    let memory = Arc::new(InMemoryStore::new());

    // 4. Credentials -- KeyringStore with InMemory fallback
    #[cfg(feature = "keyring")]
    let credentials: Arc<dyn CredentialStore> =
        crate::credential::keyring_store::keyring_or_fallback(&config).await;
    #[cfg(not(feature = "keyring"))]
    let credentials: Arc<dyn CredentialStore> =
        Arc::new(crate::credential::InMemoryCredentialStore::new());

    // 5. Security
    let security = Arc::new(SecurityPolicy::default_policy());

    // 6. Tools
    let tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(crate::tools::system_info::SystemInfoTool::new()))?;
    tool_registry.register(Arc::new(crate::tools::web_search::WebSearchTool::new(
        credentials.clone(),
        config.web_search_timeout_secs,
        config.web_search_max_results,
    )))?;
    tool_registry.register(Arc::new(crate::tools::file_ops::FileReadTool::new(
        security.clone(),
    )))?;
    tool_registry.register(Arc::new(crate::tools::file_ops::FileWriteTool::new(
        security.clone(),
    )))?;
    tool_registry.register(Arc::new(crate::tools::file_ops::FileListTool::new(
        security.clone(),
    )))?;
    tool_registry.register(Arc::new(crate::tools::file_search::FileSearchTool::new(
        config.tool_file_search_max_results,
    )))?;
    tool_registry.register(Arc::new(crate::tools::shell::ShellTool::new(
        security.clone(),
        config.tool_shell_timeout_secs,
    )))?;
    tool_registry.register(Arc::new(crate::tools::process::ProcessTool::new(
        security.clone(),
        config.tool_process_list_limit,
    )))?;
    tool_registry.register(Arc::new(crate::tools::patch::PatchTool::new()))?;

    // 10. User learner (needed before tools that reference it)
    let user_learner = Arc::new(UserLearner::new(pool.clone(), &config));

    // Runtime toggles (mutable via PUT /config)
    let context_injection_enabled = Arc::new(AtomicBool::new(config.context_injection_enabled));
    let self_evolution_enabled = Arc::new(AtomicBool::new(config.self_evolution_enabled));

    // Register LearnTool and SkillProposalTool
    tool_registry.register(Arc::new(crate::tools::learn::LearnTool::new(
        user_learner.clone(),
        self_evolution_enabled.clone(),
    )))?;
    tool_registry.register(Arc::new(
        crate::tools::skill_proposal::SkillProposalTool::new(
            pool.clone(),
            self_evolution_enabled.clone(),
        ),
    ))?;

    // Register MemoryTool
    tool_registry.register(Arc::new(crate::tools::memory_tool::MemoryTool::new(
        memory.clone(),
    )))?;

    // Register ConfigTool
    tool_registry.register(Arc::new(crate::tools::config_tool::ConfigTool::new(
        config.clone(),
        crate::config::default_config_path(),
        context_injection_enabled.clone(),
        self_evolution_enabled.clone(),
    )))?;

    let tools = Arc::new(tool_registry);
    info!("Registered {} tools", tools.len());

    // 7. Session manager
    #[cfg(feature = "ai")]
    let session_manager = Arc::new(SessionManager::new(pool.clone()));

    // 8. Identity (SoulLoader)
    let data_dir = config
        .data_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(crate::config::default_data_dir);

    let identity_dir = config
        .identity_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("identity"));
    let soul_loader = Arc::new(SoulLoader::new(&identity_dir)?);
    info!("Identity loaded from {}", identity_dir.display());

    // 9. Skills (SkillRegistry)
    let skills_dir = config
        .skills_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("skills"));
    let skill_registry = Arc::new(SkillRegistry::new(
        &skills_dir,
        config.skill_max_content_size,
    )?);
    info!("Skills loaded from {}", skills_dir.display());

    info!("User learner initialized");

    // Run consolidation on boot
    if let Err(e) = user_learner
        .consolidate(
            config.learning_archive_threshold,
            config.learning_archive_after_days,
        )
        .await
    {
        tracing::warn!("User learner consolidation failed: {e}");
    }

    // 11. Boot context
    #[cfg(feature = "ai")]
    let boot_context = BootContext::from_system();
    #[cfg(feature = "ai")]
    info!(
        "Boot context: {} {} ({})",
        boot_context.os, boot_context.arch, boot_context.hostname
    );

    // Generate/refresh context summaries on boot
    if config.context_injection_enabled {
        let context_engine = crate::ai::context::ContextEngine::new(
            pool.clone(),
            config.clone(),
            config.context_injection_enabled,
        );
        if let Err(e) = context_engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skill_registry)
            .await
        {
            tracing::warn!("Context summary generation failed: {e}");
        } else {
            info!("Context summaries refreshed");
        }
    }

    // 12a. Reasoning Engine
    #[cfg(feature = "ai")]
    let reasoning_engine = {
        let mut engine = ReasoningEngine::new(config.agent_max_continuations);
        engine.add_strategy(ContinuationStrategy::new(config.agent_max_continuations));
        Arc::new(engine)
    };
    #[cfg(feature = "ai")]
    info!(
        "Reasoning engine initialized with max {} continuations",
        config.agent_max_continuations
    );

    // 12b. ContextBuilder
    #[cfg(feature = "ai")]
    let context_builder = Arc::new(ContextBuilder::new(
        session_manager.clone(),
        memory.clone(),
        soul_loader.clone(),
        user_learner.clone(),
        config.clone(),
    ));
    #[cfg(feature = "ai")]
    info!("Context builder initialized");

    // 12. Provider Registry -- seed built-ins, load from DB
    #[cfg(feature = "ai")]
    let provider_registry = Arc::new(ProviderRegistry::new(pool.clone()));
    #[cfg(feature = "ai")]
    provider_registry.seed_builtin_providers().await?;
    #[cfg(feature = "ai")]
    info!("Provider registry initialized");

    // 12. Agent (may fail if no API key configured — that's OK)
    #[cfg(feature = "ai")]
    let tool_vec = tools.to_vec();
    let agent = match MesoAgent::new(&config, credentials.as_ref(), &tool_vec).await {
        Ok(a) => {
            info!(
                "AI agent initialized with provider '{}'",
                config.provider_type
            );
            Some(Arc::new(a))
        }
        Err(e) => {
            tracing::warn!("AI agent not available: {e}");
            None
        }
    };

    // 13. Channel registry and router
    #[cfg(feature = "channels")]
    let channel_registry = Arc::new(ChannelRegistry::new());
    #[cfg(feature = "channels")]
    let channel_router = {
        #[cfg(feature = "gateway")]
        {
            let router = Arc::new(crate::channels::router::ChannelRouter::new(
                config.channel_router_buffer_size,
            ));
            info!("Channel router initialized");
            Some(router)
        }
        #[cfg(not(feature = "gateway"))]
        {
            None::<Arc<crate::channels::router::ChannelRouter>>
        }
    };
    // Auto-register and connect channels from stored credentials.
    // Channels with valid stored credentials are connected automatically on boot.
    // The listen task is spawned so channels can receive incoming messages.
    #[cfg(feature = "channels-telegram")]
    if matches!(credentials.get("channel:telegram:token").await, Ok(Some(_))) {
        let mut tg_config =
            crate::channels::telegram::config::TelegramConfig::from_app_config(&config);
        if let Ok(Some(ids_str)) = credentials.get("channel:telegram:allowed_chat_ids").await {
            tg_config.allowed_chat_ids = ids_str
                .split(',')
                .filter_map(|s| s.trim().parse::<i64>().ok())
                .collect();
        }
        let tg: Arc<dyn crate::channels::traits::Channel> =
            Arc::new(crate::channels::telegram::TelegramChannel::new(
                tg_config,
                credentials.clone(),
                config.clone(),
            ));
        if let Err(e) = channel_registry.register_or_replace(tg.clone()) {
            tracing::warn!("Failed to register telegram: {e}");
        } else if let Err(e) = tg.connect().await {
            tracing::warn!("Failed to connect telegram: {e}");
        } else {
            #[cfg(feature = "gateway")]
            if let Some(ref router) = channel_router {
                let tx = router.sender();
                let ch = tg.clone();
                tokio::spawn(async move {
                    if let Err(e) = ch.listen(tx).await {
                        tracing::error!("Telegram listen failed: {e}");
                    }
                });
            }
            info!("Telegram auto-connected from stored credentials");
        }
    }

    #[cfg(feature = "channels-slack")]
    if matches!(
        credentials.get("channel:slack:bot_token").await,
        Ok(Some(_))
    ) {
        let sl: Arc<dyn crate::channels::traits::Channel> = Arc::new(
            crate::channels::slack::SlackChannel::new(credentials.clone()),
        );
        if let Err(e) = channel_registry.register_or_replace(sl.clone()) {
            tracing::warn!("Failed to register slack: {e}");
        } else if let Err(e) = sl.connect().await {
            tracing::warn!("Failed to connect slack: {e}");
        } else {
            #[cfg(feature = "gateway")]
            if let Some(ref router) = channel_router {
                let tx = router.sender();
                let ch = sl.clone();
                tokio::spawn(async move {
                    if let Err(e) = ch.listen(tx).await {
                        tracing::error!("Slack listen failed: {e}");
                    }
                });
            }
            info!("Slack auto-connected from stored credentials");
        }
    }

    #[cfg(feature = "channels-discord")]
    if matches!(credentials.get("channel:discord:token").await, Ok(Some(_))) {
        let dc_config = crate::channels::discord::config::DiscordConfig::from_app_config(&config);
        let dc: Arc<dyn crate::channels::traits::Channel> = Arc::new(
            crate::channels::discord::DiscordChannel::new(dc_config, credentials.clone()),
        );
        if let Err(e) = channel_registry.register_or_replace(dc.clone()) {
            tracing::warn!("Failed to register discord: {e}");
        } else if let Err(e) = dc.connect().await {
            tracing::warn!("Failed to connect discord: {e}");
        } else {
            #[cfg(feature = "gateway")]
            if let Some(ref router) = channel_router {
                let tx = router.sender();
                let ch = dc.clone();
                tokio::spawn(async move {
                    if let Err(e) = ch.listen(tx).await {
                        tracing::error!("Discord listen failed: {e}");
                    }
                });
            }
            info!("Discord auto-connected from stored credentials");
        }
    }

    // Register ChannelSendTool (post-Arc, DashMap allows it)
    #[cfg(feature = "channels")]
    tools
        .register(Arc::new(crate::tools::channel_tool::ChannelSendTool::new(
            channel_registry.clone(),
        )))
        .unwrap_or_else(|e| tracing::warn!("Failed to register channel_send tool: {e}"));

    #[cfg(feature = "channels")]
    info!("Channel registry initialized");

    // 14. Scheduler
    #[cfg(feature = "scheduler")]
    let scheduler = {
        let sched = TokioScheduler::new(pool.clone(), event_bus.clone(), &config);
        if let Err(e) = sched.load_from_db().await {
            tracing::warn!("Failed to load scheduler jobs from DB: {e}");
        }
        sched.start().await;
        info!("Scheduler initialized and started");
        Some(sched)
    };

    // Register SchedulerTool (post-Arc, DashMap allows it)
    #[cfg(feature = "scheduler")]
    if let Some(ref sched) = scheduler {
        tools
            .register(Arc::new(crate::tools::scheduler_tool::SchedulerTool::new(
                sched.clone(),
            )))
            .unwrap_or_else(|e| tracing::warn!("Failed to register scheduler tool: {e}"));
    }

    info!("All services initialized");

    Ok(Services {
        config,
        config_path: crate::config::default_config_path(),
        db: pool,
        event_bus,
        memory,
        credentials,
        security,
        tools,
        #[cfg(feature = "ai")]
        session_manager,
        #[cfg(feature = "ai")]
        agent,
        #[cfg(feature = "ai")]
        provider_registry,
        #[cfg(feature = "ai")]
        boot_context,
        #[cfg(feature = "ai")]
        last_used_model: Arc::new(RwLock::new(None)),
        #[cfg(feature = "ai")]
        context_builder,
        #[cfg(feature = "ai")]
        reasoning_engine,
        context_injection_enabled,
        self_evolution_enabled,
        soul_loader,
        skill_registry,
        user_learner,
        #[cfg(feature = "channels")]
        channel_registry,
        #[cfg(feature = "channels")]
        channel_router,
        #[cfg(feature = "scheduler")]
        scheduler,
    })
}

/// Convert Services into gateway AppState.
/// After wrapping in Arc, call `state.wire_scheduler()` to enable payload execution.
#[cfg(feature = "gateway")]
impl From<Services> for AppState {
    fn from(s: Services) -> Self {
        Self {
            config: s.config,
            config_path: s.config_path,
            db: s.db,
            event_bus: s.event_bus,
            memory: s.memory,
            credentials: s.credentials,
            security: s.security,
            tools: s.tools,
            #[cfg(feature = "ai")]
            session_manager: s.session_manager,
            #[cfg(feature = "ai")]
            agent: s.agent,
            #[cfg(feature = "ai")]
            provider_registry: s.provider_registry,
            #[cfg(feature = "ai")]
            boot_context: s.boot_context,
            #[cfg(feature = "ai")]
            last_used_model: s.last_used_model,
            #[cfg(feature = "ai")]
            context_builder: s.context_builder,
            #[cfg(feature = "ai")]
            reasoning_engine: s.reasoning_engine,
            context_injection_enabled: s.context_injection_enabled,
            self_evolution_enabled: s.self_evolution_enabled,
            soul_loader: s.soul_loader,
            skill_registry: s.skill_registry,
            user_learner: s.user_learner,
            #[cfg(feature = "channels")]
            channel_registry: s.channel_registry,
            #[cfg(feature = "channels")]
            channel_router: s.channel_router,
            #[cfg(feature = "scheduler")]
            scheduler: s.scheduler,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(dir: &tempfile::TempDir) -> AppConfig {
        AppConfig {
            db_path: Some(dir.path().join("test.db").to_string_lossy().into()),
            identity_dir: Some(dir.path().join("identity").to_string_lossy().into()),
            skills_dir: Some(dir.path().join("skills").to_string_lossy().into()),
            ..Default::default()
        }
    }

    // 5.1 — init services with default config
    #[tokio::test]
    async fn init_services_default_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await;
        assert!(services.is_ok());
    }

    // 5.2 — init services creates DB file
    #[tokio::test]
    async fn init_services_creates_db() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let config = test_config(&dir);
        init_services(config).await.unwrap();
        assert!(db_path.exists());
    }

    // 5.3 — init services runs migrations
    #[tokio::test]
    async fn init_services_runs_migrations() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();

        // Verify sessions table exists
        let result = db::with_db(&services.db, |conn| {
            conn.execute("SELECT 1 FROM sessions LIMIT 0", [])
                .map(|_| ())
                .map_err(crate::MesoError::from)
        })
        .await;
        assert!(result.is_ok());
    }

    // 5.4 — init services registers tools (base 13 + feature-gated)
    #[tokio::test]
    async fn init_services_builds_tools() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        let mut expected = 13; // base tools + memory + config
        #[cfg(feature = "channels")]
        {
            expected += 1; // channel_send
        }
        #[cfg(feature = "scheduler")]
        {
            expected += 1; // scheduler
        }
        assert_eq!(services.tools.len(), expected);
    }

    // WS.12 — WebSearchTool registered with credential store access
    #[tokio::test]
    async fn web_search_tool_registered_with_credentials() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        let ws = services.tools.get("web_search");
        assert!(ws.is_some(), "web_search tool must be registered");
    }

    // 5.5 — agent is None when no API key is configured
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn init_services_agent_none_without_key() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&dir);
        // Use a provider name that definitely won't have a key in the system keyring
        config.provider_name = "nonexistent-test-provider".into();
        let services = init_services(config).await.unwrap();
        assert!(
            services.agent.is_none(),
            "Agent should be None when no API key is configured"
        );
    }

    // 8.11.20 — init_services creates ReasoningEngine with ContinuationStrategy
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn init_services_creates_reasoning_engine() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        // ReasoningEngine is created and accessible
        // It should have at least the ContinuationStrategy
        assert!(
            Arc::strong_count(&services.reasoning_engine) >= 1,
            "reasoning_engine should be initialized"
        );
    }

    // 5.6 — Services is Send + Sync
    #[allow(dead_code)]
    const _: () = {
        fn assert_send_sync<T: Send + Sync>() {}
        fn check() {
            assert_send_sync::<Services>();
        }
    };

    // Phase 4 boot tests
    #[tokio::test]
    async fn boot_initializes_soul_loader() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        let identity = services.soul_loader.get().await;
        assert_eq!(identity.files.len(), 3);
    }

    #[tokio::test]
    async fn boot_initializes_skill_registry() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        let skills = services.skill_registry.list().await;
        assert_eq!(skills.len(), 2); // 2 bundled
    }

    #[tokio::test]
    async fn boot_initializes_user_learner() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        let count = services.user_learner.count().await.unwrap();
        assert_eq!(count, 0);
    }

    // 8.6.1.22 — After init_services with scheduler feature, scheduler is present
    #[cfg(feature = "scheduler")]
    #[tokio::test]
    async fn boot_wires_scheduler_to_appstate() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        assert!(
            services.scheduler.is_some(),
            "Scheduler should be wired in Services after init_services"
        );
    }

    // 8.7.11 — With channels feature, boot creates a channel_router
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    async fn boot_creates_channel_router() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        assert!(
            services.channel_router.is_some(),
            "Channel router should be created when both channels and gateway features are enabled"
        );
    }

    // 8.7.12 — Channel router exists but no channels are started when none configured
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    async fn boot_router_not_started_when_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&dir);
        config.channels_enabled = vec![]; // no channels configured
        let services = init_services(config).await.unwrap();
        // Router exists
        assert!(services.channel_router.is_some());
        // Channel registry is empty (no channels registered)
        assert!(
            services.channel_registry.list().is_empty(),
            "No channels should be registered when channels_enabled is empty"
        );
    }

    #[cfg(feature = "gateway")]
    #[tokio::test]
    async fn boot_services_to_appstate_includes_phase4() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        let state: AppState = services.into();
        // Verify Phase 4 fields are accessible
        let identity = state.soul_loader.get().await;
        assert_eq!(identity.meta.name, "MesoClaw");
        let skills = state.skill_registry.list().await;
        assert_eq!(skills.len(), 2);
    }
}
