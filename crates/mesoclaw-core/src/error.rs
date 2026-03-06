use thiserror::Error;

#[derive(Debug, Error)]
pub enum MesoError {
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

    #[error("security policy denied: {0}")]
    PolicyDenied(String),

    #[error("channel error: {0}")]
    Channel(String),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = MesoError::Config("missing field".into());
        assert_eq!(err.to_string(), "config error: missing field");
    }

    #[test]
    fn error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: MesoError = io_err.into();
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn error_from_sqlite() {
        let sqlite_err = rusqlite::Error::InvalidParameterName("bad".into());
        let err: MesoError = sqlite_err.into();
        assert!(matches!(err, MesoError::Sqlite(_)));
    }
}
