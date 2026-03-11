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
        };
        let info = SkillInfo::from(&skill);
        assert_eq!(info.id, "test");
        assert_eq!(info.name, "Test Skill");
        assert_eq!(info.source, SkillSource::Bundled);
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
        };
        let json = serde_json::to_string(&skill).unwrap();
        let parsed: Skill = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.source, SkillSource::User);
    }
}
