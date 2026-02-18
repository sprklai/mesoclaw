//! Lightweight prompt template system.
//!
//! Replaces the previous complex skills system with a simple filesystem-based
//! template registry backed by the Tera templating engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

pub mod loader;

pub use loader::TemplateRegistry;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Lightweight prompt template (replaces SkillDefinition).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub template: String,
    pub parameters: Vec<TemplateParameter>,
}

/// A single parameter accepted by a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// Shape expected by the frontend `SkillInfo` interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub default_enabled: bool,
    pub source: String,
}

/// Alias kept for IPC compatibility (`get_skill_details_command`).
pub type SkillDefinition = PromptTemplate;

/// Shape expected by the frontend `SkillSettings` interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSettings {
    pub auto_select: bool,
    pub skills: Vec<SkillUserConfig>,
}

/// Per-skill user configuration (always-enabled in the template system).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUserConfig {
    pub skill_id: String,
    pub enabled: bool,
    pub priority_override: Option<i32>,
}

/// Context used when selecting skills for a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionContext {
    pub request: String,
}

/// Skill suggestion returned by `suggest_skills_command`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSuggestion {
    pub skill_id: String,
    pub skill_name: String,
    pub description: String,
    pub relevance_score: f32,
    pub matched_triggers: Vec<String>,
}

/// Output produced by skill execution.
///
/// Kept as a minimal struct to satisfy references from the adapter layer.
/// Actual skill execution returns rendered template strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillOutput {
    pub skill_id: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Global registry singleton
// ---------------------------------------------------------------------------

static REGISTRY: OnceLock<Arc<TemplateRegistry>> = OnceLock::new();

/// Return the process-wide template registry, initialising it on first call.
pub async fn get_or_init_registry() -> Arc<TemplateRegistry> {
    if let Some(r) = REGISTRY.get() {
        return r.clone();
    }
    let registry = TemplateRegistry::new();
    registry.load().await;
    // A concurrent call may have won the race; ignore the error if so.
    let _ = REGISTRY.set(registry.clone());
    // Return the version that was actually stored (the winner of any concurrent race).
    // SAFETY: either we just called set() above, or get() already returned Some.
    #[allow(clippy::expect_used)]
    REGISTRY.get().expect("registry was just set").clone()
}
