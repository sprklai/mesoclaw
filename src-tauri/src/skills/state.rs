//! Shared skill system state for Tauri.
//!
//! Provides a singleton SkillRegistry that persists across commands.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::config::app_identity::SKILLS_CONFIG_DIR_NAME;
use crate::skills::loader::SkillLoader;
use crate::skills::error::SkillResult;
use crate::skills::registry::SkillRegistry;

/// Global skill registry instance.
static SKILL_REGISTRY: OnceCell<Arc<SkillRegistry>> = OnceCell::const_new();

/// Initialize the global skill registry with the given paths.
///
/// This should be called once during app startup.
pub async fn initialize_skill_registry(
    local_path: Option<PathBuf>,
    remote_url: Option<String>,
) -> SkillResult<Arc<SkillRegistry>> {
    let registry = SKILL_REGISTRY
        .get_or_init(|| async {
            let loader = Arc::new(SkillLoader::new(local_path, remote_url));
            let registry = Arc::new(SkillRegistry::new(loader));

            // Initialize the registry
            if let Err(e) = registry.initialize().await {
                tracing::error!("Failed to initialize skill registry: {}", e);
            } else {
                tracing::info!("Skill registry initialized successfully");
            }

            registry
        })
        .await;

    Ok(registry.clone())
}

/// Get the global skill registry.
///
/// Returns None if the registry hasn't been initialized yet.
pub fn get_skill_registry() -> Option<Arc<SkillRegistry>> {
    SKILL_REGISTRY.get().cloned()
}

/// Get the global skill registry, initializing with defaults if needed.
///
/// This is a convenience function for commands that need the registry
/// but don't have access to the app paths.
pub async fn get_or_init_registry() -> Arc<SkillRegistry> {
    match SKILL_REGISTRY.get() {
        Some(registry) => registry.clone(),
        None => {
            // Initialize with defaults (embedded skills only)
            initialize_skill_registry(None, None)
                .await
                .unwrap_or_else(|e| panic!("Failed to initialize skill registry: {e}"))
        }
    }
}

/// Get the default local skills directory path.
///
/// Returns ~/.config/<slug>/skills/ on Linux/macOS or %APPDATA%/<slug>/skills/ on Windows.
pub fn get_default_skills_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join(SKILLS_CONFIG_DIR_NAME).join("skills"))
}

/// Reload the skill registry.
///
/// This reloads all skills from their sources without reinitializing
/// the global registry.
pub async fn reload_registry() -> SkillResult<()> {
    if let Some(registry) = SKILL_REGISTRY.get() {
        registry.reload().await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_skills_path() {
        let path = get_default_skills_path();
        // Should return Some on most systems
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("skills"));
        }
    }
}
