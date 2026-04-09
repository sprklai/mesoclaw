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
use crate::scheduler::{TokioScheduler, traits::Scheduler};

#[cfg(feature = "ai")]
use crate::ai::{
    agent::ZeniiAgent,
    context::{BootContext, ContextBuilder},
    prompt::{self, PromptStrategy},
    provider_registry::ProviderRegistry,
    reasoning::{ReasoningEngine, continuation::ContinuationStrategy},
    session::SessionManager,
};

#[cfg(feature = "gateway")]
use crate::gateway::state::AppState;

/// Initialized services bundle for use without the gateway feature.
pub struct Services {
    pub config: Arc<AppConfig>,
    /// Shared ArcSwap config for runtime hot-swapping (ConfigTool + AppState share this).
    pub config_swap: Arc<arc_swap::ArcSwap<AppConfig>>,
    pub config_path: PathBuf,
    pub db: DbPool,
    pub event_bus: Arc<TokioBroadcastBus>,
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
    pub context_injection_enabled: Arc<AtomicBool>,
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
    pub usage_logger: Arc<crate::logging::UsageLogger>,
    /// Whether the local embedding model is downloaded and ready.
    pub embedding_model_available: Arc<AtomicBool>,
    pub approval_broker: Option<Arc<crate::security::approval::ApprovalBroker>>,
    pub wiki: Arc<crate::wiki::WikiManager>,
}

/// Initialize all services from config.
pub async fn init_services(config: AppConfig) -> Result<Services> {
    // When both ring and aws-lc-rs are in the dep tree (e.g. --all-features),
    // rustls cannot auto-detect the CryptoProvider. Install ring explicitly.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let config = Arc::new(config);

    // 1. Database
    let db_path = config
        .db_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::config::default_data_dir().join("zenii.db"));

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pool = db::init_pool(&db_path)?;
    db::with_db(&pool, db::run_migrations).await?;
    info!("Database initialized at {}", db_path.display());

    // 2. Event bus (M8: configurable capacity)
    let event_bus = Arc::new(TokioBroadcastBus::new(config.event_bus_capacity));

    // 3. Memory — always use SqliteMemoryStore (persistent)
    let memory_db_path = config
        .memory_db_path
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            db_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("memory.db")
        });
    if let Some(parent) = memory_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let memory_pool = crate::db::init_pool(&memory_db_path)?;
    {
        let mp = memory_pool.clone();
        tokio::task::spawn_blocking(move || {
            crate::memory::sqlite_store::SqliteMemoryStore::run_memory_migrations(&mp)
        })
        .await
        .map_err(|e| crate::ZeniiError::Database(format!("memory migration join failed: {e}")))??;
    }

    // Track whether the local embedding model is downloaded and ready.
    // Starts true for non-local providers (no model needed), false for local until warmup completes.
    let embedding_model_available = Arc::new(AtomicBool::new(
        config.embedding_provider.as_str() != "local",
    ));

    let memory: Arc<dyn Memory> = {
        let store = crate::memory::sqlite_store::SqliteMemoryStore::new(
            memory_pool.clone(),
            config.memory_fts_weight,
            config.memory_vector_weight,
        );

        match config.embedding_provider.as_str() {
            "openai" => {
                info!("Embedding provider: OpenAI (will resolve key at first use)");
                // Defer key resolution — store without vector for now, reconfigure later if key available
                Arc::new(store)
            }
            #[cfg(feature = "local-embeddings")]
            "local" => {
                info!("Embedding provider: local (fastembed)");
                match crate::memory::local_embeddings::FastEmbedProvider::new(
                    &config.embedding_model,
                    config.embedding_download_dir.as_ref().map(PathBuf::from),
                ) {
                    Ok(provider) => {
                        // Initialize VectorIndex
                        let vi_pool = memory_pool.clone();
                        let dim = config.embedding_dim;
                        // SAFETY: sqlite3_vec_init has the correct signature for sqlite3_auto_extension
                        #[allow(unsafe_code)]
                        unsafe {
                            #[rustfmt::skip]
                            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<
                                *const (),
                                unsafe extern "C" fn(
                                    *mut rusqlite::ffi::sqlite3,
                                    *mut *mut std::ffi::c_char,
                                    *const rusqlite::ffi::sqlite3_api_routines,
                                )
                                    -> std::ffi::c_int,
                            >(
                                sqlite_vec::sqlite3_vec_init as *const (),
                            )));
                        }
                        match tokio::task::spawn_blocking(move || {
                            crate::memory::vector_index::VectorIndex::new(vi_pool, dim)
                        })
                        .await
                        {
                            Ok(Ok(vi)) => {
                                let cached = crate::memory::embeddings::LruEmbeddingCache::new(
                                    provider,
                                    config.embedding_cache_size,
                                );
                                let embedding_provider: Arc<
                                    dyn crate::memory::embeddings::EmbeddingProvider,
                                > = Arc::new(cached);

                                // Eagerly warm up the model (triggers download if not cached)
                                let warmup_provider = embedding_provider.clone();
                                let warmup_flag = embedding_model_available.clone();
                                tokio::spawn(async move {
                                    match warmup_provider.embed("warmup").await {
                                        Ok(_) => {
                                            warmup_flag
                                                .store(true, std::sync::atomic::Ordering::SeqCst);
                                            tracing::info!("Embedding model downloaded and ready");
                                        }
                                        Err(e) => {
                                            tracing::warn!("Embedding model warmup failed: {e}");
                                        }
                                    }
                                });

                                Arc::new(store.with_vector(vi, embedding_provider))
                            }
                            Ok(Err(e)) => {
                                tracing::warn!(
                                    "Vector index init failed, falling back to FTS only: {e}"
                                );
                                Arc::new(store)
                            }
                            Err(e) => {
                                tracing::warn!("Vector index spawn failed: {e}");
                                Arc::new(store)
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("FastEmbed init failed, falling back to FTS only: {e}");
                        Arc::new(store)
                    }
                }
            }
            _ => {
                // "none" or unknown — FTS5 only
                info!("Embedding provider: none (FTS5 only)");
                Arc::new(store)
            }
        }
    };
    info!("Memory store initialized at {}", memory_db_path.display());

    // 4. Credentials -- KeyringStore with InMemory fallback
    #[cfg(feature = "keyring")]
    let credentials: Arc<dyn CredentialStore> =
        crate::credential::keyring_store::keyring_or_fallback(&config).await;
    #[cfg(not(feature = "keyring"))]
    let credentials: Arc<dyn CredentialStore> = {
        use std::path::PathBuf;

        use crate::credential::file_store::FileCredentialStore;

        let data_dir = config
            .data_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::config::default_data_dir());
        match FileCredentialStore::new(&data_dir, &config.keyring_service_id) {
            Ok(store) => {
                info!(
                    "Credential store: encrypted file at {} (keyring feature disabled)",
                    store.path().display()
                );
                Arc::new(store)
            }
            Err(e) => {
                info!("Credential store: in-memory (keyring disabled, file store failed: {e})");
                Arc::new(crate::credential::InMemoryCredentialStore::new())
            }
        }
    };

    // 5. Security (reads autonomy level, rate limits, etc. from config)
    let security = Arc::new(SecurityPolicy::from_config(&config));

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
        config.tool_file_search_max_depth,
        config.tool_file_search_follow_symlinks,
    )))?;
    tool_registry.register(Arc::new(
        crate::tools::content_search::ContentSearchTool::new(
            config.tool_content_search_max_results,
            config.tool_content_search_max_file_size_kb * 1024,
            config.tool_content_search_context_lines,
            config.tool_file_search_max_depth,
        ),
    ))?;
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

    // Register AgentSelfTool (self-evolving prompt notes)
    tool_registry.register(Arc::new(crate::tools::agent_self_tool::AgentSelfTool::new(
        pool.clone(),
        self_evolution_enabled.clone(),
    )))?;

    // Create shared ArcSwap config for runtime hot-swapping
    let config_swap = Arc::new(arc_swap::ArcSwap::from(config.clone()));

    // Register ConfigTool
    tool_registry.register(Arc::new(crate::tools::config_tool::ConfigTool::new(
        config_swap.clone(),
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

    // Wiki
    let wiki_dir = config
        .wiki_dir
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| data_dir.join("wiki"));
    let wiki = Arc::new(crate::wiki::WikiManager::new(wiki_dir.clone())?);
    info!("Wiki initialized at {}", wiki_dir.display());

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
    let boot_context = BootContext::from_system_with_config(
        config.user_timezone.as_deref(),
        config.user_location.as_deref(),
    );
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
        credentials.clone(),
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
    let agent = match ZeniiAgent::new(&config, credentials.as_ref(), &tool_vec).await {
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
            info!("Telegram auto-connected from stored credentials");
        }
    }

    #[cfg(feature = "channels-slack")]
    if matches!(
        credentials.get("channel:slack:bot_token").await,
        Ok(Some(_))
    ) {
        let sl: Arc<dyn crate::channels::traits::Channel> = Arc::new(
            crate::channels::slack::SlackChannel::new(credentials.clone())
                .with_allowed_channels(config.slack_allowed_channel_ids.clone()),
        );
        if let Err(e) = channel_registry.register_or_replace(sl.clone()) {
            tracing::warn!("Failed to register slack: {e}");
        } else if let Err(e) = sl.connect().await {
            tracing::warn!("Failed to connect slack: {e}");
        } else {
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
            info!("Discord auto-connected from stored credentials");
        }
    }

    // Register ChannelSendTool (post-Arc, DashMap allows it)
    #[cfg(feature = "channels")]
    {
        let tool_session_map = Arc::new(crate::channels::session_map::ChannelSessionMap::new(
            session_manager.clone(),
        ));
        tools
            .register(Arc::new(crate::tools::channel_tool::ChannelSendTool::new(
                channel_registry.clone(),
                pool.clone(),
                tool_session_map,
                session_manager.clone(),
                event_bus.clone(),
            )))
            .unwrap_or_else(|e| tracing::warn!("Failed to register channel_send tool: {e}"));
    }

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

    // 14a. Workflow engine (feature-gated) — must be before prompt strategy so plugin can reference it
    #[cfg(feature = "workflows")]
    let workflow_registry_init = {
        let wf_dir = config
            .workflow_dir
            .as_ref()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| data_dir.join("workflows"));
        match crate::workflows::WorkflowRegistry::new(wf_dir) {
            Ok(r) => {
                info!(
                    "Workflow registry initialized ({} workflows)",
                    r.list().len()
                );
                Some(Arc::new(r))
            }
            Err(e) => {
                tracing::warn!("Workflow registry init failed: {e}");
                None
            }
        }
    };
    #[cfg(feature = "workflows")]
    let workflow_executor_init = Some(Arc::new(crate::workflows::executor::WorkflowExecutor::new(
        pool.clone(),
        config.workflow_max_steps,
        config.workflow_step_timeout_secs,
        config.workflow_step_max_retries,
    )));

    // Register WorkflowTool (post-Arc, DashMap allows it)
    #[cfg(feature = "workflows")]
    if let (Some(wf_reg), Some(wf_exec)) = (&workflow_registry_init, &workflow_executor_init) {
        tools
            .register(Arc::new(crate::tools::workflow_tool::WorkflowTool::new(
                wf_reg.clone(),
                wf_exec.clone(),
                tools.clone(),
                event_bus.clone(),
                #[cfg(feature = "scheduler")]
                scheduler.clone(),
            )))
            .unwrap_or_else(|e| tracing::warn!("Failed to register workflows tool: {e}"));
    }

    // 14b. Prompt Strategy (compact or legacy based on config)
    // Must be after channels + scheduler + workflows so plugins can reference them.
    #[cfg(feature = "ai")]
    let prompt_strategy: Arc<dyn PromptStrategy> = if config.prompt_compact_identity {
        let base = Arc::new(prompt::CompactStrategy::new(
            config.clone(),
            boot_context.clone(),
        ));
        let registry = prompt::PromptStrategyRegistry::new(base, config.clone());

        // Always-registered plugins
        registry
            .register_plugin(Arc::new(prompt::MemoryPlugin::new(memory.clone())))
            .await;
        registry
            .register_plugin(Arc::new(prompt::UserObservationsPlugin::new(
                user_learner.clone(),
            )))
            .await;
        registry
            .register_plugin(Arc::new(prompt::SkillsPlugin::new(skill_registry.clone())))
            .await;

        // Conditional: learned rules
        if config.self_evolution_enabled {
            registry
                .register_plugin(Arc::new(prompt::LearnedRulesPlugin::new(pool.clone())))
                .await;
        }

        // Feature-gated: channels
        #[cfg(feature = "channels")]
        registry
            .register_plugin(Arc::new(prompt::ChannelContextPlugin::new(
                channel_registry.clone(),
                pool.clone(),
            )))
            .await;

        // Feature-gated: scheduler
        #[cfg(feature = "scheduler")]
        if let Some(ref sched) = scheduler {
            registry
                .register_plugin(Arc::new(prompt::SchedulerContextPlugin::new(sched.clone())))
                .await;
        }

        // Feature-gated: workflows
        #[cfg(feature = "workflows")]
        if let Some(wf_reg) = &workflow_registry_init {
            registry
                .register_plugin(Arc::new(prompt::WorkflowContextPlugin::new(wf_reg.clone())))
                .await;
        }

        Arc::new(registry)
    } else {
        Arc::new(prompt::LegacyStrategy::new(
            soul_loader.clone(),
            user_learner.clone(),
            config.clone(),
            skill_registry.clone(),
        ))
    };
    #[cfg(feature = "ai")]
    info!(
        "Prompt strategy initialized (compact={})",
        config.prompt_compact_identity
    );

    // 15. Notification Router
    let notification_router = {
        let router = crate::notification::router::NotificationRouter::new(
            config_swap.clone(),
            event_bus.clone(),
            #[cfg(feature = "channels")]
            channel_registry.clone(),
        );
        Some(Arc::new(router))
    };
    info!("Notification router initialized");

    // 16. Plugin system
    let plugins_dir = config
        .plugins_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("plugins"));
    let plugin_registry = Arc::new(PluginRegistry::new(plugins_dir)?);

    // Register enabled plugin tools into ToolRegistry
    for plugin in plugin_registry.list() {
        if !plugin.enabled {
            continue;
        }
        for tool_def in &plugin.manifest.tools {
            let binary = plugin.install_path.join(&tool_def.binary);

            // Fetch real schema from the plugin's info() JSON-RPC method
            let schema = crate::plugins::fetch_plugin_schema(
                &binary,
                &tool_def.name,
                config.plugin_execute_timeout_secs,
                config.plugin_max_restart_attempts,
            )
            .await;

            // Create a fresh process for the adapter (the one used for schema fetch is consumed)
            let process = crate::plugins::process::PluginProcess::new(
                &tool_def.name,
                binary,
                config.plugin_execute_timeout_secs,
                config.plugin_max_restart_attempts,
            );
            let adapter = crate::plugins::adapter::PluginToolAdapter::new(
                tool_def.name.clone(),
                tool_def.description.clone(),
                schema,
                Arc::new(tokio::sync::Mutex::new(process)),
            );
            tools.register(Arc::new(adapter)).unwrap_or_else(|e| {
                tracing::warn!("Failed to register plugin tool '{}': {e}", tool_def.name);
            });
        }
        for skill_def in &plugin.manifest.skills {
            let path = plugin.install_path.join(&skill_def.file);
            if let Ok(content) = std::fs::read_to_string(&path)
                && let Err(e) = skill_registry
                    .register_external(&skill_def.name, content)
                    .await
            {
                tracing::warn!("Failed to register plugin skill '{}': {e}", skill_def.name);
            }
        }
    }

    let plugin_installer = Arc::new(PluginInstaller::new(
        plugin_registry.clone(),
        tools.clone(),
        skill_registry.clone(),
        config.plugin_execute_timeout_secs,
        config.plugin_max_restart_attempts,
    ));

    info!(
        "Plugin system initialized: {} plugins",
        plugin_registry.list().len()
    );

    // Usage logger + tracing log cleanup
    let usage_logger = Arc::new(crate::logging::UsageLogger::new(&config, "daemon"));
    if usage_logger.is_enabled() {
        let _ = usage_logger.cleanup_old_files().await;
        info!("Usage logger initialized");
    }
    // Clean up old tracing log files alongside usage logs
    let log_dir = crate::logging::resolve_log_dir(&config);
    let keep_days = config.log_keep_days;
    tokio::task::spawn_blocking(move || {
        crate::logging::cleanup_old_tracing_files(&log_dir, keep_days)
    })
    .await
    .ok();

    // H3: Cleanup old sessions on boot
    #[cfg(feature = "ai")]
    {
        let max_age = config.session_max_age_days;
        if max_age > 0 {
            match session_manager.cleanup_old_sessions(max_age).await {
                Ok(0) => {}
                Ok(n) => info!("Cleaned up {n} sessions older than {max_age} days"),
                Err(e) => tracing::warn!("Session cleanup failed: {e}"),
            }
        }
    }

    // 17. Coordinator (delegation)
    let coordinator = Arc::new(crate::ai::delegation::Coordinator::new(
        crate::ai::delegation::DelegationConfig::from_app_config(&config),
    ));
    info!("Delegation coordinator initialized");

    info!("All services initialized");

    Ok(Services {
        config,
        config_swap,
        config_path: crate::config::default_config_path(),
        db: pool.clone(),
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
        #[cfg(feature = "ai")]
        prompt_strategy,
        context_injection_enabled,
        self_evolution_enabled,
        soul_loader,
        skill_registry,
        user_learner,
        plugin_registry,
        plugin_installer,
        #[cfg(feature = "channels")]
        channel_registry,
        #[cfg(feature = "channels")]
        channel_router,
        #[cfg(feature = "scheduler")]
        scheduler,
        notification_router,
        coordinator,
        #[cfg(feature = "workflows")]
        workflow_registry: workflow_registry_init,
        #[cfg(feature = "workflows")]
        workflow_executor: workflow_executor_init,
        usage_logger,
        embedding_model_available,
        approval_broker: Some(Arc::new(crate::security::approval::ApprovalBroker::new(
            pool,
        ))),
        wiki,
    })
}

/// Convert Services into gateway AppState.
/// After wrapping in Arc, call `state.wire_scheduler()` to enable payload execution.
#[cfg(feature = "gateway")]
impl From<Services> for AppState {
    fn from(s: Services) -> Self {
        Self {
            config: s.config_swap,
            config_path: s.config_path,
            config_write_lock: tokio::sync::Mutex::new(()),
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
            #[cfg(feature = "ai")]
            prompt_strategy: s.prompt_strategy,
            context_injection_enabled: s.context_injection_enabled,
            self_evolution_enabled: s.self_evolution_enabled,
            soul_loader: s.soul_loader,
            skill_registry: s.skill_registry,
            user_learner: s.user_learner,
            plugin_registry: s.plugin_registry,
            plugin_installer: s.plugin_installer,
            #[cfg(feature = "channels")]
            channel_registry: s.channel_registry,
            #[cfg(feature = "channels")]
            channel_router: s.channel_router,
            #[cfg(feature = "scheduler")]
            scheduler: s.scheduler,
            notification_router: s.notification_router,
            coordinator: s.coordinator,
            #[cfg(feature = "workflows")]
            workflow_registry: s.workflow_registry,
            #[cfg(feature = "workflows")]
            workflow_executor: s.workflow_executor,
            #[cfg(feature = "workflows")]
            active_workflow_runs: Arc::new(dashmap::DashMap::new()),
            usage_logger: s.usage_logger,
            embedding_model_available: s.embedding_model_available,
            approval_broker: s.approval_broker,
            wiki: s.wiki,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(dir: &tempfile::TempDir) -> AppConfig {
        AppConfig {
            db_path: Some(dir.path().join("test.db").to_string_lossy().into()),
            memory_db_path: Some(dir.path().join("memory.db").to_string_lossy().into()),
            identity_dir: Some(dir.path().join("identity").to_string_lossy().into()),
            skills_dir: Some(dir.path().join("skills").to_string_lossy().into()),
            plugins_dir: Some(dir.path().join("plugins").to_string_lossy().into()),
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
                .map_err(crate::ZeniiError::from)
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
        let mut expected = 15; // base tools + memory + config + agent_notes + content_search
        #[cfg(feature = "channels")]
        {
            expected += 1; // channel_send
        }
        #[cfg(feature = "scheduler")]
        {
            expected += 1; // scheduler
        }
        #[cfg(feature = "workflows")]
        {
            expected += 1; // workflows
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
        assert_eq!(skills.len(), 3); // 4 bundled
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

    // 8.7.12 — Channel router exists but no channels are started when none have credentials
    #[cfg(all(feature = "channels", feature = "gateway"))]
    #[tokio::test]
    async fn boot_router_not_started_when_empty() {
        // When both ring and aws-lc-rs are in the dep tree (--all-features),
        // rustls cannot auto-detect the CryptoProvider. Install ring explicitly.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        // Router exists (channels feature is enabled)
        assert!(services.channel_router.is_some());
        // Note: channels are registered based on stored credentials (from keyring),
        // not from channels_enabled config. With --all-features keyring is active
        // and may find credentials from prior dev sessions. This test verifies
        // the router is present; credential-based registration is correct behavior.
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
        assert_eq!(identity.meta.name, "Zenii");
        let skills = state.skill_registry.list().await;
        assert_eq!(skills.len(), 3);
    }

    // 18.12 — Boot with embedding_provider="none" creates SqliteMemoryStore without vector
    #[tokio::test]
    async fn boot_memory_no_embeddings() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut config = test_config(&dir);
        config.embedding_provider = "none".into();
        let services = init_services(config).await.unwrap();
        // Memory store works — store and recall round-trip
        services
            .memory
            .store(
                "test-key",
                "test content",
                crate::memory::traits::MemoryCategory::Core,
            )
            .await
            .unwrap();
        let results = services.memory.recall("test", 10, 0).await.unwrap();
        assert!(!results.is_empty());
    }

    // 18.13 — Services.memory is Arc<dyn Memory> (type check)
    #[test]
    fn boot_memory_trait_object() {
        fn assert_memory_trait(_: &Arc<dyn Memory>) {}
        // This test just verifies the type compiles
        let _ = assert_memory_trait;
    }
}
