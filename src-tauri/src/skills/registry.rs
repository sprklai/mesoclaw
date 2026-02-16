//! Central registry for all available skills.
//!
//! Provides indexing by ID and category for fast lookups.

use crate::skills::error::{SkillError, SkillResult};
use crate::skills::loader::SkillLoader;
use crate::skills::types::{SkillDefinition, SkillInfo};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Central registry of all available skills.
pub struct SkillRegistry {
    /// All skills indexed by ID
    skills: RwLock<HashMap<String, SkillDefinition>>,

    /// Skills indexed by category
    by_category: RwLock<HashMap<String, Vec<String>>>,

    /// The skill loader for refreshing
    loader: Arc<SkillLoader>,

    /// Whether the registry has been initialized
    initialized: RwLock<bool>,
}

impl SkillRegistry {
    /// Create a new skill registry with the given loader.
    pub fn new(loader: Arc<SkillLoader>) -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
            by_category: RwLock::new(HashMap::new()),
            loader,
            initialized: RwLock::new(false),
        }
    }

    /// Initialize the registry by loading all skills.
    pub async fn initialize(&self) -> SkillResult<()> {
        let all_skills = self.loader.load_all().await?;

        let mut skills = self.skills.write().map_err(|e| {
            SkillError::ExecutionError(format!("Failed to acquire skills lock: {}", e))
        })?;

        let mut by_category = self.by_category.write().map_err(|e| {
            SkillError::ExecutionError(format!("Failed to acquire category lock: {}", e))
        })?;

        skills.clear();
        by_category.clear();

        for skill in all_skills {
            let category = skill.feature.category.clone();
            let id = skill.id.clone();

            // Index by ID
            skills.insert(id.clone(), skill);

            // Index by category
            by_category
                .entry(category)
                .or_insert_with(Vec::new)
                .push(id);
        }

        let mut initialized = self.initialized.write().map_err(|e| {
            SkillError::ExecutionError(format!("Failed to acquire initialized lock: {}", e))
        })?;
        *initialized = true;

        tracing::info!("Skill registry initialized with {} skills", skills.len());

        Ok(())
    }

    /// Check if the registry has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
            .read()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    /// Get a skill by ID.
    pub fn get(&self, id: &str) -> Option<SkillDefinition> {
        self.skills
            .read()
            .ok()
            .and_then(|guard| guard.get(id).cloned())
    }

    /// List all available skills.
    pub fn list_all(&self) -> Vec<SkillDefinition> {
        self.skills
            .read()
            .map(|guard| guard.values().cloned().collect())
            .unwrap_or_default()
    }

    /// List skills by category.
    pub fn list_by_category(&self, category: &str) -> Vec<SkillDefinition> {
        let by_category = match self.by_category.read() {
            Ok(guard) => guard,
            Err(_) => return vec![],
        };

        let skill_ids = match by_category.get(category) {
            Some(ids) => ids.clone(),
            None => return vec![],
        };

        drop(by_category);

        let skills = match self.skills.read() {
            Ok(guard) => guard,
            Err(_) => return vec![],
        };

        skill_ids
            .iter()
            .filter_map(|id| skills.get(id).cloned())
            .collect()
    }

    /// Get all categories.
    pub fn list_categories(&self) -> Vec<String> {
        self.by_category
            .read()
            .map(|guard| guard.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get skill info for all skills (lightweight version for UI).
    pub fn get_skill_infos(&self) -> Vec<SkillInfo> {
        self.loader.get_skill_infos()
    }

    /// Reload all skills from sources.
    pub async fn reload(&self) -> SkillResult<()> {
        tracing::info!("Reloading skill registry");

        // Refresh remote skills if configured
        self.loader.refresh_remote().await?;

        // Re-initialize
        self.initialize().await
    }

    /// Get a skill with database-specific override applied.
    pub async fn get_with_database_override(
        &self,
        skill_id: &str,
        database_type: &str,
    ) -> SkillResult<SkillDefinition> {
        let base_skill = self
            .get(skill_id)
            .ok_or_else(|| SkillError::NotFound(skill_id.to_string()))?;

        // Try to get database-specific override
        match self
            .loader
            .get_database_override(skill_id, database_type)
            .await?
        {
            Some(override_skill) => {
                Ok(self.loader.apply_database_override(&base_skill, &override_skill))
            }
            None => Ok(base_skill),
        }
    }

    /// Get multiple skills by IDs.
    pub fn get_many(&self, ids: &[String]) -> Vec<SkillDefinition> {
        let skills = match self.skills.read() {
            Ok(guard) => guard,
            Err(_) => return vec![],
        };

        ids.iter()
            .filter_map(|id| skills.get(id).cloned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_registry_initialization() {
        let loader = Arc::new(SkillLoader::new(None, None));
        let registry = SkillRegistry::new(loader);

        assert!(!registry.is_initialized());

        registry.initialize().await.unwrap();

        assert!(registry.is_initialized());
    }
}
