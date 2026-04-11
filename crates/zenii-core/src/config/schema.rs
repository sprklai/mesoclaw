use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::notification::routing::NotificationRouting;
use crate::security::permissions::ToolPermissions;

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
    pub tool_file_search_max_depth: usize,
    pub tool_file_search_follow_symlinks: bool,
    pub tool_process_list_limit: usize,

    // Content Search
    pub tool_content_search_max_results: usize,
    pub tool_content_search_max_file_size_kb: usize,
    pub tool_content_search_context_lines: usize,

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
    /// Override path for encrypted credential file. Default: {data_dir}/credentials.enc
    pub credential_file_path: Option<String>,
    /// Timeout in seconds for the initial keyring probe. Falls back to file/in-memory on timeout.
    pub keyring_probe_timeout_secs: u64,

    // Phase 19: Tool Permissions
    pub tool_permissions: ToolPermissions,

    // Phase 8: Channels (channel_tool_policy kept for backward TOML compat)
    pub channel_tool_policy: HashMap<String, Vec<String>>,
    pub channels_enabled: Vec<String>,
    pub telegram_polling_timeout_secs: u32,
    pub telegram_http_timeout_buffer_secs: u32,
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

    // Channel Supervisor
    pub channel_supervisor_max_restarts: u32,
    pub channel_supervisor_backoff_min_ms: u64,
    pub channel_supervisor_backoff_max_ms: u64,

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
    pub context_extraction_max_tokens: usize,

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
    pub scheduler_agent_turn_timeout_secs: u64,
    pub scheduler_heartbeat_file: Option<String>,
    /// Maximum consecutive failures before a job is auto-disabled.
    pub scheduler_max_consecutive_failures: u32,

    // Phase 8.11: Autonomous Reasoning
    pub agent_max_continuations: u32,
    pub agent_reasoning_guidance: Option<String>,

    // Audit: Agent execution safety
    pub agent_timeout_secs: u64,

    // Audit: Event bus capacity
    pub event_bus_capacity: usize,

    // Audit: Session cleanup
    pub session_max_age_days: u32,

    // Phase 8: Inbox
    pub inbox_page_size: usize,
    pub inbox_sessions_page_size: usize,
    pub inbox_desktop_notifications: bool,

    // Phase 8.11: Embedding Provider
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_download_dir: Option<String>,

    // Environment overrides
    /// User's display name (e.g., "John"). Used in greetings and personalization.
    pub user_name: Option<String>,
    /// User's IANA timezone (e.g., "America/New_York"). Auto-detected if not set.
    pub user_timezone: Option<String>,
    /// User's location/region description (e.g., "New York, US"). Used for context injection.
    pub user_location: Option<String>,
    /// Explicit flag set to true when the onboarding wizard completes successfully.
    /// Prevents re-triggering onboarding when the credential store loses API keys
    /// (e.g., macOS Keychain after dev recompilation, in-memory fallback on Linux).
    pub onboarding_completed: bool,

    // Phase 9: Plugins
    pub plugins_dir: Option<String>,
    pub plugin_idle_timeout_secs: u64,
    pub plugin_max_restart_attempts: u32,
    pub plugin_execute_timeout_secs: u64,
    pub plugin_auto_update: bool,
    pub official_plugins_repo: String,

    // Phase 8.12: Notification Routing
    pub notification_routing: NotificationRouting,

    // Tool Deduplication
    pub tool_dedup_enabled: bool,
    /// Per-tool call limits within a single request. Tools listed here are capped
    /// at the specified number of executions per request, regardless of args.
    /// Unlisted tools are unlimited. Default: `{"web_search": 1}`.
    pub tool_call_limits: HashMap<String, usize>,

    // Phase 8.13: Prompt Efficiency
    pub prompt_max_preamble_tokens: usize,
    pub prompt_compact_identity: bool,

    // Phase 8.14: Usage Logging
    pub usage_tracking_enabled: bool,
    pub log_dir: String,
    pub log_keep_days: u32,

    // Feature 5: Workflow Engine
    pub workflow_dir: Option<String>,
    pub workflow_max_concurrent: usize,
    pub workflow_max_steps: usize,
    pub workflow_step_timeout_secs: u64,
    pub workflow_step_max_retries: u32,

    // Feature 7: Agent Delegation
    pub delegation_max_sub_agents: usize,
    pub delegation_per_agent_token_budget: usize,
    pub delegation_per_agent_timeout_secs: u64,
    pub delegation_decomposition_model: Option<String>,
    /// When true, delegation skips human approval for sub-agent tool calls.
    pub delegation_skip_approval: bool,

    // Tool Approval
    pub approval_timeout_secs: u64,

    // Phase 8: Self-Evolution
    pub self_evolution_enabled: bool,
    pub learning_archive_threshold: f64,
    pub learning_archive_after_days: u32,
    pub skill_proposal_expiry_days: u32,

    // Wiki
    /// Override path for the wiki directory. Default: {data_dir}/wiki/
    pub wiki_dir: Option<String>,
    /// Maximum size of a single wiki page file in kibibytes. Requests that produce
    /// pages larger than this are logged as a warning (advisory — not hard-blocked,
    /// since page content comes from the LLM). Default: 512 KiB.
    pub wiki_max_page_size_kb: u64,
    /// Maximum size of a single source upload in mebibytes. Requests that exceed
    /// this limit are rejected with HTTP 400. Default: 50 MiB.
    pub wiki_max_source_size_mb: u64,
    /// Binary name or path for document-to-markdown conversion (default: "markitdown").
    /// Install with: pip install markitdown[all]
    /// Override to use a different binary, e.g. "pandoc" or "/usr/local/bin/markitdown".
    pub doc_converter_bin: String,
    /// Timeout in seconds for the document converter subprocess (default: 60 seconds).
    /// A hanging PDF or slow conversion will be killed and return an error after this many seconds.
    pub wiki_convert_timeout_secs: u64,
    // Wiki Graph visualization tunables (served to frontend via GET /config)
    pub wiki_graph_pre_ticks: u32,
    pub wiki_graph_initial_alpha: f64,
    pub wiki_graph_alpha_decay: f64,
    pub wiki_graph_link_distance: f64,
    pub wiki_graph_charge_strength: f64,
    pub wiki_graph_collide_radius: f64,
    pub wiki_graph_center_force_strength: f64,
    pub wiki_graph_cluster_force_strength: f64,
    /// Cluster center distance from canvas center, as fraction of min(width, height).
    pub wiki_graph_cluster_radius_factor: f64,
    pub wiki_graph_cluster_scatter: f64,
    pub wiki_graph_drag_alpha_target: f64,
    pub wiki_graph_label_auto_hide_threshold: u32,
    pub wiki_graph_zoom_min: f64,
    pub wiki_graph_zoom_max: f64,
    pub wiki_graph_zoom_step: f64,
    pub wiki_graph_fit_padding: f64,
    pub wiki_graph_fit_scale_factor: f64,
    pub wiki_graph_node_radius: f64,
    pub wiki_graph_node_radius_selected: f64,
    pub wiki_graph_node_hit_radius: f64,
    pub wiki_graph_selection_ring_radius: f64,
    pub wiki_graph_label_font_size: f64,
    pub wiki_graph_label_y_offset: f64,
    pub wiki_graph_wheel_zoom_step: f64,
    /// Maximum number of lines to keep in wiki/log.md before trimming oldest entries.
    /// When the line count exceeds this limit after an append, the oldest lines are removed.
    /// Default: 10000 lines.
    pub wiki_log_max_lines: usize,

    // MCP (Model Context Protocol)
    /// Prefix prepended to tool names when exposed via MCP (e.g., "zenii_")
    pub mcp_server_tool_prefix: String,
    /// Allowlist of tool names to expose via MCP. Empty = expose all.
    pub mcp_server_exposed_tools: Vec<String>,
    /// Denylist of tool names to hide from MCP. Applied after allowlist.
    pub mcp_server_hidden_tools: Vec<String>,
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
            identity_name: "Zenii".into(),
            identity_description: "AI-powered assistant".into(),
            provider_name: "anthropic".into(),
            provider_type: "anthropic".into(),
            provider_base_url: None,
            provider_model_id: "claude-sonnet-4-6".into(),
            provider_api_key_env: None,
            security_autonomy_level: "full".into(),
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
            tool_file_search_max_depth: 20,
            tool_file_search_follow_symlinks: false,
            tool_process_list_limit: 200,

            // Content Search
            tool_content_search_max_results: 50,
            tool_content_search_max_file_size_kb: 1024,
            tool_content_search_context_lines: 2,

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
            keyring_service_id: "com.sprklai.zenii".into(),
            credential_file_path: None,
            keyring_probe_timeout_secs: 5,

            // Tool Permissions
            tool_permissions: ToolPermissions::default(),

            // Channels (channel_tool_policy kept for backward compat)
            channel_tool_policy: HashMap::new(),
            channels_enabled: vec![],
            telegram_polling_timeout_secs: 30,
            telegram_http_timeout_buffer_secs: 10,
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

            // Channel Supervisor
            channel_supervisor_max_restarts: 0, // 0 = infinite
            channel_supervisor_backoff_min_ms: 5_000,
            channel_supervisor_backoff_max_ms: 300_000,

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
            context_extraction_max_tokens: 512,

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
            scheduler_agent_turn_timeout_secs: 120,
            scheduler_heartbeat_file: None,
            scheduler_max_consecutive_failures: 10,

            // Autonomous Reasoning
            agent_max_continuations: 1,
            agent_reasoning_guidance: None,

            // Agent execution safety
            agent_timeout_secs: 300,

            // Event bus capacity
            event_bus_capacity: 256,

            // Session cleanup
            session_max_age_days: 90,

            // Inbox
            inbox_page_size: 50,
            inbox_sessions_page_size: 30,
            inbox_desktop_notifications: true,

            // Embedding Provider
            embedding_provider: "none".into(),
            embedding_model: "bge-small-en-v1.5".into(),
            embedding_download_dir: None,

            // Environment overrides
            user_name: None,
            user_timezone: None,
            user_location: None,
            onboarding_completed: false,

            // Plugins
            plugins_dir: None,
            plugin_idle_timeout_secs: 300,
            plugin_max_restart_attempts: 3,
            plugin_execute_timeout_secs: 60,
            plugin_auto_update: false,
            official_plugins_repo: "https://github.com/sprklai/zenii-plugins.git".into(),

            // Tool Deduplication
            tool_dedup_enabled: true,
            tool_call_limits: HashMap::from([("web_search".into(), 5)]),

            // Notification Routing
            notification_routing: NotificationRouting::default(),

            // Prompt Efficiency
            prompt_max_preamble_tokens: 1500,
            prompt_compact_identity: true,

            // Usage Logging
            usage_tracking_enabled: true,
            log_dir: String::new(),
            log_keep_days: 30,

            // Workflow Engine
            workflow_dir: None,
            workflow_max_concurrent: 5,
            workflow_max_steps: 50,
            workflow_step_timeout_secs: 300,
            workflow_step_max_retries: 3,

            // Agent Delegation
            delegation_max_sub_agents: 4,
            delegation_per_agent_token_budget: 4000,
            delegation_per_agent_timeout_secs: 120,
            delegation_decomposition_model: None,
            delegation_skip_approval: true,

            // Tool Approval
            approval_timeout_secs: 120,

            // Self-Evolution
            self_evolution_enabled: true,
            learning_archive_threshold: 0.3,
            learning_archive_after_days: 30,
            skill_proposal_expiry_days: 7,

            // Wiki
            wiki_dir: None,
            wiki_max_page_size_kb: 512,
            wiki_max_source_size_mb: 50,
            doc_converter_bin: "markitdown".into(),
            wiki_convert_timeout_secs: 60,
            wiki_graph_pre_ticks: 40,
            wiki_graph_initial_alpha: 0.5,
            wiki_graph_alpha_decay: 0.02,
            wiki_graph_link_distance: 60.0,
            wiki_graph_charge_strength: -120.0,
            wiki_graph_collide_radius: 18.0,
            wiki_graph_center_force_strength: 0.02,
            wiki_graph_cluster_force_strength: 0.12,
            wiki_graph_cluster_radius_factor: 0.28,
            wiki_graph_cluster_scatter: 60.0,
            wiki_graph_drag_alpha_target: 0.3,
            wiki_graph_label_auto_hide_threshold: 200,
            wiki_graph_zoom_min: 0.2,
            wiki_graph_zoom_max: 3.0,
            wiki_graph_zoom_step: 1.2,
            wiki_graph_fit_padding: 30.0,
            wiki_graph_fit_scale_factor: 0.9,
            wiki_graph_node_radius: 6.0,
            wiki_graph_node_radius_selected: 8.0,
            wiki_graph_node_hit_radius: 20.0,
            wiki_graph_selection_ring_radius: 12.0,
            wiki_graph_label_font_size: 10.0,
            wiki_graph_label_y_offset: 14.0,
            wiki_graph_wheel_zoom_step: 1.1,
            wiki_log_max_lines: 10000,

            // MCP
            mcp_server_tool_prefix: "zenii_".into(),
            mcp_server_exposed_tools: vec![],
            mcp_server_hidden_tools: vec![],
        }
    }
}

impl AppConfig {
    /// Validate and clamp config values to acceptable ranges.
    /// Call this after loading config or before saving.
    pub fn validate(&mut self) {
        self.learning_min_confidence = self.learning_min_confidence.clamp(0.0, 1.0);
        self.agent_max_turns = self.agent_max_turns.clamp(1, 32);
        self.agent_max_continuations = self.agent_max_continuations.clamp(0, 5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // P19.12 — Default AppConfig has ToolPermissions with correct defaults
    #[test]
    fn config_tool_permissions_default() {
        use crate::security::permissions::PermissionState;
        let config = AppConfig::default();
        assert_eq!(
            config.tool_permissions.low_risk_default,
            PermissionState::Allowed
        );
        assert_eq!(
            config.tool_permissions.medium_risk_default,
            PermissionState::Allowed
        );
        assert_eq!(
            config.tool_permissions.high_risk_default,
            PermissionState::Denied
        );
        assert!(config.tool_permissions.overrides.contains_key("desktop"));
        assert!(config.tool_permissions.overrides.contains_key("cli"));
        assert!(config.tool_permissions.overrides.contains_key("tui"));
    }

    // P19.13 — ToolPermissions deserializes from TOML
    #[test]
    fn config_tool_permissions_toml() {
        let toml_str = r#"
            [tool_permissions]
            low_risk_default = "allowed"
            medium_risk_default = "allowed"
            high_risk_default = "denied"

            [tool_permissions.overrides.telegram]
            memory = "denied"
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config
                .tool_permissions
                .overrides
                .get("telegram")
                .unwrap()
                .get("memory")
                .unwrap(),
            &crate::security::permissions::PermissionState::Denied
        );
    }

    // P19.14 — Old channel_tool_policy still deserializes (backward compat)
    #[test]
    fn config_backward_compat_channel_tool_policy() {
        let toml_str = r#"
            [channel_tool_policy]
            default = ["web_search", "system_info"]
            telegram = ["web_search"]
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.channel_tool_policy.get("telegram").unwrap(),
            &vec!["web_search".to_string()]
        );
        // tool_permissions uses defaults since not specified
        assert_eq!(
            config.tool_permissions.low_risk_default,
            crate::security::permissions::PermissionState::Allowed
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

    // 8.6.1.3 — scheduler agent turn timeout and heartbeat config
    #[test]
    fn scheduler_agent_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.scheduler_agent_turn_timeout_secs, 120);
        assert!(config.scheduler_heartbeat_file.is_none());
    }

    // 8.6.1.4 — scheduler agent config from TOML
    #[test]
    fn scheduler_agent_config_from_toml() {
        let toml_str = r#"
            scheduler_agent_turn_timeout_secs = 60
            scheduler_heartbeat_file = "/tmp/heartbeat.md"
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
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

    // 8.11.19 — default agent_max_continuations is 1
    #[test]
    fn default_agent_max_continuations() {
        let config = AppConfig::default();
        assert_eq!(config.agent_max_continuations, 1);
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

    // SUP.8 — supervisor config defaults
    #[test]
    fn supervisor_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.channel_supervisor_max_restarts, 0); // infinite
        assert_eq!(config.channel_supervisor_backoff_min_ms, 5_000);
        assert_eq!(config.channel_supervisor_backoff_max_ms, 300_000);
    }

    // SUP.9 — supervisor config from TOML
    #[test]
    fn supervisor_config_from_toml() {
        let toml_str = r#"
            channel_supervisor_max_restarts = 5
            channel_supervisor_backoff_min_ms = 2000
            channel_supervisor_backoff_max_ms = 60000
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.channel_supervisor_max_restarts, 5);
        assert_eq!(config.channel_supervisor_backoff_min_ms, 2000);
        assert_eq!(config.channel_supervisor_backoff_max_ms, 60000);
    }

    // 8.12.15 — AppConfig default includes notification_routing with toast+desktop
    #[test]
    fn notification_routing_defaults() {
        let config = AppConfig::default();
        let routing = &config.notification_routing;
        assert_eq!(routing.scheduler_notification.len(), 2);
        assert!(
            routing
                .scheduler_notification
                .contains(&crate::notification::target::NotificationTarget::Toast)
        );
        assert!(
            routing
                .scheduler_notification
                .contains(&crate::notification::target::NotificationTarget::Desktop)
        );
        assert_eq!(routing.scheduler_job_completed.len(), 2);
        assert_eq!(routing.channel_message.len(), 2);
    }

    // 8.12.16 — AppConfig TOML deser with custom notification_routing
    #[test]
    fn notification_routing_toml_deser() {
        let toml_str = r#"
            [notification_routing]
            scheduler_notification = ["toast", "telegram"]
            scheduler_job_completed = ["desktop"]
            channel_message = ["toast"]
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.notification_routing.scheduler_notification.len(), 2);
        assert!(
            config
                .notification_routing
                .scheduler_notification
                .contains(&crate::notification::target::NotificationTarget::Telegram)
        );
        assert_eq!(config.notification_routing.scheduler_job_completed.len(), 1);
        assert_eq!(config.notification_routing.channel_message.len(), 1);
    }

    // 8.12.17 — AppConfig TOML deser without notification_routing uses defaults
    #[test]
    fn notification_routing_missing_uses_default() {
        let toml_str = r#"
            gateway_host = "127.0.0.1"
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.notification_routing.scheduler_notification.len(), 2);
        assert_eq!(config.notification_routing.scheduler_job_completed.len(), 2);
        assert_eq!(config.notification_routing.channel_message.len(), 2);
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

    // TC-S1 — default agent_max_continuations is 1
    #[test]
    fn tc_s1_default_agent_max_continuations() {
        let config = AppConfig::default();
        assert_eq!(config.agent_max_continuations, 1);
    }

    // TC-S2 — default agent_max_turns is 8
    #[test]
    fn tc_s2_default_agent_max_turns() {
        let config = AppConfig::default();
        assert_eq!(config.agent_max_turns, 8);
    }

    // TC-S3 — default tool_dedup_enabled is true
    #[test]
    fn tc_s3_default_tool_dedup_enabled() {
        let config = AppConfig::default();
        assert!(config.tool_dedup_enabled);
    }

    // TC-S4 — TOML deser for tool_dedup_enabled
    #[test]
    fn tc_s4_tool_dedup_enabled_from_toml() {
        let toml_str = r#"
            tool_dedup_enabled = false
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.tool_dedup_enabled);
    }

    // TC-S5 — validate clamps agent_max_turns to 1..=32
    #[test]
    fn tc_s5_validate_clamps_agent_max_turns() {
        let mut config = AppConfig::default();
        config.agent_max_turns = 0;
        config.validate();
        assert_eq!(config.agent_max_turns, 1);

        config.agent_max_turns = 100;
        config.validate();
        assert_eq!(config.agent_max_turns, 32);

        config.agent_max_turns = 16;
        config.validate();
        assert_eq!(config.agent_max_turns, 16);
    }

    // TC-S6 — validate clamps agent_max_continuations to 0..=5
    #[test]
    fn tc_s6_validate_clamps_agent_max_continuations() {
        let mut config = AppConfig::default();
        config.agent_max_continuations = 10;
        config.validate();
        assert_eq!(config.agent_max_continuations, 5);

        config.agent_max_continuations = 0;
        config.validate();
        assert_eq!(config.agent_max_continuations, 0);
    }

    // TC-CL1 — Default tool_call_limits contains web_search: 5
    #[test]
    fn tc_cl1_default_tool_call_limits() {
        let config = AppConfig::default();
        assert_eq!(config.tool_call_limits.get("web_search"), Some(&5));
    }

    // TC-CL2 — TOML deser for tool_call_limits
    #[test]
    fn tc_cl2_tool_call_limits_from_toml() {
        let toml_str = r#"
            [tool_call_limits]
            web_search = 2
            file_read = 10
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.tool_call_limits.get("web_search"), Some(&2));
        assert_eq!(config.tool_call_limits.get("file_read"), Some(&10));
    }

    // 8.14.17 — Default has usage_tracking_enabled = true
    #[test]
    fn default_usage_tracking_enabled() {
        let config = AppConfig::default();
        assert!(config.usage_tracking_enabled);
    }

    // 8.14.18 — Default has log_keep_days = 30
    #[test]
    fn default_log_keep_days() {
        let config = AppConfig::default();
        assert_eq!(config.log_keep_days, 30);
    }

    // 8.14.19 — Default has empty log_dir
    #[test]
    fn default_log_dir_empty() {
        let config = AppConfig::default();
        assert!(config.log_dir.is_empty());
    }

    // 7.28 — delegation config defaults and TOML deser
    #[test]
    fn delegation_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.delegation_max_sub_agents, 4);
        assert_eq!(config.delegation_per_agent_token_budget, 4000);
        assert_eq!(config.delegation_per_agent_timeout_secs, 120);
        assert!(config.delegation_decomposition_model.is_none());
    }

    #[test]
    fn delegation_config_from_toml() {
        let toml_str = r#"
            delegation_max_sub_agents = 8
            delegation_per_agent_token_budget = 8000
            delegation_per_agent_timeout_secs = 300
            delegation_decomposition_model = "openai:gpt-4o"
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.delegation_max_sub_agents, 8);
        assert_eq!(config.delegation_per_agent_token_budget, 8000);
        assert_eq!(config.delegation_per_agent_timeout_secs, 300);
        assert_eq!(
            config.delegation_decomposition_model.as_deref(),
            Some("openai:gpt-4o")
        );
    }

    // AUDIT — default agent_timeout_secs is 300
    #[test]
    fn audit_default_agent_timeout() {
        let config = AppConfig::default();
        assert_eq!(config.agent_timeout_secs, 300);
    }

    // AUDIT — default event_bus_capacity is 256
    #[test]
    fn audit_default_event_bus_capacity() {
        let config = AppConfig::default();
        assert_eq!(config.event_bus_capacity, 256);
    }

    // AUDIT — default session_max_age_days is 90
    #[test]
    fn audit_default_session_max_age_days() {
        let config = AppConfig::default();
        assert_eq!(config.session_max_age_days, 90);
    }

    // AUDIT — new config fields from TOML
    #[test]
    fn audit_config_from_toml() {
        let toml_str = r#"
            agent_timeout_secs = 600
            event_bus_capacity = 512
            session_max_age_days = 30
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.agent_timeout_secs, 600);
        assert_eq!(config.event_bus_capacity, 512);
        assert_eq!(config.session_max_age_days, 30);
    }

    // TA.13 — approval_timeout_secs default value is 120
    #[test]
    fn approval_timeout_secs_default() {
        let config = AppConfig::default();
        assert_eq!(config.approval_timeout_secs, 120);
    }

    // TA.14 — approval_timeout_secs from TOML
    #[test]
    fn approval_timeout_secs_from_toml() {
        let toml_str = r#"
            approval_timeout_secs = 60
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.approval_timeout_secs, 60);
    }

    // 5.50 — workflow config defaults
    #[test]
    fn workflow_config_defaults() {
        let config = AppConfig::default();
        assert!(config.workflow_dir.is_none());
        assert_eq!(config.workflow_max_concurrent, 5);
        assert_eq!(config.workflow_max_steps, 50);
        assert_eq!(config.workflow_step_timeout_secs, 300);
        assert_eq!(config.workflow_step_max_retries, 3);
    }

    // 5.51 — workflow config from TOML
    #[test]
    fn workflow_config_from_toml() {
        let toml_str = r#"
            workflow_dir = "/tmp/workflows"
            workflow_max_concurrent = 10
            workflow_max_steps = 100
            workflow_step_timeout_secs = 600
            workflow_step_max_retries = 5
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.workflow_dir.as_deref(), Some("/tmp/workflows"));
        assert_eq!(config.workflow_max_concurrent, 10);
        assert_eq!(config.workflow_max_steps, 100);
        assert_eq!(config.workflow_step_timeout_secs, 600);
        assert_eq!(config.workflow_step_max_retries, 5);
    }
}
