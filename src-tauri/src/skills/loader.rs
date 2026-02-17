//! Skill loader for parsing and loading skill definitions.
//!
//! Loads skills from three sources:
//! 1. Embedded skills (compiled into binary)
//! 2. Local skills (user's config directory)
//! 3. Remote skills (from registry URL)

use crate::skills::error::{SkillError, SkillResult};
use crate::skills::types::{
    ComposeConfig, FeatureConfig, SkillDefinition, SkillInfo, SkillRequirements, SkillSource,
    SkillTriggers,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Skill frontmatter parsed from YAML.
#[derive(Debug, serde::Deserialize)]
struct SkillFrontmatter {
    id: String,
    version: String,
    name: String,
    description: String,
    #[serde(default)]
    feature: FeatureConfig,
    #[serde(default)]
    requires: SkillRequirements,
    #[serde(default)]
    triggers: SkillTriggers,
    #[serde(default)]
    compose: ComposeConfig,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            category: "general".to_string(),
            default_enabled: true,
        }
    }
}

/// Loads and caches skill definitions from multiple sources.
pub struct SkillLoader {
    /// Skills compiled into the binary
    embedded_skills: HashMap<String, SkillDefinition>,

    /// Path to local skills directory
    local_path: Option<PathBuf>,

    /// URL for remote skill registry
    remote_registry_url: Option<String>,

    /// Cache of loaded skills (only updated after async operations complete)
    cache: Mutex<HashMap<String, (SkillDefinition, SkillSource)>>,
}

impl SkillLoader {
    /// Create a new skill loader with the given paths.
    pub fn new(local_path: Option<PathBuf>, remote_url: Option<String>) -> Self {
        Self {
            embedded_skills: Self::load_embedded_skills(),
            local_path,
            remote_registry_url: remote_url,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Load all embedded skills from compiled resources.
    fn load_embedded_skills() -> HashMap<String, SkillDefinition> {
        let mut skills = HashMap::new();

        // Load embedded skills using include_str!
        let embedded_sources = crate::skills_embedded::get_embedded_skills();

        for (id, content) in embedded_sources {
            match Self::parse_skill_content(content) {
                Ok(mut skill) => {
                    skill.id = id.clone();
                    skills.insert(id, skill);
                }
                Err(e) => {
                    tracing::warn!("Failed to parse embedded skill '{}': {}", id, e);
                }
            }
        }

        skills
    }

    /// Load all skills from all sources.
    pub async fn load_all(&self) -> SkillResult<Vec<SkillDefinition>> {
        let mut all_skills = Vec::new();

        // 1. Load embedded skills (sync, no async needed)
        for skill in self.embedded_skills.values() {
            all_skills.push(skill.clone());
        }

        // 2. Load local skills (async file I/O)
        let local_skills = if let Some(local_path) = &self.local_path {
            if local_path.exists() {
                match self.load_from_directory(local_path).await {
                    Ok(skills) => skills,
                    Err(e) => {
                        tracing::warn!("Failed to load local skills: {}", e);
                        vec![]
                    }
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        all_skills.extend(local_skills);

        // 3. Resolve inheritance for all skills (sync)
        let resolved = self.resolve_all_inheritance(&all_skills)?;

        // 4. Update cache synchronously (after all async operations)
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
            for skill in &resolved {
                let source = if self.embedded_skills.contains_key(&skill.id) {
                    SkillSource::Embedded
                } else {
                    SkillSource::Local
                };
                cache.insert(skill.id.clone(), (skill.clone(), source));
            }
        }

        Ok(resolved)
    }

    /// Load skills from a directory.
    async fn load_from_directory(&self, path: &PathBuf) -> SkillResult<Vec<SkillDefinition>> {
        let mut skills = Vec::new();

        if !path.is_dir() {
            return Ok(skills);
        }

        let entries = std::fs::read_dir(path)?;

        for entry in entries.flatten() {
            let entry_path = entry.path();

            if entry_path.is_dir() {
                // Look for skill.md in subdirectory
                let skill_file = entry_path.join("skill.md");
                if skill_file.exists() {
                    match self.load_skill_file(&skill_file).await {
                        Ok(skill) => skills.push(skill),
                        Err(e) => {
                            tracing::warn!("Failed to load skill from {:?}: {}", skill_file, e);
                        }
                    }
                }
            } else if entry_path.extension().is_some_and(|ext| ext == "md") {
                // Load standalone .md file
                match self.load_skill_file(&entry_path).await {
                    Ok(skill) => skills.push(skill),
                    Err(e) => {
                        tracing::warn!("Failed to load skill from {:?}: {}", entry_path, e);
                    }
                }
            }
        }

        Ok(skills)
    }

    /// Load a single skill file.
    async fn load_skill_file(&self, path: &PathBuf) -> SkillResult<SkillDefinition> {
        let content = tokio::fs::read_to_string(path).await?;
        Self::parse_skill_content(&content)
    }

    /// Parse skill content from markdown with YAML frontmatter.
    fn parse_skill_content(content: &str) -> SkillResult<SkillDefinition> {
        // Extract YAML frontmatter between --- delimiters
        let content = content.trim();

        if !content.starts_with("---") {
            return Err(SkillError::ParseError(
                "Skill file must start with YAML frontmatter (---)".to_string(),
            ));
        }

        let after_first = &content[3..];
        let end_index = after_first.find("---").ok_or_else(|| {
            SkillError::ParseError("Missing closing --- for YAML frontmatter".to_string())
        })?;

        let yaml_content = &after_first[..end_index].trim();
        let prompt_content = after_first[end_index + 3..].trim().to_string();

        // Parse YAML frontmatter
        let frontmatter: SkillFrontmatter =
            serde_yaml::from_str(yaml_content).map_err(SkillError::from)?;

        Ok(SkillDefinition {
            id: frontmatter.id,
            version: frontmatter.version,
            name: frontmatter.name,
            description: frontmatter.description,
            feature: frontmatter.feature,
            requires: frontmatter.requires,
            triggers: frontmatter.triggers,
            compose: frontmatter.compose,
            prompt_content,
        })
    }

    /// Load a specific skill by ID.
    pub async fn load_skill(&self, id: &str) -> SkillResult<SkillDefinition> {
        // Check cache first (sync lock, no await while holding)
        if let Ok(cache) = self.cache.lock()
            && let Some((skill, _)) = cache.get(id)
        {
            return Ok(skill.clone());
        }

        // Check embedded skills
        if let Some(skill) = self.embedded_skills.get(id) {
            return Ok(skill.clone());
        }

        // Try to find in local directory
        if let Some(local_path) = &self.local_path {
            let skill_dir = local_path.join(id);
            let skill_file = skill_dir.join("skill.md");

            if skill_file.exists() {
                let skill = self.load_skill_file(&skill_file).await?;
                return Ok(skill);
            }

            // Try direct file
            let direct_file = local_path.join(format!("{}.md", id));
            if direct_file.exists() {
                let skill = self.load_skill_file(&direct_file).await?;
                return Ok(skill);
            }
        }

        Err(SkillError::NotFound(id.to_string()))
    }

    /// Resolve inheritance for all skills.
    fn resolve_all_inheritance(
        &self,
        skills: &[SkillDefinition],
    ) -> SkillResult<Vec<SkillDefinition>> {
        let skill_map: HashMap<_, _> = skills.iter().map(|s| (s.id.as_str(), s)).collect();

        let mut resolved = Vec::new();

        for skill in skills {
            let resolved_skill = self.resolve_inheritance_single(skill, &skill_map, &mut vec![])?;
            resolved.push(resolved_skill);
        }

        Ok(resolved)
    }

    /// Resolve inheritance for a single skill.
    fn resolve_inheritance_single(
        &self,
        skill: &SkillDefinition,
        skill_map: &HashMap<&str, &SkillDefinition>,
        visited: &mut Vec<String>,
    ) -> SkillResult<SkillDefinition> {
        // Check for cycles
        if visited.contains(&skill.id) {
            return Err(SkillError::InheritanceCycle(format!(
                "{} -> {}",
                visited.join(" -> "),
                skill.id
            )));
        }

        let Some(parent_id) = &skill.compose.extends else {
            return Ok(skill.clone());
        };

        visited.push(skill.id.clone());

        let parent = skill_map
            .get(parent_id.as_str())
            .ok_or_else(|| SkillError::ParentNotFound(parent_id.clone()))?;

        // Recursively resolve parent
        let resolved_parent = self.resolve_inheritance_single(parent, skill_map, visited)?;

        // Merge child onto parent
        Ok(self.merge_skill_with_parent(skill, &resolved_parent))
    }

    /// Merge a child skill with its resolved parent.
    fn merge_skill_with_parent(
        &self,
        child: &SkillDefinition,
        parent: &SkillDefinition,
    ) -> SkillDefinition {
        SkillDefinition {
            id: child.id.clone(),
            version: child.version.clone(),
            name: child.name.clone(),
            description: child.description.clone(),
            feature: child.feature.clone(),
            requires: parent.requires.merge(&child.requires),
            triggers: SkillTriggers {
                task_types: if child.triggers.task_types.is_empty() {
                    parent.triggers.task_types.clone()
                } else {
                    child.triggers.task_types.clone()
                },
                entity_types: if child.triggers.entity_types.is_empty() {
                    parent.triggers.entity_types.clone()
                } else {
                    child.triggers.entity_types.clone()
                },
            },
            compose: child.compose.clone(),
            // Combine prompts: parent first, then child
            prompt_content: format!(
                "{}\n\n---\n\n{}",
                parent.prompt_content, child.prompt_content
            ),
        }
    }

    /// Get database-specific skill override if available.
    pub async fn get_database_override(
        &self,
        base_skill_id: &str,
        database_type: &str,
    ) -> SkillResult<Option<SkillDefinition>> {
        // Look for database-specific override file
        let override_id = format!("{}-{}", base_skill_id, database_type.to_lowercase());

        match self.load_skill(&override_id).await {
            Ok(skill) => Ok(Some(skill)),
            Err(SkillError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Apply database-specific override to a base skill.
    pub fn apply_database_override(
        &self,
        base: &SkillDefinition,
        override_skill: &SkillDefinition,
    ) -> SkillDefinition {
        self.merge_skill_with_parent(override_skill, base)
    }

    /// Refresh remote skills (placeholder for future implementation).
    pub async fn refresh_remote(&self) -> SkillResult<()> {
        if let Some(url) = &self.remote_registry_url {
            tracing::info!("Refreshing skills from remote registry: {}", url);
            // TODO: Implement remote registry fetch
        }
        Ok(())
    }

    /// Get info about all available skills (sync version).
    pub fn get_skill_infos(&self) -> Vec<SkillInfo> {
        let cache = match self.cache.lock() {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        cache
            .iter()
            .map(|(_, (skill, source))| {
                let mut info = SkillInfo::from(skill);
                info.source = *source;
                info
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const SAMPLE_SKILL: &str = r#"---
id: test-skill
version: 1.0.0
name: Test Skill
description: A test skill for unit testing

feature:
  category: testing
  default_enabled: true

requires:
  context: [query]
  tools: [execute_query]

triggers:
  task_types: [test]
  entity_types: [query]

compose:
  priority: 100
  mode: merge
---

# Test Skill Prompt

This is the prompt content for the test skill.
"#;

    #[test]
    fn test_parse_skill_content() {
        let skill = SkillLoader::parse_skill_content(SAMPLE_SKILL).unwrap();

        assert_eq!(skill.id, "test-skill");
        assert_eq!(skill.version, "1.0.0");
        assert_eq!(skill.name, "Test Skill");
        assert_eq!(skill.feature.category, "testing");
        assert!(skill.feature.default_enabled);
        assert_eq!(skill.requires.context, vec!["query"]);
        assert_eq!(skill.requires.tools, vec!["execute_query"]);
        assert!(skill.prompt_content.contains("Test Skill Prompt"));
    }

    #[test]
    fn test_parse_skill_missing_frontmatter() {
        let content = "# Just Markdown\n\nNo frontmatter here.";
        let result = SkillLoader::parse_skill_content(content);
        assert!(result.is_err());
    }
}
