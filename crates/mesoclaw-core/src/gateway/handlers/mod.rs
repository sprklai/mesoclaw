pub mod chat;
pub mod config;
pub mod health;
pub mod identity;
pub mod memory;
pub mod messages;
pub mod models;
pub mod providers;
pub mod sessions;
pub mod skills;
pub mod system;
pub mod tools;
pub mod user;
pub mod ws;

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Arc;

    use crate::config::AppConfig;
    use crate::credential::InMemoryCredentialStore;
    use crate::gateway::state::AppState;
    use crate::identity::SoulLoader;
    use crate::memory::in_memory_store::InMemoryStore;
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
            gateway_cors_origins: vec!["*".into()],
            ..Default::default()
        };

        let identity_dir = dir.path().join("identity");
        let skills_dir = dir.path().join("skills");

        let soul_loader = Arc::new(SoulLoader::new(&identity_dir).unwrap());
        let skill_registry =
            Arc::new(SkillRegistry::new(&skills_dir, config.skill_max_content_size).unwrap());
        let user_learner = Arc::new(UserLearner::new(pool.clone(), &config));

        let state = Arc::new(AppState {
            config: Arc::new(config),
            db: pool.clone(),
            event_bus: Arc::new(crate::event_bus::TokioBroadcastBus::new(16)),
            memory: Arc::new(InMemoryStore::new()),
            credentials: Arc::new(InMemoryCredentialStore::new()),
            security: Arc::new(SecurityPolicy::default_policy()),
            tools: Arc::new(crate::tools::ToolRegistry::new()),
            session_manager: Arc::new(crate::ai::session::SessionManager::new(pool)),
            agent: None,
            soul_loader,
            skill_registry,
            user_learner,
        });
        (dir, state)
    }
}
