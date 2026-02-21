//! TOML configuration schema for MesoClaw.
//!
//! All fields have `#[serde(default)]` so a partially-filled `config.toml`
//! works correctly.  Missing sections fall back to their `Default` impl.
//!
//! Example `~/.mesoclaw/config.toml`:
//! ```toml
//! [provider]
//! default_id = "openai"
//! default_model = "gpt-4o-mini"
//!
//! [security]
//! autonomy_level = "supervised"
//!
//! [scheduler]
//! heartbeat_interval_secs = 1800
//!
//! [memory]
//! enabled = true
//! embedding_cache_size = 10000
//!
//! [identity]
//! dir = "/home/user/.mesoclaw/identity"
//!
//! [notifications]
//! enabled = true
//! do_not_disturb = false
//! ```

use serde::{Deserialize, Serialize};

// ─── ProviderConfig ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ProviderConfig {
    /// ID of the default LLM provider (e.g. `"openai"`, `"anthropic"`).
    pub default_id: String,
    /// Default model identifier (e.g. `"gpt-4o-mini"`, `"claude-3-haiku"`).
    pub default_model: String,
    /// Request timeout in seconds.
    pub request_timeout_secs: u64,
    /// Maximum retry attempts on transient errors.
    pub max_retries: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            default_id: "openai".to_owned(),
            default_model: "gpt-4o-mini".to_owned(),
            request_timeout_secs: 60,
            max_retries: 3,
        }
    }
}

// ─── SandboxMode ─────────────────────────────────────────────────────────────

/// Controls which tool executions are sandboxed in containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    /// No sandboxing - all tools run directly on the host.
    Off,
    /// Only non-main-thread tools are sandboxed (tools spawned by agents).
    #[default]
    NonMain,
    /// All tool executions are sandboxed in containers.
    All,
}

impl SandboxMode {
    /// Returns true if this mode requires sandboxing for tool execution.
    pub fn is_sandboxed(&self, is_main_thread: bool) -> bool {
        match self {
            SandboxMode::Off => false,
            SandboxMode::NonMain => !is_main_thread,
            SandboxMode::All => true,
        }
    }
}

// ─── SandboxConfig ───────────────────────────────────────────────────────────

/// Configuration for container-based tool sandboxing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SandboxConfig {
    /// Which tools should be sandboxed.
    pub mode: SandboxMode,
    /// Default container image for sandboxed tools.
    pub default_image: String,
    /// Memory limit in MB for sandboxed containers.
    pub memory_limit_mb: Option<u64>,
    /// Whether to disable network access in sandboxed containers.
    pub network_disabled: bool,
    /// Timeout in seconds for sandboxed tool execution.
    pub timeout_secs: Option<u64>,
    /// Additional volume mounts (host_path:container_path format).
    pub volumes: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            mode: SandboxMode::default(),
            default_image: "alpine:3.20".to_string(),
            memory_limit_mb: Some(256),
            network_disabled: true,
            timeout_secs: Some(60),
            volumes: Vec::new(),
        }
    }
}

// ─── SecurityConfig ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SecurityConfig {
    /// Autonomy level: `"readonly"`, `"supervised"`, or `"autonomous"`.
    pub autonomy_level: String,
    /// Optional workspace root path (restricts file tool access).
    pub workspace_root: Option<String>,
    /// Commands blocked regardless of autonomy level.
    pub blocked_commands: Vec<String>,
    /// Rate limit: max tool calls per minute.
    pub rate_limit_per_minute: u32,
    /// Rate limit: max tool calls per hour.
    pub rate_limit_per_hour: u32,
    /// Sandbox configuration for tool isolation.
    pub sandbox: SandboxConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            autonomy_level: "supervised".to_owned(),
            workspace_root: None,
            blocked_commands: Vec::new(),
            rate_limit_per_minute: 60,
            rate_limit_per_hour: 600,
            sandbox: SandboxConfig::default(),
        }
    }
}

// ─── SchedulerConfig ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SchedulerConfig {
    /// Heartbeat interval in seconds (default: 30 minutes).
    pub heartbeat_interval_secs: u64,
    /// Whether the heartbeat scheduler is enabled.
    pub heartbeat_enabled: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_secs: 30 * 60,
            heartbeat_enabled: true,
        }
    }
}

// ─── MemoryConfig ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MemoryConfig {
    /// Whether the memory subsystem is enabled.
    pub enabled: bool,
    /// LRU cache size for embeddings.
    pub embedding_cache_size: usize,
    /// Maximum number of entries returned by recall.
    pub recall_limit: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            embedding_cache_size: 10_000,
            recall_limit: 10,
        }
    }
}

// ─── IdentityConfig ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct IdentityConfig {
    /// Override path to the identity directory (default: `~/.mesoclaw/identity`).
    pub dir: Option<String>,
}

// ─── NotificationsConfig ──────────────────────────────────────────────────────

fn default_dnd_start() -> u8 {
    22 // 10 pm
}

fn default_dnd_end() -> u8 {
    7 // 7 am
}

fn bool_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct NotificationsConfig {
    /// Whether desktop notifications are enabled globally.
    pub enabled: bool,
    /// Global Do Not Disturb mode (boolean toggle, env-var controlled).
    pub do_not_disturb: bool,
    /// When `true`, the DND time-window is enforced.  Default `false` (opt-in).
    pub dnd_schedule_enabled: bool,
    /// DND window start hour (0–23), inclusive. Default 22 (10 pm).
    #[serde(default = "default_dnd_start")]
    pub dnd_start_hour: u8,
    /// DND window end hour (0–23), exclusive. Default 7 (7 am).
    #[serde(default = "default_dnd_end")]
    pub dnd_end_hour: u8,
    /// Per-category enable flags (category name → enabled).
    pub categories: std::collections::HashMap<String, bool>,
    /// Notify on heartbeat ticks.
    #[serde(default = "bool_true")]
    pub notify_heartbeat: bool,
    /// Notify when a cron job fires.
    #[serde(default = "bool_true")]
    pub notify_cron_reminder: bool,
    /// Notify when an agent task completes.
    #[serde(default = "bool_true")]
    pub notify_agent_complete: bool,
    /// Notify when an approval is requested.
    #[serde(default = "bool_true")]
    pub notify_approval_request: bool,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            do_not_disturb: false,
            dnd_schedule_enabled: false,
            dnd_start_hour: default_dnd_start(),
            dnd_end_hour: default_dnd_end(),
            categories: std::collections::HashMap::new(),
            notify_heartbeat: true,
            notify_cron_reminder: true,
            notify_agent_complete: true,
            notify_approval_request: true,
        }
    }
}

// ─── AppConfig ────────────────────────────────────────────────────────────────

/// Top-level application configuration.
///
/// Loaded from `~/.mesoclaw/config.toml`, falling back to defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct AppConfig {
    pub provider: ProviderConfig,
    pub security: SecurityConfig,
    pub scheduler: SchedulerConfig,
    pub memory: MemoryConfig,
    pub identity: IdentityConfig,
    pub notifications: NotificationsConfig,
}
