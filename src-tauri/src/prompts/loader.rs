//! Template loader and registry.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{PromptTemplate, SkillInfo};

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Registry of loaded prompt templates, backed by a `RwLock`-protected map.
pub struct TemplateRegistry {
    templates: RwLock<HashMap<String, PromptTemplate>>,
}

impl TemplateRegistry {
    /// Create a new, empty registry wrapped in an `Arc`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            templates: RwLock::new(HashMap::new()),
        })
    }

    /// (Re-)load all templates from `~/.mesoclaw/prompts/`.
    ///
    /// Files must have a `.md` extension. Files without valid frontmatter are
    /// treated as plain templates using the filename as the template id.
    pub async fn load(&self) {
        let mut map = self.templates.write().await;
        map.clear();

        let Some(home) = dirs::home_dir() else {
            return;
        };
        let prompts_dir = home.join(".mesoclaw").join("prompts");
        if !prompts_dir.exists() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(&prompts_dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(template) = parse_template(&content, &path) {
                        map.insert(template.id.clone(), template);
                    }
                }
            }
        }
    }

    /// Return all loaded templates.
    pub async fn all(&self) -> Vec<PromptTemplate> {
        self.templates.read().await.values().cloned().collect()
    }

    /// Look up a single template by id.
    pub async fn get(&self, id: &str) -> Option<PromptTemplate> {
        self.templates.read().await.get(id).cloned()
    }

    /// Return all templates as `SkillInfo` objects (for IPC compatibility).
    pub async fn skill_infos(&self) -> Vec<SkillInfo> {
        self.templates
            .read()
            .await
            .values()
            .map(skill_info_from_template)
            .collect()
    }

    /// Return templates grouped by category.
    pub async fn by_category(&self) -> HashMap<String, Vec<SkillInfo>> {
        let mut map: HashMap<String, Vec<SkillInfo>> = HashMap::new();
        for t in self.templates.read().await.values() {
            map.entry(t.category.clone())
                .or_default()
                .push(skill_info_from_template(t));
        }
        map
    }

    /// Render a template with the provided variables using Tera.
    pub async fn render(
        &self,
        id: &str,
        vars: &HashMap<String, String>,
    ) -> Result<String, String> {
        let template = self
            .get(id)
            .await
            .ok_or_else(|| format!("Template not found: {id}"))?;

        let mut context = tera::Context::new();
        for (k, v) in vars {
            context.insert(k, v);
        }

        let mut engine = tera::Tera::default();
        engine
            .add_raw_template(&template.id, &template.template)
            .map_err(|e| format!("Template parse error: {e}"))?;
        engine
            .render(&template.id, &context)
            .map_err(|e| format!("Template render error: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn skill_info_from_template(t: &PromptTemplate) -> SkillInfo {
    SkillInfo {
        id: t.id.clone(),
        name: t.name.clone(),
        description: t.description.clone(),
        category: t.category.clone(),
        default_enabled: true,
        source: "filesystem".to_string(),
    }
}

/// Parse a markdown file (with optional YAML frontmatter) into a `PromptTemplate`.
///
/// Frontmatter format:
/// ```text
/// ---
/// id: my-template
/// name: My Template
/// description: What it does
/// category: general
/// ---
/// Template content with {{ variable }} placeholders
/// ```
fn parse_template(content: &str, path: &PathBuf) -> Option<PromptTemplate> {
    let stem = path.file_stem()?.to_string_lossy().to_string();

    if content.starts_with("---") {
        let rest = &content[3..];
        if let Some(end) = rest.find("---") {
            let frontmatter = &rest[..end];
            let body = rest[end + 3..].trim().to_string();

            let mut id = stem.clone();
            let mut name = stem.clone();
            let mut description = String::new();
            let mut category = "general".to_string();

            for line in frontmatter.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    match k.trim() {
                        "id" => id = v.trim().to_string(),
                        "name" => name = v.trim().to_string(),
                        "description" => description = v.trim().to_string(),
                        "category" => category = v.trim().to_string(),
                        _ => {}
                    }
                }
            }

            return Some(PromptTemplate {
                id,
                name,
                description,
                category,
                template: body,
                parameters: vec![],
            });
        }
    }

    // No frontmatter — use the filename as id and name.
    Some(PromptTemplate {
        id: stem.clone(),
        name: stem,
        description: String::new(),
        category: "general".to_string(),
        template: content.to_string(),
        parameters: vec![],
    })
}
