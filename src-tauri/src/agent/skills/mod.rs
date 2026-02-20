//! Enhanced skill system with three-tier loading and structured metadata.
//!
//! Provides:
//! - Three-tier skill loading (workspace > global > bundled)
//! - YAML frontmatter parsing for metadata
//! - Tool schema definitions
//! - Skill requirement validation

pub mod skill_metadata;

pub use skill_metadata::{
    Skill, SkillMetadata, SkillRequirements, SkillSnapshot, SkillSource, TemplateParameter,
    ToolSchema,
};

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::prompts::TemplateRegistry;

/// Skill registry with three-tier loading support.
pub struct SkillRegistry {
    /// Loaded skills indexed by ID
    skills: RwLock<HashMap<String, Skill>>,
    /// Template registry for prompt rendering
    template_registry: Arc<TemplateRegistry>,
}

impl SkillRegistry {
    /// Create a new skill registry.
    pub fn new(template_registry: Arc<TemplateRegistry>) -> Arc<Self> {
        Arc::new(Self {
            skills: RwLock::new(HashMap::new()),
            template_registry,
        })
    }

    /// Build a skill snapshot by loading from all three tiers.
    ///
    /// Loading precedence (highest to lowest):
    /// 1. Workspace skills: `<workspace>/.mesoclaw/skills/`
    /// 2. Global skills: `~/.mesoclaw/skills/`
    /// 3. Bundled skills: Built into application
    pub async fn build_skill_snapshot(
        &self,
        workspace_path: Option<&Path>,
    ) -> Result<SkillSnapshot, String> {
        let mut skills = HashMap::new();
        let version = chrono::Utc::now().to_rfc3339();

        // Load bundled skills (lowest precedence)
        self.load_bundled_skills(&mut skills).await?;

        // Load global skills (medium precedence)
        self.load_global_skills(&mut skills).await?;

        // Load workspace skills (highest precedence)
        if let Some(workspace) = workspace_path {
            self.load_workspace_skills(workspace, &mut skills).await?;
        }

        Ok(SkillSnapshot { version, skills })
    }

    /// Load bundled skills (built into application).
    async fn load_bundled_skills(
        &self,
        _skills: &mut HashMap<String, Skill>,
    ) -> Result<(), String> {
        // Bundled skills would be embedded in the binary or shipped with the app
        // For now, this is a placeholder
        Ok(())
    }

    /// Load global user skills from ~/.mesoclaw/skills/.
    async fn load_global_skills(&self, skills: &mut HashMap<String, Skill>) -> Result<(), String> {
        let Some(home) = dirs::home_dir() else {
            return Ok(());
        };

        let global_skills_dir = home.join(".mesoclaw").join("skills");
        if !global_skills_dir.exists() {
            return Ok(());
        }

        self.load_skills_from_directory(&global_skills_dir, SkillSource::Global, skills)
            .await
    }

    /// Load workspace-specific skills from <workspace>/.mesoclaw/skills/.
    async fn load_workspace_skills(
        &self,
        workspace: &Path,
        skills: &mut HashMap<String, Skill>,
    ) -> Result<(), String> {
        let workspace_skills_dir = workspace.join(".mesoclaw").join("skills");
        if !workspace_skills_dir.exists() {
            return Ok(());
        }

        self.load_skills_from_directory(&workspace_skills_dir, SkillSource::Workspace, skills)
            .await
    }

    /// Load all skill files from a directory.
    async fn load_skills_from_directory(
        &self,
        dir: &Path,
        source: SkillSource,
        skills: &mut HashMap<String, Skill>,
    ) -> Result<(), String> {
        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Failed to read skills directory: {e}"))?;

        for entry in entries.flatten() {
            let path = entry.path();

            // Only process .md files
            if !path.extension().map(|e| e == "md").unwrap_or(false) {
                continue;
            }

            if let Some(skill) = self.parse_skill_file(&path, source.clone()).await? {
                skills.insert(skill.id.clone(), skill);
            }
        }

        Ok(())
    }

    /// Parse a skill file with YAML frontmatter.
    async fn parse_skill_file(
        &self,
        path: &Path,
        source: SkillSource,
    ) -> Result<Option<Skill>, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read skill file {}: {e}", path.display()))?;

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Parse frontmatter and template
        if let Some((metadata, template)) = parse_skill_content(&content)? {
            let skill = Skill {
                id: metadata.id.unwrap_or(stem.clone()),
                name: metadata.name.unwrap_or(stem),
                description: metadata.description.unwrap_or_default(),
                category: metadata.category.unwrap_or_else(|| "general".to_string()),
                metadata: metadata.metadata.unwrap_or_default(),
                enabled: true,
                template,
                parameters: metadata.parameters.unwrap_or_default(),
                source,
                file_path: Some(path.to_string_lossy().to_string()),
            };
            return Ok(Some(skill));
        }

        Ok(None)
    }

    /// Get a skill by ID.
    pub async fn get(&self, id: &str) -> Option<Skill> {
        self.skills.read().await.get(id).cloned()
    }

    /// Get all skills.
    pub async fn all(&self) -> Vec<Skill> {
        self.skills.read().await.values().cloned().collect()
    }

    /// Render a skill template with the provided variables.
    pub async fn render(
        &self,
        skill_id: &str,
        vars: &HashMap<String, String>,
    ) -> Result<String, String> {
        let skill = self
            .get(skill_id)
            .await
            .ok_or_else(|| format!("Skill not found: {skill_id}"))?;

        // Use the template registry for rendering
        // For now, delegate to template registry
        self.template_registry.render(&skill.id, vars).await
    }

    /// Check if skill requirements are satisfied.
    pub fn check_requirements(&self, skill: &Skill) -> Result<Vec<String>, Vec<String>> {
        let mut satisfied = Vec::new();
        let mut missing = Vec::new();

        if let Some(ref reqs) = skill.metadata.requires {
            // Check any_bins (at least one must be present)
            if let Some(ref any_bins) = reqs.any_bins {
                let has_any = any_bins.iter().any(|bin| which::which(bin).is_ok());
                if has_any {
                    satisfied.push(format!("Has one of: {}", any_bins.join(", ")));
                } else {
                    missing.push(format!("Missing any of: {}", any_bins.join(", ")));
                }
            }

            // Check all_bins (all must be present)
            if let Some(ref all_bins) = reqs.all_bins {
                let missing_bins: Vec<&str> = all_bins
                    .iter()
                    .filter(|bin| which::which(bin).is_err())
                    .map(|s| s.as_str())
                    .collect();
                if missing_bins.is_empty() {
                    satisfied.push(format!("Has all of: {}", all_bins.join(", ")));
                } else {
                    missing.push(format!("Missing binaries: {}", missing_bins.join(", ")));
                }
            }

            // Check api_keys (must be in environment or config)
            if let Some(ref api_keys) = reqs.api_keys {
                for key in api_keys {
                    if std::env::var(key).is_ok() {
                        satisfied.push(format!("API key {key} is set"));
                    } else {
                        missing.push(format!("API key {key} is not configured"));
                    }
                }
            }

            // Check env variables
            if let Some(ref env_vars) = reqs.env {
                for var in env_vars {
                    if std::env::var(var).is_ok() {
                        satisfied.push(format!("Environment variable {var} is set"));
                    } else {
                        missing.push(format!("Environment variable {var} is not set"));
                    }
                }
            }
        }

        if missing.is_empty() {
            Ok(satisfied)
        } else {
            Err(missing)
        }
    }
}

/// Parsed skill frontmatter.
#[derive(Debug, Default, Deserialize)]
struct SkillFrontmatter {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    metadata: Option<SkillMetadata>,
    #[serde(default)]
    parameters: Option<Vec<TemplateParameter>>,
}

/// Parse skill content with YAML frontmatter.
///
/// Format:
/// ```markdown
/// ---
/// id: my-skill
/// name: My Skill
/// description: What this skill does
/// category: general
/// metadata:
///   emoji: "🔧"
///   requires:
///     anyBins: ["python", "python3"]
///     apiKeys: ["OPENAI_API_KEY"]
/// parameters:
///   - name: input
///     description: Input text
///     required: true
/// ---
///
/// Skill template content with {{input}} placeholder.
/// ```
fn parse_skill_content(content: &str) -> Result<Option<(SkillFrontmatter, String)>, String> {
    // Check for frontmatter
    if !content.starts_with("---") {
        // No frontmatter, treat entire content as template
        return Ok(None);
    }

    // Find the closing ---
    let rest = &content[3..];
    let end = rest
        .find("---")
        .ok_or_else(|| "Invalid frontmatter: missing closing ---".to_string())?;

    let frontmatter_text = &rest[..end];
    let template = rest[end + 3..].trim().to_string();

    // Parse YAML frontmatter
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(frontmatter_text)
        .map_err(|e| format!("Failed to parse frontmatter: {e}"))?;

    Ok(Some((frontmatter, template)))
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
