//! Skill metadata structures for the enhanced skill system.
//!
//! Provides structured metadata for skills including requirements,
//! tool schemas, and configuration options.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete skill snapshot containing all loaded skills with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSnapshot {
    /// Snapshot version (timestamp-based for cache invalidation)
    pub version: String,
    /// All loaded skills indexed by ID
    pub skills: HashMap<String, Skill>,
}

/// A complete skill definition with metadata and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    /// Unique skill identifier
    pub id: String,
    /// Human-readable skill name
    pub name: String,
    /// Brief description of what this skill does
    pub description: String,
    /// Skill category (e.g., "general", "coding", "analysis")
    pub category: String,
    /// Extended metadata including requirements and tool schemas
    pub metadata: SkillMetadata,
    /// Whether this skill is currently enabled
    pub enabled: bool,
    /// The prompt template content (markdown with {{parameter}} placeholders)
    pub template: String,
    /// Parameters accepted by this skill
    pub parameters: Vec<TemplateParameter>,
    /// Source location (workspace, global, or bundled)
    pub source: SkillSource,
    /// File path (if loaded from filesystem)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// Extended skill metadata with requirements and configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMetadata {
    /// Emoji icon for UI display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// Tool requirements for this skill
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires: Option<SkillRequirements>,
    /// Tool schemas this skill provides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_schemas: Option<Vec<ToolSchema>>,
    /// Additional custom metadata
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Requirements for a skill to function properly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRequirements {
    /// At least one of these binaries must be available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_bins: Option<Vec<String>>,
    /// All of these binaries must be available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_bins: Option<Vec<String>>,
    /// These API keys must be configured
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_keys: Option<Vec<String>>,
    /// These environment variables must be set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<String>>,
}

/// Tool schema definition for skills that expose tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSchema {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON schema for tool parameters
    pub parameters: serde_json::Value,
}

/// Skill parameter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Whether this parameter is required
    pub required: bool,
}

/// Source location for a skill.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkillSource {
    /// Workspace-specific skill (highest precedence)
    Workspace,
    /// Global user skill (medium precedence)
    Global,
    /// Bundled with application (lowest precedence)
    Bundled,
}

impl Default for SkillMetadata {
    fn default() -> Self {
        Self {
            emoji: None,
            requires: None,
            tool_schemas: None,
            extra: HashMap::new(),
        }
    }
}
