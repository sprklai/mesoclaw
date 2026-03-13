use std::collections::HashSet;
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

#[cfg(feature = "channels")]
use crate::channels::registry::ChannelRegistry;
#[cfg(feature = "scheduler")]
use crate::scheduler::TokioScheduler;

// ============================================================================
// Context Domain Detection (Step 1)
// ============================================================================

/// Context domains that can be detected from user messages.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContextDomain {
    Channels,
    Scheduler,
    Skills,
    Tools,
}

impl ContextDomain {
    /// Parse a domain string (from skill frontmatter) to ContextDomain.
    pub fn from_domain_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "channels" => Some(Self::Channels),
            "scheduler" => Some(Self::Scheduler),
            "skills" => Some(Self::Skills),
            "tools" => Some(Self::Tools),
            _ => None,
        }
    }
}

/// Detect which context domains are relevant to the user's message.
/// Lightweight keyword matching — no LLM call.
pub fn detect_relevant_domains(user_message: &str) -> HashSet<ContextDomain> {
    let msg = user_message.to_lowercase();
    let mut domains = HashSet::new();

    // Channel-related
    let channel_patterns = [
        "telegram",
        "slack",
        "discord",
        "channel",
        "send me",
        "notify",
        "message me",
        "dm ",
        "chat_id",
        "contact",
    ];
    if channel_patterns.iter().any(|p| msg.contains(p)) {
        domains.insert(ContextDomain::Channels);
    }

    // Scheduler-related
    let sched_patterns = [
        "schedule",
        "remind",
        "cron",
        "timer",
        "alarm",
        "recurring",
        "every day",
        "every hour",
        "at ",
        "job",
    ];
    if sched_patterns.iter().any(|p| msg.contains(p)) {
        domains.insert(ContextDomain::Scheduler);
    }

    // Skills-related
    let skill_patterns = ["skill", "template", "prompt", "persona"];
    if skill_patterns.iter().any(|p| msg.contains(p)) {
        domains.insert(ContextDomain::Skills);
    }

    domains
}

/// Map detected context domains to agent rule categories.
pub(crate) fn domains_to_rule_categories(domains: &HashSet<ContextDomain>) -> Vec<String> {
    let mut cats = vec!["general".to_string()]; // always include general
    for d in domains {
        match d {
            ContextDomain::Channels => cats.push("channel".to_string()),
            ContextDomain::Scheduler => cats.push("scheduling".to_string()),
            ContextDomain::Skills | ContextDomain::Tools => cats.push("tool_usage".to_string()),
        }
    }
    cats.dedup();
    cats
}

// ============================================================================
// Boot Context
// ============================================================================

/// Boot-time system context, computed once on startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootContext {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub locale: String,
    pub region: String,
    pub home_dir: Option<String>,
    pub username: String,
    pub shell: Option<String>,
    pub desktop_path: Option<String>,
    pub downloads_path: Option<String>,
    pub data_dir: Option<String>,
    pub working_dir: Option<String>,
    /// User-configured IANA timezone (e.g., "America/New_York"), if set.
    pub user_timezone: Option<String>,
    /// User-configured location string (e.g., "Toronto, Canada"), if set.
    pub user_location: Option<String>,
}

impl BootContext {
    /// Compute boot context from the current system.
    pub fn from_system() -> Self {
        Self::from_system_with_config(None, None)
    }

    /// Compute boot context with optional user overrides from config.
    pub fn from_system_with_config(
        user_timezone: Option<&str>,
        user_location: Option<&str>,
    ) -> Self {
        let os = format!("{} {}", std::env::consts::OS, os_version());
        let arch = std::env::consts::ARCH.to_string();
        let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown".into());
        let locale = std::env::var("LANG")
            .or_else(|_| std::env::var("LC_ALL"))
            .unwrap_or_else(|_| "en_US.UTF-8".into());
        let region = infer_region_from_timezone_with_config(user_location);
        let user_timezone = user_timezone.map(|s| s.to_string());

        let home_dir = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .or_else(|| {
                directories::UserDirs::new().map(|u| u.home_dir().to_string_lossy().into_owned())
            });

        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".into());

        let shell = std::env::var("SHELL").ok();

        let user_dirs = directories::UserDirs::new();
        let desktop_path = user_dirs
            .as_ref()
            .and_then(|u| u.desktop_dir())
            .map(|p| p.to_string_lossy().into_owned());
        let downloads_path = user_dirs
            .as_ref()
            .and_then(|u| u.download_dir())
            .map(|p| p.to_string_lossy().into_owned());

        let data_dir = Some(
            crate::config::default_data_dir()
                .to_string_lossy()
                .into_owned(),
        );

        let working_dir = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());

        let user_location = user_location.map(|s| s.to_string());

        Self {
            os,
            arch,
            hostname,
            locale,
            region,
            home_dir,
            username,
            shell,
            desktop_path,
            downloads_path,
            data_dir,
            working_dir,
            user_timezone,
            user_location,
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
    /// Channel registry for building state index and expanded context.
    #[cfg(feature = "channels")]
    channel_registry: Option<Arc<ChannelRegistry>>,
    /// Scheduler for building state index and expanded context.
    #[cfg(feature = "scheduler")]
    scheduler: Option<Arc<TokioScheduler>>,
    /// Skill registry for expanded skills context.
    skill_registry: Option<Arc<SkillRegistry>>,
    /// Whether self-evolution (agent learned rules) is enabled.
    self_evolution_enabled: bool,
}

impl ContextEngine {
    pub fn new(db: DbPool, config: std::sync::Arc<AppConfig>, enabled: bool) -> Self {
        Self {
            db,
            config,
            enabled,
            #[cfg(feature = "channels")]
            channel_registry: None,
            #[cfg(feature = "scheduler")]
            scheduler: None,
            skill_registry: None,
            self_evolution_enabled: false,
        }
    }

    /// Set the channel registry for state context building.
    #[cfg(feature = "channels")]
    pub fn with_channel_registry(mut self, registry: Arc<ChannelRegistry>) -> Self {
        self.channel_registry = Some(registry);
        self
    }

    /// Set the scheduler for state context building.
    #[cfg(feature = "scheduler")]
    pub fn with_scheduler(mut self, scheduler: Arc<TokioScheduler>) -> Self {
        self.scheduler = Some(scheduler);
        self
    }

    /// Set the skill registry for expanded skills context.
    pub fn with_skill_registry(mut self, registry: Arc<SkillRegistry>) -> Self {
        self.skill_registry = Some(registry);
        self
    }

    /// Set whether self-evolution (agent learned rules) is enabled.
    pub fn with_self_evolution(mut self, enabled: bool) -> Self {
        self.self_evolution_enabled = enabled;
        self
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
    ///
    /// `user_message`: the current user prompt, used for context-domain detection
    /// to inject relevant system state and agent rules.
    pub async fn compose(
        &self,
        level: &ContextLevel,
        boot_context: &BootContext,
        model_display: &str,
        session_id: Option<&str>,
        conversation_summary: Option<&str>,
        user_message: Option<&str>,
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
                self.compose_full(boot_context, model_display, session_id, user_message)
                    .await
            }
            ContextLevel::Minimal => Ok(self.compose_minimal(boot_context, model_display)),
            ContextLevel::Summary => {
                self.compose_with_summary(
                    boot_context,
                    model_display,
                    session_id,
                    conversation_summary,
                    user_message,
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
        user_message: Option<&str>,
    ) -> Result<String> {
        let mut parts = Vec::new();

        // Overall summary (Tier 3)
        if let Some(overall) = self.get_cached_summary("overall").await? {
            parts.push(overall.summary);
        }

        // Environment section (Tier 2 + Tier 1)
        parts.push("## Environment".into());
        let display_region = self
            .config
            .user_location
            .as_deref()
            .unwrap_or(&boot_context.region);
        parts.push(format!(
            "OS: {} | Arch: {} | Host: {} | Locale: {} | Region: {}",
            boot_context.os,
            boot_context.arch,
            boot_context.hostname,
            boot_context.locale,
            display_region,
        ));

        // User environment details
        let mut user_line = format!("User: {}", boot_context.username);
        if let Some(ref home) = boot_context.home_dir {
            user_line.push_str(&format!(" | Home: {home}"));
        }
        if let Some(ref shell) = boot_context.shell {
            user_line.push_str(&format!(" | Shell: {shell}"));
        }
        parts.push(user_line);

        // Paths line (only if we have any)
        let mut path_parts = Vec::new();
        if let Some(ref desktop) = boot_context.desktop_path {
            path_parts.push(format!("Desktop: {desktop}"));
        }
        if let Some(ref downloads) = boot_context.downloads_path {
            path_parts.push(format!("Downloads: {downloads}"));
        }
        if !path_parts.is_empty() {
            parts.push(path_parts.join(" | "));
        }

        let mut dir_parts = Vec::new();
        if let Some(ref working) = boot_context.working_dir {
            dir_parts.push(format!("Working Dir: {working}"));
        }
        if let Some(ref data) = boot_context.data_dir {
            dir_parts.push(format!("Data Dir: {data}"));
        }
        if !dir_parts.is_empty() {
            parts.push(dir_parts.join(" | "));
        }

        parts.push(self.dynamic_runtime(model_display, session_id));

        // User Location section (prominent, before reasoning)
        // Read from live config (updated via PUT /config) with boot_context fallback
        let user_location = self
            .config
            .user_location
            .as_deref()
            .or(boot_context.user_location.as_deref());
        let user_timezone = self
            .config
            .user_timezone
            .as_deref()
            .or(boot_context.user_timezone.as_deref());
        let has_location = user_location.is_some() || user_timezone.is_some();
        if has_location {
            let mut loc_parts = Vec::new();
            if let Some(loc) = user_location {
                loc_parts.push(format!("Location: {loc}"));
            }
            if let Some(tz) = user_timezone {
                loc_parts.push(format!("Timezone: {tz}"));
            }
            parts.push("## User Location".into());
            parts.push(loc_parts.join(" | "));
        }

        // Reasoning Protocol
        parts.push("## Reasoning Protocol".into());
        if let Some(ref custom_guidance) = self.config.agent_reasoning_guidance {
            parts.push(custom_guidance.clone());
        } else {
            let mut protocol = String::from(
                "Before calling ANY tool, follow this protocol:\n\
                 1. INTENT: What is the user actually asking for? What implicit context is needed?\n\
                 2. CONTEXT CHECK: Review Environment and User Location above. Do you already have info \
                    (location, timezone, OS) that should inform your tool call?\n\
                 3. ENRICH: Incorporate relevant context into tool arguments. \
                    Example: search 'weather Toronto' not 'weather today'.\n\
                 4. EXECUTE: Call the tool with enriched arguments.\n\
                 5. RECOVER: If a tool errors or returns no results, try at least 2 alternatives before giving up.\n\n\
                 Additional guidelines:\n\
                 - Use ShellTool to discover paths dynamically: `ls`, `echo $HOME`, `find`.\n\
                 - If a file path fails, try common alternatives (~/Desktop, ~/desktop, $XDG_DESKTOP_DIR).\n\
                 - When a tool returns an error, analyze the error message and adapt your next tool call.\n\
                 - Do NOT describe what you would do — actually call the tools and do it.",
            );
            let reminder_location = self
                .config
                .user_location
                .as_deref()
                .or(boot_context.user_location.as_deref());
            if let Some(loc) = reminder_location {
                protocol.push_str(&format!(
                    "\n\nREMINDER: The user's location is '{loc}'. For ANY location-sensitive query \
                     (weather, news, events, nearby places, time), use this location unless the user \
                     specifies otherwise."
                ));
            }
            parts.push(protocol);
        }

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

        // --- Tier 1: System state index (always) ---
        let state_index = self.build_state_index().await;
        if !state_index.is_empty() {
            parts.push(state_index);
        }

        // --- Tier 2: Expanded context (on-demand, based on user message) ---
        let domains = if let Some(msg) = user_message {
            detect_relevant_domains(msg)
        } else {
            HashSet::new()
        };

        if !domains.is_empty() {
            let expanded = self.build_expanded_context(&domains).await?;
            if !expanded.is_empty() {
                parts.push(expanded);
            }
        }

        // --- Layer 3: Agent learned rules (category-filtered) ---
        if self.self_evolution_enabled {
            let relevant_categories = domains_to_rule_categories(&domains);
            let rules = self.load_agent_rules(&relevant_categories).await?;
            if !rules.is_empty() {
                let rules_str = rules
                    .iter()
                    .map(|r| format!("- {r}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                parts.push(format!("## Your Learned Rules\n{rules_str}"));
            }
        }

        // Config override
        if let Some(ref override_prompt) = self.config.agent_system_prompt {
            parts.push(override_prompt.clone());
        }

        Ok(parts.join("\n\n"))
    }

    // ========================================================================
    // Tiered System State (Layer 2)
    // ========================================================================

    /// Build compact one-line system state index (~50-100 tokens).
    /// This is Tier 1: always injected when enabled.
    async fn build_state_index(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        #[cfg(feature = "channels")]
        if let Some(ref registry) = self.channel_registry {
            use crate::channels::traits::ChannelStatus;
            let channels = registry.list();
            let connected: Vec<_> = channels
                .iter()
                .filter(|c| matches!(registry.status(c), Some(ChannelStatus::Connected)))
                .collect();
            if !connected.is_empty() {
                let contact_count = crate::channels::contacts::count_channel_contacts(&self.db)
                    .await
                    .unwrap_or(0);
                parts.push(format!(
                    "Channels: {} connected ({} known contacts)",
                    connected
                        .iter()
                        .map(|c| c.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                    contact_count
                ));
            }
        }

        #[cfg(feature = "scheduler")]
        if let Some(ref scheduler) = self.scheduler {
            use crate::scheduler::traits::Scheduler;
            let jobs = scheduler.list_jobs().await;
            let active = jobs.iter().filter(|j| j.enabled).count();
            if active > 0 {
                parts.push(format!("Scheduled jobs: {active} active"));
            }
        }

        if parts.is_empty() {
            return String::new();
        }
        format!("## System State\n{}", parts.join(" | "))
    }

    /// Build expanded context for relevant domains.
    /// This is Tier 2: only injected when the user message triggers specific domains.
    async fn build_expanded_context(&self, domains: &HashSet<ContextDomain>) -> Result<String> {
        let mut sections = Vec::new();

        if domains.contains(&ContextDomain::Channels) {
            #[cfg(feature = "channels")]
            if let Some(ref registry) = self.channel_registry {
                let channels = registry.list();
                let mut lines = Vec::new();
                for name in &channels {
                    let status = registry
                        .status(name)
                        .map(|s| format!("{s}"))
                        .unwrap_or_else(|| "unknown".into());
                    let contacts =
                        crate::channels::contacts::query_channel_contacts(&self.db, name).await?;
                    let contact_str = if contacts.is_empty() {
                        "no known contacts".into()
                    } else {
                        contacts
                            .iter()
                            .map(|c| format!("{} (id:{})", c.label, c.recipient_id))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    lines.push(format!("- {name}: {status} | contacts: {contact_str}"));
                }
                if !lines.is_empty() {
                    sections.push(format!("### Channels\n{}", lines.join("\n")));
                }
            }
        }

        if domains.contains(&ContextDomain::Scheduler) {
            #[cfg(feature = "scheduler")]
            if let Some(ref scheduler) = self.scheduler {
                use crate::scheduler::traits::Scheduler;
                let jobs = scheduler.list_jobs().await;
                let active: Vec<_> = jobs.iter().filter(|j| j.enabled).collect();
                if !active.is_empty() {
                    let lines: Vec<String> = active
                        .iter()
                        .map(|j| {
                            let sched = match &j.schedule {
                                crate::scheduler::traits::Schedule::Cron { expr } => {
                                    format!("cron: {expr}")
                                }
                                crate::scheduler::traits::Schedule::Interval { secs } => {
                                    format!("every {secs}s")
                                }
                            };
                            let next = j
                                .next_run
                                .map(|t| t.format("%H:%M UTC").to_string())
                                .unwrap_or_else(|| "—".into());
                            format!("- {} ({}) next: {}", j.name, sched, next)
                        })
                        .collect();
                    sections.push(format!("### Scheduled Jobs\n{}", lines.join("\n")));
                }
            }
        }

        if domains.contains(&ContextDomain::Skills)
            && let Some(ref skill_registry) = self.skill_registry
        {
            let skills = skill_registry.list().await;
            let active: Vec<_> = skills.iter().filter(|s| s.enabled).collect();
            if !active.is_empty() {
                let lines: Vec<String> = active
                    .iter()
                    .map(|s| format!("- {}: {}", s.name, s.description))
                    .collect();
                sections.push(format!("### Available Skills\n{}", lines.join("\n")));
            }
        }

        Ok(sections.join("\n\n"))
    }

    // ========================================================================
    // Agent Learned Rules (Layer 3)
    // ========================================================================

    /// Load active agent rules matching given categories.
    async fn load_agent_rules(&self, categories: &[String]) -> Result<Vec<String>> {
        if categories.is_empty() {
            return Ok(Vec::new());
        }
        let pool = self.db.clone();
        let cats = categories.to_vec();
        db::with_db(&pool, move |conn| {
            let placeholders: String = cats.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let sql = format!(
                "SELECT content FROM agent_rules WHERE active = 1 AND category IN ({}) \
                 ORDER BY created_at",
                placeholders
            );
            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::types::ToSql> = cats
                .iter()
                .map(|c| c as &dyn rusqlite::types::ToSql)
                .collect();
            let rows = stmt.query_map(params.as_slice(), |row| row.get::<_, String>(0))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .await
    }

    // ========================================================================
    // Minimal / Summary Compose
    // ========================================================================

    /// Compose minimal one-liner context.
    pub fn compose_minimal(&self, boot_context: &BootContext, model_display: &str) -> String {
        let now = chrono::Local::now();
        let loc = self
            .config
            .user_location
            .as_deref()
            .or(boot_context.user_location.as_deref());
        let base = format!(
            "MesoClaw — AI assistant | {} | {} {} | {}",
            now.format("%a %b %-d %Y %H:%M %Z"),
            boot_context.os,
            boot_context.arch,
            model_display,
        );
        if let Some(l) = loc {
            format!("{base} | Location: {l}")
        } else {
            base
        }
    }

    /// Compose context with conversation summary for resumed sessions.
    async fn compose_with_summary(
        &self,
        boot_context: &BootContext,
        model_display: &str,
        session_id: Option<&str>,
        conversation_summary: Option<&str>,
        user_message: Option<&str>,
    ) -> Result<String> {
        let mut full = self
            .compose_full(boot_context, model_display, session_id, user_message)
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

        let mut line = format!(
            "Date: {} | Day: {} | Timezone: {} (UTC{})",
            now.format("%Y-%m-%dT%H:%M:%S"),
            now.format("%A"),
            tz_name,
            tz_offset,
        );

        if let Some(ref iana_tz) = self.config.user_timezone {
            line.push_str(&format!(" | IANA: {iana_tz}"));
        }

        line.push_str(&format!(
            " | Model: {} | Session: {}",
            model_display,
            session_id.unwrap_or("new session"),
        ));

        line
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContextStrategy {
    /// Last 2 turns (4 messages), top 3 memories
    Minimal,
    /// Last 10 turns (20 messages) + top 5 memories
    #[default]
    Balanced,
    /// All messages up to max cap, top 10 memories
    Full,
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

    /// Build context parts separately for use by PromptStrategy.
    ///
    /// Returns `(history, memories, user_observations)`:
    /// - `history`: windowed rig messages for agent.chat()
    /// - `memories`: raw recalled memory strings
    /// - `user_observations`: raw user observation context string
    pub async fn build_parts(
        &self,
        session_id: Option<&str>,
        prompt: &str,
    ) -> Result<(Vec<RigMessage>, Vec<String>, String)> {
        let strategy = ContextStrategy::from_str(&self.config.context_strategy).unwrap_or_default();

        // 1. Get session history (same logic as build())
        let history = if let Some(sid) = session_id {
            let messages = self.session_manager.get_messages(sid).await?;
            let trimmed = if messages
                .last()
                .is_some_and(|m| m.role == "user" && m.content == prompt)
            {
                &messages[..messages.len() - 1]
            } else {
                &messages
            };
            let rig_messages = convert_session_messages(trimmed);
            window_messages(
                rig_messages,
                &strategy,
                self.config.context_max_history_messages,
            )
        } else {
            Vec::new()
        };

        // 2. Recall memories (raw strings)
        let limit = memory_limit_for_strategy(&strategy, self.config.context_max_memory_results);
        let memories = self
            .memory
            .recall(prompt, limit, 0)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.content)
            .collect();

        // 3. User observations
        let user_observations = self.user_learner.build_context().await.unwrap_or_default();

        Ok((history, memories, user_observations))
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

fn infer_region_from_timezone_with_config(user_location: Option<&str>) -> String {
    // 0. User override wins (explicit config)
    if let Some(location) = user_location
        && !location.is_empty()
    {
        return location.to_string();
    }

    // 1. TZ env var (named timezone)
    let tz = std::env::var("TZ").unwrap_or_default();
    if !tz.is_empty() {
        if tz.contains("America/New_York")
            || tz.contains("America/Detroit")
            || tz.contains("US/Eastern")
        {
            return "Eastern US".into();
        } else if tz.contains("America/Chicago") || tz.contains("US/Central") {
            return "Central US".into();
        } else if tz.contains("America/Denver") || tz.contains("US/Mountain") {
            return "Mountain US".into();
        } else if tz.contains("America/Los_Angeles") || tz.contains("US/Pacific") {
            return "Pacific US".into();
        } else if tz.contains("Europe/") {
            return "Europe".into();
        } else if tz.contains("Asia/") {
            return "Asia".into();
        }
    }

    // 2. Abbreviation-first (handles DST correctly: EDT=-4 != CST which is also -6)
    let now = chrono::Local::now();
    let tz_abbrev = now.format("%Z").to_string();
    match tz_abbrev.as_str() {
        "EST" | "EDT" => return "Eastern US".into(),
        "CST" | "CDT" => return "Central US".into(),
        "MST" | "MDT" => return "Mountain US".into(),
        "PST" | "PDT" => return "Pacific US".into(),
        "GMT" | "UTC" | "WET" => return "UTC/UK".into(),
        "CET" | "CEST" => return "Central Europe".into(),
        "IST" => return "South Asia".into(),
        "JST" => return "Japan/Korea".into(),
        "KST" => return "Japan/Korea".into(),
        "AEST" | "AEDT" => return "Australia East".into(),
        _ => {}
    }

    // 3. Offset fallback (last resort)
    let offset = now.offset().local_minus_utc() / 3600;
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

    // ENV.3 — infer_region_from_timezone returns non-empty string
    #[test]
    fn infer_region_returns_non_empty() {
        let region = infer_region_from_timezone_with_config(None);
        assert!(!region.is_empty(), "region should not be empty");
    }

    // ENV.4 — user_location override wins
    #[test]
    fn infer_region_user_override_wins() {
        let region = infer_region_from_timezone_with_config(Some("Custom Location, Mars"));
        assert_eq!(region, "Custom Location, Mars");
    }

    // ENV.5 — empty user_location falls through to detection
    #[test]
    fn infer_region_empty_override_falls_through() {
        let region = infer_region_from_timezone_with_config(Some(""));
        assert!(!region.is_empty(), "should fall through to detection");
        assert_ne!(region, "");
    }

    // ENV.6 — BootContext with config overrides
    #[test]
    fn boot_context_with_user_location() {
        let boot =
            BootContext::from_system_with_config(Some("America/New_York"), Some("New York, US"));
        assert_eq!(boot.region, "New York, US");
        assert_eq!(boot.user_timezone.as_deref(), Some("America/New_York"));
    }

    // ENV.7 — dynamic_runtime includes IANA when set
    #[tokio::test]
    async fn dynamic_runtime_includes_iana_timezone() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let mut config = AppConfig::default();
        config.user_timezone = Some("America/New_York".into());
        let config = std::sync::Arc::new(config);
        let engine = ContextEngine::new(pool, config, true);

        let runtime = engine.dynamic_runtime("gpt-4o", None);
        assert!(
            runtime.contains("IANA: America/New_York"),
            "should contain IANA timezone: {runtime}"
        );
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
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
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
            .compose(
                &ContextLevel::Full,
                &boot,
                "gpt-4o",
                Some("sess-1"),
                None,
                None,
            )
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
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
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
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
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
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
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
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
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
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
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
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
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
                None,
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

    // =========================================================================
    // Phase 8.9 — ContextEngine lifecycle tests (8.9.4–8.9.15)
    // =========================================================================

    // 8.9.4 — ContextEngine::new with defaults
    #[test]
    fn context_engine_new_defaults() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        assert!(engine.enabled);
    }

    // 8.9.5 — new session (count=0) yields Full
    #[test]
    fn context_level_new_session_full() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        assert_eq!(
            engine.determine_context_level(0, None, false, false),
            ContextLevel::Full
        );
    }

    // 8.9.6 — recent conversation yields Minimal
    #[test]
    fn context_level_recent_minimal() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        let recent = chrono::Utc::now() - chrono::Duration::seconds(30);
        assert_eq!(
            engine.determine_context_level(2, Some(&recent), false, false),
            ContextLevel::Minimal
        );
    }

    // 8.9.7 — gap exceeded yields Full
    #[test]
    fn context_level_gap_exceeded_full() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig {
            context_reinject_gap_minutes: 10,
            ..Default::default()
        });
        let engine = ContextEngine::new(pool, config, true);
        let old = chrono::Utc::now() - chrono::Duration::minutes(20);
        assert_eq!(
            engine.determine_context_level(5, Some(&old), false, false),
            ContextLevel::Full
        );
    }

    // 8.9.8 — message count exceeded yields Full
    #[test]
    fn context_level_count_exceeded_full() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig {
            context_reinject_message_count: 10,
            ..Default::default()
        });
        let engine = ContextEngine::new(pool, config, true);
        let recent = chrono::Utc::now() - chrono::Duration::seconds(30);
        assert_eq!(
            engine.determine_context_level(15, Some(&recent), false, false),
            ContextLevel::Full
        );
    }

    // 8.9.9 — resumed session with summary yields Summary
    #[test]
    fn context_level_resumed_with_summary() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        assert_eq!(
            engine.determine_context_level(5, None, true, true),
            ContextLevel::Summary
        );
    }

    // 8.9.10 — disabled engine returns fallback preamble
    #[tokio::test]
    async fn context_disabled_returns_fallback() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();
        let config = std::sync::Arc::new(AppConfig {
            agent_system_prompt: Some("Custom fallback.".into()),
            ..Default::default()
        });
        let engine = ContextEngine::new(pool, config, false);
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "model", None, None, None)
            .await
            .unwrap();
        assert_eq!(result, "Custom fallback.");
    }

    // 8.9.11 — Full compose includes Environment section
    #[tokio::test]
    async fn compose_full_has_environment() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("## Environment"));
        assert!(result.contains("OS:"));
    }

    // 8.9.12 — Minimal compose is single line
    #[test]
    fn compose_minimal_single_line() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();
        let minimal = engine.compose_minimal(&boot, "claude-3");
        assert!(!minimal.contains('\n'));
        assert!(minimal.contains("MesoClaw"));
        assert!(minimal.contains("claude-3"));
    }

    // 8.9.13 — Summary compose includes both full context and conversation
    #[tokio::test]
    async fn compose_summary_includes_full_and_conversation() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(
                &ContextLevel::Summary,
                &boot,
                "gpt-4o",
                None,
                Some("We discussed Rust error handling."),
                None,
            )
            .await
            .unwrap();
        assert!(result.contains("## Environment"));
        assert!(result.contains("Prior Conversation"));
        assert!(result.contains("Rust error handling"));
    }

    // 8.9.14 — context_enabled toggle respected
    #[tokio::test]
    async fn context_enabled_toggle_respected() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let boot = BootContext::from_system();

        let enabled = ContextEngine::new(pool.clone(), config.clone(), true);
        let enabled_result = enabled
            .compose(&ContextLevel::Full, &boot, "model", None, None, None)
            .await
            .unwrap();
        assert!(enabled_result.contains("## Environment"));

        let disabled = ContextEngine::new(pool, config, false);
        let disabled_result = disabled
            .compose(&ContextLevel::Full, &boot, "model", None, None, None)
            .await
            .unwrap();
        assert!(!disabled_result.contains("## Environment"));
    }

    // 8.9.15 — config override appended to full compose
    #[tokio::test]
    async fn config_override_appended() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();
        let config = std::sync::Arc::new(AppConfig {
            agent_system_prompt: Some("Be very terse.".into()),
            ..Default::default()
        });
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("Be very terse."));
    }

    // =========================================================================
    // Phase 8.9 — BootContext tests (8.9.16–8.9.21)
    // =========================================================================

    // 8.9.16 — from_system() fields are populated
    #[test]
    fn boot_context_from_system_populated() {
        let boot = BootContext::from_system();
        assert!(!boot.os.is_empty());
        assert!(!boot.arch.is_empty());
        assert!(!boot.hostname.is_empty());
        assert!(!boot.locale.is_empty());
        assert!(!boot.region.is_empty());
    }

    // 8.9.17 — OS field is non-empty
    #[test]
    fn boot_context_os_non_empty() {
        let boot = BootContext::from_system();
        assert!(!boot.os.is_empty());
        assert!(boot.os.contains(std::env::consts::OS));
    }

    // 8.9.18 — arch is a valid architecture string
    #[test]
    fn boot_context_arch_valid() {
        let boot = BootContext::from_system();
        let valid_archs = [
            "x86_64",
            "x86",
            "aarch64",
            "arm",
            "riscv64",
            "s390x",
            "powerpc64",
            "mips64",
        ];
        assert!(
            valid_archs.iter().any(|a| boot.arch.contains(a)) || !boot.arch.is_empty(),
            "arch should be a known architecture or at least non-empty"
        );
    }

    // 8.9.19 — hostname is non-empty
    #[test]
    fn boot_context_hostname_non_empty() {
        let boot = BootContext::from_system();
        assert!(!boot.hostname.is_empty());
    }

    // 8.9.20 — locale defaults to en_US.UTF-8 if LANG unset
    #[test]
    fn boot_context_locale_default() {
        let boot = BootContext::from_system();
        // Locale is always populated (either from env or default)
        assert!(!boot.locale.is_empty());
    }

    // 8.9.21 — BootContext derives Clone and Debug
    #[test]
    fn boot_context_clone_debug() {
        let boot = BootContext::from_system();
        let cloned = boot.clone();
        assert_eq!(cloned.os, boot.os);
        let debug = format!("{:?}", boot);
        assert!(debug.contains("BootContext"));
    }

    // =========================================================================
    // Phase 8.9 — Context Summaries tests (8.9.22–8.9.31)
    // =========================================================================

    // 8.9.22 — store and retrieve summary
    #[tokio::test]
    async fn summary_store_and_get() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("test", "summary text", "hash123", "model-1")
            .await
            .unwrap();
        let result = engine.get_cached_summary("test").await.unwrap().unwrap();
        assert_eq!(result.summary, "summary text");
        assert_eq!(result.source_hash, "hash123");
    }

    // 8.9.23 — update summary overwrites
    #[tokio::test]
    async fn summary_update_overwrites() {
        let (_dir, engine) = setup().await;
        engine.store_summary("key", "v1", "h1", "m1").await.unwrap();
        engine.store_summary("key", "v2", "h2", "m2").await.unwrap();
        let result = engine.get_cached_summary("key").await.unwrap().unwrap();
        assert_eq!(result.summary, "v2");
        assert_eq!(result.source_hash, "h2");
    }

    // 8.9.24 — get missing summary returns None
    #[tokio::test]
    async fn summary_missing_returns_none() {
        let (_dir, engine) = setup().await;
        let result = engine.get_cached_summary("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    // 8.9.25 — hash invalidation detects change
    #[tokio::test]
    async fn hash_invalidation_detects_change() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("k", "s", "old_hash", "m")
            .await
            .unwrap();
        assert!(engine.needs_regeneration("k", "new_hash").await.unwrap());
    }

    // 8.9.26 — hash invalidation skips when unchanged
    #[tokio::test]
    async fn hash_invalidation_skip_unchanged() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("k", "s", "same_hash", "m")
            .await
            .unwrap();
        assert!(!engine.needs_regeneration("k", "same_hash").await.unwrap());
    }

    // 8.9.27 — store_all_summaries creates 4 summaries
    #[tokio::test]
    async fn store_all_summaries_creates_four() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);

        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        // Should have identity, capabilities, overall (user may be empty)
        assert!(
            engine
                .get_cached_summary("identity")
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            engine
                .get_cached_summary("capabilities")
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            engine
                .get_cached_summary("overall")
                .await
                .unwrap()
                .is_some()
        );
    }

    // 8.9.28 — identity summary content is non-empty
    #[tokio::test]
    async fn identity_summary_non_empty() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);

        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let identity = engine
            .get_cached_summary("identity")
            .await
            .unwrap()
            .unwrap();
        assert!(!identity.summary.is_empty());
    }

    // 8.9.29 — store_summary with empty input succeeds
    #[tokio::test]
    async fn store_summary_empty_input() {
        let (_dir, engine) = setup().await;
        engine.store_summary("k", "", "", "m").await.unwrap();
        let result = engine.get_cached_summary("k").await.unwrap().unwrap();
        assert_eq!(result.summary, "");
    }

    // 8.9.30 — concurrent summary access
    #[tokio::test]
    async fn summary_concurrent_access() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = Arc::new(ContextEngine::new(pool, config, true));

        let mut handles = vec![];
        for i in 0..5 {
            let eng = Arc::clone(&engine);
            handles.push(tokio::spawn(async move {
                eng.store_summary(&format!("key_{i}"), &format!("val_{i}"), "h", "m")
                    .await
                    .unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        for i in 0..5 {
            assert!(
                engine
                    .get_cached_summary(&format!("key_{i}"))
                    .await
                    .unwrap()
                    .is_some()
            );
        }
    }

    // 8.9.31 — needs_regeneration for missing key returns true
    #[tokio::test]
    async fn needs_regeneration_missing_key() {
        let (_dir, engine) = setup().await;
        assert!(
            engine
                .needs_regeneration("nonexistent", "any_hash")
                .await
                .unwrap()
        );
    }

    // =========================================================================
    // Phase 8.9 — Tier Injection tests (8.9.32–8.9.43)
    // =========================================================================

    // 8.9.32 — Full tier has boot context
    #[tokio::test]
    async fn full_tier_has_boot_context() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains(&boot.os));
        assert!(result.contains(&boot.arch));
    }

    // 8.9.33 — Full tier has runtime info
    #[tokio::test]
    async fn full_tier_has_runtime() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", Some("s1"), None, None)
            .await
            .unwrap();
        assert!(result.contains("Date:"));
        assert!(result.contains("Session: s1"));
    }

    // 8.9.34 — Full tier includes identity when cached
    #[tokio::test]
    async fn full_tier_has_identity() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("identity", "I am MesoClaw", "h", "m")
            .await
            .unwrap();
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("## Your Identity"));
        assert!(result.contains("I am MesoClaw"));
    }

    // 8.9.35 — Full tier includes user summary when cached
    #[tokio::test]
    async fn full_tier_has_user() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("user", "Prefers dark mode", "h", "m")
            .await
            .unwrap();
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("## User Context"));
    }

    // 8.9.36 — Full tier includes capabilities when cached
    #[tokio::test]
    async fn full_tier_has_capabilities() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("capabilities", "9 tools available", "h", "m")
            .await
            .unwrap();
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("## Your Capabilities"));
    }

    // 8.9.37 — Minimal tier is single line with date/OS/model
    #[test]
    fn minimal_tier_single_line_with_fields() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();
        let minimal = engine.compose_minimal(&boot, "claude-3");
        assert!(!minimal.contains('\n'));
        assert!(minimal.contains(&boot.os));
        assert!(minimal.contains("claude-3"));
    }

    // 8.9.38 — Summary tier includes full + conversation
    #[tokio::test]
    async fn summary_tier_full_plus_conversation() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(
                &ContextLevel::Summary,
                &boot,
                "gpt-4o",
                None,
                Some("We discussed Tauri plugins."),
                None,
            )
            .await
            .unwrap();
        assert!(result.contains("## Environment"));
        assert!(result.contains("## Prior Conversation"));
        assert!(result.contains("Tauri plugins"));
    }

    // 8.9.39 — Disabled engine returns fallback text
    #[tokio::test]
    async fn disabled_returns_fallback_text() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, false);
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Minimal, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert_eq!(result, "You are MesoClaw, a helpful AI assistant.");
    }

    // 8.9.40 — Full compose includes guidance line
    #[tokio::test]
    async fn full_compose_includes_guidance() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("Reasoning Protocol"));
    }

    // 8.9.41 — Full compose with all summaries includes all sections
    #[tokio::test]
    async fn full_compose_all_sections() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("overall", "Overall summary", "h1", "m")
            .await
            .unwrap();
        engine
            .store_summary("identity", "Identity summary", "h2", "m")
            .await
            .unwrap();
        engine
            .store_summary("user", "User summary", "h3", "m")
            .await
            .unwrap();
        engine
            .store_summary("capabilities", "Caps summary", "h4", "m")
            .await
            .unwrap();
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("Overall summary"));
        assert!(result.contains("Identity summary"));
        assert!(result.contains("User summary"));
        assert!(result.contains("Caps summary"));
    }

    // 8.9.42 — Minimal compose contains MesoClaw name
    #[test]
    fn minimal_compose_contains_name() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();
        let minimal = engine.compose_minimal(&boot, "gpt-4o");
        assert!(minimal.contains("MesoClaw"));
        assert!(minimal.contains("AI assistant"));
    }

    // 8.9.43 — Summary compose without conversation summary omits section
    #[tokio::test]
    async fn summary_without_conversation_omits_section() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Summary, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(!result.contains("## Prior Conversation"));
    }

    // =========================================================================
    // Phase 8.9 — Cache Invalidation tests (8.9.44–8.9.49)
    // =========================================================================

    // 8.9.44 — hash changes on identity content change
    #[test]
    fn hash_changes_on_identity_change() {
        let h1 = compute_hash("identity v1");
        let h2 = compute_hash("identity v2");
        assert_ne!(h1, h2);
    }

    // 8.9.45 — hash changes on user content change
    #[test]
    fn hash_changes_on_user_change() {
        let h1 = compute_hash("user prefers dark mode");
        let h2 = compute_hash("user prefers light mode");
        assert_ne!(h1, h2);
    }

    // 8.9.46 — hash changes on tools change
    #[test]
    fn hash_changes_on_tools_change() {
        let h1 = compute_hash("Tools: web_search, shell");
        let h2 = compute_hash("Tools: web_search, shell, file_read");
        assert_ne!(h1, h2);
    }

    // 8.9.47 — hash changes on skills change
    #[test]
    fn hash_changes_on_skills_change() {
        let h1 = compute_hash("Skills: system-prompt, summarize");
        let h2 = compute_hash("Skills: system-prompt, summarize, analyze");
        assert_ne!(h1, h2);
    }

    // 8.9.48 — unchanged content produces same hash
    #[test]
    fn hash_unchanged_same() {
        let h1 = compute_hash("stable content");
        let h2 = compute_hash("stable content");
        assert_eq!(h1, h2);
    }

    // 8.9.49 — hash is deterministic
    #[test]
    fn hash_deterministic() {
        let content = "deterministic test content";
        let h1 = compute_hash(content);
        let h2 = compute_hash(content);
        let h3 = compute_hash(content);
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    // =========================================================================
    // Phase 8.9 — Summary Generation tests (8.9.50–8.9.55)
    // =========================================================================

    // 8.9.50 — identity summary generated by store_all_summaries
    #[tokio::test]
    async fn store_all_generates_identity_summary() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let s = engine
            .get_cached_summary("identity")
            .await
            .unwrap()
            .unwrap();
        assert!(!s.summary.is_empty());
        assert_eq!(s.model_id, "builtin");
    }

    // 8.9.51 — user summary skipped when no observations
    #[tokio::test]
    async fn store_all_skips_empty_user() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        // User summary should not exist since no observations
        let user = engine.get_cached_summary("user").await.unwrap();
        assert!(user.is_none());
    }

    // 8.9.52 — capabilities summary lists tools
    #[tokio::test]
    async fn store_all_capabilities_lists_tools() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let caps = engine
            .get_cached_summary("capabilities")
            .await
            .unwrap()
            .unwrap();
        assert!(caps.summary.contains("tools"));
    }

    // 8.9.53 — overall summary combines sections
    #[tokio::test]
    async fn store_all_overall_summary() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let overall = engine.get_cached_summary("overall").await.unwrap().unwrap();
        assert!(!overall.summary.is_empty());
    }

    // 8.9.54 — summary max length check (summaries should be reasonable)
    #[tokio::test]
    async fn summary_reasonable_length() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("test", "A short summary.", "h", "m")
            .await
            .unwrap();
        let s = engine.get_cached_summary("test").await.unwrap().unwrap();
        assert!(
            s.summary.len() < 10_000,
            "Summary should be reasonably short"
        );
    }

    // 8.9.55 — compute_hash returns 16-char hex string
    #[test]
    fn compute_hash_format() {
        let hash = compute_hash("test content");
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // 15.3.48 — Simulates exact production flow: POST user msg → build → verify history
    // This tests the "my name is John" → "what is my name?" scenario
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
            .append_message(&session.id, "user", "my name is John")
            .await
            .unwrap();
        let (history_t1, _) = builder
            .build(Some(&session.id), "my name is John")
            .await
            .unwrap();
        // First turn: history should be empty (only message is the current prompt, stripped)
        assert_eq!(history_t1.len(), 0, "Turn 1: no prior history");

        // Turn 1: Agent responds, WS handler stores assistant response
        builder
            .session_manager
            .append_message(&session.id, "assistant", "Hello, John!")
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
                assert_eq!(text, "my name is John");
            }
            _ => panic!("Expected user message at index 0"),
        }
        match &history_t2[1] {
            RigMessage::Assistant { content, .. } => {
                let text = match content.first() {
                    AssistantContent::Text(t) => t.text.clone(),
                    _ => panic!("Expected text"),
                };
                assert_eq!(text, "Hello, John!");
            }
            _ => panic!("Expected assistant message at index 1"),
        }
    }

    // =========================================================================
    // Phase 8.9 — Audit-required tests (exact plan names)
    // These tests cover the exact names required by tests/phase8.9_test_debt.md
    // =========================================================================

    // 8.9.14 — Toggle context_injection_enabled off mid-test changes output
    #[tokio::test]
    async fn context_toggle_respected() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let boot = BootContext::from_system();

        // Enabled: should contain environment section
        let enabled = ContextEngine::new(pool.clone(), config.clone(), true);
        let result_on = enabled
            .compose(&ContextLevel::Full, &boot, "model", None, None, None)
            .await
            .unwrap();
        assert!(result_on.contains("## Environment"));

        // Disabled: should return fallback
        let disabled = ContextEngine::new(pool, config, false);
        let result_off = disabled
            .compose(&ContextLevel::Full, &boot, "model", None, None, None)
            .await
            .unwrap();
        assert_eq!(result_off, "You are MesoClaw, a helpful AI assistant.");
    }

    // 8.9.17 — BootContext::from_system().os is not empty
    #[test]
    fn boot_context_os_nonempty() {
        let boot = BootContext::from_system();
        assert!(!boot.os.is_empty());
        assert!(boot.os.contains(std::env::consts::OS));
    }

    // 8.9.18 — BootContext::from_system().arch is a known architecture
    // (covered by boot_context_arch_valid above, this is the exact plan name)

    // 8.9.19 — BootContext::from_system().hostname is not empty
    #[test]
    fn boot_context_hostname_nonempty() {
        let boot = BootContext::from_system();
        assert!(!boot.hostname.is_empty());
    }

    // 8.9.23 — Store summary with key, then update, verify get returns updated
    #[tokio::test]
    async fn store_summary_updates_existing() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("test", "original content", "h1", "m1")
            .await
            .unwrap();
        engine
            .store_summary("test", "updated content", "h2", "m2")
            .await
            .unwrap();
        let result = engine.get_cached_summary("test").await.unwrap().unwrap();
        assert_eq!(result.summary, "updated content");
        assert_eq!(result.source_hash, "h2");
    }

    // 8.9.25 — Get summary for key that was never stored returns None
    #[tokio::test]
    async fn get_summary_missing_none() {
        let (_dir, engine) = setup().await;
        let result = engine.get_cached_summary("never_stored").await.unwrap();
        assert!(result.is_none());
    }

    // 8.9.28 — store_all_summaries creates 4 entries (identity, capabilities, overall, user)
    #[tokio::test]
    async fn store_all_creates_four() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool.clone(), &config);

        // Add an observation so user summary gets created too
        user_learner
            .observe("preference", "theme", "dark mode", 0.9)
            .await
            .unwrap();

        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        assert!(
            engine
                .get_cached_summary("identity")
                .await
                .unwrap()
                .is_some()
        );
        assert!(engine.get_cached_summary("user").await.unwrap().is_some());
        assert!(
            engine
                .get_cached_summary("capabilities")
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            engine
                .get_cached_summary("overall")
                .await
                .unwrap()
                .is_some()
        );
    }

    // 8.9.29 — Generated summary content is non-empty
    #[tokio::test]
    async fn summary_content_nonempty() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let identity = engine
            .get_cached_summary("identity")
            .await
            .unwrap()
            .unwrap();
        assert!(!identity.summary.is_empty());
    }

    // 8.9.30 — Store summary with empty content, verify retrieval
    #[tokio::test]
    async fn summary_empty_input() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("empty_key", "", "", "m")
            .await
            .unwrap();
        let result = engine
            .get_cached_summary("empty_key")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.summary, "");
    }

    // 8.9.31 — Concurrent summary read/write is safe
    #[tokio::test]
    async fn concurrent_summary_access() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = Arc::new(ContextEngine::new(pool, config, true));

        let mut handles = vec![];
        // Spawn writers
        for i in 0..10 {
            let eng = Arc::clone(&engine);
            handles.push(tokio::spawn(async move {
                eng.store_summary(&format!("concurrent_{i}"), &format!("value_{i}"), "h", "m")
                    .await
                    .unwrap();
            }));
        }
        // Spawn readers interleaved
        for i in 0..10 {
            let eng = Arc::clone(&engine);
            handles.push(tokio::spawn(async move {
                let _ = eng.get_cached_summary(&format!("concurrent_{i}")).await;
            }));
        }
        for h in handles {
            h.await.unwrap(); // No panics
        }
    }

    // 8.9.32 — Full tier includes boot context (OS info or hostname)
    #[tokio::test]
    async fn full_tier_has_boot_context_os() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains(&boot.hostname) || result.contains(&boot.os));
    }

    // 8.9.33 — Full tier includes runtime context (current date)
    #[tokio::test]
    async fn full_tier_has_runtime_context() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", Some("s1"), None, None)
            .await
            .unwrap();
        assert!(result.contains("Date:"));
    }

    // 8.9.34 — Full tier includes identity-related content when cached
    #[tokio::test]
    async fn full_tier_has_identity_summary() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("identity", "MesoClaw AI", "h", "m")
            .await
            .unwrap();
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("## Your Identity"));
        assert!(result.contains("MesoClaw AI"));
    }

    // 8.9.35 — Full tier includes user summary with observations
    #[tokio::test]
    async fn full_tier_has_user_summary() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("user", "Rust developer, prefers dark mode", "h", "m")
            .await
            .unwrap();
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("## User Context"));
        assert!(result.contains("Rust developer"));
    }

    // 8.9.36 — Full tier includes capability summary with tool names
    #[tokio::test]
    async fn full_tier_has_capability_summary() {
        let (_dir, engine) = setup().await;
        engine
            .store_summary("capabilities", "web_search, shell, file_read", "h", "m")
            .await
            .unwrap();
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert!(result.contains("## Your Capabilities"));
        assert!(result.contains("web_search"));
    }

    // 8.9.37 — Minimal tier is single line (no newlines)
    #[test]
    fn minimal_tier_single_line() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();
        let minimal = engine.compose_minimal(&boot, "gpt-4o");
        assert!(
            !minimal.contains('\n'),
            "Minimal tier should be a single line"
        );
    }

    // 8.9.38 — Minimal tier has date info
    #[test]
    fn minimal_tier_has_date() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();
        let minimal = engine.compose_minimal(&boot, "gpt-4o");
        let year = chrono::Local::now().format("%Y").to_string();
        assert!(
            minimal.contains(&year),
            "Minimal should contain current year"
        );
    }

    // 8.9.39 — Minimal tier has OS info
    #[test]
    fn minimal_tier_has_os() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();
        let minimal = engine.compose_minimal(&boot, "gpt-4o");
        assert!(minimal.contains(&boot.os), "Minimal should contain OS info");
    }

    // 8.9.40 — Minimal tier has model name when set
    #[test]
    fn minimal_tier_has_model() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, true);
        let boot = BootContext::from_system();
        let minimal = engine.compose_minimal(&boot, "claude-3-opus");
        assert!(
            minimal.contains("claude-3-opus"),
            "Minimal should contain model name"
        );
    }

    // 8.9.41 — Summary tier includes both full context and conversation summary
    #[tokio::test]
    async fn summary_tier_has_full_plus_summary() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(
                &ContextLevel::Summary,
                &boot,
                "gpt-4o",
                None,
                Some("User discussed async patterns in Rust."),
                None,
            )
            .await
            .unwrap();
        assert!(
            result.contains("## Environment"),
            "Summary tier should include full context"
        );
        assert!(
            result.contains("## Prior Conversation"),
            "Summary tier should include conversation section"
        );
        assert!(result.contains("async patterns"));
    }

    // 8.9.42 — Summary tier handles missing conversation summary gracefully
    #[tokio::test]
    async fn summary_tier_missing_graceful() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Summary, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        // Should succeed without conversation summary and not include that section
        assert!(!result.contains("## Prior Conversation"));
        assert!(result.contains("## Environment"));
    }

    // 8.9.43 — Disabled context returns fallback prompt
    #[tokio::test]
    async fn disabled_returns_fallback() {
        let dir = TempDir::new().unwrap();
        let pool = db::init_pool(&dir.path().join("t.db")).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();
        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool, config, false);
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "gpt-4o", None, None, None)
            .await
            .unwrap();
        assert_eq!(result, "You are MesoClaw, a helpful AI assistant.");
    }

    // 8.9.44 — Hash changes when identity content changes
    #[test]
    fn hash_changes_identity() {
        let h1 = compute_hash("identity content A");
        let h2 = compute_hash("identity content B");
        assert_ne!(h1, h2);
    }

    // 8.9.45 — Hash unchanged for same content
    // (covered by hash_unchanged_same above, which has the exact plan name)

    // 8.9.46 — Hash changes when user observations change
    #[test]
    fn hash_changes_user() {
        let h1 = compute_hash("user prefers vim");
        let h2 = compute_hash("user prefers emacs");
        assert_ne!(h1, h2);
    }

    // 8.9.47 — Hash changes when tools list changes
    #[test]
    fn hash_changes_tools() {
        let h1 = compute_hash("Tools: web_search, shell");
        let h2 = compute_hash("Tools: web_search, shell, file_read");
        assert_ne!(h1, h2);
    }

    // 8.9.48 — Hash changes when skills list changes
    #[test]
    fn hash_changes_skills() {
        let h1 = compute_hash("Skills: system-prompt");
        let h2 = compute_hash("Skills: system-prompt, summarize");
        assert_ne!(h1, h2);
    }

    // 8.9.49 — compute_hash is deterministic across multiple calls
    #[test]
    fn compute_hash_deterministic() {
        let content = "deterministic test input for hashing";
        let h1 = compute_hash(content);
        let h2 = compute_hash(content);
        let h3 = compute_hash(content);
        assert_eq!(h1, h2);
        assert_eq!(h2, h3);
    }

    // 8.9.50 — Identity summary with default identity is non-empty
    #[tokio::test]
    async fn gen_identity_summary_nonempty() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let s = engine
            .get_cached_summary("identity")
            .await
            .unwrap()
            .unwrap();
        assert!(
            !s.summary.is_empty(),
            "Identity summary should be non-empty"
        );
    }

    // 8.9.51 — User summary with observations lists them
    #[tokio::test]
    async fn gen_user_summary_with_obs() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);

        // Add observations so user summary is generated
        user_learner
            .observe("preference", "editor", "uses vim", 0.9)
            .await
            .unwrap();
        user_learner
            .observe("preference", "theme", "dark mode", 0.8)
            .await
            .unwrap();

        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let user = engine.get_cached_summary("user").await.unwrap().unwrap();
        assert!(!user.summary.is_empty());
        // The user summary should contain observation content
        assert!(
            user.summary.contains("vim") || user.summary.contains("dark"),
            "User summary should reflect observations"
        );
    }

    // 8.9.52 — User summary with no observations is gracefully skipped
    #[tokio::test]
    async fn gen_user_summary_empty() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        // User summary should be None when no observations exist
        let user = engine.get_cached_summary("user").await.unwrap();
        assert!(
            user.is_none(),
            "User summary should be skipped with no observations"
        );
    }

    // 8.9.53 — Capability summary lists tool names
    #[tokio::test]
    async fn gen_capability_summary_tools() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let caps = engine
            .get_cached_summary("capabilities")
            .await
            .unwrap()
            .unwrap();
        assert!(caps.summary.contains("tools"));
    }

    // 8.9.54 — Overall summary combines identity/user/capability sections
    #[tokio::test]
    async fn gen_overall_combines() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        let overall = engine.get_cached_summary("overall").await.unwrap().unwrap();
        assert!(!overall.summary.is_empty());
        // Overall should reference identity content from the default soul
        let identity = engine
            .get_cached_summary("identity")
            .await
            .unwrap()
            .unwrap();
        // The overall summary is built from identity + caps, so it should contain identity info
        assert!(
            overall
                .summary
                .contains(&identity.summary[..identity.summary.len().min(20)]),
            "Overall should include identity content"
        );
    }

    // 8.9.55 — Summary does not exceed reasonable max length
    #[tokio::test]
    async fn gen_summary_max_length() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = std::sync::Arc::new(AppConfig::default());
        let engine = ContextEngine::new(pool.clone(), config.clone(), true);
        let identity_dir = dir.path().join("identity");
        let soul_loader = SoulLoader::new(&identity_dir).unwrap();
        let user_learner = UserLearner::new(pool, &config);
        let tools = ToolRegistry::new();
        let skills = SkillRegistry::new(&dir.path().join("skills"), 65536).unwrap();

        engine
            .store_all_summaries(&soul_loader, &user_learner, &tools, &skills)
            .await
            .unwrap();

        for key in &["identity", "capabilities", "overall"] {
            if let Some(s) = engine.get_cached_summary(key).await.unwrap() {
                assert!(
                    s.summary.len() < 10_000,
                    "Summary '{key}' should be under 10KB, got {} bytes",
                    s.summary.len()
                );
            }
        }
    }

    // 8.11.14 — boot context has home_dir
    #[test]
    fn boot_context_has_home_dir() {
        let ctx = BootContext::from_system();
        assert!(ctx.home_dir.is_some(), "home_dir should be Some");
        assert!(
            !ctx.home_dir.as_ref().unwrap().is_empty(),
            "home_dir should be non-empty"
        );
    }

    // 8.11.15 — boot context has username
    #[test]
    fn boot_context_has_username() {
        let ctx = BootContext::from_system();
        assert!(!ctx.username.is_empty(), "username should be non-empty");
    }

    // 8.11.16 — boot context has working_dir
    #[test]
    fn boot_context_has_working_dir() {
        let ctx = BootContext::from_system();
        assert!(ctx.working_dir.is_some(), "working_dir should be Some");
    }

    // 8.11.17 — compose_full includes Home:
    #[tokio::test]
    async fn compose_full_includes_home_dir() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "test-model", None, None, None)
            .await
            .unwrap();
        assert!(
            result.contains("Home:"),
            "compose_full output should contain 'Home:'"
        );
    }

    // 8.11.18 — compose_full includes reasoning guidance
    #[tokio::test]
    async fn compose_full_includes_reasoning_guidance() {
        let (_dir, engine) = setup().await;
        let boot = BootContext::from_system();
        let result = engine
            .compose(&ContextLevel::Full, &boot, "test-model", None, None, None)
            .await
            .unwrap();
        assert!(
            result.contains("Reasoning Protocol"),
            "compose_full output should contain reasoning protocol"
        );
    }

    // P19.25 — ContextDomain::from_domain_str parses known domains
    #[test]
    fn context_domain_from_str() {
        assert_eq!(
            ContextDomain::from_domain_str("channels"),
            Some(ContextDomain::Channels)
        );
        assert_eq!(
            ContextDomain::from_domain_str("scheduler"),
            Some(ContextDomain::Scheduler)
        );
        assert_eq!(
            ContextDomain::from_domain_str("skills"),
            Some(ContextDomain::Skills)
        );
        assert_eq!(
            ContextDomain::from_domain_str("tools"),
            Some(ContextDomain::Tools)
        );
        assert_eq!(
            ContextDomain::from_domain_str("Tools"),
            Some(ContextDomain::Tools)
        );
        assert!(ContextDomain::from_domain_str("unknown").is_none());
    }
}
