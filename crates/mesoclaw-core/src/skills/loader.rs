use std::path::Path;

use crate::{MesoError, Result};

use super::types::{Skill, SkillFrontmatter, SkillSource};

/// Parse YAML frontmatter and markdown body from a skill file.
/// Returns (frontmatter, body). If no frontmatter delimiters found, body is the full content.
pub fn parse_frontmatter(content: &str) -> (Option<SkillFrontmatter>, String) {
    let trimmed = content.trim();
    if !trimmed.starts_with("---") {
        return (None, content.to_string());
    }

    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("---") {
        let yaml_str = &after_first[..end];
        let body = after_first[end + 3..].trim().to_string();
        let frontmatter: Option<SkillFrontmatter> = serde_yaml::from_str(yaml_str).ok();
        (frontmatter, body)
    } else {
        (None, content.to_string())
    }
}

/// Load a skill from raw content with a given id and source.
pub fn load_skill_from_content(id: &str, content: &str, source: SkillSource) -> Skill {
    let (frontmatter, body) = parse_frontmatter(content);

    match frontmatter {
        Some(fm) => Skill {
            id: id.to_string(),
            name: fm.name,
            description: fm.description,
            category: fm.category,
            content: body,
            source,
            enabled: true,
        },
        None => Skill {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            category: "general".into(),
            content: content.to_string(),
            source,
            enabled: true,
        },
    }
}

/// Load a skill from a .md file on disk.
pub fn load_skill_from_file(path: &Path, max_size: usize) -> Result<Skill> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() as usize > max_size {
        return Err(MesoError::Skill(format!(
            "skill file '{}' exceeds max size ({} > {max_size})",
            path.display(),
            metadata.len()
        )));
    }

    let content = std::fs::read_to_string(path)?;
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(load_skill_from_content(&id, &content, SkillSource::User))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_frontmatter_valid() {
        let content = "---\nname: test\ndescription: A test skill\ncategory: meta\n---\n# Body\nContent here.";
        let (fm, body) = parse_frontmatter(content);
        let fm = fm.unwrap();
        assert_eq!(fm.name, "test");
        assert_eq!(fm.description, "A test skill");
        assert_eq!(fm.category, "meta");
        assert!(body.contains("Body"));
    }

    #[test]
    fn parse_frontmatter_no_yaml() {
        let content = "# Just markdown\nNo frontmatter.";
        let (fm, body) = parse_frontmatter(content);
        assert!(fm.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn parse_frontmatter_empty_yaml() {
        let content = "---\n---\n# Body";
        let (fm, body) = parse_frontmatter(content);
        // Empty YAML won't parse into SkillFrontmatter (name is required)
        assert!(fm.is_none());
        assert!(body.contains("Body"));
    }

    #[test]
    fn load_skill_from_content_with_frontmatter() {
        let content =
            "---\nname: coding\ndescription: Code helper\ncategory: dev\n---\nWrite clean code.";
        let skill = load_skill_from_content("coding", content, SkillSource::Bundled);
        assert_eq!(skill.name, "coding");
        assert_eq!(skill.category, "dev");
        assert!(skill.content.contains("Write clean code"));
        assert_eq!(skill.source, SkillSource::Bundled);
    }

    #[test]
    fn load_skill_from_file_works() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test-skill.md");
        std::fs::write(
            &path,
            "---\nname: test-skill\ndescription: Test\ncategory: test\n---\nSkill body.",
        )
        .unwrap();

        let skill = load_skill_from_file(&path, 100_000).unwrap();
        assert_eq!(skill.id, "test-skill");
        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.source, SkillSource::User);
    }

    #[test]
    fn load_skill_respects_max_size() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("big.md");
        std::fs::write(&path, "x".repeat(1000)).unwrap();

        let result = load_skill_from_file(&path, 100);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MesoError::Skill(_)));
    }
}
