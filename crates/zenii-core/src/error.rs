use thiserror::Error;

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

    #[error("{0}")]
    Other(String),
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
}
