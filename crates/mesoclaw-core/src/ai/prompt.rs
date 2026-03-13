use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::debug;

use crate::Result;
use crate::ai::context::{BootContext, ContextDomain};
use crate::config::AppConfig;
use crate::identity::SoulLoader;
use crate::memory::traits::Memory;
use crate::skills::SkillRegistry;
use crate::user::UserLearner;

#[cfg(feature = "channels")]
use crate::channels::registry::ChannelRegistry;
#[cfg(feature = "scheduler")]
use crate::scheduler::TokioScheduler;

// ============================================================================
// Core Types
// ============================================================================

/// Which layer a prompt fragment belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PromptSection {
    /// Layer 0: core identity (~80 tokens)
    CoreIdentity,
    /// Layer 1: runtime + state + reasoning axioms (~60 tokens)
    RuntimeState,
    /// Layer 2: dynamic context (memories, observations, skills, domain details)
    DynamicContext,
    /// Layer 3: overrides (agent_system_prompt, conversation summary)
    Overrides,
}

/// A prompt section contributed by a strategy plugin.
#[derive(Debug, Clone)]
pub struct PromptFragment {
    pub section: PromptSection,
    pub content: String,
    /// Lower number = higher priority (trimmed last). Range: 1-10.
    pub priority: u8,
}

/// All inputs needed for prompt assembly.
#[derive(Debug, Clone)]
pub struct AssemblyRequest {
    pub boot_context: BootContext,
    pub model_display: String,
    pub session_id: Option<String>,
    pub user_message: Option<String>,
    pub conversation_summary: Option<String>,
    /// Hint for channel-originated requests.
    pub channel_hint: Option<String>,
    /// Number of tools registered.
    pub tool_count: usize,
    /// Number of skills loaded.
    pub skill_count: usize,
    /// App version string.
    pub version: String,
}

// ============================================================================
// Traits
// ============================================================================

/// Base compositor: assembles the system preamble.
#[async_trait]
pub trait PromptStrategy: Send + Sync {
    async fn assemble(&self, request: &AssemblyRequest) -> Result<String>;
}

/// Pluggable prompt contributor. Each plugin can contribute fragments.
#[async_trait]
pub trait PromptPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn domains(&self) -> Vec<ContextDomain>;
    async fn contribute(&self, request: &AssemblyRequest) -> Result<Vec<PromptFragment>>;
}

// ============================================================================
// Token Budget
// ============================================================================

pub struct TokenBudget {
    pub max_preamble_tokens: usize,
}

impl TokenBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_preamble_tokens: max_tokens,
        }
    }

    /// Estimate tokens using word count with safety margin.
    pub fn estimate_tokens(content: &str) -> usize {
        let word_count = content.split_whitespace().count();
        (word_count as f64 * 1.3).ceil() as usize
    }
}

// ============================================================================
// PromptStrategyRegistry
// ============================================================================

/// Registry that holds active plugins and orchestrates assembly.
pub struct PromptStrategyRegistry {
    base: Arc<dyn PromptStrategy>,
    plugins: RwLock<Vec<Arc<dyn PromptPlugin>>>,
    config: Arc<AppConfig>,
}

impl PromptStrategyRegistry {
    pub fn new(base: Arc<dyn PromptStrategy>, config: Arc<AppConfig>) -> Self {
        Self {
            base,
            plugins: RwLock::new(Vec::new()),
            config,
        }
    }

    pub async fn register_plugin(&self, plugin: Arc<dyn PromptPlugin>) {
        let mut plugins = self.plugins.write().await;
        // Replace if same ID already registered
        plugins.retain(|p| p.id() != plugin.id());
        debug!("Registering prompt plugin: {}", plugin.id());
        plugins.push(plugin);
    }

    pub async fn unregister_plugin(&self, id: &str) {
        let mut plugins = self.plugins.write().await;
        plugins.retain(|p| p.id() != id);
        debug!("Unregistered prompt plugin: {id}");
    }

    /// Merge base output + plugin fragments, apply token budget.
    async fn assemble_with_plugins(&self, request: &AssemblyRequest) -> Result<String> {
        // 1. Get base preamble
        let base_output = self.base.assemble(request).await?;

        // 2. Collect plugin fragments
        let plugins = self.plugins.read().await;
        let mut dynamic_fragments: Vec<PromptFragment> = Vec::new();

        // Detect relevant domains from user message
        let domains = request
            .user_message
            .as_deref()
            .map(crate::ai::context::detect_relevant_domains)
            .unwrap_or_default();

        for plugin in plugins.iter() {
            let plugin_domains = plugin.domains();
            // Plugin is relevant if: it has no domain filter (always active) OR its domain matches
            let relevant =
                plugin_domains.is_empty() || plugin_domains.iter().any(|d| domains.contains(d));

            if relevant {
                match plugin.contribute(request).await {
                    Ok(fragments) => dynamic_fragments.extend(fragments),
                    Err(e) => {
                        debug!("Plugin '{}' failed (non-fatal): {e}", plugin.id());
                    }
                }
            }
        }

        // 3. Sort by priority (lower = higher priority = trimmed last)
        dynamic_fragments.sort_by_key(|f| f.priority);

        // 4. Merge base + plugin fragments
        let mut parts = vec![base_output];
        for frag in &dynamic_fragments {
            if !frag.content.trim().is_empty() {
                parts.push(frag.content.clone());
            }
        }

        let mut result = parts.join("\n\n");

        // 5. Apply token budget (trim from lowest priority = highest number)
        let budget = TokenBudget::new(self.config.prompt_max_preamble_tokens);
        let estimated = TokenBudget::estimate_tokens(&result);

        if estimated > budget.max_preamble_tokens && !dynamic_fragments.is_empty() {
            debug!(
                "Preamble over budget ({estimated} > {}), trimming dynamic context",
                budget.max_preamble_tokens
            );
            // Rebuild: base + fragments, dropping from tail (lowest priority)
            let base_output_2 = parts.first().cloned().unwrap_or_default();
            let mut trimmed_parts = vec![base_output_2];
            let mut current_tokens = trimmed_parts
                .first()
                .map_or(0, |s| TokenBudget::estimate_tokens(s));

            for frag in &dynamic_fragments {
                if frag.content.trim().is_empty() {
                    continue;
                }
                let frag_tokens = TokenBudget::estimate_tokens(&frag.content);
                if current_tokens + frag_tokens <= budget.max_preamble_tokens {
                    trimmed_parts.push(frag.content.clone());
                    current_tokens += frag_tokens;
                } else {
                    debug!(
                        "Trimmed fragment (priority={}, tokens={frag_tokens})",
                        frag.priority
                    );
                }
            }

            result = trimmed_parts.join("\n\n");
        }

        debug!(
            "Final preamble: ~{} tokens",
            TokenBudget::estimate_tokens(&result)
        );
        Ok(result)
    }
}

#[async_trait]
impl PromptStrategy for PromptStrategyRegistry {
    async fn assemble(&self, request: &AssemblyRequest) -> Result<String> {
        self.assemble_with_plugins(request).await
    }
}

// ============================================================================
// CompactStrategy (base compositor for Layer 0 + 1 + 3)
// ============================================================================

pub struct CompactStrategy {
    config: Arc<AppConfig>,
    boot_context: BootContext,
}

impl CompactStrategy {
    pub fn new(config: Arc<AppConfig>, boot_context: BootContext) -> Self {
        Self {
            config,
            boot_context,
        }
    }

    fn build_core_identity(&self, request: &AssemblyRequest) -> String {
        let display_region = self
            .config
            .user_location
            .as_deref()
            .unwrap_or(&self.boot_context.region);

        let tz_info = self
            .config
            .user_timezone
            .as_deref()
            .or(self.boot_context.user_timezone.as_deref())
            .map(|tz| format!(" | TZ: {tz}"))
            .unwrap_or_default();

        format!(
            "# MesoClaw v{version}\n\
             AI assistant. Local-first, privacy-respecting. Direct, accurate, actionable.\n\
             Location: {region}{tz}\n\
             OS: {os} {arch} | User: {user} | Shell: {shell}\n\
             Tools: {tools} | Skills: {skills}",
            version = request.version,
            region = display_region,
            tz = tz_info,
            os = self.boot_context.os,
            arch = self.boot_context.arch,
            user = self.boot_context.username,
            shell = self.boot_context.shell.as_deref().unwrap_or("unknown"),
            tools = request.tool_count,
            skills = request.skill_count,
        )
    }

    fn build_runtime_and_state(&self, request: &AssemblyRequest) -> String {
        let now = chrono::Local::now();
        let date_str = now.format("%Y-%m-%dT%H:%M:%S %a").to_string();

        let session_str = request.session_id.as_deref().unwrap_or("none");

        let mut lines = vec![format!(
            "Date: {date_str} | Model: {model} | Session: {session}",
            model = request.model_display,
            session = session_str,
        )];

        // Compact reasoning axioms (replaces ~300 token protocol)
        lines.push(
            "Rules: Enrich tool args with context. Recover on failure (2 retries). \
             Use shell for path discovery. Act, don't describe. \
             Use multiple tool calls for multi-step tasks. Avoid redundant identical calls."
                .into(),
        );

        lines.join("\n")
    }

    fn build_overrides(&self, request: &AssemblyRequest) -> String {
        let mut parts = Vec::new();

        if let Some(ref override_prompt) = self.config.agent_system_prompt
            && !override_prompt.trim().is_empty()
        {
            parts.push(override_prompt.clone());
        }

        if let Some(ref summary) = request.conversation_summary
            && !summary.trim().is_empty()
        {
            parts.push(format!("## Conversation Summary\n{summary}"));
        }

        parts.join("\n\n")
    }
}

#[async_trait]
impl PromptStrategy for CompactStrategy {
    async fn assemble(&self, request: &AssemblyRequest) -> Result<String> {
        let mut parts = Vec::new();

        // Layer 0: Core Identity
        parts.push(self.build_core_identity(request));

        // Layer 1: Runtime + State + Reasoning
        parts.push(self.build_runtime_and_state(request));

        // Layer 3: Overrides
        let overrides = self.build_overrides(request);
        if !overrides.is_empty() {
            parts.push(overrides);
        }

        Ok(parts.join("\n\n"))
    }
}

// ============================================================================
// LegacyStrategy (backward compat: delegates to PromptComposer + ContextEngine)
// ============================================================================

pub struct LegacyStrategy {
    soul_loader: Arc<SoulLoader>,
    user_learner: Arc<UserLearner>,
    config: Arc<AppConfig>,
    skill_registry: Arc<SkillRegistry>,
}

impl LegacyStrategy {
    pub fn new(
        soul_loader: Arc<SoulLoader>,
        user_learner: Arc<UserLearner>,
        config: Arc<AppConfig>,
        skill_registry: Arc<SkillRegistry>,
    ) -> Self {
        Self {
            soul_loader,
            user_learner,
            config,
            skill_registry,
        }
    }
}

#[async_trait]
impl PromptStrategy for LegacyStrategy {
    async fn assemble(&self, _request: &AssemblyRequest) -> Result<String> {
        // Delegate to existing PromptComposer for backward compat
        let identity = self.soul_loader.get().await;
        let observations = self.user_learner.build_context().await.unwrap_or_default();

        // Get active skills
        let all_skills = self.skill_registry.list().await;
        let mut active_skills = Vec::new();
        for info in &all_skills {
            if info.enabled
                && let Ok(skill) = self.skill_registry.get(&info.id).await
            {
                active_skills.push((skill.name.clone(), skill.content.clone()));
            }
        }

        let preamble = crate::identity::PromptComposer::compose(
            &identity,
            &active_skills,
            &observations,
            &self.config,
        );

        Ok(preamble)
    }
}

// ============================================================================
// Built-in Plugins
// ============================================================================

/// Contributes cross-session memories recalled by keyword matching.
pub struct MemoryPlugin {
    memory: Arc<dyn Memory>,
}

impl MemoryPlugin {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl PromptPlugin for MemoryPlugin {
    fn id(&self) -> &str {
        "memory"
    }

    fn domains(&self) -> Vec<ContextDomain> {
        vec![] // always active
    }

    async fn contribute(&self, request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
        let prompt = request.user_message.as_deref().unwrap_or("");
        if prompt.is_empty() {
            return Ok(vec![]);
        }

        let memories = self.memory.recall(prompt, 5, 0).await.unwrap_or_default();
        if memories.is_empty() {
            return Ok(vec![]);
        }

        let mut lines = vec!["[Relevant Memories]".to_string()];
        for mem in &memories {
            lines.push(format!("- {}", mem.content));
        }

        Ok(vec![PromptFragment {
            section: PromptSection::DynamicContext,
            content: lines.join("\n"),
            priority: 3,
        }])
    }
}

/// Contributes user observations/preferences.
pub struct UserObservationsPlugin {
    user_learner: Arc<UserLearner>,
}

impl UserObservationsPlugin {
    pub fn new(user_learner: Arc<UserLearner>) -> Self {
        Self { user_learner }
    }
}

#[async_trait]
impl PromptPlugin for UserObservationsPlugin {
    fn id(&self) -> &str {
        "user-observations"
    }

    fn domains(&self) -> Vec<ContextDomain> {
        vec![] // always active
    }

    async fn contribute(&self, _request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
        let context = self.user_learner.build_context().await.unwrap_or_default();
        if context.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![PromptFragment {
            section: PromptSection::DynamicContext,
            content: format!("[User Preferences]\n{context}"),
            priority: 3,
        }])
    }
}

/// Bridges SkillRegistry into the plugin system.
/// Maps skill categories to priorities.
pub struct SkillsPlugin {
    skill_registry: Arc<SkillRegistry>,
}

impl SkillsPlugin {
    pub fn new(skill_registry: Arc<SkillRegistry>) -> Self {
        Self { skill_registry }
    }

    fn category_to_priority(category: &str) -> u8 {
        match category {
            "system" => 2,
            "meta" => 3,
            "general" => 5,
            _ => 4, // user-defined
        }
    }
}

#[async_trait]
impl PromptPlugin for SkillsPlugin {
    fn id(&self) -> &str {
        "skills"
    }

    fn domains(&self) -> Vec<ContextDomain> {
        vec![] // always active
    }

    async fn contribute(&self, request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
        let detected_domains = request
            .user_message
            .as_deref()
            .map(crate::ai::context::detect_relevant_domains)
            .unwrap_or_default();

        let surface = request.channel_hint.as_deref().unwrap_or("desktop");

        let all_skills = self.skill_registry.list().await;
        let mut fragments = Vec::new();

        for info in &all_skills {
            if !info.enabled {
                continue;
            }
            if let Ok(skill) = self.skill_registry.get(&info.id).await {
                // Check surface filter
                if let Some(ref skill_surface) = skill.surface
                    && skill_surface != "all"
                    && skill_surface != surface
                {
                    continue;
                }

                // Check domain filter (None = always active)
                if let Some(ref domain_str) = skill.domain
                    && let Some(d) = ContextDomain::from_domain_str(domain_str)
                    && !detected_domains.contains(&d)
                {
                    continue;
                }

                fragments.push(PromptFragment {
                    section: PromptSection::DynamicContext,
                    content: format!("### Skill: {}\n{}", skill.name, skill.content),
                    priority: Self::category_to_priority(&skill.category),
                });
            }
        }

        Ok(fragments)
    }
}

/// Contributes self-evolution learned rules.
pub struct LearnedRulesPlugin {
    db: crate::db::DbPool,
}

impl LearnedRulesPlugin {
    pub fn new(db: crate::db::DbPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PromptPlugin for LearnedRulesPlugin {
    fn id(&self) -> &str {
        "learned-rules"
    }

    fn domains(&self) -> Vec<ContextDomain> {
        vec![] // always active when registered
    }

    async fn contribute(&self, request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
        // Load agent learned rules from DB
        let pool = self.db.clone();
        let domains = request
            .user_message
            .as_deref()
            .map(crate::ai::context::detect_relevant_domains)
            .unwrap_or_default();

        let categories = crate::ai::context::domains_to_rule_categories(&domains);

        let rules = crate::db::with_db(&pool, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT category, rule FROM agent_learned_rules \
                 WHERE active = 1 AND category IN (SELECT value FROM json_each(?1)) \
                 ORDER BY category, updated_at DESC LIMIT 10",
            )?;
            let rows = stmt
                .query_map(
                    rusqlite::params![serde_json::to_string(&categories).unwrap_or_default()],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )?
                .filter_map(|r| r.ok())
                .collect::<Vec<_>>();
            Ok(rows)
        })
        .await
        .unwrap_or_default();

        if rules.is_empty() {
            return Ok(vec![]);
        }

        let mut lines = vec!["[Learned Rules]".to_string()];
        for (cat, rule) in &rules {
            lines.push(format!("- [{cat}] {rule}"));
        }

        Ok(vec![PromptFragment {
            section: PromptSection::DynamicContext,
            content: lines.join("\n"),
            priority: 6,
        }])
    }
}

/// Contributes channel state and contacts when channels feature is active.
#[cfg(feature = "channels")]
pub struct ChannelContextPlugin {
    channel_registry: Arc<ChannelRegistry>,
    db: crate::db::DbPool,
}

#[cfg(feature = "channels")]
impl ChannelContextPlugin {
    pub fn new(channel_registry: Arc<ChannelRegistry>, db: crate::db::DbPool) -> Self {
        Self {
            channel_registry,
            db,
        }
    }
}

#[cfg(feature = "channels")]
#[async_trait]
impl PromptPlugin for ChannelContextPlugin {
    fn id(&self) -> &str {
        "channels"
    }

    fn domains(&self) -> Vec<ContextDomain> {
        vec![ContextDomain::Channels]
    }

    async fn contribute(&self, _request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
        let channels = self.channel_registry.list();
        if channels.is_empty() {
            return Ok(vec![]);
        }

        let mut lines = vec!["### Channels".to_string()];
        for name in &channels {
            if let Some(ch) = self.channel_registry.get_channel(name) {
                let status = ch.status();
                let contacts =
                    crate::channels::contacts::query_channel_contacts(&self.db, name).await;
                let contact_info = match contacts {
                    Ok(ref c) if !c.is_empty() => {
                        let items: Vec<String> = c
                            .iter()
                            .map(|ct| format!("{} (id: {})", ct.label, ct.recipient_id))
                            .collect();
                        format!("\n  Contacts: {}", items.join(", "))
                    }
                    _ => String::new(),
                };
                lines.push(format!("- {name}: {status:?}{contact_info}"));
            }
        }

        Ok(vec![PromptFragment {
            section: PromptSection::DynamicContext,
            content: lines.join("\n"),
            priority: 4,
        }])
    }
}

/// Contributes scheduler job state.
#[cfg(feature = "scheduler")]
pub struct SchedulerContextPlugin {
    scheduler: Arc<TokioScheduler>,
}

#[cfg(feature = "scheduler")]
impl SchedulerContextPlugin {
    pub fn new(scheduler: Arc<TokioScheduler>) -> Self {
        Self { scheduler }
    }
}

#[cfg(feature = "scheduler")]
#[async_trait]
impl PromptPlugin for SchedulerContextPlugin {
    fn id(&self) -> &str {
        "scheduler"
    }

    fn domains(&self) -> Vec<ContextDomain> {
        vec![ContextDomain::Scheduler]
    }

    async fn contribute(&self, _request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
        use crate::scheduler::traits::Scheduler;
        let jobs = self.scheduler.list_jobs().await;
        if jobs.is_empty() {
            return Ok(vec![]);
        }

        let active_count = jobs.iter().filter(|j| j.enabled).count();
        let mut lines = vec![format!("### Scheduled Jobs ({active_count} active)")];
        for job in &jobs {
            if job.enabled {
                let schedule_str = match &job.schedule {
                    crate::scheduler::traits::Schedule::Cron { expr } => format!("cron: {expr}"),
                    crate::scheduler::traits::Schedule::Interval { secs } => {
                        format!("every {secs}s")
                    }
                };
                lines.push(format!("- {} ({schedule_str})", job.name));
            }
        }

        Ok(vec![PromptFragment {
            section: PromptSection::DynamicContext,
            content: lines.join("\n"),
            priority: 4,
        }])
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::context::BootContext;

    fn test_boot_context() -> BootContext {
        BootContext {
            os: "linux 6.17".into(),
            arch: "x86_64".into(),
            hostname: "testhost".into(),
            locale: "en_US.UTF-8".into(),
            region: "Toronto, Canada".into(),
            home_dir: Some("/home/test".into()),
            username: "testuser".into(),
            shell: Some("/bin/bash".into()),
            desktop_path: None,
            downloads_path: None,
            data_dir: None,
            working_dir: None,
            user_timezone: Some("America/Toronto".into()),
            user_location: Some("Toronto, Canada".into()),
        }
    }

    fn test_request() -> AssemblyRequest {
        AssemblyRequest {
            boot_context: test_boot_context(),
            model_display: "claude-sonnet-4-6".into(),
            session_id: Some("test-session".into()),
            user_message: Some("hello".into()),
            conversation_summary: None,
            channel_hint: None,
            tool_count: 13,
            skill_count: 3,
            version: "0.0.13".into(),
        }
    }

    // 8.13.1 — CompactStrategy produces all layers in correct order
    #[tokio::test]
    async fn compact_strategy_produces_all_layers() {
        let config = Arc::new(AppConfig::default());
        let strategy = CompactStrategy::new(config, test_boot_context());
        let request = test_request();

        let result = strategy.assemble(&request).await.unwrap();

        // Layer 0: Core identity
        assert!(result.contains("# MesoClaw v0.0.13"));
        assert!(result.contains("AI assistant"));
        assert!(result.contains("Tools: 13"));
        assert!(result.contains("Skills: 3"));

        // Layer 1: Runtime
        assert!(result.contains("Model: claude-sonnet-4-6"));
        assert!(result.contains("Session: test-session"));
        assert!(result.contains("Rules: Enrich"));
    }

    // 8.13.2 — CompactStrategy includes location and timezone
    #[tokio::test]
    async fn compact_strategy_includes_location() {
        let mut config = AppConfig::default();
        config.user_location = Some("Toronto, Canada".into());
        config.user_timezone = Some("America/Toronto".into());
        let config = Arc::new(config);
        let strategy = CompactStrategy::new(config, test_boot_context());

        let result = strategy.assemble(&test_request()).await.unwrap();
        assert!(result.contains("Toronto, Canada"));
        assert!(result.contains("America/Toronto"));
    }

    // 8.13.3 — CompactStrategy includes overrides when set
    #[tokio::test]
    async fn compact_strategy_includes_overrides() {
        let mut config = AppConfig::default();
        config.agent_system_prompt = Some("Be extra helpful".into());
        let config = Arc::new(config);
        let strategy = CompactStrategy::new(config, test_boot_context());

        let result = strategy.assemble(&test_request()).await.unwrap();
        assert!(result.contains("Be extra helpful"));
    }

    // 8.13.4 — CompactStrategy includes conversation summary in overrides
    #[tokio::test]
    async fn compact_strategy_includes_conversation_summary() {
        let config = Arc::new(AppConfig::default());
        let strategy = CompactStrategy::new(config, test_boot_context());
        let mut request = test_request();
        request.conversation_summary = Some("User was asking about weather".into());

        let result = strategy.assemble(&request).await.unwrap();
        assert!(result.contains("Conversation Summary"));
        assert!(result.contains("User was asking about weather"));
    }

    // 8.13.5 — Token budget estimation works
    #[test]
    fn token_budget_estimation() {
        let text = "hello world this is a test";
        let estimate = TokenBudget::estimate_tokens(text);
        // 6 words * 1.3 = 7.8, ceil = 8
        assert_eq!(estimate, 8);
    }

    // 8.13.6 — Token budget trims over-budget fragments
    #[tokio::test]
    async fn token_budget_trims_fragments() {
        let mut config = AppConfig::default();
        config.prompt_max_preamble_tokens = 50; // very low budget
        let config = Arc::new(config.clone());
        let strategy = CompactStrategy::new(config.clone(), test_boot_context());
        let registry = PromptStrategyRegistry::new(Arc::new(strategy), config.clone());

        // Register a plugin that contributes a large fragment
        struct BigPlugin;

        #[async_trait]
        impl PromptPlugin for BigPlugin {
            fn id(&self) -> &str {
                "big"
            }
            fn domains(&self) -> Vec<ContextDomain> {
                vec![]
            }
            async fn contribute(&self, _request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
                Ok(vec![PromptFragment {
                    section: PromptSection::DynamicContext,
                    content: "word ".repeat(200), // 200 words = ~260 tokens
                    priority: 8,
                }])
            }
        }

        registry.register_plugin(Arc::new(BigPlugin)).await;

        let result = registry.assemble(&test_request()).await.unwrap();
        // The big fragment should be trimmed since it would exceed budget
        let estimated = TokenBudget::estimate_tokens(&result);
        // Result should be smaller than 50 + some overhead since big plugin was trimmed
        // (base compact is ~80 tokens, so it will exceed 50 regardless, but the big plugin should be gone)
        assert!(
            !result.contains(&"word ".repeat(200)),
            "Big fragment should be trimmed"
        );
        assert!(
            estimated < 260,
            "Should be much less than the big fragment alone"
        );
    }

    // 8.13.7 — Registry registers and unregisters plugins
    #[tokio::test]
    async fn registry_register_unregister() {
        let config = Arc::new(AppConfig::default());
        let strategy = CompactStrategy::new(config.clone(), test_boot_context());
        let registry = PromptStrategyRegistry::new(Arc::new(strategy), config);

        struct TestPlugin;

        #[async_trait]
        impl PromptPlugin for TestPlugin {
            fn id(&self) -> &str {
                "test"
            }
            fn domains(&self) -> Vec<ContextDomain> {
                vec![]
            }
            async fn contribute(&self, _request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
                Ok(vec![PromptFragment {
                    section: PromptSection::DynamicContext,
                    content: "TEST_MARKER".into(),
                    priority: 5,
                }])
            }
        }

        // Register
        registry.register_plugin(Arc::new(TestPlugin)).await;
        let result = registry.assemble(&test_request()).await.unwrap();
        assert!(result.contains("TEST_MARKER"));

        // Unregister
        registry.unregister_plugin("test").await;
        let result = registry.assemble(&test_request()).await.unwrap();
        assert!(!result.contains("TEST_MARKER"));
    }

    // 8.13.8 — MemoryPlugin contributes memories when present
    #[tokio::test]
    async fn memory_plugin_contributes() {
        let memory = Arc::new(crate::memory::in_memory_store::InMemoryStore::new());
        memory
            .store(
                "dark-mode-pref",
                "User prefers dark mode",
                crate::memory::traits::MemoryCategory::Core,
            )
            .await
            .unwrap();

        let plugin = MemoryPlugin::new(memory);
        let mut request = test_request();
        request.user_message = Some("dark mode".into());

        let fragments = plugin.contribute(&request).await.unwrap();
        assert!(!fragments.is_empty());
        assert!(fragments[0].content.contains("dark mode"));
    }

    // 8.13.9 — MemoryPlugin returns empty for empty prompt
    #[tokio::test]
    async fn memory_plugin_empty_prompt() {
        let memory = Arc::new(crate::memory::in_memory_store::InMemoryStore::new());
        let plugin = MemoryPlugin::new(memory);
        let mut request = test_request();
        request.user_message = Some(String::new());

        let fragments = plugin.contribute(&request).await.unwrap();
        assert!(fragments.is_empty());
    }

    // 8.13.10 — UserObservationsPlugin contributes when observations exist
    #[tokio::test]
    async fn user_observations_plugin_empty_when_no_observations() {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&db_path).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        let config = AppConfig::default();
        let learner = Arc::new(UserLearner::new(pool, &config));
        let plugin = UserObservationsPlugin::new(learner);

        let fragments = plugin.contribute(&test_request()).await.unwrap();
        assert!(fragments.is_empty());
    }

    // 8.13.11 — SkillsPlugin maps categories to priorities
    #[test]
    fn skills_plugin_category_priorities() {
        assert_eq!(SkillsPlugin::category_to_priority("system"), 2);
        assert_eq!(SkillsPlugin::category_to_priority("meta"), 3);
        assert_eq!(SkillsPlugin::category_to_priority("general"), 5);
        assert_eq!(SkillsPlugin::category_to_priority("custom"), 4);
    }

    // 8.13.12 — SkillsPlugin contributes active skills
    #[tokio::test]
    async fn skills_plugin_contributes_active_skills() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        let registry = Arc::new(SkillRegistry::new(&skills_dir, 100_000).unwrap());

        let plugin = SkillsPlugin::new(registry.clone());
        let fragments = plugin.contribute(&test_request()).await.unwrap();
        // Bundled skills should be present (3 bundled by default)
        assert!(!fragments.is_empty(), "Should contribute bundled skills");
    }

    // 8.13.13 — LegacyStrategy produces output matching PromptComposer
    #[tokio::test]
    async fn legacy_strategy_produces_output() {
        let dir = tempfile::TempDir::new().unwrap();
        let identity_dir = dir.path().join("identity");
        let skills_dir = dir.path().join("skills");
        let db_path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&db_path).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        let config = Arc::new(AppConfig::default());
        let soul_loader = Arc::new(SoulLoader::new(&identity_dir).unwrap());
        let user_learner = Arc::new(UserLearner::new(pool, &config));
        let skill_registry = Arc::new(SkillRegistry::new(&skills_dir, 100_000).unwrap());

        let strategy = LegacyStrategy::new(soul_loader, user_learner, config, skill_registry);

        let result = strategy.assemble(&test_request()).await.unwrap();
        // Should contain identity elements
        assert!(result.contains("MesoClaw"));
    }

    // 8.13.14 — Compact output is significantly smaller than legacy
    #[tokio::test]
    async fn compact_smaller_than_legacy() {
        let dir = tempfile::TempDir::new().unwrap();
        let identity_dir = dir.path().join("identity");
        let skills_dir = dir.path().join("skills");
        let db_path = dir.path().join("test.db");
        let pool = crate::db::init_pool(&db_path).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        let config = Arc::new(AppConfig::default());

        // Legacy
        let soul_loader = Arc::new(SoulLoader::new(&identity_dir).unwrap());
        let user_learner = Arc::new(UserLearner::new(pool, &config));
        let skill_registry = Arc::new(SkillRegistry::new(&skills_dir, 100_000).unwrap());
        let legacy = LegacyStrategy::new(soul_loader, user_learner, config.clone(), skill_registry);
        let legacy_output = legacy.assemble(&test_request()).await.unwrap();

        // Compact
        let compact = CompactStrategy::new(config, test_boot_context());
        let compact_output = compact.assemble(&test_request()).await.unwrap();

        let legacy_tokens = TokenBudget::estimate_tokens(&legacy_output);
        let compact_tokens = TokenBudget::estimate_tokens(&compact_output);

        assert!(
            compact_tokens < legacy_tokens,
            "Compact ({compact_tokens}) should be smaller than legacy ({legacy_tokens})"
        );
    }

    // 8.13.15 — Registry merges base + plugin fragments correctly
    #[tokio::test]
    async fn registry_merges_fragments() {
        let config = Arc::new(AppConfig::default());
        let strategy = CompactStrategy::new(config.clone(), test_boot_context());
        let registry = PromptStrategyRegistry::new(Arc::new(strategy), config);

        struct MarkerPlugin;

        #[async_trait]
        impl PromptPlugin for MarkerPlugin {
            fn id(&self) -> &str {
                "marker"
            }
            fn domains(&self) -> Vec<ContextDomain> {
                vec![]
            }
            async fn contribute(&self, _request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
                Ok(vec![PromptFragment {
                    section: PromptSection::DynamicContext,
                    content: "PLUGIN_OUTPUT_HERE".into(),
                    priority: 5,
                }])
            }
        }

        registry.register_plugin(Arc::new(MarkerPlugin)).await;

        let result = registry.assemble(&test_request()).await.unwrap();
        // Should have both base (MesoClaw) and plugin output
        assert!(result.contains("MesoClaw"));
        assert!(result.contains("PLUGIN_OUTPUT_HERE"));
    }

    // 8.13.16 — Config defaults for prompt fields
    #[test]
    fn prompt_config_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.prompt_max_preamble_tokens, 1500);
        assert!(config.prompt_compact_identity);
    }

    // 8.13.17 — Config prompt fields from TOML
    #[test]
    fn prompt_config_from_toml() {
        let toml_str = r#"
            prompt_max_preamble_tokens = 800
            prompt_compact_identity = false
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.prompt_max_preamble_tokens, 800);
        assert!(!config.prompt_compact_identity);
    }

    // 8.13.18 — Domain-specific plugin only contributes when domain matches
    #[tokio::test]
    async fn domain_specific_plugin_filtering() {
        let config = Arc::new(AppConfig::default());
        let strategy = CompactStrategy::new(config.clone(), test_boot_context());
        let registry = PromptStrategyRegistry::new(Arc::new(strategy), config);

        struct SchedulerOnlyPlugin;

        #[async_trait]
        impl PromptPlugin for SchedulerOnlyPlugin {
            fn id(&self) -> &str {
                "sched-only"
            }
            fn domains(&self) -> Vec<ContextDomain> {
                vec![ContextDomain::Scheduler]
            }
            async fn contribute(&self, _request: &AssemblyRequest) -> Result<Vec<PromptFragment>> {
                Ok(vec![PromptFragment {
                    section: PromptSection::DynamicContext,
                    content: "SCHEDULER_INFO".into(),
                    priority: 4,
                }])
            }
        }

        registry
            .register_plugin(Arc::new(SchedulerOnlyPlugin))
            .await;

        // Non-scheduler message: plugin should NOT contribute
        let mut req = test_request();
        req.user_message = Some("tell me something interesting".into());
        let result = registry.assemble(&req).await.unwrap();
        assert!(!result.contains("SCHEDULER_INFO"));

        // Scheduler message: plugin SHOULD contribute
        req.user_message = Some("schedule a reminder for tomorrow".into());
        let result = registry.assemble(&req).await.unwrap();
        assert!(result.contains("SCHEDULER_INFO"));
    }
}
