//! `ModelRouter` — maps task types and model aliases to concrete provider+model targets.
//!
//! # Responsibilities
//! - **Alias resolution**: `"claude"` → `("vercel-ai-gateway", "anthropic/claude-sonnet-4-5")`
//! - **Task routing**: `TaskType::Code` → preferred provider list, in priority order
//! - **Cost-tier selection**: prefer cheaper models unless `CostTier::High` is requested
//! - **Availability fallback**: if the primary target is unavailable, walk the priority list
//!
//! # Configuration
//! Routing rules are supplied as [`RouterConfig`], which can be constructed from defaults
//! or deserialised from a `[router]` TOML section.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ─── CostTier ─────────────────────────────────────────────────────────────────

/// Indicates which cost tier of model is acceptable for a given request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CostTier {
    /// Cheapest / fastest models (e.g. claude-haiku, gpt-4o-mini).
    Low,
    /// Mid-tier models (default).
    #[default]
    Medium,
    /// Most capable / expensive models.
    High,
}

// ─── TaskType ─────────────────────────────────────────────────────────────────

/// Semantic task categories used to select an appropriate model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Software development, code review, debugging.
    Code,
    /// Open-ended reasoning and conversation.
    General,
    /// Low-latency responses where speed > quality.
    Fast,
    /// Creative writing, brainstorming, ideation.
    Creative,
    /// Data analysis, summarisation, structured extraction.
    Analysis,
    /// Catch-all for tasks that don't fit the above categories.
    Other,
}

impl TaskType {
    /// Parse a task type from a string (case-insensitive).
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "code" | "coding" | "development" => TaskType::Code,
            "general" | "chat" | "conversation" => TaskType::General,
            "fast" | "quick" | "instant" => TaskType::Fast,
            "creative" | "write" | "brainstorm" => TaskType::Creative,
            "analysis" | "analyze" | "summarize" | "extract" => TaskType::Analysis,
            _ => TaskType::Other,
        }
    }
}

// ─── ModelTarget ──────────────────────────────────────────────────────────────

/// A concrete provider + model pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelTarget {
    /// Provider ID as understood by [`crate::ai::providers::ProviderType`] (e.g. `"vercel-ai-gateway"`).
    pub provider_id: String,
    /// Model identifier forwarded to the provider (e.g. `"anthropic/claude-sonnet-4-5"`).
    pub model: String,
    /// Cost tier of this target (for tier-based selection).
    #[serde(default)]
    pub cost_tier: CostTier,
}

impl ModelTarget {
    pub fn new(provider_id: impl Into<String>, model: impl Into<String>, cost_tier: CostTier) -> Self {
        Self {
            provider_id: provider_id.into(),
            model: model.into(),
            cost_tier,
        }
    }
}

// ─── TaskRoute ────────────────────────────────────────────────────────────────

/// Ordered list of [`ModelTarget`]s for a given [`TaskType`].
///
/// Targets are tried in order; the first available one wins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRoute {
    pub targets: Vec<ModelTarget>,
}

impl TaskRoute {
    pub fn new(targets: Vec<ModelTarget>) -> Self {
        Self { targets }
    }
}

// ─── RouterConfig ─────────────────────────────────────────────────────────────

/// Complete routing configuration.
///
/// Serialisable so it can be embedded in `AppConfig` as a `[router]` TOML section
/// in future iterations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Model aliases: short name → concrete target.
    /// Example: `"claude" → ModelTarget { provider_id: "vercel-ai-gateway", model: "anthropic/claude-sonnet-4-5" }`.
    #[serde(default)]
    pub aliases: HashMap<String, ModelTarget>,

    /// Task-type routing rules.
    #[serde(default)]
    pub task_routes: HashMap<String, TaskRoute>, // key = TaskType serialised string

    /// Absolute fallback when nothing else matches.
    pub fallback: ModelTarget,
}

impl RouterConfig {
    /// Sensible defaults suitable for production use via Vercel AI Gateway.
    pub fn default_vercel() -> Self {
        let mut aliases = HashMap::new();
        // Anthropic aliases
        aliases.insert(
            "claude".to_string(),
            ModelTarget::new("vercel-ai-gateway", "anthropic/claude-sonnet-4-5", CostTier::Medium),
        );
        aliases.insert(
            "claude-opus".to_string(),
            ModelTarget::new("vercel-ai-gateway", "anthropic/claude-opus-4-5", CostTier::High),
        );
        aliases.insert(
            "claude-haiku".to_string(),
            ModelTarget::new("vercel-ai-gateway", "anthropic/claude-haiku-4-5-20251001", CostTier::Low),
        );
        // OpenAI aliases
        aliases.insert(
            "gpt4o".to_string(),
            ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o", CostTier::Medium),
        );
        aliases.insert(
            "gpt4o-mini".to_string(),
            ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o-mini", CostTier::Low),
        );
        // Google aliases
        aliases.insert(
            "gemini".to_string(),
            ModelTarget::new("vercel-ai-gateway", "google/gemini-2.0-flash", CostTier::Low),
        );

        let mut task_routes = HashMap::new();

        // Code: Claude Sonnet → GPT-4o → GPT-4o-mini (low-cost fallback)
        task_routes.insert(
            "code".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new("vercel-ai-gateway", "anthropic/claude-sonnet-4-5", CostTier::Medium),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o", CostTier::Medium),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o-mini", CostTier::Low),
            ]),
        );

        // General: GPT-4o-mini → Gemini Flash → fallback
        task_routes.insert(
            "general".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o-mini", CostTier::Low),
                ModelTarget::new("vercel-ai-gateway", "google/gemini-2.0-flash", CostTier::Low),
            ]),
        );

        // Fast: Gemini Flash → GPT-4o-mini
        task_routes.insert(
            "fast".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new("vercel-ai-gateway", "google/gemini-2.0-flash", CostTier::Low),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o-mini", CostTier::Low),
            ]),
        );

        // Creative: Claude Sonnet → GPT-4o
        task_routes.insert(
            "creative".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new("vercel-ai-gateway", "anthropic/claude-sonnet-4-5", CostTier::Medium),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o", CostTier::Medium),
            ]),
        );

        // Analysis: Claude Opus → Claude Sonnet → GPT-4o
        task_routes.insert(
            "analysis".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new("vercel-ai-gateway", "anthropic/claude-opus-4-5", CostTier::High),
                ModelTarget::new("vercel-ai-gateway", "anthropic/claude-sonnet-4-5", CostTier::Medium),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o", CostTier::Medium),
            ]),
        );

        Self {
            aliases,
            task_routes,
            fallback: ModelTarget::new(
                "vercel-ai-gateway",
                "openai/gpt-4o-mini",
                CostTier::Low,
            ),
        }
    }

    /// Minimal defaults useful for testing (no external deps).
    pub fn default_test() -> Self {
        let mut task_routes = HashMap::new();
        task_routes.insert(
            "code".to_string(),
            TaskRoute::new(vec![
                // Different provider IDs so availability-fallback tests work correctly.
                ModelTarget::new("provider-a", "test-model-a", CostTier::Medium),
                ModelTarget::new("provider-b", "test-model-b", CostTier::Low),
            ]),
        );
        Self {
            aliases: HashMap::new(),
            task_routes,
            fallback: ModelTarget::new("provider-fallback", "test-fallback", CostTier::Low),
        }
    }
}

// ─── ModelRouter ──────────────────────────────────────────────────────────────

/// Routes requests to the most appropriate model based on task type, cost tier,
/// and configured aliases.
pub struct ModelRouter {
    config: RouterConfig,
}

impl ModelRouter {
    /// Create a router from a [`RouterConfig`].
    pub fn new(config: RouterConfig) -> Self {
        Self { config }
    }

    /// Create a router with sensible production defaults (Vercel AI Gateway).
    pub fn default_vercel() -> Self {
        Self::new(RouterConfig::default_vercel())
    }

    // ── Alias resolution ───────────────────────────────────────────────────

    /// Resolve a model alias (e.g. `"claude"`) to a concrete [`ModelTarget`].
    ///
    /// Returns `None` if the alias is not registered.
    pub fn resolve_alias(&self, alias: &str) -> Option<&ModelTarget> {
        self.config.aliases.get(alias)
    }

    /// Resolve an alias or, if it's not found, treat the input as a raw model
    /// identifier and use the fallback provider.
    pub fn resolve_alias_or_raw(&self, alias_or_model: &str) -> ModelTarget {
        if let Some(target) = self.resolve_alias(alias_or_model) {
            return target.clone();
        }
        // Treat as a raw model string; use fallback provider.
        ModelTarget {
            provider_id: self.config.fallback.provider_id.clone(),
            model: alias_or_model.to_string(),
            cost_tier: CostTier::Medium,
        }
    }

    // ── Task routing ───────────────────────────────────────────────────────

    /// Return all targets for a task type, ordered by priority.
    ///
    /// Falls back to the global fallback list when no specific route is configured.
    pub fn targets_for(&self, task: TaskType) -> Vec<&ModelTarget> {
        let key = self.task_type_key(task);
        if let Some(route) = self.config.task_routes.get(&key) {
            route.targets.iter().collect()
        } else {
            vec![&self.config.fallback]
        }
    }

    /// Return the primary (highest-priority) target for a task type.
    pub fn primary_for(&self, task: TaskType) -> &ModelTarget {
        self.targets_for(task)
            .into_iter()
            .next()
            .unwrap_or(&self.config.fallback)
    }

    /// Return the best target for a task type that matches the requested cost tier.
    ///
    /// If no target with the exact tier exists, falls back to the primary target.
    pub fn route_by_cost(&self, task: TaskType, tier: CostTier) -> &ModelTarget {
        self.targets_for(task)
            .into_iter()
            .find(|t| t.cost_tier == tier)
            .unwrap_or_else(|| self.primary_for(task))
    }

    /// Return the first available target for a task type, skipping unavailable providers.
    ///
    /// `is_available` is a predicate that returns `true` when a provider ID is reachable.
    /// In production, this wraps a registry health-check; in tests, pass `|_| true`.
    pub fn route_with_fallback<F>(&self, task: TaskType, is_available: F) -> &ModelTarget
    where
        F: Fn(&str) -> bool,
    {
        for target in self.targets_for(task) {
            if is_available(&target.provider_id) {
                return target;
            }
        }
        // All preferred providers unavailable — use global fallback.
        &self.config.fallback
    }

    // ── Accessors ─────────────────────────────────────────────────────────

    /// Return the global fallback target.
    pub fn fallback(&self) -> &ModelTarget {
        &self.config.fallback
    }

    /// Return all registered alias keys.
    pub fn alias_keys(&self) -> Vec<&str> {
        self.config.aliases.keys().map(String::as_str).collect()
    }

    // ── Private helpers ───────────────────────────────────────────────────

    fn task_type_key(&self, task: TaskType) -> String {
        let s = match task {
            TaskType::Code => "code",
            TaskType::General => "general",
            TaskType::Fast => "fast",
            TaskType::Creative => "creative",
            TaskType::Analysis => "analysis",
            TaskType::Other => "general",
        };
        s.to_string()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_router() -> ModelRouter {
        ModelRouter::new(RouterConfig::default_test())
    }

    fn vercel_router() -> ModelRouter {
        ModelRouter::default_vercel()
    }

    // ── Alias tests ────────────────────────────────────────────────────────

    #[test]
    fn alias_resolution_known_alias() {
        let router = vercel_router();
        let target = router.resolve_alias("claude").unwrap();
        assert_eq!(target.provider_id, "vercel-ai-gateway");
        assert!(target.model.contains("claude-sonnet"));
    }

    #[test]
    fn alias_resolution_unknown_alias_returns_none() {
        let router = vercel_router();
        assert!(router.resolve_alias("nonexistent-alias").is_none());
    }

    #[test]
    fn alias_or_raw_known_alias() {
        let router = vercel_router();
        let target = router.resolve_alias_or_raw("gpt4o");
        assert_eq!(target.provider_id, "vercel-ai-gateway");
        assert!(target.model.contains("gpt-4o"));
    }

    #[test]
    fn alias_or_raw_unknown_returns_raw_model_with_fallback_provider() {
        let router = vercel_router();
        let target = router.resolve_alias_or_raw("some-custom/model");
        assert_eq!(target.provider_id, "vercel-ai-gateway");
        assert_eq!(target.model, "some-custom/model");
    }

    // ── Task routing tests ────────────────────────────────────────────────

    #[test]
    fn primary_for_code_task() {
        let router = vercel_router();
        let target = router.primary_for(TaskType::Code);
        assert_eq!(target.provider_id, "vercel-ai-gateway");
        assert!(target.model.contains("claude"));
    }

    #[test]
    fn primary_for_fast_task() {
        let router = vercel_router();
        let target = router.primary_for(TaskType::Fast);
        // Fast should use a low-cost model.
        assert_eq!(target.cost_tier, CostTier::Low);
    }

    #[test]
    fn targets_for_returns_ordered_list() {
        let router = vercel_router();
        let targets = router.targets_for(TaskType::Analysis);
        // Analysis route should have ≥2 targets in priority order.
        assert!(targets.len() >= 2);
        // First should be the highest-tier model.
        assert_eq!(targets[0].cost_tier, CostTier::High);
    }

    #[test]
    fn route_unregistered_task_falls_back_to_global_fallback() {
        let router = test_router();
        // `Other` has no explicit route in test config.
        let target = router.primary_for(TaskType::Other);
        assert_eq!(target.model, "test-fallback");
    }

    // ── Cost-tier routing ─────────────────────────────────────────────────

    #[test]
    fn route_by_cost_low_tier() {
        let router = vercel_router();
        let target = router.route_by_cost(TaskType::Code, CostTier::Low);
        // Code route has a low-tier fallback (gpt-4o-mini or similar).
        assert_eq!(target.cost_tier, CostTier::Low);
    }

    #[test]
    fn route_by_cost_falls_back_to_primary_when_tier_missing() {
        // Fast route has only Low-tier targets; requesting High should return primary.
        let router = vercel_router();
        let target = router.route_by_cost(TaskType::Fast, CostTier::High);
        // Should return primary (first in list) since no High-tier Fast target exists.
        assert_eq!(target, router.primary_for(TaskType::Fast));
    }

    // ── Availability-based fallback ────────────────────────────────────────

    #[test]
    fn route_with_fallback_all_available() {
        let router = test_router();
        let target = router.route_with_fallback(TaskType::Code, |_| true);
        // All available → returns primary.
        assert_eq!(target.model, "test-model-a");
    }

    #[test]
    fn route_with_fallback_primary_unavailable() {
        let router = test_router();
        // "provider-a" (primary) is unavailable; "provider-b" (second) is available.
        let target = router.route_with_fallback(TaskType::Code, |id| id == "provider-b");
        assert_eq!(target.model, "test-model-b");
    }

    #[test]
    fn route_with_fallback_all_unavailable_returns_global_fallback() {
        let router = test_router();
        let target = router.route_with_fallback(TaskType::Code, |_| false);
        assert_eq!(target.model, "test-fallback");
    }

    // ── TaskType parsing ──────────────────────────────────────────────────

    #[test]
    fn task_type_from_str_known_values() {
        assert_eq!(TaskType::from_str("code"), TaskType::Code);
        assert_eq!(TaskType::from_str("coding"), TaskType::Code);
        assert_eq!(TaskType::from_str("general"), TaskType::General);
        assert_eq!(TaskType::from_str("FAST"), TaskType::Fast);
        assert_eq!(TaskType::from_str("creative"), TaskType::Creative);
        assert_eq!(TaskType::from_str("analysis"), TaskType::Analysis);
    }

    #[test]
    fn task_type_from_str_unknown_falls_back_to_other() {
        assert_eq!(TaskType::from_str("unknown-type"), TaskType::Other);
    }

    // ── Alias keys ────────────────────────────────────────────────────────

    #[test]
    fn alias_keys_contains_all_registered_aliases() {
        let router = vercel_router();
        let keys = router.alias_keys();
        assert!(keys.contains(&"claude"));
        assert!(keys.contains(&"gpt4o"));
        assert!(keys.contains(&"gemini"));
    }

    // ── Fallback accessor ─────────────────────────────────────────────────

    #[test]
    fn fallback_returns_global_fallback() {
        let router = vercel_router();
        let fb = router.fallback();
        assert_eq!(fb.cost_tier, CostTier::Low);
    }
}
