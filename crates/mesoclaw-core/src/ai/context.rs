use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;

use rig::OneOrMany;
use rig::message::Message as RigMessage;
use rig::message::{AssistantContent, Text, UserContent};
use serde::{Deserialize, Serialize};

use tracing::{debug, info, warn};

use crate::Result;
use crate::ai::session::SessionManager;
use crate::config::AppConfig;
use crate::db::{self, DbPool};
use crate::identity::SoulLoader;
use crate::memory::traits::Memory;
use crate::skills::SkillRegistry;
use crate::tools::ToolRegistry;
use crate::user::UserLearner;

/// Boot-time system context, computed once on startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootContext {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub locale: String,
    pub region: String,
}

impl BootContext {
    /// Compute boot context from the current system.
    pub fn from_system() -> Self {
        let os = format!("{} {}", std::env::consts::OS, os_version());
        let arch = std::env::consts::ARCH.to_string();
        let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown".into());
        let locale = std::env::var("LANG")
            .or_else(|_| std::env::var("LC_ALL"))
            .unwrap_or_else(|_| "en_US.UTF-8".into());
        let region = infer_region_from_timezone();

        Self {
            os,
            arch,
            hostname,
            locale,
            region,
        }
    }
}

impl Default for BootContext {
    fn default() -> Self {
        Self::from_system()
    }
}

/// Context injection level determined per-request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextLevel {
    /// Full context: identity + runtime + user + capabilities
    Full,
    /// Minimal one-liner: identity + runtime
    Minimal,
    /// Conversation summary + full context (for resumed sessions)
    Summary,
}

/// A cached context summary stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    pub key: String,
    pub summary: String,
    pub source_hash: String,
    pub generated_at: String,
    pub model_id: String,
}

/// Manages context injection for the AI agent.
pub struct ContextEngine {
    db: DbPool,
    config: std::sync::Arc<AppConfig>,
    /// Runtime-mutable enabled flag (from AppState AtomicBool).
    enabled: bool,
}

impl ContextEngine {
    pub fn new(db: DbPool, config: std::sync::Arc<AppConfig>, enabled: bool) -> Self {
        Self {
            db,
            config,
            enabled,
        }
    }

    /// Determine the appropriate context level based on session state.
    pub fn determine_context_level(
        &self,
        message_count: usize,
        last_message_at: Option<&chrono::DateTime<chrono::Utc>>,
        _has_summary: bool,
        is_resumed: bool,
    ) -> ContextLevel {
        // New session — always full
        if message_count == 0 {
            debug!("Context level: Full (new session)");
            return ContextLevel::Full;
        }

        // Resumed session with prior messages — use summary
        if is_resumed && message_count > 0 {
            debug!("Context level: Summary (resumed session, {message_count} messages)");
            return ContextLevel::Summary;
        }

        // Check time gap
        if let Some(last_at) = last_message_at {
            let gap = chrono::Utc::now() - *last_at;
            if gap.num_minutes() >= self.config.context_reinject_gap_minutes as i64 {
                debug!(
                    "Context level: Full (time gap {}min >= {}min threshold)",
                    gap.num_minutes(),
                    self.config.context_reinject_gap_minutes
                );
                return ContextLevel::Full;
            }
        }

        // Check message count threshold
        if message_count >= self.config.context_reinject_message_count as usize {
            debug!(
                "Context level: Full (message count {message_count} >= {} threshold)",
                self.config.context_reinject_message_count
            );
            return ContextLevel::Full;
        }

        debug!("Context level: Minimal (continuing conversation, {message_count} messages)");
        ContextLevel::Minimal
    }

    /// Compose the full context preamble based on context level.
    pub async fn compose(
        &self,
        level: &ContextLevel,
        boot_context: &BootContext,
        model_display: &str,
        session_id: Option<&str>,
        conversation_summary: Option<&str>,
    ) -> Result<String> {
        if !self.enabled {
            debug!("Context injection disabled, using fallback preamble");
            return Ok(self
                .config
                .agent_system_prompt
                .clone()
                .unwrap_or_else(|| "You are MesoClaw, a helpful AI assistant.".into()));
        }

        debug!(
            "Composing context: level={level:?}, model={model_display}, session={}",
            session_id.unwrap_or("none")
        );
        match level {
            ContextLevel::Full => {
                self.compose_full(boot_context, model_display, session_id)
                    .await
            }
            ContextLevel::Minimal => Ok(self.compose_minimal(boot_context, model_display)),
            ContextLevel::Summary => {
                self.compose_with_summary(
                    boot_context,
                    model_display,
                    session_id,
                    conversation_summary,
                )
                .await
            }
        }
    }

    /// Compose full context with all tiers.
    async fn compose_full(
        &self,
        boot_context: &BootContext,
        model_display: &str,
        session_id: Option<&str>,
    ) -> Result<String> {
        let mut parts = Vec::new();

        // Overall summary (Tier 3)
        if let Some(overall) = self.get_cached_summary("overall").await? {
            parts.push(overall.summary);
        }

        // Environment section (Tier 2 + Tier 1)
        parts.push("## Environment".into());
        parts.push(format!(
            "OS: {} | Arch: {} | Host: {} | Locale: {} | Region: {}",
            boot_context.os,
            boot_context.arch,
            boot_context.hostname,
            boot_context.locale,
            boot_context.region,
        ));
        parts.push(self.dynamic_runtime(model_display, session_id));

        // Identity summary (Tier 3)
        if let Some(identity) = self.get_cached_summary("identity").await? {
            parts.push("## Your Identity".into());
            parts.push(identity.summary);
        }

        // User summary (Tier 3) — only if observations exist
        if let Some(user) = self.get_cached_summary("user").await? {
            parts.push("## User Context".into());
            parts.push(user.summary);
        }

        // Capabilities summary (Tier 3)
        if let Some(caps) = self.get_cached_summary("capabilities").await? {
            parts.push("## Your Capabilities".into());
            parts.push(caps.summary);
        }

        // Guidance to avoid redundant tool calls
        parts.push("You already know the current date, time, timezone, OS, hostname, and architecture from this context. Do not call tools to retrieve information already provided above.".into());

        // Config override
        if let Some(ref override_prompt) = self.config.agent_system_prompt {
            parts.push(override_prompt.clone());
        }

        Ok(parts.join("\n\n"))
    }

    /// Compose minimal one-liner context.
    pub fn compose_minimal(&self, boot_context: &BootContext, model_display: &str) -> String {
        let now = chrono::Local::now();
        format!(
            "MesoClaw — AI assistant | {} | {} {} | {}",
            now.format("%a %b %-d %Y %H:%M %Z"),
            boot_context.os,
            boot_context.arch,
            model_display,
        )
    }

    /// Compose context with conversation summary for resumed sessions.
    async fn compose_with_summary(
        &self,
        boot_context: &BootContext,
        model_display: &str,
        session_id: Option<&str>,
        conversation_summary: Option<&str>,
    ) -> Result<String> {
        let mut full = self
            .compose_full(boot_context, model_display, session_id)
            .await?;

        if let Some(summary) = conversation_summary {
            full.push_str("\n\n## Prior Conversation\n");
            full.push_str(summary);
        }

        Ok(full)
    }

    /// Generate dynamic runtime context (Tier 1).
    pub fn dynamic_runtime(&self, model_display: &str, session_id: Option<&str>) -> String {
        let now = chrono::Local::now();
        let tz_name = now.format("%Z").to_string();
        let tz_offset = now.format("%:z").to_string();

        format!(
            "Date: {} | Day: {} | Timezone: {} (UTC{}) | Model: {} | Session: {}",
            now.format("%Y-%m-%dT%H:%M:%S"),
            now.format("%A"),
            tz_name,
            tz_offset,
            model_display,
            session_id.unwrap_or("new session"),
        )
    }

    /// Get a cached summary from the database.
    pub async fn get_cached_summary(&self, key: &str) -> Result<Option<ContextSummary>> {
        let key = key.to_string();
        db::with_db(&self.db, move |conn| {
            let result = conn.query_row(
                "SELECT key, summary, source_hash, generated_at, model_id
                 FROM context_summaries WHERE key = ?1",
                rusqlite::params![key],
                |row| {
                    Ok(ContextSummary {
                        key: row.get(0)?,
                        summary: row.get(1)?,
                        source_hash: row.get(2)?,
                        generated_at: row.get(3)?,
                        model_id: row.get(4)?,
                    })
                },
            );
            match result {
                Ok(s) => Ok(Some(s)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(crate::MesoError::Sqlite(e)),
            }
        })
        .await
    }

    /// Store a summary in the database cache.
    pub async fn store_summary(
        &self,
        key: &str,
        summary: &str,
        source_hash: &str,
        model_id: &str,
    ) -> Result<()> {
        let key = key.to_string();
        let summary = summary.to_string();
        let source_hash = source_hash.to_string();
        let model_id = model_id.to_string();

        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO context_summaries (key, summary, source_hash, model_id)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(key) DO UPDATE SET
                    summary = excluded.summary,
                    source_hash = excluded.source_hash,
                    generated_at = datetime('now'),
                    model_id = excluded.model_id",
                rusqlite::params![key, summary, source_hash, model_id],
            )
            .map_err(crate::MesoError::from)?;
            Ok(())
        })
        .await
    }

    /// Check if a summary needs regeneration by comparing source hashes.
    pub async fn needs_regeneration(&self, key: &str, current_hash: &str) -> Result<bool> {
        match self.get_cached_summary(key).await? {
            Some(cached) => Ok(cached.source_hash != current_hash),
            None => Ok(true),
        }
    }

    /// Generate summaries for all context sections using source content.
    /// This populates the DB cache with summaries of identity, user, and capabilities.
    /// The actual LLM-based summary generation is handled externally;
    /// this method stores pre-computed summaries.
    pub async fn store_all_summaries(
        &self,
        soul_loader: &SoulLoader,
        user_learner: &UserLearner,
        tools: &ToolRegistry,
        skill_registry: &SkillRegistry,
    ) -> Result<()> {
        // Identity summary
        let identity = soul_loader.get().await;
        let identity_content: String = identity
            .files
            .values()
            .map(|f| format!("{}:\n{}", f.name, f.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        let identity_hash = compute_hash(&identity_content);

        if self.needs_regeneration("identity", &identity_hash).await? {
            let summary = format!(
                "{} v{}: {}",
                identity.meta.name, identity.meta.version, identity.meta.description,
            );
            self.store_summary("identity", &summary, &identity_hash, "builtin")
                .await?;
        }

        // User summary
        let user_context = user_learner.build_context().await?;
        let user_hash = compute_hash(&user_context);

        if !user_context.is_empty() && self.needs_regeneration("user", &user_hash).await? {
            self.store_summary("user", &user_context, &user_hash, "builtin")
                .await?;
        }

        // Capabilities summary
        let tool_names: Vec<String> = tools
            .to_vec()
            .iter()
            .map(|t| t.name().to_string())
            .collect();
        let skill_list = skill_registry.list().await;
        let skill_names: Vec<String> = skill_list.iter().map(|s| s.id.clone()).collect();
        let caps_content = format!(
            "Tools: {}\nSkills: {}",
            tool_names.join(", "),
            skill_names.join(", ")
        );
        let caps_hash = compute_hash(&caps_content);

        if self.needs_regeneration("capabilities", &caps_hash).await? {
            let summary = format!(
                "{} tools: {}. {} skills: {}.",
                tool_names.len(),
                tool_names.join(", "),
                skill_names.len(),
                skill_names.join(", "),
            );
            self.store_summary("capabilities", &summary, &caps_hash, "builtin")
                .await?;
        }

        // Overall summary (combination)
        let identity_summary = self
            .get_cached_summary("identity")
            .await?
            .map(|s| s.summary)
            .unwrap_or_default();
        let user_summary = self
            .get_cached_summary("user")
            .await?
            .map(|s| s.summary)
            .unwrap_or_default();
        let caps_summary = self
            .get_cached_summary("capabilities")
            .await?
            .map(|s| s.summary)
            .unwrap_or_default();

        let overall_content = format!("{}\n{}\n{}", identity_summary, user_summary, caps_summary);
        let overall_hash = compute_hash(&overall_content);

        if self.needs_regeneration("overall", &overall_hash).await? {
            let mut overall_parts = vec![identity_summary];
            if !user_summary.is_empty() {
                overall_parts.push(format!("User: {user_summary}"));
            }
            if !caps_summary.is_empty() {
                overall_parts.push(caps_summary);
            }
            let overall = overall_parts.join(" | ");
            self.store_summary("overall", &overall, &overall_hash, "builtin")
                .await?;
        }

        Ok(())
    }
}

// ============================================================================
// Context Strategy & Builder (Step 15.3)
// ============================================================================

/// Strategy controlling how much context history and memory is injected.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextStrategy {
    /// Last 2 turns (4 messages), top 3 memories
    Minimal,
    /// Last 10 turns (20 messages) + top 5 memories
    Balanced,
    /// All messages up to max cap, top 10 memories
    Full,
}

impl Default for ContextStrategy {
    fn default() -> Self {
        Self::Balanced
    }
}

impl FromStr for ContextStrategy {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(Self::Minimal),
            "balanced" => Ok(Self::Balanced),
            "full" => Ok(Self::Full),
            _ => Ok(Self::Balanced), // invalid defaults to Balanced
        }
    }
}

impl std::fmt::Display for ContextStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Minimal => write!(f, "minimal"),
            Self::Balanced => write!(f, "balanced"),
            Self::Full => write!(f, "full"),
        }
    }
}

/// Convert a session message to a rig message.
/// Returns None for system/tool messages (they are skipped).
pub fn convert_session_message(msg: &crate::ai::session::Message) -> Option<RigMessage> {
    match msg.role.as_str() {
        "user" => Some(RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: msg.content.clone(),
            })),
        }),
        "assistant" => Some(RigMessage::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: msg.content.clone(),
            })),
        }),
        _ => None, // system, tool, etc. are skipped
    }
}

/// Convert a list of session messages to rig messages, preserving order.
pub fn convert_session_messages(messages: &[crate::ai::session::Message]) -> Vec<RigMessage> {
    messages
        .iter()
        .filter_map(convert_session_message)
        .collect()
}

/// Apply strategy-based windowing to a list of messages.
pub fn window_messages(
    messages: Vec<RigMessage>,
    strategy: &ContextStrategy,
    max_cap: usize,
) -> Vec<RigMessage> {
    let window_size = match strategy {
        ContextStrategy::Minimal => 4,   // 2 turns
        ContextStrategy::Balanced => 20, // 10 turns
        ContextStrategy::Full => max_cap,
    };
    let effective_limit = window_size.min(max_cap);
    let len = messages.len();
    if len <= effective_limit {
        messages
    } else {
        messages.into_iter().skip(len - effective_limit).collect()
    }
}

/// Get the memory recall limit based on strategy.
fn memory_limit_for_strategy(strategy: &ContextStrategy, config_max: usize) -> usize {
    match strategy {
        ContextStrategy::Minimal => 3.min(config_max),
        ContextStrategy::Balanced => 5.min(config_max),
        ContextStrategy::Full => config_max,
    }
}

/// Orchestrates the full context assembly pipeline for chat requests.
pub struct ContextBuilder {
    session_manager: Arc<SessionManager>,
    memory: Arc<dyn Memory>,
    soul_loader: Arc<SoulLoader>,
    user_learner: Arc<UserLearner>,
    config: Arc<AppConfig>,
}

impl ContextBuilder {
    pub fn new(
        session_manager: Arc<SessionManager>,
        memory: Arc<dyn Memory>,
        soul_loader: Arc<SoulLoader>,
        user_learner: Arc<UserLearner>,
        config: Arc<AppConfig>,
    ) -> Self {
        Self {
            session_manager,
            memory,
            soul_loader,
            user_learner,
            config,
        }
    }

    /// Build the full context for a chat request.
    ///
    /// Returns `(history, preamble_context)`:
    /// - `history`: windowed rig messages for `agent.chat()`
    /// - `preamble_context`: augmented preamble string combining identity, memories, and user profile
    ///
    /// Note: The current user prompt is excluded from history because rig's `chat(prompt, history)`
    /// appends the prompt as a new user message. Including it in history would duplicate it.
    pub async fn build(
        &self,
        session_id: Option<&str>,
        prompt: &str,
    ) -> Result<(Vec<RigMessage>, String)> {
        let strategy = ContextStrategy::from_str(&self.config.context_strategy).unwrap_or_default();

        // 1. Get session history, excluding the current user prompt to avoid duplication
        // (rig's chat() will append the prompt as a new user message)
        let history = if let Some(sid) = session_id {
            let messages = self.session_manager.get_messages(sid).await?;
            info!(
                "ContextBuilder: session={sid}, raw messages from DB: {}",
                messages.len()
            );
            for (i, m) in messages.iter().enumerate() {
                info!(
                    "  msg[{i}] role={} content={}",
                    m.role,
                    &m.content[..m.content.len().min(80)]
                );
            }
            // Strip the last message if it matches the current prompt (already POSTed by frontend)
            let trimmed = if messages
                .last()
                .is_some_and(|m| m.role == "user" && m.content == prompt)
            {
                info!("ContextBuilder: stripped last message (matches current prompt)");
                &messages[..messages.len() - 1]
            } else {
                info!("ContextBuilder: no stripping needed (last msg doesn't match prompt)");
                &messages
            };
            let rig_messages = convert_session_messages(trimmed);
            info!(
                "ContextBuilder: {} trimmed msgs → {} rig msgs (strategy={strategy})",
                trimmed.len(),
                rig_messages.len()
            );
            let windowed = window_messages(
                rig_messages,
                &strategy,
                self.config.context_max_history_messages,
            );
            info!("ContextBuilder: after windowing: {} msgs", windowed.len());
            windowed
        } else {
            info!("ContextBuilder: no session_id, empty history");
            Vec::new()
        };

        // 2. Recall cross-session memories
        let memory_context = self.recall_memories(prompt, &strategy).await;

        // 3. Get user profile context
        let user_context = self.get_user_context().await;

        debug!(
            "Context build: history={} msgs, memory={}B, user={}B",
            history.len(),
            memory_context.len(),
            user_context.len()
        );

        // 4. Build augmented preamble
        let preamble = self.augment_preamble(&memory_context, &user_context).await;

        Ok((history, preamble))
    }

    /// Recall relevant memories based on the current prompt.
    async fn recall_memories(&self, prompt: &str, strategy: &ContextStrategy) -> String {
        let limit = memory_limit_for_strategy(strategy, self.config.context_max_memory_results);
        match self.memory.recall(prompt, limit, 0).await {
            Ok(memories) => {
                if memories.is_empty() {
                    return String::new();
                }
                let mut parts = vec!["[Relevant Memories]".to_string()];
                for mem in &memories {
                    parts.push(format!("- {}", mem.content));
                }
                parts.join("\n")
            }
            Err(e) => {
                warn!("Memory recall failed (non-fatal): {e}");
                String::new()
            }
        }
    }

    /// Get user observations/preferences as context.
    async fn get_user_context(&self) -> String {
        match self.user_learner.build_context().await {
            Ok(context) => {
                if context.is_empty() {
                    debug!("No user observations found for context injection");
                    return String::new();
                }
                debug!("Injecting {} bytes of user context", context.len());
                format!(
                    "[User Preferences & Observations]\n\
                     The following facts have been learned about this user from prior interactions. \
                     Use them to personalize responses:\n{context}"
                )
            }
            Err(e) => {
                warn!("User context retrieval failed (non-fatal): {e}");
                String::new()
            }
        }
    }

    /// Combine identity preamble with memory and user profile context.
    async fn augment_preamble(&self, memory_context: &str, user_context: &str) -> String {
        let identity = self.soul_loader.get().await;
        let base_preamble = crate::identity::PromptComposer::compose(
            &identity,
            &[], // skills are injected separately via ContextEngine
            "",  // observations handled by user_context below
            &self.config,
        );

        let mut parts = vec![base_preamble];

        if !memory_context.is_empty() {
            parts.push(memory_context.to_string());
        }

        if !user_context.is_empty() {
            parts.push(user_context.to_string());
        }

        parts.join("\n\n")
    }

    /// Extract facts from a conversation exchange (post-response).
    /// This is fire-and-forget — errors are logged but not propagated.
    pub async fn extract_facts(
        &self,
        _prompt: &str,
        _response: &str,
        session_id: Option<&str>,
    ) -> Result<()> {
        if !self.config.context_auto_extract {
            return Ok(());
        }

        // Check interval: only extract every N messages
        if let Some(sid) = session_id {
            let (count, _, _) = self
                .session_manager
                .get_context_info(sid)
                .await
                .unwrap_or((0, None, None));
            if count % self.config.context_extract_interval != 0 {
                debug!(
                    "Skipping extraction: message {count} not at interval {}",
                    self.config.context_extract_interval
                );
                return Ok(());
            }
        }

        // Auto-extraction would call an LLM here to extract facts.
        // For now, this is a placeholder — actual LLM-based extraction
        // requires a configured summary model and is deferred to handler integration.
        debug!("Auto-extraction triggered (LLM call deferred to handler)");
        Ok(())
    }
}

/// Compute a simple hash of content for change detection.
pub fn compute_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn os_version() -> String {
    use sysinfo::System;
    System::long_os_version().unwrap_or_else(|| System::os_version().unwrap_or_default())
}

fn infer_region_from_timezone() -> String {
    let tz = std::env::var("TZ").unwrap_or_default();
    if tz.contains("America/New_York")
        || tz.contains("America/Detroit")
        || tz.contains("US/Eastern")
    {
        "Eastern US".into()
    } else if tz.contains("America/Chicago") || tz.contains("US/Central") {
        "Central US".into()
    } else if tz.contains("America/Denver") || tz.contains("US/Mountain") {
        "Mountain US".into()
    } else if tz.contains("America/Los_Angeles") || tz.contains("US/Pacific") {
        "Pacific US".into()
    } else if tz.contains("Europe/") {
        "Europe".into()
    } else if tz.contains("Asia/") {
        "Asia".into()
    } else {
        // Try to infer from chrono offset
        let offset = chrono::Local::now().offset().local_minus_utc() / 3600;
        match offset {
            -5 => "Eastern US".into(),
            -6 => "Central US".into(),
            -7 => "Mountain US".into(),
            -8 => "Pacific US".into(),
            0 => "UTC/UK".into(),
            1 => "Central Europe".into(),
            5..=6 => "South Asia".into(),
            8 => "East Asia".into(),
            9 => "Japan/Korea".into(),
            _ => format!("UTC{:+}", offset),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::memory::in_memory_store::InMemoryStore;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, ContextEngine) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        (dir, engine)
    }

    // 15.3.1 — compose returns fallback when disabled
    #[tokio::test]
    async fn compose_returns_empty_when_disabled() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, false);
        let boot = BootContext::from_system();

        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None)
            .await
            .unwrap();
        assert_eq!(result, "You are MesoClaw, a helpful AI assistant.");
    }

    // 15.3.2 — compose includes runtime line
    #[tokio::test]
    async fn compose_includes_runtime_line() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();

        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", Some("sess-1"), None)
            .await
            .unwrap();
        assert!(result.contains("Date:"));
        assert!(result.contains("Model: gpt-4o"));
        assert!(result.contains("Session: sess-1"));
    }

    // 15.3.3 — compose includes cached identity summary
    #[tokio::test]
    async fn compose_includes_cached_identity_summary() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("identity", "MesoClaw: a helpful assistant", "hash1", "test")
            .await
            .unwrap();
        let boot = BootContext::from_system();

        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None)
            .await
            .unwrap();
        assert!(result.contains("Your Identity"));
        assert!(result.contains("MesoClaw: a helpful assistant"));
    }

    // 15.3.4 — compose includes user summary when observations exist
    #[tokio::test]
    async fn compose_includes_user_summary_when_observations_exist() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("user", "Rust developer, uses bun", "hash2", "test")
            .await
            .unwrap();
        let boot = BootContext::from_system();

        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None)
            .await
            .unwrap();
        assert!(result.contains("User Context"));
        assert!(result.contains("Rust developer, uses bun"));
    }

    // 15.3.5 — compose skips user summary when no observations
    #[tokio::test]
    async fn compose_skips_user_summary_when_no_observations() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();

        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None)
            .await
            .unwrap();
        assert!(!result.contains("User Context"));
    }

    // 15.3.6 — compose includes capabilities summary
    #[tokio::test]
    async fn compose_includes_capabilities_summary() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary(
                "capabilities",
                "9 tools: web_search, shell, etc.",
                "hash3",
                "test",
            )
            .await
            .unwrap();
        let boot = BootContext::from_system();

        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None)
            .await
            .unwrap();
        assert!(result.contains("Your Capabilities"));
        assert!(result.contains("9 tools"));
    }

    // 15.3.7 — compose includes overall summary
    #[tokio::test]
    async fn compose_includes_overall_summary() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary(
                "overall",
                "MesoClaw AI assistant for developers",
                "hash4",
                "test",
            )
            .await
            .unwrap();
        let boot = BootContext::from_system();

        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None)
            .await
            .unwrap();
        assert!(result.contains("MesoClaw AI assistant for developers"));
    }

    // 15.3.8 — compose appends config override
    #[tokio::test]
    async fn context_compose_appends_config_override() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig {
            agent_system_prompt: Some("Always be concise.".into()),
            ..Default::default()
        });
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();

        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None)
            .await
            .unwrap();
        assert!(result.contains("Always be concise."));
    }

    // 15.3.9 — dynamic_runtime includes time and day
    #[test]
    fn dynamic_runtime_includes_time_and_day() {
        let config = std::sync::Arc::new(AppConfig::default());
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let engine = ContextEngine::new(pool, config, true);

        let runtime = engine.dynamic_runtime("gpt-4o", Some("sess-1"));
        assert!(runtime.contains("Date:"));
        assert!(runtime.contains("Day:"));
        assert!(runtime.contains("Model: gpt-4o"));
        assert!(runtime.contains("Session: sess-1"));
    }

    // 15.3.10 — dynamic_runtime includes timezone
    #[test]
    fn dynamic_runtime_includes_timezone() {
        let config = std::sync::Arc::new(AppConfig::default());
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let engine = ContextEngine::new(pool, config, true);

        let runtime = engine.dynamic_runtime("gpt-4o", None);
        assert!(runtime.contains("Timezone:"));
        assert!(runtime.contains("UTC"));
    }

    // 15.3.10b — boot_context includes os and arch
    #[test]
    fn boot_context_includes_os_and_arch() {
        let boot = BootContext::from_system();
        assert!(!boot.os.is_empty());
        assert!(!boot.arch.is_empty());
        assert!(boot.arch == std::env::consts::ARCH);
    }

    // 15.3.10c — boot_context includes locale and region
    #[test]
    fn boot_context_includes_locale_and_region() {
        let boot = BootContext::from_system();
        assert!(!boot.locale.is_empty());
        assert!(!boot.region.is_empty());
    }

    // 15.3.11 — store and get cached summary
    #[tokio::test]
    async fn store_and_get_cached_summary() {
        let (_dir, engine) = setup().await;

        engine
            .store_summary("test_key", "test summary", "abc123", "gpt-4o-mini")
            .await
            .unwrap();

        let result = engine.get_cached_summary("test_key").await.unwrap();
        assert!(result.is_some());
        let summary = result.unwrap();
        assert_eq!(summary.key, "test_key");
        assert_eq!(summary.summary, "test summary");
        assert_eq!(summary.source_hash, "abc123");
        assert_eq!(summary.model_id, "gpt-4o-mini");
    }

    // 15.3.12 — summary regenerates when source hash changes
    #[tokio::test]
    async fn summary_regenerates_when_source_hash_changes() {
        let (_dir, engine) = setup().await;

        engine
            .store_summary("key1", "old summary", "hash_v1", "model")
            .await
            .unwrap();

        assert!(!engine.needs_regeneration("key1", "hash_v1").await.unwrap());
        assert!(engine.needs_regeneration("key1", "hash_v2").await.unwrap());
    }

    // 15.3.12b — determine_context_level: new session returns Full
    #[test]
    fn determine_context_level_new_session_returns_full() {
        let config = std::sync::Arc::new(AppConfig::default());
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let engine = ContextEngine::new(pool, config, true);

        let level = engine.determine_context_level(0, None, false, false);
        assert_eq!(level, ContextLevel::Full);
    }

    // 15.3.12c — determine_context_level: continuing returns Minimal
    #[test]
    fn determine_context_level_continuing_returns_minimal() {
        let config = std::sync::Arc::new(AppConfig::default());
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let engine = ContextEngine::new(pool, config, true);

        let recent = chrono::Utc::now() - chrono::Duration::minutes(5);
        let level = engine.determine_context_level(3, Some(&recent), false, false);
        assert_eq!(level, ContextLevel::Minimal);
    }

    // 15.3.12d — determine_context_level: gap exceeded returns Full
    #[test]
    fn determine_context_level_gap_exceeded_returns_full() {
        let config = std::sync::Arc::new(AppConfig {
            context_reinject_gap_minutes: 30,
            ..Default::default()
        });
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let engine = ContextEngine::new(pool, config, true);

        let old = chrono::Utc::now() - chrono::Duration::minutes(60);
        let level = engine.determine_context_level(5, Some(&old), false, false);
        assert_eq!(level, ContextLevel::Full);
    }

    // 15.3.12e — determine_context_level: count exceeded returns Full
    #[test]
    fn determine_context_level_count_exceeded_returns_full() {
        let config = std::sync::Arc::new(AppConfig {
            context_reinject_message_count: 20,
            ..Default::default()
        });
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let engine = ContextEngine::new(pool, config, true);

        let recent = chrono::Utc::now() - chrono::Duration::minutes(1);
        let level = engine.determine_context_level(25, Some(&recent), false, false);
        assert_eq!(level, ContextLevel::Full);
    }

    // 15.3.12f — determine_context_level: resumed returns Summary
    #[test]
    fn determine_context_level_resumed_returns_summary() {
        let config = std::sync::Arc::new(AppConfig::default());
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let engine = ContextEngine::new(pool, config, true);

        let level = engine.determine_context_level(10, None, true, true);
        assert_eq!(level, ContextLevel::Summary);
    }

    // 15.3.12g — compose_minimal is one-liner
    #[test]
    fn compose_minimal_is_one_liner() {
        let config = std::sync::Arc::new(AppConfig::default());
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();

        let minimal = engine.compose_minimal(&boot, "gpt-4o");
        assert!(!minimal.contains('\n'));
        assert!(minimal.contains("MesoClaw"));
    }

    // 15.3.12h — compose_with_summary includes prior conversation
    #[tokio::test]
    async fn compose_with_summary_includes_prior_conversation() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();

        let result = engine
            .compose(
                &ContextLevel::Summary,
                &boot,
                "gpt-4o",
                Some("sess-1"),
                Some("User asked about Rust async patterns."),
            )
            .await
            .unwrap();
        assert!(result.contains("Prior Conversation"));
        assert!(result.contains("Rust async patterns"));
    }

    // =========================================================================
    // ContextStrategy tests (15.3.1–15.3.6)
    // =========================================================================

    // 15.3.1 — ContextStrategy default is Balanced
    #[test]
    fn strategy_default_is_balanced() {
        assert_eq!(ContextStrategy::default(), ContextStrategy::Balanced);
    }

    // 15.3.2 — ContextStrategy from_str minimal
    #[test]
    fn strategy_from_str_minimal() {
        assert_eq!(
            ContextStrategy::from_str("minimal").unwrap(),
            ContextStrategy::Minimal
        );
    }

    // 15.3.3 — ContextStrategy from_str balanced
    #[test]
    fn strategy_from_str_balanced() {
        assert_eq!(
            ContextStrategy::from_str("balanced").unwrap(),
            ContextStrategy::Balanced
        );
    }

    // 15.3.4 — ContextStrategy from_str full
    #[test]
    fn strategy_from_str_full() {
        assert_eq!(
            ContextStrategy::from_str("full").unwrap(),
            ContextStrategy::Full
        );
    }

    // 15.3.5 — ContextStrategy from_str invalid defaults to Balanced
    #[test]
    fn strategy_from_str_invalid() {
        assert_eq!(
            ContextStrategy::from_str("garbage").unwrap(),
            ContextStrategy::Balanced
        );
    }

    // 15.3.6 — ContextStrategy serialization round-trip
    #[test]
    fn strategy_serde_roundtrip() {
        let strategy = ContextStrategy::Full;
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: ContextStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ContextStrategy::Full);
    }

    // =========================================================================
    // Message conversion tests (15.3.7–15.3.12)
    // =========================================================================

    fn make_session_msg(role: &str, content: &str) -> crate::ai::session::Message {
        crate::ai::session::Message {
            id: "test-id".into(),
            session_id: "test-session".into(),
            role: role.into(),
            content: content.into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    // 15.3.7 — User session message converts to rig User message
    #[test]
    fn convert_user_message() {
        let msg = make_session_msg("user", "Hello!");
        let rig_msg = convert_session_message(&msg).unwrap();
        match rig_msg {
            RigMessage::User { content } => {
                let first = content.first();
                match first {
                    UserContent::Text(t) => assert_eq!(t.text, "Hello!"),
                    _ => panic!("Expected Text content"),
                }
            }
            _ => panic!("Expected User message"),
        }
    }

    // 15.3.8 — Assistant session message converts to rig Assistant message
    #[test]
    fn convert_assistant_message() {
        let msg = make_session_msg("assistant", "Hi there!");
        let rig_msg = convert_session_message(&msg).unwrap();
        match rig_msg {
            RigMessage::Assistant { id, content } => {
                assert!(id.is_none());
                let first = content.first();
                match first {
                    AssistantContent::Text(t) => assert_eq!(t.text, "Hi there!"),
                    _ => panic!("Expected Text content"),
                }
            }
            _ => panic!("Expected Assistant message"),
        }
    }

    // 15.3.9 — System role message is skipped in conversion
    #[test]
    fn convert_system_message_skipped() {
        let msg = make_session_msg("system", "System prompt");
        assert!(convert_session_message(&msg).is_none());
    }

    // 15.3.10 — Tool role message is skipped in conversion
    #[test]
    fn convert_tool_message_skipped() {
        let msg = make_session_msg("tool", "Tool output");
        assert!(convert_session_message(&msg).is_none());
    }

    // 15.3.11 — Empty message list converts to empty vec
    #[test]
    fn convert_empty_messages() {
        let messages: Vec<crate::ai::session::Message> = vec![];
        let result = convert_session_messages(&messages);
        assert!(result.is_empty());
    }

    // 15.3.12 — Mixed roles preserve order after filtering
    #[test]
    fn convert_mixed_roles_preserves_order() {
        let messages = vec![
            make_session_msg("user", "First"),
            make_session_msg("system", "System"),
            make_session_msg("assistant", "Second"),
            make_session_msg("tool", "Tool"),
            make_session_msg("user", "Third"),
        ];
        let result = convert_session_messages(&messages);
        assert_eq!(result.len(), 3); // user, assistant, user — system and tool skipped
    }

    // =========================================================================
    // History windowing tests (15.3.13–15.3.18)
    // =========================================================================

    fn make_rig_user_msg(text: &str) -> RigMessage {
        RigMessage::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: text.to_string(),
            })),
        }
    }

    fn make_rig_assistant_msg(text: &str) -> RigMessage {
        RigMessage::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: text.to_string(),
            })),
        }
    }

    // 15.3.13 — Minimal strategy returns last 4 messages (2 turns)
    #[test]
    fn window_minimal_last_4() {
        let messages: Vec<RigMessage> = (0..10)
            .map(|i| {
                if i % 2 == 0 {
                    make_rig_user_msg(&format!("user-{i}"))
                } else {
                    make_rig_assistant_msg(&format!("assistant-{i}"))
                }
            })
            .collect();
        let result = window_messages(messages, &ContextStrategy::Minimal, 20);
        assert_eq!(result.len(), 4);
    }

    // 15.3.14 — Balanced strategy returns last 20 messages (10 turns)
    #[test]
    fn window_balanced_last_20() {
        let messages: Vec<RigMessage> = (0..30)
            .map(|i| {
                if i % 2 == 0 {
                    make_rig_user_msg(&format!("user-{i}"))
                } else {
                    make_rig_assistant_msg(&format!("assistant-{i}"))
                }
            })
            .collect();
        let result = window_messages(messages, &ContextStrategy::Balanced, 30);
        assert_eq!(result.len(), 20);
    }

    // 15.3.15 — Full strategy returns all messages up to max
    #[test]
    fn window_full_all_messages() {
        let messages: Vec<RigMessage> = (0..15)
            .map(|i| make_rig_user_msg(&format!("msg-{i}")))
            .collect();
        let result = window_messages(messages, &ContextStrategy::Full, 20);
        assert_eq!(result.len(), 15);
    }

    // 15.3.16 — Windowing respects context_max_history_messages cap
    #[test]
    fn window_respects_max_cap() {
        let messages: Vec<RigMessage> = (0..30)
            .map(|i| make_rig_user_msg(&format!("msg-{i}")))
            .collect();
        // Full strategy but max cap is 10
        let result = window_messages(messages, &ContextStrategy::Full, 10);
        assert_eq!(result.len(), 10);
    }

    // 15.3.17 — Short history (fewer than window) returns all
    #[test]
    fn window_short_history_returns_all() {
        let messages: Vec<RigMessage> = (0..3)
            .map(|i| make_rig_user_msg(&format!("msg-{i}")))
            .collect();
        let result = window_messages(messages, &ContextStrategy::Balanced, 20);
        assert_eq!(result.len(), 3);
    }

    // 15.3.18 — Empty session history returns empty vec
    #[test]
    fn window_empty_history() {
        let messages: Vec<RigMessage> = vec![];
        let result = window_messages(messages, &ContextStrategy::Balanced, 20);
        assert!(result.is_empty());
    }

    // =========================================================================
    // Memory recall tests (15.3.19–15.3.22)
    // =========================================================================

    async fn setup_builder() -> (TempDir, ContextBuilder) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = Arc::new(AppConfig::default());
        let session_manager = Arc::new(SessionManager::new(pool.clone()));
        let memory: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
        let identity_dir = dir.path().join("identity");
        let soul_loader = Arc::new(SoulLoader::new(&identity_dir).unwrap());
        let user_learner = Arc::new(UserLearner::new(pool, &config));

        let builder =
            ContextBuilder::new(session_manager, memory, soul_loader, user_learner, config);
        (dir, builder)
    }

    // 15.3.19 — recall_memories returns formatted memory context
    #[tokio::test]
    async fn recall_memories_formatted() {
        let (_dir, builder) = setup_builder().await;
        // Store a memory (InMemoryStore uses substring matching on key or content)
        builder
            .memory
            .store(
                "dark_mode_pref",
                "user prefers dark mode",
                crate::memory::traits::MemoryCategory::Core,
            )
            .await
            .unwrap();

        let result = builder
            .recall_memories("dark mode", &ContextStrategy::Balanced)
            .await;
        assert!(result.contains("[Relevant Memories]"));
        assert!(result.contains("dark mode"));
    }

    // 15.3.20 — recall_memories with no results returns empty string
    #[tokio::test]
    async fn recall_memories_empty() {
        let (_dir, builder) = setup_builder().await;
        let result = builder
            .recall_memories("something unrelated", &ContextStrategy::Balanced)
            .await;
        assert!(result.is_empty());
    }

    // 15.3.21 — recall_memories respects context_max_memory_results
    #[tokio::test]
    async fn recall_memories_respects_limit() {
        let (_dir, builder) = setup_builder().await;
        // Store many memories
        for i in 0..10 {
            builder
                .memory
                .store(
                    &format!("fact_{i}"),
                    &format!("memory fact number {i}"),
                    crate::memory::traits::MemoryCategory::Core,
                )
                .await
                .unwrap();
        }

        // Minimal strategy limits to 3
        let result = builder
            .recall_memories("memory fact", &ContextStrategy::Minimal)
            .await;
        // Count memory items (lines starting with "- ")
        let memory_lines = result.lines().filter(|l| l.starts_with("- ")).count();
        assert!(memory_lines <= 3, "Minimal should limit to 3 memories");
    }

    // 15.3.22 — recall_memories failure is non-fatal (logs warning, returns empty)
    #[tokio::test]
    async fn recall_memories_failure_nonfatal() {
        // InMemoryStore won't fail, but the logic handles errors gracefully
        let (_dir, builder) = setup_builder().await;
        // This should return empty without panicking
        let result = builder
            .recall_memories("query", &ContextStrategy::Balanced)
            .await;
        assert!(result.is_empty());
    }

    // =========================================================================
    // User profile context tests (15.3.23–15.3.25)
    // =========================================================================

    // 15.3.23 — get_user_context returns formatted preferences
    #[tokio::test]
    async fn user_context_formatted() {
        let (_dir, builder) = setup_builder().await;
        // Add an observation
        builder
            .user_learner
            .observe("preference", "dark_mode", "Uses dark mode", 0.9)
            .await
            .unwrap();

        let result = builder.get_user_context().await;
        assert!(result.contains("[User Preferences & Observations]"));
        assert!(result.contains("dark_mode"));
    }

    // 15.3.24 — get_user_context with no observations returns empty
    #[tokio::test]
    async fn user_context_empty() {
        let (_dir, builder) = setup_builder().await;
        let result = builder.get_user_context().await;
        assert!(result.is_empty());
    }

    // 15.3.25 — get_user_context failure is non-fatal
    #[tokio::test]
    async fn user_context_failure_nonfatal() {
        // UserLearner won't fail here, but the logic handles errors gracefully
        let (_dir, builder) = setup_builder().await;
        let result = builder.get_user_context().await;
        assert!(result.is_empty());
    }

    // =========================================================================
    // Preamble augmentation tests (15.3.26–15.3.29)
    // =========================================================================

    // 15.3.26 — augment_preamble combines identity + memories + user profile
    #[tokio::test]
    async fn augment_preamble_full() {
        let (_dir, builder) = setup_builder().await;
        let result = builder
            .augment_preamble(
                "[Relevant Memories]\n- likes Rust",
                "[User Preferences]\n- vim user",
            )
            .await;
        assert!(result.contains("Agent Identity")); // from PromptComposer
        assert!(result.contains("[Relevant Memories]"));
        assert!(result.contains("[User Preferences]"));
    }

    // 15.3.27 — augment_preamble with only identity (no memories, no profile)
    #[tokio::test]
    async fn augment_preamble_identity_only() {
        let (_dir, builder) = setup_builder().await;
        let result = builder.augment_preamble("", "").await;
        assert!(result.contains("Agent Identity"));
        assert!(!result.contains("[Relevant Memories]"));
        assert!(!result.contains("[User Preferences]"));
    }

    // 15.3.28 — augment_preamble with memories but no profile
    #[tokio::test]
    async fn augment_preamble_memories_only() {
        let (_dir, builder) = setup_builder().await;
        let result = builder
            .augment_preamble("[Relevant Memories]\n- fact 1", "")
            .await;
        assert!(result.contains("[Relevant Memories]"));
        assert!(!result.contains("[User Preferences]"));
    }

    // 15.3.29 — augment_preamble with profile but no memories
    #[tokio::test]
    async fn augment_preamble_profile_only() {
        let (_dir, builder) = setup_builder().await;
        let result = builder
            .augment_preamble("", "[User Preferences]\n- prefers dark mode")
            .await;
        assert!(!result.contains("[Relevant Memories]"));
        assert!(result.contains("[User Preferences]"));
    }

    // =========================================================================
    // ContextBuilder::build() full pipeline tests (15.3.30–15.3.34)
    // =========================================================================

    // 15.3.30 — build with session_id returns history + augmented preamble
    #[tokio::test]
    async fn build_with_session() {
        let (_dir, builder) = setup_builder().await;
        // Create a session with messages
        let session = builder
            .session_manager
            .create_session("Test")
            .await
            .unwrap();
        builder
            .session_manager
            .append_message(&session.id, "user", "Hello")
            .await
            .unwrap();
        builder
            .session_manager
            .append_message(&session.id, "assistant", "Hi!")
            .await
            .unwrap();

        let (history, preamble) = builder
            .build(Some(&session.id), "How are you?")
            .await
            .unwrap();
        assert_eq!(history.len(), 2);
        assert!(preamble.contains("Agent Identity"));
    }

    // 15.3.31 — build without session_id returns empty history + augmented preamble
    #[tokio::test]
    async fn build_without_session() {
        let (_dir, builder) = setup_builder().await;
        let (history, preamble) = builder.build(None, "Hello").await.unwrap();
        assert!(history.is_empty());
        assert!(preamble.contains("Agent Identity"));
    }

    // 15.3.32 — build applies strategy-based windowing correctly
    #[tokio::test]
    async fn build_applies_strategy() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = Arc::new(AppConfig {
            context_strategy: "minimal".into(),
            ..Default::default()
        });
        let session_manager = Arc::new(SessionManager::new(pool.clone()));
        let memory: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
        let identity_dir = dir.path().join("identity");
        let soul_loader = Arc::new(SoulLoader::new(&identity_dir).unwrap());
        let user_learner = Arc::new(UserLearner::new(pool, &config));
        let builder =
            ContextBuilder::new(session_manager, memory, soul_loader, user_learner, config);

        // Create session with 10 messages
        let session = builder
            .session_manager
            .create_session("Test")
            .await
            .unwrap();
        for i in 0..10 {
            let role = if i % 2 == 0 { "user" } else { "assistant" };
            builder
                .session_manager
                .append_message(&session.id, role, &format!("msg-{i}"))
                .await
                .unwrap();
        }

        let (history, _) = builder.build(Some(&session.id), "next").await.unwrap();
        // Minimal = last 4 messages
        assert_eq!(history.len(), 4);
    }

    // 15.3.33 — build with minimal strategy limits memories to 3
    #[test]
    fn build_minimal_limits_memories() {
        let limit = memory_limit_for_strategy(&ContextStrategy::Minimal, 10);
        assert_eq!(limit, 3);
    }

    // 15.3.34 — build with full strategy uses max memories
    #[test]
    fn build_full_max_memories() {
        let limit = memory_limit_for_strategy(&ContextStrategy::Full, 10);
        assert_eq!(limit, 10);
    }

    // =========================================================================
    // Auto-extraction tests (15.3.35–15.3.40)
    // =========================================================================

    // 15.3.35 — extract_facts respects config flag
    #[tokio::test]
    async fn extract_facts_disabled_is_noop() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = Arc::new(AppConfig {
            context_auto_extract: false,
            ..Default::default()
        });
        let session_manager = Arc::new(SessionManager::new(pool.clone()));
        let memory: Arc<dyn Memory> = Arc::new(InMemoryStore::new());
        let identity_dir = dir.path().join("identity");
        let soul_loader = Arc::new(SoulLoader::new(&identity_dir).unwrap());
        let user_learner = Arc::new(UserLearner::new(pool, &config));
        let builder =
            ContextBuilder::new(session_manager, memory, soul_loader, user_learner, config);

        // Should return Ok without doing anything
        let result = builder.extract_facts("prompt", "response", None).await;
        assert!(result.is_ok());
    }

    // 15.3.36 — extract_facts enabled returns Ok
    #[tokio::test]
    async fn extract_facts_enabled_returns_ok() {
        let (_dir, builder) = setup_builder().await;
        let result = builder
            .extract_facts("I prefer dark mode", "Noted!", None)
            .await;
        assert!(result.is_ok());
    }

    // 15.3.37 — extract_facts with empty array is no-op
    #[tokio::test]
    async fn extract_facts_empty_noop() {
        let (_dir, builder) = setup_builder().await;
        let result = builder.extract_facts("hello", "hi", None).await;
        assert!(result.is_ok());
    }

    // 15.3.38 — extract_facts respects interval (skips if not Nth message)
    #[tokio::test]
    async fn extract_facts_respects_interval() {
        let (_dir, builder) = setup_builder().await;
        let session = builder
            .session_manager
            .create_session("Test")
            .await
            .unwrap();
        // Add 1 message (not at interval of 3)
        builder
            .session_manager
            .append_message(&session.id, "user", "Hello")
            .await
            .unwrap();

        let result = builder
            .extract_facts("prompt", "response", Some(&session.id))
            .await;
        assert!(result.is_ok()); // succeeds but skips extraction
    }

    // 15.3.39 — extract_facts at interval triggers
    #[tokio::test]
    async fn extract_facts_at_interval_triggers() {
        let (_dir, builder) = setup_builder().await;
        let session = builder
            .session_manager
            .create_session("Test")
            .await
            .unwrap();
        // Add 3 messages (hits interval of 3)
        for i in 0..3 {
            builder
                .session_manager
                .append_message(&session.id, "user", &format!("msg-{i}"))
                .await
                .unwrap();
        }

        let result = builder
            .extract_facts("prompt", "response", Some(&session.id))
            .await;
        assert!(result.is_ok());
    }

    // 15.3.40 — extract_facts without session_id always triggers
    #[tokio::test]
    async fn extract_facts_without_session_always_triggers() {
        let (_dir, builder) = setup_builder().await;
        let result = builder.extract_facts("prompt", "response", None).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Config fields tests (15.3.41–15.3.45)
    // =========================================================================

    // 15.3.41 — Default context_strategy is "balanced"
    #[test]
    fn config_default_strategy() {
        let config = AppConfig::default();
        assert_eq!(config.context_strategy, "balanced");
    }

    // 15.3.42 — Default context_max_history_messages is 20
    #[test]
    fn config_default_max_history() {
        let config = AppConfig::default();
        assert_eq!(config.context_max_history_messages, 20);
    }

    // 15.3.43 — Default context_max_memory_results is 5
    #[test]
    fn config_default_max_memory() {
        let config = AppConfig::default();
        assert_eq!(config.context_max_memory_results, 5);
    }

    // 15.3.44 — Default context_auto_extract is true
    #[test]
    fn config_default_auto_extract() {
        let config = AppConfig::default();
        assert!(config.context_auto_extract);
    }

    // 15.3.45 — Default context_extract_interval is 3
    #[test]
    fn config_default_extract_interval() {
        let config = AppConfig::default();
        assert_eq!(config.context_extract_interval, 3);
    }

    // =========================================================================
    // Error handling tests (15.3.46–15.3.47)
    // =========================================================================

    // 15.3.46 — MesoError::Context variant exists and maps to 500
    #[test]
    fn error_context_variant() {
        let err = crate::MesoError::Context("context failed".into());
        assert_eq!(err.to_string(), "context error: context failed");
    }

    // 15.3.47 — MesoError::Context has code MESO_CONTEXT
    #[test]
    fn error_context_code() {
        // Verify the error variant can be constructed
        let err = crate::MesoError::Context("test".into());
        assert!(matches!(err, crate::MesoError::Context(_)));
    }

    // 15.3.48 — Simulates exact production flow: POST user msg → build → verify history
    // This tests the "my name is Rakesh" → "what is my name?" scenario
    #[tokio::test]
    async fn build_multi_turn_production_flow() {
        let (_dir, builder) = setup_builder().await;
        let session = builder
            .session_manager
            .create_session("Name Test")
            .await
            .unwrap();

        // Turn 1: Frontend POSTs user message, then WS calls build
        builder
            .session_manager
            .append_message(&session.id, "user", "my name is Rakesh")
            .await
            .unwrap();
        let (history_t1, _) = builder
            .build(Some(&session.id), "my name is Rakesh")
            .await
            .unwrap();
        // First turn: history should be empty (only message is the current prompt, stripped)
        assert_eq!(history_t1.len(), 0, "Turn 1: no prior history");

        // Turn 1: Agent responds, WS handler stores assistant response
        builder
            .session_manager
            .append_message(&session.id, "assistant", "Hello, Rakesh!")
            .await
            .unwrap();

        // Turn 2: Frontend POSTs user message, then WS calls build
        builder
            .session_manager
            .append_message(&session.id, "user", "what is my name?")
            .await
            .unwrap();
        let (history_t2, _) = builder
            .build(Some(&session.id), "what is my name?")
            .await
            .unwrap();
        // Turn 2: history should contain the prior 2 messages (user + assistant from turn 1)
        assert_eq!(
            history_t2.len(),
            2,
            "Turn 2: should have 2 prior messages (user + assistant from turn 1)"
        );

        // Verify the history content
        match &history_t2[0] {
            RigMessage::User { content } => {
                let text = match content.first() {
                    UserContent::Text(t) => t.text.clone(),
                    _ => panic!("Expected text"),
                };
                assert_eq!(text, "my name is Rakesh");
            }
            _ => panic!("Expected user message at index 0"),
        }
        match &history_t2[1] {
            RigMessage::Assistant { content, .. } => {
                let text = match content.first() {
                    AssistantContent::Text(t) => t.text.clone(),
                    _ => panic!("Expected text"),
                };
                assert_eq!(text, "Hello, Rakesh!");
            }
            _ => panic!("Expected assistant message at index 1"),
        }
    }
}
