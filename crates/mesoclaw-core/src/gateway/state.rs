use std::sync::Arc;

use crate::config::AppConfig;
use crate::credential::CredentialStore;
use crate::db::DbPool;
use crate::event_bus::EventBus;
use crate::identity::SoulLoader;
use crate::memory::traits::Memory;
use crate::security::policy::SecurityPolicy;
use crate::skills::SkillRegistry;
use crate::tools::traits::Tool;
use crate::user::UserLearner;

#[cfg(feature = "ai")]
use crate::ai::agent::MesoAgent;
#[cfg(feature = "ai")]
use crate::ai::session::SessionManager;

pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: DbPool,
    pub event_bus: Arc<dyn EventBus>,
    pub memory: Arc<dyn Memory>,
    pub credentials: Arc<dyn CredentialStore>,
    pub security: Arc<SecurityPolicy>,
    pub tools: Vec<Arc<dyn Tool>>,
    #[cfg(feature = "ai")]
    pub session_manager: Arc<SessionManager>,
    #[cfg(feature = "ai")]
    pub agent: Option<Arc<MesoAgent>>,
    pub soul_loader: Arc<SoulLoader>,
    pub skill_registry: Arc<SkillRegistry>,
    pub user_learner: Arc<UserLearner>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppState>();
    }
}
