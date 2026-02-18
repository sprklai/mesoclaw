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
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            autonomy_level: "supervised".to_owned(),
            workspace_root: None,
            blocked_commands: Vec::new(),
            rate_limit_per_minute: 60,
            rate_limit_per_hour: 600,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct NotificationsConfig {
    /// Whether desktop notifications are enabled globally.
    pub enabled: bool,
    /// Global Do Not Disturb mode.
    pub do_not_disturb: bool,
    /// Per-category enable flags (category name → enabled).
    pub categories: std::collections::HashMap<String, bool>,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            do_not_disturb: false,
            categories: std::collections::HashMap::new(),
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
