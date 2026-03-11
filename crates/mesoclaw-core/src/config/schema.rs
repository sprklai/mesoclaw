use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AppConfig {
    pub gateway_host: String,
    pub gateway_port: u16,
    pub log_level: String,
    pub data_dir: Option<String>,
    pub db_path: Option<String>,
    pub memory_db_path: Option<String>,
    pub identity_name: String,
    pub identity_description: String,
    #[serde(alias = "default_provider")]
    pub provider_name: String,
    pub provider_type: String,
    pub provider_base_url: Option<String>,
    #[serde(alias = "default_model")]
    pub provider_model_id: String,
    pub provider_api_key_env: Option<String>,
    pub security_autonomy_level: String,
    pub max_tool_retries: u32,

    // Phase 2: Memory system
    pub memory_fts_weight: f32,
    pub memory_vector_weight: f32,
    pub memory_default_limit: usize,
    pub embedding_dim: usize,
    pub embedding_cache_size: usize,

    // Phase 2: Security
    pub security_rate_limit_max: u32,
    pub security_rate_limit_window_secs: u64,
    pub security_audit_log_capacity: usize,

    // Phase 2: Tools
    pub tool_shell_timeout_secs: u64,
    pub tool_file_read_max_lines: usize,
    pub tool_file_search_max_results: usize,
    pub tool_process_list_limit: usize,

    // Web Search
    pub web_search_timeout_secs: u64,
    pub web_search_max_results: usize,

    // Phase 3: Gateway
    pub gateway_auth_token: Option<String>,
    pub ws_max_connections: usize,
    pub gateway_cors_origins: Vec<String>,

    // Phase 3: Agent
    pub agent_max_turns: usize,
    pub agent_max_tokens: usize,
    pub agent_system_prompt: Option<String>,

    // Phase 4: Identity
    pub identity_dir: Option<String>,

    // Phase 4: Skills
    pub skills_dir: Option<String>,
    pub skill_max_content_size: usize,

    // Phase 8: Credentials
    pub keyring_service_id: String,

    // Phase 8: Channels
    pub channel_tool_policy: HashMap<String, Vec<String>>,
    pub channels_enabled: Vec<String>,
    pub telegram_polling_timeout_secs: u32,
    pub telegram_dm_policy: String,
    pub telegram_retry_min_ms: u64,
    pub telegram_retry_max_ms: u64,
    pub telegram_require_group_mention: bool,
    pub telegram_status_refresh_secs: u32,
    pub telegram_show_tool_status: bool,
    pub slack_allowed_channel_ids: Vec<String>,
    pub discord_allowed_guild_ids: Vec<u64>,
    pub discord_allowed_channel_ids: Vec<u64>,
    pub channel_router_buffer_size: usize,
    pub channel_reconnect_max_attempts: u32,

    // Phase 4: User Learning
    pub learning_enabled: bool,
    pub learning_denied_categories: Vec<String>,
    pub learning_max_observations: usize,
    pub learning_observation_ttl_days: u32,
    pub learning_min_confidence: f32,

    // Phase 8: Context Injection
    pub context_injection_enabled: bool,
    pub context_summary_model_id: String,
    pub context_summary_provider_id: String,
    pub context_reinject_gap_minutes: u32,
    pub context_reinject_message_count: u32,

    // Phase 8: Context Management (Step 15.3)
    pub context_strategy: String,
    pub context_max_history_messages: usize,
    pub context_max_memory_results: usize,
    pub context_auto_extract: bool,
    pub context_extract_interval: usize,
    pub context_summary_model: String,

    // Phase 8: Scheduler
    pub scheduler_tick_interval_secs: u64,
    pub scheduler_stuck_threshold_secs: u64,
    pub scheduler_error_backoff_secs: Vec<u64>,
    pub scheduler_max_history_per_job: usize,
    pub scheduler_notification_via_ws: bool,
    pub scheduler_agent_turn_timeout_secs: u64,
    pub scheduler_heartbeat_file: Option<String>,

    // Phase 8.11: Autonomous Reasoning
    pub agent_max_continuations: u32,
    pub agent_reasoning_guidance: Option<String>,

    // Phase 8: Inbox
    pub inbox_page_size: usize,
    pub inbox_sessions_page_size: usize,
    pub inbox_desktop_notifications: bool,

    // Phase 8.11: Embedding Provider
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_download_dir: Option<String>,

    // Environment overrides
    /// User's IANA timezone (e.g., "America/New_York"). Auto-detected if not set.
    pub user_timezone: Option<String>,
    /// User's location/region description (e.g., "New York, US"). Used for context injection.
    pub user_location: Option<String>,

    // Phase 9: Plugins
    pub plugins_dir: Option<String>,
    pub plugin_idle_timeout_secs: u64,
    pub plugin_max_restart_attempts: u32,
    pub plugin_execute_timeout_secs: u64,
    pub plugin_auto_update: bool,

    // Phase 8: Self-Evolution
    pub self_evolution_enabled: bool,
    pub learning_archive_threshold: f64,
    pub learning_archive_after_days: u32,
    pub skill_proposal_expiry_days: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway_host: "127.0.0.1".into(),
            gateway_port: 18981,
            log_level: "info".into(),
            data_dir: None,
            db_path: None,
            memory_db_path: None,
            identity_name: "MesoClaw".into(),
            identity_description: "AI-powered assistant".into(),
            provider_name: "openai".into(),
            provider_type: "openai".into(),
            provider_base_url: None,
            provider_model_id: "gpt-4o".into(),
            provider_api_key_env: None,
            security_autonomy_level: "supervised".into(),
            max_tool_retries: 3,

            // Memory
            memory_fts_weight: 0.4,
            memory_vector_weight: 0.6,
            memory_default_limit: 10,
            embedding_dim: 384,
            embedding_cache_size: 1000,

            // Security
            security_rate_limit_max: 60,
            security_rate_limit_window_secs: 60,
            security_audit_log_capacity: 1000,

            // Tools
            tool_shell_timeout_secs: 30,
            tool_file_read_max_lines: 10000,
            tool_file_search_max_results: 100,
            tool_process_list_limit: 200,

            // Web Search
            web_search_timeout_secs: 30,
            web_search_max_results: 20,

            // Gateway
            gateway_auth_token: None,
            ws_max_connections: 32,
            gateway_cors_origins: vec![
                "http://localhost:18971".into(),
                "tauri://localhost".into(),
                "https://tauri.localhost".into(),
            ],

            // Agent
            agent_max_turns: 8,
            agent_max_tokens: 4096,
            agent_system_prompt: None,

            // Identity
            identity_dir: None,

            // Skills
            skills_dir: None,
            skill_max_content_size: 100_000,

            // Credentials
            keyring_service_id: "com.sprklai.mesoclaw".into(),

            // Channels
            channel_tool_policy: HashMap::from([(
                "default".into(),
                vec!["web_search".into(), "system_info".into()],
            )]),
            channels_enabled: vec![],
            telegram_polling_timeout_secs: 30,
            telegram_dm_policy: "allowlist".into(),
            telegram_retry_min_ms: 1000,
            telegram_retry_max_ms: 60_000,
            telegram_require_group_mention: true,
            telegram_status_refresh_secs: 4,
            telegram_show_tool_status: true,
            slack_allowed_channel_ids: vec![],
            discord_allowed_guild_ids: vec![],
            discord_allowed_channel_ids: vec![],
            channel_router_buffer_size: 256,
            channel_reconnect_max_attempts: 10,

            // User Learning
            learning_enabled: true,
            learning_denied_categories: vec![],
            learning_max_observations: 10_000,
            learning_observation_ttl_days: 365,
            learning_min_confidence: 0.5,

            // Context Injection
            context_injection_enabled: true,
            context_summary_model_id: "gpt-4o-mini".into(),
            context_summary_provider_id: "openai".into(),
            context_reinject_gap_minutes: 30,
            context_reinject_message_count: 20,

            // Context Management (Step 15.3)
            context_strategy: "balanced".into(),
            context_max_history_messages: 20,
            context_max_memory_results: 5,
            context_auto_extract: true,
            context_extract_interval: 3,
            context_summary_model: "gpt-4o-mini".into(),

            // Scheduler
            scheduler_tick_interval_secs: 1,
            scheduler_stuck_threshold_secs: 120,
            scheduler_error_backoff_secs: vec![30, 60, 300, 900, 3600],
            scheduler_max_history_per_job: 100,
            scheduler_notification_via_ws: true,
            scheduler_agent_turn_timeout_secs: 120,
            scheduler_heartbeat_file: None,

            // Autonomous Reasoning
            agent_max_continuations: 3,
            agent_reasoning_guidance: None,

            // Inbox
            inbox_page_size: 50,
            inbox_sessions_page_size: 30,
            inbox_desktop_notifications: true,

            // Embedding Provider
            embedding_provider: "none".into(),
            embedding_model: "bge-small-en-v1.5".into(),
            embedding_download_dir: None,

            // Environment overrides
            user_timezone: None,
            user_location: None,

            // Plugins
            plugins_dir: None,
            plugin_idle_timeout_secs: 300,
            plugin_max_restart_attempts: 3,
            plugin_execute_timeout_secs: 60,
            plugin_auto_update: false,

            // Self-Evolution
            self_evolution_enabled: true,
            learning_archive_threshold: 0.3,
            learning_archive_after_days: 30,
            skill_proposal_expiry_days: 7,
        }
    }
}

impl AppConfig {
    /// Validate and clamp config values to acceptable ranges.
    /// Call this after loading config or before saving.
    pub fn validate(&mut self) {
        self.learning_min_confidence = self.learning_min_confidence.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // CR.31 — channel_tool_policy defaults to default allowlist
    #[test]
    fn channel_tool_policy_default() {
        let config = AppConfig::default();
        let default_tools = config.channel_tool_policy.get("default").unwrap();
        assert!(default_tools.contains(&"web_search".to_string()));
        assert!(default_tools.contains(&"system_info".to_string()));
        assert_eq!(default_tools.len(), 2);
    }

    // CR.32 — channel_tool_policy deserializes from TOML
    #[test]
    fn channel_tool_policy_from_toml() {
        let toml_str = r#"
            [channel_tool_policy]
            default = ["web_search", "system_info"]
            telegram = ["web_search"]
            discord = []
        "#;

        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.channel_tool_policy.get("telegram").unwrap(),
            &vec!["web_search".to_string()]
        );
        assert!(
            config
                .channel_tool_policy
                .get("discord")
                .unwrap()
                .is_empty()
        );
    }

    // 16.39 — Scheduler config defaults correct
    #[test]
    fn scheduler_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.scheduler_tick_interval_secs, 1);
        assert_eq!(config.scheduler_stuck_threshold_secs, 120);
        assert_eq!(
            config.scheduler_error_backoff_secs,
            vec![30, 60, 300, 900, 3600]
        );
        assert_eq!(config.scheduler_max_history_per_job, 100);
    }

    // 8.6.1.3 — scheduler notification config defaults
    #[test]
    fn scheduler_notification_config_defaults() {
        let config = AppConfig::default();
        assert!(config.scheduler_notification_via_ws);
        assert_eq!(config.scheduler_agent_turn_timeout_secs, 120);
        assert!(config.scheduler_heartbeat_file.is_none());
    }

    // 8.6.1.4 — scheduler notification config from TOML
    #[test]
    fn scheduler_notification_config_from_toml() {
        let toml_str = r#"
            scheduler_notification_via_ws = false
            scheduler_agent_turn_timeout_secs = 60
            scheduler_heartbeat_file = "/tmp/heartbeat.md"
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.scheduler_notification_via_ws);
        assert_eq!(config.scheduler_agent_turn_timeout_secs, 60);
        assert_eq!(
            config.scheduler_heartbeat_file.as_deref(),
            Some("/tmp/heartbeat.md")
        );
    }

    // IN.13 — inbox config defaults
    #[test]
    fn inbox_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.inbox_page_size, 50);
        assert_eq!(config.inbox_sessions_page_size, 30);
        assert!(config.inbox_desktop_notifications);
    }

    // IN.14 — inbox config from TOML
    #[test]
    fn inbox_config_from_toml() {
        let toml_str = r#"
            inbox_page_size = 100
            inbox_sessions_page_size = 20
            inbox_desktop_notifications = false
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.inbox_page_size, 100);
        assert_eq!(config.inbox_sessions_page_size, 20);
        assert!(!config.inbox_desktop_notifications);
    }

    // 8.11.19 — default agent_max_continuations is 3
    #[test]
    fn default_agent_max_continuations() {
        let config = AppConfig::default();
        assert_eq!(config.agent_max_continuations, 3);
        assert!(config.agent_reasoning_guidance.is_none());
    }

    // 15.3.40 — config context defaults
    #[test]
    fn config_context_defaults() {
        let config = AppConfig::default();
        assert!(config.context_injection_enabled);
        assert_eq!(config.context_summary_model_id, "gpt-4o-mini");
        assert_eq!(config.context_summary_provider_id, "openai");
        assert_eq!(config.context_reinject_gap_minutes, 30);
        assert_eq!(config.context_reinject_message_count, 20);
    }

    // 15.3.41 — config evolution defaults
    #[test]
    fn config_evolution_defaults() {
        let config = AppConfig::default();
        assert!(config.self_evolution_enabled);
        assert!((config.learning_archive_threshold - 0.3).abs() < f64::EPSILON);
        assert_eq!(config.learning_archive_after_days, 30);
        assert_eq!(config.skill_proposal_expiry_days, 7);
    }

    // ENV.1 — user_timezone and user_location default to None
    #[test]
    fn default_user_timezone_and_location() {
        let config = AppConfig::default();
        assert!(config.user_timezone.is_none());
        assert!(config.user_location.is_none());
    }

    // ENV.2 — user_timezone and user_location from TOML
    #[test]
    fn user_timezone_and_location_from_toml() {
        let toml_str = r#"
            user_timezone = "America/New_York"
            user_location = "New York, US"
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.user_timezone.as_deref(), Some("America/New_York"));
        assert_eq!(config.user_location.as_deref(), Some("New York, US"));
    }

    // WS-4.1 — default CORS origins include all required origins
    #[test]
    fn default_cors_origins_not_empty() {
        let config = AppConfig::default();
        assert!(
            !config.gateway_cors_origins.is_empty(),
            "Default CORS origins must not be empty"
        );
        assert!(
            config
                .gateway_cors_origins
                .contains(&"http://localhost:18971".to_string())
        );
        assert!(
            config
                .gateway_cors_origins
                .contains(&"tauri://localhost".to_string())
        );
        assert!(
            config
                .gateway_cors_origins
                .contains(&"https://tauri.localhost".to_string())
        );
    }

    // 18.10 — default embedding_provider is "none"
    #[test]
    fn default_embedding_provider_none() {
        let config = AppConfig::default();
        assert_eq!(config.embedding_provider, "none");
    }

    // 18.11 — default embedding_model is "bge-small-en-v1.5"
    #[test]
    fn default_embedding_model() {
        let config = AppConfig::default();
        assert_eq!(config.embedding_model, "bge-small-en-v1.5");
        assert!(config.embedding_download_dir.is_none());
    }

    // 15.3.42 — config deserializes with new fields
    #[test]
    fn config_deserialize_with_new_fields() {
        let toml_str = r#"
            context_injection_enabled = false
            context_summary_model_id = "claude-haiku"
            context_summary_provider_id = "anthropic"
            context_reinject_gap_minutes = 60
            context_reinject_message_count = 50
            self_evolution_enabled = false
            learning_archive_threshold = 0.5
            learning_archive_after_days = 14
            skill_proposal_expiry_days = 3
        "#;

        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.context_injection_enabled);
        assert_eq!(config.context_summary_model_id, "claude-haiku");
        assert_eq!(config.context_summary_provider_id, "anthropic");
        assert_eq!(config.context_reinject_gap_minutes, 60);
        assert_eq!(config.context_reinject_message_count, 50);
        assert!(!config.self_evolution_enabled);
        assert!((config.learning_archive_threshold - 0.5).abs() < f64::EPSILON);
        assert_eq!(config.learning_archive_after_days, 14);
        assert_eq!(config.skill_proposal_expiry_days, 3);
    }

    // WS-3.6a — context_summary_model has default value
    #[test]
    fn context_summary_model_has_default() {
        let config = AppConfig::default();
        assert!(!config.context_summary_model.is_empty());
        assert_eq!(config.context_summary_model, "gpt-4o-mini");
    }

    // WS-3.6b — learning_min_confidence clamped to [0.0, 1.0]
    #[test]
    fn learning_min_confidence_clamped() {
        let mut config = AppConfig::default();
        config.learning_min_confidence = 1.5;
        config.validate();
        assert!(config.learning_min_confidence <= 1.0);
        config.learning_min_confidence = -0.5;
        config.validate();
        assert!(config.learning_min_confidence >= 0.0);
    }
}
