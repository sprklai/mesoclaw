//! Domain-agnostic skill system for AI capabilities.
//!
//! This module provides a composable, reusable skill engine that can be
//! adapted to different applications. Skills are AI prompts with metadata
//! that can be selected, composed, and executed based on context.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                 SKILL ENGINE (reusable core)            │
//! │  Loader → Registry → Selector → Composer → Executor    │
//! └─────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │              APPLICATION ADAPTER (app-specific)         │
//! │  Provides: context, tools, output mapping               │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Components
//!
//! - [`types`]: Core data structures for skills
//! - [`loader`]: Parses skill files from markdown + YAML frontmatter
//! - [`registry`]: Central index of available skills
//! - [`selector`]: Two-stage selection (structured pre-filter + LLM)
//! - [`composer`]: Combines multiple skills for execution
//! - [`executor`]: Runs composed skills through the LLM
//! - [`settings`]: Per-workspace skill configuration
//! - [`error`]: Error types for the skill system
//!
//! # Usage
//!
//! ```rust,ignore
//! use skills::{SkillEngine, SkillLoader, SkillRegistry};
//! use adapters::KnoAdapter;
//!
//! // Initialize the skill engine
//! let loader = SkillLoader::new(local_path, remote_url);
//! let registry = SkillRegistry::new(Arc::new(loader));
//! registry.initialize().await?;
//!
//! // Create an adapter for your application
//! let adapter = KnoAdapter::new(connection_manager, workspace_id);
//!
//! // Create the skill engine
//! let engine = SkillEngine::new(adapter, registry);
//!
//! // Execute a skill-based request
//! let result = engine.execute("Optimize this query", llm_provider).await?;
//! ```

pub mod composer;
pub mod error;
pub mod executor;
pub mod loader;
pub mod registry;
pub mod selector;
pub mod settings;
pub mod state;
pub mod types;

pub use composer::SkillComposer;
pub use error::{SkillError, SkillResult};
pub use executor::{ExecutionRequest, SkillExecutor};
pub use loader::SkillLoader;
pub use registry::SkillRegistry;
pub use selector::SkillSelector;
pub use settings::SkillSettingsService;
pub use state::{get_or_init_registry, get_skill_registry, initialize_skill_registry, reload_registry};
pub use types::*;

use crate::adapters::ApplicationAdapter;
use crate::ai::provider::LLMProvider;
use std::sync::Arc;

/// The main skill engine that orchestrates skill selection and execution.
///
/// The engine is generic over the application adapter, allowing it to be
/// reused across different applications.
pub struct SkillEngine<A: ApplicationAdapter> {
    /// Application adapter for context and tools
    adapter: Arc<A>,

    /// Registry of available skills
    registry: Arc<SkillRegistry>,

    /// Skill selector for choosing appropriate skills
    selector: SkillSelector,

    /// Skill composer for combining skills
    composer: SkillComposer,
}

impl<A: ApplicationAdapter + 'static> SkillEngine<A> {
    /// Create a new skill engine.
    pub fn new(adapter: Arc<A>, registry: Arc<SkillRegistry>) -> Self {
        let selector = SkillSelector::new(registry.clone());
        let composer = SkillComposer::new();

        Self {
            adapter,
            registry,
            selector,
            composer,
        }
    }

    /// Initialize the skill engine by loading all skills.
    pub async fn initialize(&self) -> SkillResult<()> {
        if !self.registry.is_initialized() {
            self.registry.initialize().await?;
        }
        Ok(())
    }

    /// Execute a skill-based request.
    ///
    /// This method:
    /// 1. Selects appropriate skills based on the request
    /// 2. Composes the selected skills
    /// 3. Executes through the LLM
    /// 4. Returns the result
    pub async fn execute(
        &self,
        request: &str,
        context: SelectionContext,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<SkillOutput> {
        // Select skills
        let selection = self
            .selector
            .select(&context, Some(llm_provider.clone()))
            .await?;

        if selection.selected_skills.is_empty() {
            return Err(SkillError::NoSkillsSelected);
        }

        // Get the selected skill definitions
        let skills = self.selector.get_skills_by_ids(&selection.selected_skills);

        // Compose skills
        let composed = self.composer.compose(skills)?;

        // Execute
        let executor = SkillExecutor::new(self.adapter.clone());
        let exec_request = ExecutionRequest {
            user_request: request.to_string(),
            composed_skill: composed,
            context_overrides: None,
        };

        executor.execute(exec_request, llm_provider).await
    }

    /// Execute with a specific skill (bypassing selection).
    pub async fn execute_skill(
        &self,
        skill_id: &str,
        request: &str,
        llm_provider: Arc<dyn LLMProvider>,
    ) -> SkillResult<SkillOutput> {
        let skill = self
            .registry
            .get(skill_id)
            .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))?;

        let composed = self.composer.compose(vec![skill])?;

        let executor = SkillExecutor::new(self.adapter.clone());
        let exec_request = ExecutionRequest {
            user_request: request.to_string(),
            composed_skill: composed,
            context_overrides: None,
        };

        executor.execute(exec_request, llm_provider).await
    }

    /// Get the skill registry.
    pub fn registry(&self) -> &Arc<SkillRegistry> {
        &self.registry
    }

    /// Get the adapter.
    pub fn adapter(&self) -> &Arc<A> {
        &self.adapter
    }

    /// Get skill infos for the UI.
    pub fn get_skill_infos(&self) -> Vec<SkillInfo> {
        self.registry.get_skill_infos()
    }

    /// Reload skills from all sources.
    pub async fn reload(&self) -> SkillResult<()> {
        self.registry.reload().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Basic smoke test
    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _: fn() -> SkillError = || SkillError::NoSkillsSelected;
        let _: SkillComposer = SkillComposer::new();
    }
}
