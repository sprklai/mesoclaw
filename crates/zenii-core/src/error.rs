use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Actionable hint attached to user-facing errors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorHint {
    /// User-friendly summary of what went wrong.
    pub summary: String,
    /// Concrete action the user can take to fix it.
    pub action: String,
}

#[derive(Debug, Error)]
pub enum ZeniiError {
    #[error("config error: {0}")]
    Config(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("event bus error: {0}")]
    EventBus(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("credential error: {0}")]
    Credential(String),

    #[error("agent error: {0}")]
    Agent(String),

    #[error("gateway error: {0}")]
    Gateway(String),

    #[error("authentication required: {0}")]
    Auth(String),

    #[error("security policy denied: {0}")]
    PolicyDenied(String),

    #[error("channel error: {0}")]
    Channel(String),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("memory error: {0}")]
    Memory(String),

    #[error("embedding error: {0}")]
    Embedding(String),

    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("reqwest error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("identity error: {0}")]
    Identity(String),

    #[error("identity not found: {0}")]
    IdentityNotFound(String),

    #[error("skill error: {0}")]
    Skill(String),

    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("user error: {0}")]
    User(String),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("context error: {0}")]
    Context(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("scheduler error: {0}")]
    Scheduler(String),

    #[error("plugin error: {0}")]
    Plugin(String),

    #[error("plugin not found: {0}")]
    PluginNotFound(String),

    #[error("workflow error: {0}")]
    Workflow(String),

    #[error("model capability error: {0}")]
    ModelCapability(String),

    #[error("approval denied: {0}")]
    ApprovalDenied(String),

    #[error("approval timed out: {0}")]
    ApprovalTimeout(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("{0}")]
    Other(String),
}

/// Produce an actionable hint for a ZeniiError, if one applies.
/// Returns None for internal/opaque errors where no user action helps.
pub fn enrich_error(err: &ZeniiError) -> Option<ErrorHint> {
    match err {
        ZeniiError::Credential(_) => Some(ErrorHint {
            summary: "API key not configured".into(),
            action: "Set your API key in Settings > Providers, or via CLI: `zenii key set <provider> <key>`".into(),
        }),
        ZeniiError::ModelCapability(_) => Some(ErrorHint {
            summary: "Model doesn't support this operation".into(),
            action: "Select a model that supports tool usage in Settings > Providers".into(),
        }),
        ZeniiError::ApprovalDenied(_) => Some(ErrorHint {
            summary: "Tool execution was denied".into(),
            action: "Approve the tool when prompted, or adjust permissions in Settings > Security".into(),
        }),
        ZeniiError::ApprovalTimeout(_) => Some(ErrorHint {
            summary: "Approval request timed out".into(),
            action: "Respond to approval prompts faster, or increase `approval_timeout_secs` in config.toml".into(),
        }),
        ZeniiError::RateLimited(_) => Some(ErrorHint {
            summary: "Rate limit exceeded".into(),
            action: "Wait a moment and retry, or increase `security_rate_limit_max` in config.toml".into(),
        }),
        ZeniiError::PolicyDenied(_) => Some(ErrorHint {
            summary: "Security policy blocked this action".into(),
            action: "Check autonomy level and tool permissions in Settings > Security".into(),
        }),
        ZeniiError::Agent(msg) => enrich_agent_message(msg),
        ZeniiError::Channel(msg) => enrich_channel_message(msg),
        // Internal/opaque errors — no hint (avoid information leakage)
        ZeniiError::Sqlite(_) | ZeniiError::Database(_) | ZeniiError::Io(_) => None,
        _ => None,
    }
}

fn enrich_agent_message(msg: &str) -> Option<ErrorHint> {
    let lower = msg.to_lowercase();
    if lower.contains("maxturn") || lower.contains("max turn") {
        Some(ErrorHint {
            summary: "Agent exceeded maximum tool loop turns".into(),
            action:
                "Increase `agent_max_turns` in config.toml (Settings > Agent) for tool-heavy tasks"
                    .into(),
        })
    } else if lower.contains("401")
        || lower.contains("unauthorized")
        || lower.contains("invalid api key")
        || lower.contains("invalid x-api-key")
        || lower.contains("authentication")
    {
        Some(ErrorHint {
            summary: "API key is invalid or expired".into(),
            action: "Check your API key in Settings > Providers, or set it via CLI: `zenii key set <provider> <key>`".into(),
        })
    } else if lower.contains("429")
        || lower.contains("rate limit")
        || lower.contains("rate_limit")
        || lower.contains("too many requests")
        || lower.contains("quota")
    {
        Some(ErrorHint {
            summary: "Provider rate limit exceeded".into(),
            action: "Wait a moment and retry, or switch to a different model/provider".into(),
        })
    } else if lower.contains("context length")
        || lower.contains("too many tokens")
        || lower.contains("maximum context")
        || lower.contains("token limit")
        || lower.contains("input too long")
    {
        Some(ErrorHint {
            summary: "Input too long for the model's context window".into(),
            action: "Shorten your message, start a new session, or use a model with a larger context window".into(),
        })
    } else if lower.contains("model")
        && (lower.contains("not found")
            || lower.contains("does not exist")
            || lower.contains("not available"))
    {
        Some(ErrorHint {
            summary: "Model not found or unavailable".into(),
            action: "Check the model ID in Settings > Providers, or select a different model"
                .into(),
        })
    } else if lower.contains("timeout") || lower.contains("timed out") || lower.contains("deadline")
    {
        Some(ErrorHint {
            summary: "Request timed out".into(),
            action: "Check your internet connection, or retry. For slow models, increase timeouts in config.toml".into(),
        })
    } else if lower.contains("connection refused")
        || lower.contains("connect error")
        || lower.contains("dns")
        || lower.contains("unreachable")
    {
        Some(ErrorHint {
            summary: "Cannot reach the AI provider".into(),
            action:
                "Check your internet connection and the provider's base URL in Settings > Providers"
                    .into(),
        })
    } else if lower.contains("no agent configured") || lower.contains("no provider") {
        Some(ErrorHint {
            summary: "No AI provider configured".into(),
            action: "Set up a provider and API key in Settings > Providers".into(),
        })
    } else if lower.contains("toolnotfound") {
        Some(ErrorHint {
            summary: "Agent tried to use a tool that doesn't exist".into(),
            action: "This is usually a model hallucination. Retry or use a different model".into(),
        })
    } else {
        None
    }
}

fn enrich_channel_message(msg: &str) -> Option<ErrorHint> {
    let lower = msg.to_lowercase();
    if lower.contains("unauthorized")
        || lower.contains("invalid token")
        || lower.contains("401")
        || lower.contains("403")
    {
        Some(ErrorHint {
            summary: "Channel authentication failed".into(),
            action: "Check your bot token in Settings > Channels".into(),
        })
    } else if lower.contains("rate limit")
        || lower.contains("429")
        || lower.contains("too many requests")
    {
        Some(ErrorHint {
            summary: "Channel rate limit hit".into(),
            action: "Wait a moment. The bot is sending too many messages".into(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = ZeniiError::Config("missing field".into());
        assert_eq!(err.to_string(), "config error: missing field");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: ZeniiError = io_err.into();
        assert!(err.to_string().contains("file missing"));
    }

    // 16.38 — ZeniiError::Scheduler variant
    #[test]
    fn scheduler_error_variant() {
        let err = ZeniiError::Scheduler("job failed".into());
        assert_eq!(err.to_string(), "scheduler error: job failed");
    }

    #[test]
    fn error_from_sqlite() {
        let sqlite_err = rusqlite::Error::InvalidParameterName("bad".into());
        let err: ZeniiError = sqlite_err.into();
        assert!(matches!(err, ZeniiError::Sqlite(_)));
    }

    // 5.49 — ZeniiError::Workflow variant
    #[test]
    fn workflow_error_variant() {
        let err = ZeniiError::Workflow("step failed".into());
        assert_eq!(err.to_string(), "workflow error: step failed");
    }

    // TA.11 — ZeniiError::ApprovalDenied display
    #[test]
    fn approval_denied_error_display() {
        let err = ZeniiError::ApprovalDenied("user rejected shell command".into());
        assert_eq!(
            err.to_string(),
            "approval denied: user rejected shell command"
        );
    }

    // TA.12 — ZeniiError::ApprovalTimeout display
    #[test]
    fn approval_timeout_error_display() {
        let err = ZeniiError::ApprovalTimeout("shell:cargo build after 120s".into());
        assert_eq!(
            err.to_string(),
            "approval timed out: shell:cargo build after 120s"
        );
    }

    // --- Error enrichment tests ---

    #[test]
    fn hint_max_turns() {
        let err =
            ZeniiError::Agent("prompt failed: MaxTurnError: (reached max turn limit: 4)".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("agent_max_turns"));
    }

    #[test]
    fn hint_401() {
        let err = ZeniiError::Agent("prompt failed: ProviderError: 401 Unauthorized".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.summary.contains("API key"));
    }

    #[test]
    fn hint_429() {
        let err = ZeniiError::Agent("prompt failed: ProviderError: 429 rate limit exceeded".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.summary.contains("rate limit"));
    }

    #[test]
    fn hint_context_length() {
        let err = ZeniiError::Agent("prompt failed: context length exceeded".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("context window"));
    }

    #[test]
    fn hint_model_not_found() {
        let err = ZeniiError::Agent("prompt failed: model gpt-99 does not exist".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.summary.contains("Model not found"));
    }

    #[test]
    fn hint_timeout() {
        let err = ZeniiError::Agent("prompt failed: request timed out".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.summary.contains("timed out"));
    }

    #[test]
    fn hint_connection_refused() {
        let err = ZeniiError::Agent("prompt failed: connection refused".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.summary.contains("Cannot reach"));
    }

    #[test]
    fn hint_no_provider() {
        let err = ZeniiError::Agent("no agent configured".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("Settings > Providers"));
    }

    #[test]
    fn hint_tool_not_found() {
        let err = ZeniiError::Agent("ToolNotFoundError: search_web".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.summary.contains("tool"));
    }

    #[test]
    fn hint_credential() {
        let err = ZeniiError::Credential("key not found for openai".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("key"));
    }

    #[test]
    fn hint_model_capability() {
        let err = ZeniiError::ModelCapability("model does not support tools".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("tool usage"));
    }

    #[test]
    fn hint_approval_denied() {
        let err = ZeniiError::ApprovalDenied("user rejected".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("Security"));
    }

    #[test]
    fn hint_approval_timeout() {
        let err = ZeniiError::ApprovalTimeout("120s".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("approval_timeout_secs"));
    }

    #[test]
    fn hint_rate_limited() {
        let err = ZeniiError::RateLimited("too fast".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("security_rate_limit_max"));
    }

    #[test]
    fn hint_none_for_unknown_agent() {
        let err = ZeniiError::Agent("some random internal error".into());
        assert!(enrich_error(&err).is_none());
    }

    #[test]
    fn hint_none_for_internal() {
        assert!(enrich_error(&ZeniiError::Database("db error".into())).is_none());
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test");
        assert!(enrich_error(&ZeniiError::Io(io_err)).is_none());
    }

    #[test]
    fn hint_channel_auth() {
        let err = ZeniiError::Channel("telegram: 401 unauthorized".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.action.contains("bot token"));
    }

    #[test]
    fn hint_channel_rate_limit() {
        let err = ZeniiError::Channel("slack: 429 too many requests".into());
        let hint = enrich_error(&err).unwrap();
        assert!(hint.summary.contains("rate limit"));
    }
}
