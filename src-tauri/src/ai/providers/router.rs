//! `ModelRouter` — maps task types and model aliases to concrete provider+model targets.
//!
//! # Responsibilities
//! - **Alias resolution**: `"claude"` → `("vercel-ai-gateway", "anthropic/claude-sonnet-4-5")`
//! - **Task routing**: `TaskType::Code` → preferred provider list, in priority order
//! - **Cost-tier selection**: prefer cheaper models unless `CostTier::High` is requested
//! - **Availability fallback**: if the primary target is unavailable, walk the priority list
//! - **Routing profiles**: eco/balanced/premium for automatic model selection
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

// ─── RoutingProfile ───────────────────────────────────────────────────────────

/// Routing profiles for automatic model selection based on cost/quality tradeoffs.
///
/// Each profile maps to different cost tier preferences for task-based routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RoutingProfile {
    /// Cost-optimized routing. Prefers Low tier models with Medium fallback.
    /// Best for development, testing, and budget-conscious usage.
    Eco,
    /// Balanced quality/cost routing. Prefers Medium tier with Low/High fallback.
    /// Best for general use (default).
    #[default]
    Balanced,
    /// Maximum quality routing. Prefers High tier models with Medium fallback.
    /// Best for production and critical tasks.
    Premium,
}

impl RoutingProfile {
    /// Parse a routing profile from a string (case-insensitive).
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "eco" | "economy" | "budget" | "cheap" => RoutingProfile::Eco,
            "balanced" | "default" | "standard" => RoutingProfile::Balanced,
            "premium" | "pro" | "quality" | "best" => RoutingProfile::Premium,
            _ => RoutingProfile::Balanced,
        }
    }

    /// Get the preferred cost tier for this profile.
    pub fn preferred_cost_tier(&self) -> CostTier {
        match self {
            RoutingProfile::Eco => CostTier::Low,
            RoutingProfile::Balanced => CostTier::Medium,
            RoutingProfile::Premium => CostTier::High,
        }
    }

    /// Get the fallback cost tier for this profile when preferred is unavailable.
    pub fn fallback_cost_tier(&self) -> CostTier {
        match self {
            RoutingProfile::Eco => CostTier::Medium,
            RoutingProfile::Balanced => CostTier::Low,
            RoutingProfile::Premium => CostTier::Medium,
        }
    }
}

// ─── ModelModality ────────────────────────────────────────────────────────────

/// Supported modalities for multi-modal models.
///
/// Used to filter models based on the capabilities required for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelModality {
    /// Text generation (default for all LLMs).
    Text,
    /// Image understanding (vision).
    Image,
    /// Image generation (e.g., DALL-E, Stable Diffusion).
    ImageGeneration,
    /// Audio transcription (speech-to-text).
    AudioTranscription,
    /// Audio generation (text-to-speech).
    AudioGeneration,
    /// Video understanding.
    Video,
    /// Embedding generation for vector search.
    Embedding,
}

impl ModelModality {
    /// Check if this modality is a generation modality (produces output).
    pub fn is_generative(&self) -> bool {
        matches!(
            self,
            ModelModality::Text
                | ModelModality::ImageGeneration
                | ModelModality::AudioGeneration
                | ModelModality::Embedding
        )
    }

    /// Check if this modality is an understanding modality (consumes input).
    pub fn is_understanding(&self) -> bool {
        matches!(
            self,
            ModelModality::Text
                | ModelModality::Image
                | ModelModality::AudioTranscription
                | ModelModality::Video
        )
    }
}

// ─── ModelCapabilities ────────────────────────────────────────────────────────

/// Model capabilities for routing decisions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    /// Supports function/tool calling.
    #[serde(default)]
    pub tool_calling: bool,
    /// Supports structured JSON output.
    #[serde(default)]
    pub structured_output: bool,
    /// Supports streaming responses.
    #[serde(default = "default_streaming")]
    pub streaming: bool,
    /// Supports system prompts.
    #[serde(default = "default_system_prompt")]
    pub system_prompt: bool,
    /// Maximum output tokens (if known).
    #[serde(default)]
    pub max_output_tokens: Option<i32>,
}

fn default_streaming() -> bool {
    true
}
fn default_system_prompt() -> bool {
    true
}

impl ModelCapabilities {
    /// Create capabilities for a standard text-only model.
    pub fn text_only() -> Self {
        Self {
            tool_calling: false,
            structured_output: false,
            streaming: true,
            system_prompt: true,
            max_output_tokens: None,
        }
    }

    /// Create capabilities for a model with all features enabled.
    pub fn full_featured() -> Self {
        Self {
            tool_calling: true,
            structured_output: true,
            streaming: true,
            system_prompt: true,
            max_output_tokens: Some(4096),
        }
    }
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
    pub fn from_type_str(s: &str) -> Self {
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
    /// Modalities supported by this model (for multi-modal routing).
    #[serde(default = "default_modalities")]
    pub modalities: Vec<ModelModality>,
}

fn default_modalities() -> Vec<ModelModality> {
    vec![ModelModality::Text]
}

impl ModelTarget {
    pub fn new(
        provider_id: impl Into<String>,
        model: impl Into<String>,
        cost_tier: CostTier,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            model: model.into(),
            cost_tier,
            modalities: vec![ModelModality::Text],
        }
    }

    /// Create a model target with specific modalities.
    pub fn with_modalities(
        provider_id: impl Into<String>,
        model: impl Into<String>,
        cost_tier: CostTier,
        modalities: Vec<ModelModality>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            model: model.into(),
            cost_tier,
            modalities,
        }
    }

    /// Create a multi-modal model target (text + image).
    pub fn multimodal(
        provider_id: impl Into<String>,
        model: impl Into<String>,
        cost_tier: CostTier,
    ) -> Self {
        Self::with_modalities(
            provider_id,
            model,
            cost_tier,
            vec![ModelModality::Text, ModelModality::Image],
        )
    }

    /// Check if this target supports all the required modalities.
    pub fn supports_modalities(&self, required: &[ModelModality]) -> bool {
        required.iter().all(|m| self.modalities.contains(m))
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
    /// Active routing profile (eco/balanced/premium).
    #[serde(default)]
    pub profile: RoutingProfile,

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
            ModelTarget::new(
                "vercel-ai-gateway",
                "anthropic/claude-sonnet-4-5",
                CostTier::Medium,
            ),
        );
        aliases.insert(
            "claude-opus".to_string(),
            ModelTarget::new(
                "vercel-ai-gateway",
                "anthropic/claude-opus-4-5",
                CostTier::High,
            ),
        );
        aliases.insert(
            "claude-haiku".to_string(),
            ModelTarget::new(
                "vercel-ai-gateway",
                "anthropic/claude-haiku-4-5-20251001",
                CostTier::Low,
            ),
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
            ModelTarget::new(
                "vercel-ai-gateway",
                "google/gemini-2.0-flash",
                CostTier::Low,
            ),
        );

        let mut task_routes = HashMap::new();

        // Code: Claude Sonnet → GPT-4o → GPT-4o-mini (low-cost fallback)
        task_routes.insert(
            "code".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new(
                    "vercel-ai-gateway",
                    "anthropic/claude-sonnet-4-5",
                    CostTier::Medium,
                ),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o", CostTier::Medium),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o-mini", CostTier::Low),
            ]),
        );

        // General: GPT-4o-mini → Gemini Flash → fallback
        task_routes.insert(
            "general".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o-mini", CostTier::Low),
                ModelTarget::new(
                    "vercel-ai-gateway",
                    "google/gemini-2.0-flash",
                    CostTier::Low,
                ),
            ]),
        );

        // Fast: Gemini Flash → GPT-4o-mini
        task_routes.insert(
            "fast".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new(
                    "vercel-ai-gateway",
                    "google/gemini-2.0-flash",
                    CostTier::Low,
                ),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o-mini", CostTier::Low),
            ]),
        );

        // Creative: Claude Sonnet → GPT-4o
        task_routes.insert(
            "creative".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new(
                    "vercel-ai-gateway",
                    "anthropic/claude-sonnet-4-5",
                    CostTier::Medium,
                ),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o", CostTier::Medium),
            ]),
        );

        // Analysis: Claude Opus → Claude Sonnet → GPT-4o
        task_routes.insert(
            "analysis".to_string(),
            TaskRoute::new(vec![
                ModelTarget::new(
                    "vercel-ai-gateway",
                    "anthropic/claude-opus-4-5",
                    CostTier::High,
                ),
                ModelTarget::new(
                    "vercel-ai-gateway",
                    "anthropic/claude-sonnet-4-5",
                    CostTier::Medium,
                ),
                ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o", CostTier::Medium),
            ]),
        );

        Self {
            profile: RoutingProfile::Balanced,
            aliases,
            task_routes,
            fallback: ModelTarget::new("vercel-ai-gateway", "openai/gpt-4o-mini", CostTier::Low),
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
            profile: RoutingProfile::Balanced,
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
            modalities: vec![ModelModality::Text],
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

    /// Return the active routing profile.
    pub fn profile(&self) -> RoutingProfile {
        self.config.profile
    }

    /// Route based on the active profile's preferred cost tier.
    ///
    /// Uses the profile's preferred tier first, then falls back to the fallback tier,
    /// and finally to the primary target for the task.
    pub fn route_by_profile(&self, task: TaskType) -> &ModelTarget {
        let preferred = self.config.profile.preferred_cost_tier();
        let fallback = self.config.profile.fallback_cost_tier();

        // Try to find a target with the preferred cost tier
        if let Some(target) = self
            .targets_for(task)
            .into_iter()
            .find(|t| t.cost_tier == preferred)
        {
            return target;
        }

        // Fall back to the profile's fallback tier
        if let Some(target) = self
            .targets_for(task)
            .into_iter()
            .find(|t| t.cost_tier == fallback)
        {
            return target;
        }

        // Ultimate fallback: primary target
        self.primary_for(task)
    }

    /// Route to a model that supports all required modalities.
    ///
    /// Returns the first target for the task type that supports all required modalities,
    /// or `None` if no suitable model is found.
    pub fn route_for_modalities(
        &self,
        task: TaskType,
        required_modalities: &[ModelModality],
    ) -> Option<&ModelTarget> {
        self.targets_for(task)
            .into_iter()
            .find(|target| target.supports_modalities(required_modalities))
    }

    /// Combined routing: profile + modality + fallback.
    ///
    /// 1. Filter targets by required modalities
    /// 2. Find target matching profile's preferred tier
    /// 3. Fall back to profile's fallback tier
    /// 4. Fall back to any target that supports modalities
    pub fn route_with_modality_and_profile(
        &self,
        task: TaskType,
        required_modalities: &[ModelModality],
    ) -> Option<&ModelTarget> {
        let targets: Vec<_> = self
            .targets_for(task)
            .into_iter()
            .filter(|t| t.supports_modalities(required_modalities))
            .collect();

        if targets.is_empty() {
            return None;
        }

        let preferred = self.config.profile.preferred_cost_tier();
        let fallback = self.config.profile.fallback_cost_tier();

        // Try preferred tier
        if let Some(target) = targets.iter().find(|t| t.cost_tier == preferred) {
            return Some(*target);
        }

        // Try fallback tier
        if let Some(target) = targets.iter().find(|t| t.cost_tier == fallback) {
            return Some(*target);
        }

        // Return first matching target
        targets.into_iter().next()
    }

    /// Classify a task type from input text using heuristics.
    pub fn classify_task(input: &str) -> TaskType {
        let lower = input.to_lowercase();

        // Code indicators
        if lower.contains("code")
            || lower.contains("debug")
            || lower.contains("implement")
            || lower.contains("function")
            || lower.contains("class")
            || lower.contains("bug")
            || lower.contains("error")
            || lower.contains("fix")
            || lower.contains("refactor")
        {
            return TaskType::Code;
        }

        // Analysis indicators
        if lower.contains("analyze")
            || lower.contains("compare")
            || lower.contains("summarize")
            || lower.contains("explain")
            || lower.contains("why")
            || lower.contains("how does")
            || lower.contains("review")
        {
            return TaskType::Analysis;
        }

        // Creative indicators
        if lower.contains("write")
            || lower.contains("create")
            || lower.contains("design")
            || lower.contains("brainstorm")
            || lower.contains("idea")
            || lower.contains("story")
            || lower.contains("poem")
        {
            return TaskType::Creative;
        }

        // Fast indicators (short queries)
        if input.len() < 50 && !lower.contains('?') {
            return TaskType::Fast;
        }

        TaskType::General
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
        assert_eq!(TaskType::from_type_str("code"), TaskType::Code);
        assert_eq!(TaskType::from_type_str("coding"), TaskType::Code);
        assert_eq!(TaskType::from_type_str("general"), TaskType::General);
        assert_eq!(TaskType::from_type_str("FAST"), TaskType::Fast);
        assert_eq!(TaskType::from_type_str("creative"), TaskType::Creative);
        assert_eq!(TaskType::from_type_str("analysis"), TaskType::Analysis);
    }

    #[test]
    fn task_type_from_str_unknown_falls_back_to_other() {
        assert_eq!(TaskType::from_type_str("unknown-type"), TaskType::Other);
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

    // ── RoutingProfile tests ───────────────────────────────────────────────

    #[test]
    fn routing_profile_from_str() {
        assert_eq!(RoutingProfile::from_str("eco"), RoutingProfile::Eco);
        assert_eq!(RoutingProfile::from_str("economy"), RoutingProfile::Eco);
        assert_eq!(RoutingProfile::from_str("ECO"), RoutingProfile::Eco);
        assert_eq!(
            RoutingProfile::from_str("balanced"),
            RoutingProfile::Balanced
        );
        assert_eq!(
            RoutingProfile::from_str("default"),
            RoutingProfile::Balanced
        );
        assert_eq!(RoutingProfile::from_str("premium"), RoutingProfile::Premium);
        assert_eq!(RoutingProfile::from_str("pro"), RoutingProfile::Premium);
        assert_eq!(
            RoutingProfile::from_str("unknown"),
            RoutingProfile::Balanced
        );
    }

    #[test]
    fn routing_profile_cost_tier_mapping() {
        assert_eq!(RoutingProfile::Eco.preferred_cost_tier(), CostTier::Low);
        assert_eq!(RoutingProfile::Eco.fallback_cost_tier(), CostTier::Medium);

        assert_eq!(
            RoutingProfile::Balanced.preferred_cost_tier(),
            CostTier::Medium
        );
        assert_eq!(RoutingProfile::Balanced.fallback_cost_tier(), CostTier::Low);

        assert_eq!(
            RoutingProfile::Premium.preferred_cost_tier(),
            CostTier::High
        );
        assert_eq!(
            RoutingProfile::Premium.fallback_cost_tier(),
            CostTier::Medium
        );
    }

    // ── ModelModality tests ────────────────────────────────────────────────

    #[test]
    fn model_modality_is_generative() {
        assert!(ModelModality::Text.is_generative());
        assert!(ModelModality::ImageGeneration.is_generative());
        assert!(ModelModality::AudioGeneration.is_generative());
        assert!(ModelModality::Embedding.is_generative());
        assert!(!ModelModality::Image.is_generative());
        assert!(!ModelModality::AudioTranscription.is_generative());
        assert!(!ModelModality::Video.is_generative());
    }

    #[test]
    fn model_modality_is_understanding() {
        assert!(ModelModality::Text.is_understanding());
        assert!(ModelModality::Image.is_understanding());
        assert!(ModelModality::AudioTranscription.is_understanding());
        assert!(ModelModality::Video.is_understanding());
        assert!(!ModelModality::ImageGeneration.is_understanding());
        assert!(!ModelModality::AudioGeneration.is_understanding());
        assert!(!ModelModality::Embedding.is_understanding());
    }

    // ── ModelTarget modality tests ──────────────────────────────────────────

    #[test]
    fn model_target_supports_modalities() {
        let text_only = ModelTarget::new("test", "model", CostTier::Medium);
        assert!(text_only.supports_modalities(&[ModelModality::Text]));
        assert!(!text_only.supports_modalities(&[ModelModality::Image]));

        let multimodal = ModelTarget::multimodal("test", "model", CostTier::Medium);
        assert!(multimodal.supports_modalities(&[ModelModality::Text]));
        assert!(multimodal.supports_modalities(&[ModelModality::Image]));
        assert!(multimodal.supports_modalities(&[ModelModality::Text, ModelModality::Image]));
        assert!(!multimodal.supports_modalities(&[ModelModality::Video]));
    }

    // ── Profile-based routing tests ────────────────────────────────────────

    #[test]
    fn route_by_profile_balanced() {
        let router = vercel_router();
        let target = router.route_by_profile(TaskType::Code);
        // Balanced profile prefers Medium tier for Code
        assert_eq!(target.cost_tier, CostTier::Medium);
    }

    #[test]
    fn route_by_profile_uses_fallback_tier() {
        // Fast tasks only have Low tier models in the default config
        let router = vercel_router();
        let target = router.route_by_profile(TaskType::Fast);
        // Should get a Low tier model since that's all that's available
        assert_eq!(target.cost_tier, CostTier::Low);
    }

    // ── Modality-aware routing tests ────────────────────────────────────────

    #[test]
    fn route_for_modalities_text_only() {
        let router = vercel_router();
        let target = router.route_for_modalities(TaskType::Code, &[ModelModality::Text]);
        assert!(target.is_some());
        let target = target.unwrap();
        assert!(target.supports_modalities(&[ModelModality::Text]));
    }

    #[test]
    fn route_for_modalities_multimodal() {
        let router = vercel_router();
        // Default config models are text-only, so this should return None
        let target = router.route_for_modalities(TaskType::Code, &[ModelModality::Image]);
        assert!(target.is_none());
    }

    // ── Task classification tests ───────────────────────────────────────────

    #[test]
    fn classify_task_code() {
        assert_eq!(ModelRouter::classify_task("Fix this bug"), TaskType::Code);
        assert_eq!(
            ModelRouter::classify_task("Implement a new feature"),
            TaskType::Code
        );
        assert_eq!(
            ModelRouter::classify_task("Debug the error"),
            TaskType::Code
        );
        assert_eq!(
            ModelRouter::classify_task("Refactor the class"),
            TaskType::Code
        );
    }

    #[test]
    fn classify_task_analysis() {
        assert_eq!(
            ModelRouter::classify_task("Analyze the data"),
            TaskType::Analysis
        );
        assert_eq!(
            ModelRouter::classify_task("Summarize this text"),
            TaskType::Analysis
        );
        assert_eq!(
            ModelRouter::classify_task("Why did this happen?"),
            TaskType::Analysis
        );
        assert_eq!(
            ModelRouter::classify_task("Explain how does it work"),
            TaskType::Analysis
        );
    }

    #[test]
    fn classify_task_creative() {
        assert_eq!(
            ModelRouter::classify_task("Write a story"),
            TaskType::Creative
        );
        assert_eq!(
            ModelRouter::classify_task("Create a design"),
            TaskType::Creative
        );
        assert_eq!(
            ModelRouter::classify_task("Brainstorm ideas"),
            TaskType::Creative
        );
    }

    #[test]
    fn classify_task_fast() {
        assert_eq!(ModelRouter::classify_task("Hello"), TaskType::Fast);
        assert_eq!(ModelRouter::classify_task("Ok thanks"), TaskType::Fast);
    }

    #[test]
    fn classify_task_general() {
        // Use longer inputs to avoid triggering the Fast heuristic (< 50 chars without ?)
        assert_eq!(
            ModelRouter::classify_task("Tell me about the current weather conditions today"),
            TaskType::General
        );
        assert_eq!(
            ModelRouter::classify_task(
                "What is the capital city of France and can you tell me more about it?"
            ),
            TaskType::General
        );
    }
}
