use std::path::PathBuf;
use std::sync::Arc;

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

#[cfg(feature = "ai")]
use crate::ai::{agent::MesoAgent, provider_registry::ProviderRegistry, session::SessionManager};

#[cfg(feature = "gateway")]
use crate::gateway::state::AppState;

/// Initialized services bundle for use without the gateway feature.
pub struct Services {
    pub config: Arc<AppConfig>,
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
    pub soul_loader: Arc<SoulLoader>,
    pub skill_registry: Arc<SkillRegistry>,
    pub user_learner: Arc<UserLearner>,
    #[cfg(feature = "channels")]
    pub channel_registry: Arc<ChannelRegistry>,
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
    tool_registry
        .register(Arc::new(crate::tools::system_info::SystemInfoTool::new()))
        .unwrap();
    tool_registry
        .register(Arc::new(crate::tools::web_search::WebSearchTool::new(
            credentials.clone(),
            config.web_search_timeout_secs,
            config.web_search_max_results,
        )))
        .unwrap();
    tool_registry
        .register(Arc::new(crate::tools::file_ops::FileReadTool::new(
            security.clone(),
        )))
        .unwrap();
    tool_registry
        .register(Arc::new(crate::tools::file_ops::FileWriteTool::new(
            security.clone(),
        )))
        .unwrap();
    tool_registry
        .register(Arc::new(crate::tools::file_ops::FileListTool::new(
            security.clone(),
        )))
        .unwrap();
    tool_registry
        .register(Arc::new(crate::tools::file_search::FileSearchTool::new(
            config.tool_file_search_max_results,
        )))
        .unwrap();
    tool_registry
        .register(Arc::new(crate::tools::shell::ShellTool::new(
            security.clone(),
            config.tool_shell_timeout_secs,
        )))
        .unwrap();
    tool_registry
        .register(Arc::new(crate::tools::process::ProcessTool::new(
            security.clone(),
            config.tool_process_list_limit,
        )))
        .unwrap();
    tool_registry
        .register(Arc::new(crate::tools::patch::PatchTool::new()))
        .unwrap();
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

    // 10. User learner
    let user_learner = Arc::new(UserLearner::new(pool.clone(), &config));
    info!("User learner initialized");

    // 11. Provider Registry -- seed built-ins, load from DB
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

    // 13. Channel registry
    #[cfg(feature = "channels")]
    let channel_registry = Arc::new(ChannelRegistry::new());
    #[cfg(feature = "channels")]
    info!("Channel registry initialized");

    info!("All services initialized");

    Ok(Services {
        config,
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
        soul_loader,
        skill_registry,
        user_learner,
        #[cfg(feature = "channels")]
        channel_registry,
    })
}

/// Convert Services into gateway AppState.
#[cfg(feature = "gateway")]
impl From<Services> for AppState {
    fn from(s: Services) -> Self {
        Self {
            config: s.config,
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
            soul_loader: s.soul_loader,
            skill_registry: s.skill_registry,
            user_learner: s.user_learner,
            #[cfg(feature = "channels")]
            channel_registry: s.channel_registry,
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

    // 5.4 — init services registers all 9 tools
    #[tokio::test]
    async fn init_services_builds_tools() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        assert_eq!(services.tools.len(), 9);
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
        let config = test_config(&dir);
        let services = init_services(config).await.unwrap();
        assert!(
            services.agent.is_none(),
            "Agent should be None when no API key is configured"
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
