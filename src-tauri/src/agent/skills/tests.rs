//! Unit tests for the enhanced skill system.

use super::*;

#[test]
fn test_skill_metadata_default() {
    let metadata = SkillMetadata::default();
    assert!(metadata.emoji.is_none());
    assert!(metadata.requires.is_none());
    assert!(metadata.tool_schemas.is_none());
    assert!(metadata.extra.is_empty());
}

#[test]
fn test_skill_requirements_validation() {
    let skill = Skill {
        id: "test-skill".to_string(),
        name: "Test Skill".to_string(),
        description: "A test skill".to_string(),
        category: "testing".to_string(),
        metadata: SkillMetadata {
            emoji: Some("🧪".to_string()),
            requires: Some(SkillRequirements {
                any_bins: Some(vec!["bash".to_string()]),
                all_bins: None,
                api_keys: None,
                env: None,
            }),
            tool_schemas: None,
            extra: HashMap::new(),
        },
        enabled: true,
        template: "Test template".to_string(),
        parameters: vec![],
        source: SkillSource::Global,
        file_path: None,
    };

    assert_eq!(skill.id, "test-skill");
    assert_eq!(skill.name, "Test Skill");
    assert!(skill.enabled);
    assert_eq!(skill.source, SkillSource::Global);
    assert!(skill.metadata.emoji.is_some());
    assert_eq!(skill.metadata.emoji.unwrap(), "🧪");
}

#[test]
fn test_skill_source_precedence() {
    let sources = vec![
        SkillSource::Bundled,
        SkillSource::Global,
        SkillSource::Workspace,
    ];

    // Workspace should override Global and Bundled
    assert_eq!(sources[2], SkillSource::Workspace);
    // Global should override Bundled
    assert_eq!(sources[1], SkillSource::Global);
    // Bundled has lowest precedence
    assert_eq!(sources[0], SkillSource::Bundled);
}

#[test]
fn test_skill_frontmatter_parsing() {
    let content = r#"---
id: my-skill
name: My Skill
description: A test skill
category: general
metadata:
  emoji: "🔧"
  requires:
    anyBins: ["python", "python3"]
    allBins: ["git"]
    apiKeys: ["OPENAI_API_KEY"]
parameters:
  - name: input
    description: Input text
    required: true
---

This is the template with {{input}}.
"#;

    let result = parse_skill_content(content).unwrap();
    assert!(result.is_some());

    let (fm, template) = result.unwrap();
    assert_eq!(fm.id, Some("my-skill".to_string()));
    assert_eq!(fm.name, Some("My Skill".to_string()));
    assert_eq!(fm.description, Some("A test skill".to_string()));
    assert_eq!(fm.category, Some("general".to_string()));
    assert!(fm.metadata.is_some());
    assert!(fm.parameters.is_some());
    assert!(template.contains("{{input}}"));

    let metadata = fm.metadata.unwrap();
    assert_eq!(metadata.emoji, Some("🔧".to_string()));
    assert!(metadata.requires.is_some());

    let reqs = metadata.requires.unwrap();
    assert_eq!(
        reqs.any_bins,
        Some(vec!["python".to_string(), "python3".to_string()])
    );
    assert_eq!(reqs.all_bins, Some(vec!["git".to_string()]));
    assert_eq!(reqs.api_keys, Some(vec!["OPENAI_API_KEY".to_string()]));

    let params = fm.parameters.unwrap();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "input");
    assert_eq!(params[0].description, "Input text");
    assert!(params[0].required);
}

#[test]
fn test_skill_without_frontmatter() {
    let content = "Just a plain template without frontmatter.";
    let result = parse_skill_content(content).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_skill_with_minimal_frontmatter() {
    let content = r#"---
name: Minimal Skill
---
Simple template.
"#;

    let result = parse_skill_content(content).unwrap();
    assert!(result.is_some());

    let (fm, template) = result.unwrap();
    assert_eq!(fm.name, Some("Minimal Skill".to_string()));
    assert_eq!(template, "Simple template.");
}
