use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tokio::sync::RwLock;
use tracing::info;

use crate::{MesoError, Result};

use super::defaults::BUNDLED_SKILLS;
use super::loader::{load_skill_from_content, load_skill_from_file};
use super::types::{Skill, SkillInfo, SkillSource};

/// Registry for managing skills (bundled + user).
pub struct SkillRegistry {
    dir: PathBuf,
    max_content_size: usize,
    skills: RwLock<HashMap<String, Skill>>,
}

impl SkillRegistry {
    /// Create a new SkillRegistry and load all skills.
    pub fn new(dir: &Path, max_content_size: usize) -> Result<Self> {
        std::fs::create_dir_all(dir)?;

        let skills = Self::load_all(dir, max_content_size)?;

        Ok(Self {
            dir: dir.to_path_buf(),
            max_content_size,
            skills: RwLock::new(skills),
        })
    }

    fn load_all(dir: &Path, max_content_size: usize) -> Result<HashMap<String, Skill>> {
        let mut skills = HashMap::new();

        // 1. Load bundled skills
        for (id, content) in BUNDLED_SKILLS {
            let skill = load_skill_from_content(id, content, SkillSource::Bundled);
            skills.insert(skill.id.clone(), skill);
        }

        // 2. Load user skills (override bundled if same id)
        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "md") {
                    match load_skill_from_file(&path, max_content_size) {
                        Ok(skill) => {
                            info!("Loaded user skill: {}", skill.id);
                            skills.insert(skill.id.clone(), skill);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load skill from {}: {e}", path.display());
                        }
                    }
                }
            }
        }

        Ok(skills)
    }

    /// Get a skill by id.
    pub async fn get(&self, id: &str) -> Result<Skill> {
        let skills = self.skills.read().await;
        skills
            .get(id)
            .cloned()
            .ok_or_else(|| MesoError::SkillNotFound(format!("skill '{id}' not found")))
    }

    /// List all skills (summary info only).
    pub async fn list(&self) -> Vec<SkillInfo> {
        let skills = self.skills.read().await;
        let mut list: Vec<SkillInfo> = skills.values().map(SkillInfo::from).collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    /// List skills filtered by category.
    pub async fn by_category(&self, category: &str) -> Vec<SkillInfo> {
        let skills = self.skills.read().await;
        let mut list: Vec<SkillInfo> = skills
            .values()
            .filter(|s| s.category == category)
            .map(SkillInfo::from)
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    /// Create a new user skill. Writes the .md file to disk.
    pub async fn create(&self, id: String, content: String) -> Result<Skill> {
        let skill = load_skill_from_content(&id, &content, SkillSource::User);

        // Write to disk
        let path = self.dir.join(format!("{id}.md"));
        std::fs::write(&path, &content)?;

        let mut skills = self.skills.write().await;
        skills.insert(id, skill.clone());
        Ok(skill)
    }

    /// Update an existing skill's content.
    pub async fn update(&self, id: &str, content: String) -> Result<Skill> {
        {
            let skills = self.skills.read().await;
            if !skills.contains_key(id) {
                return Err(MesoError::SkillNotFound(format!("skill '{id}' not found")));
            }
        }

        let skill = load_skill_from_content(id, &content, SkillSource::User);

        // Write to disk
        let path = self.dir.join(format!("{id}.md"));
        std::fs::write(&path, &content)?;

        let mut skills = self.skills.write().await;
        skills.insert(id.to_string(), skill.clone());
        Ok(skill)
    }

    /// Delete a user skill. Bundled skills cannot be deleted.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let mut skills = self.skills.write().await;
        if let Some(skill) = skills.get(id) {
            if skill.source == SkillSource::Bundled {
                return Err(MesoError::Skill("cannot delete bundled skill".into()));
            }
            // Remove from disk
            let path = self.dir.join(format!("{id}.md"));
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            skills.remove(id);
            Ok(())
        } else {
            Err(MesoError::SkillNotFound(format!("skill '{id}' not found")))
        }
    }

    /// Reload all skills from disk.
    pub async fn reload(&self) -> Result<()> {
        let new_skills = Self::load_all(&self.dir, self.max_content_size)?;
        let mut skills = self.skills.write().await;
        *skills = new_skills;
        info!("Skills reloaded from {}", self.dir.display());
        Ok(())
    }

    /// Get all enabled skills as (name, content) pairs for prompt composition.
    pub async fn active_skills(&self) -> Vec<(String, String)> {
        let skills = self.skills.read().await;
        skills
            .values()
            .filter(|s| s.enabled)
            .map(|s| (s.name.clone(), s.content.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn registry_loads_bundled_skills() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let list = rt.block_on(registry.list());
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|s| s.id == "system-prompt"));
        assert!(list.iter().any(|s| s.id == "summarize"));
    }

    #[test]
    fn registry_loads_user_skills() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("custom.md"),
            "---\nname: custom\ndescription: Custom skill\ncategory: test\n---\nCustom content.",
        )
        .unwrap();

        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let list = rt.block_on(registry.list());
        assert_eq!(list.len(), 3); // 2 bundled + 1 user
        assert!(list.iter().any(|s| s.id == "custom"));
    }

    #[test]
    fn registry_user_overrides_bundled() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("summarize.md"),
            "---\nname: summarize\ndescription: Custom summarize\ncategory: general\n---\nOverridden.",
        )
        .unwrap();

        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let skill = rt.block_on(registry.get("summarize")).unwrap();
        assert_eq!(skill.source, SkillSource::User);
        assert!(skill.content.contains("Overridden"));
    }

    #[tokio::test]
    async fn registry_get_existing() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();
        let skill = registry.get("system-prompt").await.unwrap();
        assert_eq!(skill.id, "system-prompt");
    }

    #[tokio::test]
    async fn registry_get_not_found() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();
        let result = registry.get("nonexistent").await;
        assert!(matches!(result.unwrap_err(), MesoError::SkillNotFound(_)));
    }

    #[tokio::test]
    async fn registry_list_all() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();
        let list = registry.list().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn registry_list_by_category() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();
        let meta = registry.by_category("meta").await;
        assert_eq!(meta.len(), 1);
        assert_eq!(meta[0].id, "system-prompt");
    }

    #[tokio::test]
    async fn registry_create_skill() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();

        let content = "---\nname: new-skill\ndescription: New\ncategory: test\n---\nBody.";
        let skill = registry
            .create("new-skill".into(), content.into())
            .await
            .unwrap();
        assert_eq!(skill.id, "new-skill");
        assert_eq!(skill.source, SkillSource::User);

        // Verify on disk
        assert!(dir.path().join("new-skill.md").exists());

        // Verify in registry
        let list = registry.list().await;
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn registry_update_skill() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();

        // Create first
        registry
            .create(
                "updatable".into(),
                "---\nname: updatable\ndescription: Old\ncategory: test\n---\nOld body.".into(),
            )
            .await
            .unwrap();

        // Update
        let updated = registry
            .update(
                "updatable",
                "---\nname: updatable\ndescription: New\ncategory: test\n---\nNew body.".into(),
            )
            .await
            .unwrap();
        assert!(updated.content.contains("New body"));
    }

    #[tokio::test]
    async fn registry_delete_user_skill() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();

        registry
            .create(
                "deletable".into(),
                "---\nname: deletable\ndescription: Del\ncategory: test\n---\nBody.".into(),
            )
            .await
            .unwrap();

        registry.delete("deletable").await.unwrap();
        let result = registry.get("deletable").await;
        assert!(result.is_err());
        assert!(!dir.path().join("deletable.md").exists());
    }

    #[tokio::test]
    async fn registry_delete_bundled_fails() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();
        let result = registry.delete("system-prompt").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::Skill(_)));
    }

    #[tokio::test]
    async fn registry_reload() {
        let dir = TempDir::new().unwrap();
        let registry = SkillRegistry::new(dir.path(), 100_000).unwrap();

        // Add a file on disk
        std::fs::write(
            dir.path().join("new-on-disk.md"),
            "---\nname: new-on-disk\ndescription: New\ncategory: test\n---\nNew.",
        )
        .unwrap();

        registry.reload().await.unwrap();
        let list = registry.list().await;
        assert_eq!(list.len(), 3);
        assert!(list.iter().any(|s| s.id == "new-on-disk"));
    }
}
