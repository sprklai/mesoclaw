use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
#[non_exhaustive]
pub enum SkillSource {
    Bundled,
    User,
}

impl std::fmt::Display for SkillSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bundled => write!(f, "bundled"),
            Self::User => write!(f, "user"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub content: String,
    pub source: SkillSource,
    pub enabled: bool,
    /// Context domain this skill belongs to (None = always active).
    /// Maps to ContextDomain: "channels", "scheduler", "skills", "tools".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Surface filter: "all" or specific surface name (None = everywhere).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
}

/// Summary struct for list endpoints (excludes full content).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub source: SkillSource,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
}

impl From<&Skill> for SkillInfo {
    fn from(skill: &Skill) -> Self {
        Self {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            category: skill.category.clone(),
            source: skill.source.clone(),
            enabled: skill.enabled,
            domain: skill.domain.clone(),
            surface: skill.surface.clone(),
        }
    }
}

/// YAML frontmatter metadata for skill files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub surface: Option<String>,
}

fn default_category() -> String {
    "general".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_source_display() {
        assert_eq!(SkillSource::Bundled.to_string(), "bundled");
        assert_eq!(SkillSource::User.to_string(), "user");
    }

    #[test]
    fn skill_info_from_skill() {
        let skill = Skill {
            id: "test".into(),
            name: "Test Skill".into(),
            description: "A test".into(),
            category: "meta".into(),
            content: "Full content here".into(),
            source: SkillSource::Bundled,
            enabled: true,
            domain: Some("tools".into()),
            surface: Some("all".into()),
        };
        let info = SkillInfo::from(&skill);
        assert_eq!(info.id, "test");
        assert_eq!(info.name, "Test Skill");
        assert_eq!(info.source, SkillSource::Bundled);
        assert_eq!(info.domain.as_deref(), Some("tools"));
        assert_eq!(info.surface.as_deref(), Some("all"));
    }

    #[test]
    fn skill_serialize_roundtrip() {
        let skill = Skill {
            id: "test".into(),
            name: "Test".into(),
            description: "Desc".into(),
            category: "general".into(),
            content: "Content".into(),
            source: SkillSource::User,
            enabled: true,
            domain: None,
            surface: None,
        };
        let json = serde_json::to_string(&skill).unwrap();
        let parsed: Skill = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.source, SkillSource::User);
        assert!(parsed.domain.is_none());
    }

    // P19.22 — Skill has domain and surface optional fields
    #[test]
    fn skill_domain_surface_fields() {
        let skill = Skill {
            id: "mem".into(),
            name: "Memory".into(),
            description: "Memory usage".into(),
            category: "tools".into(),
            content: "How to use memory".into(),
            source: SkillSource::Bundled,
            enabled: true,
            domain: Some("tools".into()),
            surface: Some("channels".into()),
        };
        assert_eq!(skill.domain.as_deref(), Some("tools"));
        assert_eq!(skill.surface.as_deref(), Some("channels"));
    }

    // P19.23 — SkillFrontmatter parses domain and surface
    #[test]
    fn skill_frontmatter_domain() {
        let yaml = r#"
name: test-skill
description: A test
category: tools
domain: scheduler
surface: all
"#;
        let fm: SkillFrontmatter = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(fm.domain.as_deref(), Some("scheduler"));
        assert_eq!(fm.surface.as_deref(), Some("all"));
    }

    // Frontmatter without domain/surface defaults to None
    #[test]
    fn skill_frontmatter_no_domain() {
        let yaml = r#"
name: basic
description: Basic skill
"#;
        let fm: SkillFrontmatter = serde_yaml::from_str(yaml).unwrap();
        assert!(fm.domain.is_none());
        assert!(fm.surface.is_none());
    }
}
