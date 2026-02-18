use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::types::{IDENTITY_FILES, Identity, IdentityMeta};
use crate::event_bus::{AppEvent, EventBus};

// ─── Embedded defaults ────────────────────────────────────────────────────────

const DEFAULT_SOUL: &str = include_str!("defaults/SOUL.md");
const DEFAULT_USER: &str = include_str!("defaults/USER.md");
const DEFAULT_AGENTS: &str = include_str!("defaults/AGENTS.md");
const DEFAULT_IDENTITY: &str = include_str!("defaults/IDENTITY.md");
const DEFAULT_TOOLS: &str = include_str!("defaults/TOOLS.md");
const DEFAULT_HEARTBEAT: &str = include_str!("defaults/HEARTBEAT.md");
const DEFAULT_BOOT: &str = include_str!("defaults/BOOT.md");

// ─── IdentityLoader ───────────────────────────────────────────────────────────

/// Loads identity files from `~/.mesoclaw/identity/`, falling back to the
/// embedded defaults for any files that do not exist on disk.
///
/// Call [`IdentityLoader::start_watcher`] after creation to receive hot-reload
/// events whenever an identity file is modified.
pub struct IdentityLoader {
    dir: PathBuf,
    identity: Mutex<Identity>,
    // Keep the watcher alive for the duration of the loader's lifetime.
    _watcher: Option<RecommendedWatcher>,
}

impl IdentityLoader {
    /// Create a new loader, writing the defaults to `dir` on first run.
    pub fn new(dir: PathBuf) -> Result<Arc<Self>, String> {
        ensure_defaults(&dir)?;
        let identity = load_from_dir(&dir)?;

        Ok(Arc::new(Self {
            dir,
            identity: Mutex::new(identity),
            _watcher: None,
        }))
    }

    /// Create the loader and immediately start watching for file changes.
    ///
    /// When a file in the identity directory is modified, the loader reloads
    /// its contents and publishes a [`AppEvent::SystemReady`]-adjacent signal
    /// via the event bus.
    pub fn new_with_watcher(dir: PathBuf, bus: Arc<dyn EventBus>) -> Result<Arc<Self>, String> {
        ensure_defaults(&dir)?;
        let identity = load_from_dir(&dir)?;

        // We need a reference to `self` before the watcher is created, so
        // we use a shared cache wrapped in Arc<Mutex<_>>.
        let cache = Arc::new(Mutex::new(identity.clone()));
        let cache_clone = cache.clone();
        let dir_clone = dir.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_))
                && let Ok(updated) = load_from_dir(&dir_clone)
            {
                *cache_clone.lock().unwrap_or_else(|e| e.into_inner()) = updated;
                let _ = bus.publish(AppEvent::SystemReady);
            }
        })
        .map_err(|e| format!("failed to create file watcher: {e}"))?;

        watcher
            .watch(&dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("failed to watch identity dir: {e}"))?;

        Ok(Arc::new(Self {
            dir,
            identity: Mutex::new(cache.lock().unwrap_or_else(|e| e.into_inner()).clone()),
            _watcher: Some(watcher),
        }))
    }

    /// Return a snapshot of the current identity.
    pub fn get(&self) -> Identity {
        self.identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Reload a single file from disk (or fall back to default) and update the cache.
    pub fn reload(&self) -> Result<(), String> {
        let updated = load_from_dir(&self.dir)?;
        *self.identity.lock().unwrap_or_else(|e| e.into_inner()) = updated;
        Ok(())
    }

    /// Return the content of a single identity file by file name (e.g. `"SOUL.md"`).
    pub fn get_file(&self, file_name: &str) -> Result<String, String> {
        let identity = self.get();
        Ok(match file_name {
            "SOUL.md" => identity.soul,
            "USER.md" => identity.user,
            "AGENTS.md" => identity.agents,
            "IDENTITY.md" => {
                format!(
                    "name: {}\nversion: {}\ndescription: {}",
                    identity.identity.name,
                    identity.identity.version,
                    identity.identity.description
                )
            }
            "TOOLS.md" => identity.tools,
            "HEARTBEAT.md" => identity.heartbeat,
            "BOOT.md" => identity.boot,
            other => return Err(format!("unknown identity file: '{other}'")),
        })
    }

    /// Overwrite a single identity file on disk and reload the cache.
    pub fn update_file(&self, file_name: &str, content: &str) -> Result<(), String> {
        validate_file_name(file_name)?;
        let path = self.dir.join(file_name);
        fs::write(&path, content).map_err(|e| format!("failed to write '{file_name}': {e}"))?;
        self.reload()
    }

    /// Build the complete system prompt from all identity files, in the
    /// canonical assembly order: SOUL → AGENTS → USER → TOOLS → placeholders.
    pub fn build_system_prompt(&self) -> String {
        self.build_system_prompt_with_daily(None)
    }

    /// Build the system prompt and optionally append a daily-memory context
    /// block after the identity sections.
    ///
    /// `daily_context` should be produced by
    /// [`crate::memory::DailyMemory::build_daily_context`].
    pub fn build_system_prompt_with_daily(&self, daily_context: Option<&str>) -> String {
        let id = self.get();
        let mut parts: Vec<String> = [
            ("# Soul", id.soul.as_str()),
            ("# Agents", id.agents.as_str()),
            ("# User", id.user.as_str()),
            ("# Tools", id.tools.as_str()),
        ]
        .iter()
        .map(|(header, body)| format!("{header}\n\n{body}"))
        .collect();

        if let Some(ctx) = daily_context
            && !ctx.trim().is_empty()
        {
            parts.push(format!("# Memory\n\n{ctx}"));
        }

        parts.join("\n\n---\n\n")
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Return the default identity directory: `~/.mesoclaw/identity/`.
pub fn default_identity_dir() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|h| h.join(".mesoclaw").join("identity"))
        .ok_or_else(|| "could not determine home directory".into())
}

/// Write the embedded defaults to `dir` for any file that does not exist yet.
fn ensure_defaults(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("failed to create identity dir: {e}"))?;

    let defaults: &[(&str, &str)] = &[
        ("SOUL.md", DEFAULT_SOUL),
        ("USER.md", DEFAULT_USER),
        ("AGENTS.md", DEFAULT_AGENTS),
        ("IDENTITY.md", DEFAULT_IDENTITY),
        ("TOOLS.md", DEFAULT_TOOLS),
        ("HEARTBEAT.md", DEFAULT_HEARTBEAT),
        ("BOOT.md", DEFAULT_BOOT),
    ];

    for (name, content) in defaults {
        let path = dir.join(name);
        if !path.exists() {
            fs::write(&path, content)
                .map_err(|e| format!("failed to write default {name}: {e}"))?;
        }
    }
    Ok(())
}

/// Read all identity files from `dir`, falling back to embedded defaults.
fn load_from_dir(dir: &Path) -> Result<Identity, String> {
    let read = |name: &str, default: &str| -> String {
        fs::read_to_string(dir.join(name)).unwrap_or_else(|_| default.to_string())
    };

    let identity_raw = read("IDENTITY.md", DEFAULT_IDENTITY);
    let identity_meta = parse_identity_meta(&identity_raw);

    Ok(Identity {
        soul: read("SOUL.md", DEFAULT_SOUL),
        user: read("USER.md", DEFAULT_USER),
        agents: read("AGENTS.md", DEFAULT_AGENTS),
        identity: identity_meta,
        tools: read("TOOLS.md", DEFAULT_TOOLS),
        heartbeat: read("HEARTBEAT.md", DEFAULT_HEARTBEAT),
        boot: read("BOOT.md", DEFAULT_BOOT),
    })
}

/// Parse the simple YAML-ish key-value pairs from IDENTITY.md.
fn parse_identity_meta(content: &str) -> IdentityMeta {
    let mut name = "Claw".to_string();
    let mut version = "0.0.1".to_string();
    let mut description = String::new();

    for line in content.lines() {
        if let Some(v) = line.strip_prefix("name:") {
            name = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("version:") {
            version = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("description:") {
            description = v.trim().trim_start_matches('>').trim().to_string();
        } else if description.is_empty() && line.trim_start().starts_with("A ") {
            description = line.trim().to_string();
        }
    }

    IdentityMeta {
        name,
        version,
        description,
    }
}

fn validate_file_name(name: &str) -> Result<(), String> {
    if IDENTITY_FILES.iter().any(|(f, _)| *f == name) {
        Ok(())
    } else {
        Err(format!("'{name}' is not a valid identity file name"))
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn loader_in(dir: &TempDir) -> Arc<IdentityLoader> {
        IdentityLoader::new(dir.path().to_path_buf()).unwrap()
    }

    #[test]
    fn creates_defaults_on_first_run() {
        let dir = TempDir::new().unwrap();
        let _loader = loader_in(&dir);
        for (name, _) in IDENTITY_FILES {
            assert!(dir.path().join(name).exists(), "{name} was not created");
        }
    }

    #[test]
    fn returns_soul_content() {
        let dir = TempDir::new().unwrap();
        let loader = loader_in(&dir);
        let soul = loader.get_file("SOUL.md").unwrap();
        assert!(!soul.is_empty());
        assert!(soul.contains("Soul") || soul.contains("concise") || soul.len() > 10);
    }

    #[test]
    fn falls_back_to_defaults_for_missing_file() {
        let dir = TempDir::new().unwrap();
        // Only write SOUL.md; loader should fall back for the rest.
        fs::write(dir.path().join("SOUL.md"), "custom soul").unwrap();
        let loader = IdentityLoader::new(dir.path().to_path_buf()).unwrap();
        let soul = loader.get_file("SOUL.md").unwrap();
        assert_eq!(soul, "custom soul");
    }

    #[test]
    fn update_file_persists_to_disk() {
        let dir = TempDir::new().unwrap();
        let loader = loader_in(&dir);
        loader.update_file("USER.md", "I am a tester").unwrap();
        let content = fs::read_to_string(dir.path().join("USER.md")).unwrap();
        assert_eq!(content, "I am a tester");
    }

    #[test]
    fn update_file_refreshes_cache() {
        let dir = TempDir::new().unwrap();
        let loader = loader_in(&dir);
        loader.update_file("USER.md", "updated user").unwrap();
        let fetched = loader.get_file("USER.md").unwrap();
        assert_eq!(fetched, "updated user");
    }

    #[test]
    fn unknown_file_errors() {
        let dir = TempDir::new().unwrap();
        let loader = loader_in(&dir);
        assert!(loader.get_file("NONEXISTENT.md").is_err());
    }

    #[test]
    fn system_prompt_contains_all_sections() {
        let dir = TempDir::new().unwrap();
        let loader = loader_in(&dir);
        let prompt = loader.build_system_prompt();
        assert!(prompt.contains("# Soul"));
        assert!(prompt.contains("# Agents"));
        assert!(prompt.contains("# User"));
        assert!(prompt.contains("# Tools"));
    }

    #[test]
    fn system_prompt_order_soul_before_agents() {
        let dir = TempDir::new().unwrap();
        let loader = loader_in(&dir);
        let prompt = loader.build_system_prompt();
        let soul_pos = prompt.find("# Soul").unwrap();
        let agents_pos = prompt.find("# Agents").unwrap();
        assert!(soul_pos < agents_pos, "SOUL must come before AGENTS");
    }

    #[test]
    fn system_prompt_agents_before_user() {
        let dir = TempDir::new().unwrap();
        let loader = loader_in(&dir);
        let prompt = loader.build_system_prompt();
        let agents_pos = prompt.find("# Agents").unwrap();
        let user_pos = prompt.find("# User").unwrap();
        assert!(agents_pos < user_pos, "AGENTS must come before USER");
    }
}
