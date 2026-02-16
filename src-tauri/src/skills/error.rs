//! Error types for the skill system.

use thiserror::Error;

/// Errors that can occur in the skill system.
#[derive(Debug, Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),

    #[error("Failed to parse skill file: {0}")]
    ParseError(String),

    #[error("Invalid skill definition: {0}")]
    InvalidDefinition(String),

    #[error("Skill inheritance cycle detected: {0}")]
    InheritanceCycle(String),

    #[error("Parent skill not found: {0}")]
    ParentNotFound(String),

    #[error("Incompatible skills: {0} conflicts with {1}")]
    IncompatibleSkills(String, String),

    #[error("Missing required context: {0}")]
    MissingContext(String),

    #[error("Missing required tool: {0}")]
    MissingTool(String),

    #[error("Execution failed: {0}")]
    ExecutionError(String),

    #[error("LLM provider error: {0}")]
    LlmError(String),

    #[error("Tool execution failed: {0}")]
    ToolError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("YAML parse error: {0}")]
    YamlError(String),

    #[error("Registry not initialized")]
    RegistryNotInitialized,

    #[error("No skills selected for request")]
    NoSkillsSelected,

    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Result type for skill operations.
pub type SkillResult<T> = Result<T, SkillError>;

impl From<serde_yaml::Error> for SkillError {
    fn from(err: serde_yaml::Error) -> Self {
        SkillError::YamlError(err.to_string())
    }
}
