//! Core types for the domain-agnostic skill system.
//!
//! This module defines the fundamental data structures used throughout
//! the skill engine, including skill definitions, composition modes,
//! and execution results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete definition of a skill loaded from markdown + YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDefinition {
    /// Unique identifier for the skill (e.g., "query-optimizer")
    pub id: String,

    /// Semantic version (e.g., "1.0.0")
    pub version: String,

    /// Human-readable name
    pub name: String,

    /// Description of what the skill does
    pub description: String,

    /// Feature toggle configuration
    pub feature: FeatureConfig,

    /// Context and tools required by this skill
    pub requires: SkillRequirements,

    /// Conditions that trigger this skill
    pub triggers: SkillTriggers,

    /// Composition and inheritance configuration
    pub compose: ComposeConfig,

    /// The actual prompt content (markdown body after frontmatter)
    pub prompt_content: String,
}

/// Feature toggle configuration for a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureConfig {
    /// Category for grouping (e.g., "performance", "understanding", "security")
    pub category: String,

    /// Whether the skill is enabled by default
    #[serde(default = "default_true")]
    pub default_enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Requirements that must be satisfied for a skill to execute.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRequirements {
    /// Context keys required (e.g., ["query", "schema", "database_type"])
    #[serde(default)]
    pub context: Vec<String>,

    /// Tool names required (e.g., ["execute_query", "explain_query"])
    #[serde(default)]
    pub tools: Vec<String>,
}

/// Trigger conditions for skill selection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillTriggers {
    /// Task types that trigger this skill (e.g., ["optimize", "analyze"])
    #[serde(default)]
    pub task_types: Vec<String>,

    /// Entity types that trigger this skill (e.g., ["query", "table"])
    #[serde(default)]
    pub entity_types: Vec<String>,
}

/// Configuration for skill composition and inheritance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeConfig {
    /// Parent skill ID for inheritance
    #[serde(default)]
    pub extends: Option<String>,

    /// Skills that can be combined with this one
    #[serde(default)]
    pub compatible_with: Vec<String>,

    /// Skills that conflict with this one
    #[serde(default)]
    pub conflicts_with: Vec<String>,

    /// Priority for ordering (higher = earlier)
    #[serde(default = "default_priority")]
    pub priority: i32,

    /// How to combine multiple skills
    #[serde(default)]
    pub mode: ComposeMode,

    /// Where to place this skill's prompt in composition
    #[serde(default)]
    pub prompt_position: PromptPosition,
}

fn default_priority() -> i32 {
    100
}

impl Default for ComposeConfig {
    fn default() -> Self {
        Self {
            extends: None,
            compatible_with: Vec::new(),
            conflicts_with: Vec::new(),
            priority: 100,
            mode: ComposeMode::default(),
            prompt_position: PromptPosition::default(),
        }
    }
}

/// How multiple skills are combined during execution.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComposeMode {
    /// Combine prompts into a single LLM call
    #[default]
    Merge,

    /// Execute skills sequentially, passing output forward
    Chain,

    /// Execute skills simultaneously, aggregate results
    Parallel,
}

/// Where to position a skill's prompt during composition.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptPosition {
    /// Add before other prompts
    Prepend,

    /// Add after other prompts (default)
    #[default]
    Append,

    /// Replace other prompts entirely
    Replace,
}

/// Result of skill execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillOutput {
    /// ID of the skill that produced this output
    pub skill_id: String,

    /// The generated content (explanation, analysis, etc.)
    pub content: String,

    /// Optional structured data extracted from the response
    #[serde(default)]
    pub structured_data: Option<serde_json::Value>,

    /// Record of tool calls made during execution
    #[serde(default)]
    pub tool_calls_made: Vec<ToolCallRecord>,

    /// Whether this was served from cache
    #[serde(default)]
    pub from_cache: bool,
}

/// Record of a tool call made during skill execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallRecord {
    /// Name of the tool called
    pub tool_name: String,

    /// Parameters passed to the tool
    pub parameters: serde_json::Value,

    /// Result from the tool
    pub result: Option<serde_json::Value>,

    /// Whether the call succeeded
    pub success: bool,
}

/// Composed skill ready for execution.
#[derive(Debug, Clone)]
pub struct ComposedSkill {
    /// The skills being composed
    pub skills: Vec<SkillDefinition>,

    /// The merged/combined prompt
    pub merged_prompt: String,

    /// How to execute the skills
    pub execution_plan: ExecutionPlan,

    /// Combined requirements from all skills
    pub combined_requirements: SkillRequirements,
}

/// Execution plan for composed skills.
#[derive(Debug, Clone)]
pub enum ExecutionPlan {
    /// Single skill execution
    Single(String),

    /// Multiple skills merged into one prompt
    Merged(Vec<String>),

    /// Skills executed in sequence
    Chained(Vec<String>),

    /// Skills executed in parallel
    Parallel(Vec<String>),
}

/// Context for skill selection.
#[derive(Debug, Clone, Default)]
pub struct SelectionContext {
    /// The user's request/query
    pub request: String,

    /// Database type if known (e.g., "postgresql", "mysql")
    pub database_type: Option<String>,

    /// Entity type if known (e.g., "query", "table", "column")
    pub entity_type: Option<String>,

    /// Hint about the task type (e.g., "optimize", "explain")
    pub task_hint: Option<String>,

    /// Context keys available in the current session
    pub available_context: Vec<String>,

    /// Set of skill IDs that are enabled for this workspace
    pub enabled_skills: std::collections::HashSet<String>,
}

/// Result of skill selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionResult {
    /// IDs of the selected skills
    pub selected_skills: Vec<String>,

    /// Optional reasoning from LLM selection
    pub reasoning: Option<String>,
}

/// Skill info for frontend display (lightweight version).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    /// Unique identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Description
    pub description: String,

    /// Category for grouping
    pub category: String,

    /// Whether enabled by default
    pub default_enabled: bool,

    /// Source of the skill
    pub source: SkillSource,
}

/// Where a skill comes from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillSource {
    /// Compiled into the binary
    Embedded,

    /// Loaded from user's local directory
    Local,

    /// Downloaded from remote registry
    Remote,
}

/// User configuration for a specific skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUserConfig {
    /// Whether the skill is enabled
    pub enabled: bool,

    /// Override the default priority
    pub priority_override: Option<i32>,
}

/// Complete skill settings for a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSettings {
    /// Set of enabled skill IDs
    #[serde(default)]
    pub enabled_skills: std::collections::HashSet<String>,

    /// Per-skill configuration overrides
    #[serde(default)]
    pub skill_configs: HashMap<String, SkillUserConfig>,

    /// Whether to use LLM for skill selection
    #[serde(default = "default_true")]
    pub auto_select: bool,
}

impl Default for SkillSettings {
    fn default() -> Self {
        Self {
            enabled_skills: std::collections::HashSet::new(),
            skill_configs: HashMap::new(),
            auto_select: true, // Enable LLM-based selection by default
        }
    }
}

impl From<&SkillDefinition> for SkillInfo {
    fn from(def: &SkillDefinition) -> Self {
        Self {
            id: def.id.clone(),
            name: def.name.clone(),
            description: def.description.clone(),
            category: def.feature.category.clone(),
            default_enabled: def.feature.default_enabled,
            source: SkillSource::Embedded, // Default; overridden by loader
        }
    }
}

impl SkillRequirements {
    /// Merge two requirements, taking the union of both.
    pub fn merge(&self, other: &Self) -> Self {
        let mut context = self.context.clone();
        for c in &other.context {
            if !context.contains(c) {
                context.push(c.clone());
            }
        }

        let mut tools = self.tools.clone();
        for t in &other.tools {
            if !tools.contains(t) {
                tools.push(t.clone());
            }
        }

        Self { context, tools }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_requirements_merge() {
        let req1 = SkillRequirements {
            context: vec!["query".into(), "schema".into()],
            tools: vec!["execute_query".into()],
        };

        let req2 = SkillRequirements {
            context: vec!["schema".into(), "database_type".into()],
            tools: vec!["explain_query".into()],
        };

        let merged = req1.merge(&req2);
        assert_eq!(merged.context.len(), 3);
        assert_eq!(merged.tools.len(), 2);
    }

    #[test]
    fn test_default_compose_mode() {
        let mode = ComposeMode::default();
        assert_eq!(mode, ComposeMode::Merge);
    }
}
