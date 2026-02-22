//! Template loader and registry.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{PromptTemplate, SkillInfo};

// ---------------------------------------------------------------------------
// Default skills
// ---------------------------------------------------------------------------

/// Default skills that are created when the prompts directory is first initialized.
const DEFAULT_SKILLS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "code-review",
        "Code Review",
        "Review code for quality, bugs, and improvements",
        "security",
        r#"You are an expert code reviewer. Analyze the following code and provide:

1. **Overall Assessment**: A brief summary of the code quality
2. **Bugs & Issues**: Any bugs or potential issues found
3. **Security Concerns**: Security vulnerabilities or risks
4. **Performance**: Performance improvements if applicable
5. **Best Practices**: Deviations from best practices
6. **Suggestions**: Concrete improvement suggestions

Code to review:
```
{{ code }}
```

Focus on: {{ focus | default(value="general review") }}"#,
    ),
    (
        "explain-code",
        "Explain Code",
        "Explain code in simple terms for better understanding",
        "understanding",
        r#"You are a patient and clear code explainer. Explain the following code in simple, easy-to-understand terms.

Break down your explanation into:
1. **Purpose**: What does this code do?
2. **Key Components**: What are the main parts?
3. **How It Works**: Step-by-step walkthrough
4. **Key Concepts**: Any important programming concepts used

Code to explain:
```
{{ code }}
```

Explain it for someone with {{ level | default(value="intermediate") }} programming experience."#,
    ),
    (
        "generate-docs",
        "Generate Documentation",
        "Generate documentation comments for code",
        "documentation",
        r#"You are a documentation expert. Generate clear, comprehensive documentation for the following code.

Generate:
1. A brief description of what the code does
2. Parameter descriptions (if applicable)
3. Return value description (if applicable)
4. Usage examples
5. Any important notes or edge cases

Use the appropriate documentation format for the language (JSDoc, Rust docs, Python docstrings, etc.)

Code to document:
```
{{ code }}
```

Documentation style: {{ style | default(value="standard") }}"#,
    ),
    (
        "refactor-suggestions",
        "Refactor Suggestions",
        "Suggest refactoring improvements for cleaner code",
        "performance",
        r#"You are a refactoring expert. Analyze the following code and suggest improvements for:
1. **Readability**: Better naming, structure, clarity
2. **Maintainability**: DRY principle, separation of concerns
3. **Performance**: Optimization opportunities
4. **Testability**: Making the code easier to test

Code to analyze:
```
{{ code }}
```

Focus area: {{ focus | default(value="all aspects") }}"#,
    ),
    (
        "write-tests",
        "Write Tests",
        "Generate unit tests for the given code",
        "general",
        r#"You are a testing expert. Generate comprehensive unit tests for the following code.

Include:
1. Happy path tests
2. Edge case tests
3. Error handling tests
4. Any necessary mocks or fixtures

Use the appropriate testing framework for the language.

Code to test:
```
{{ code }}
```

Testing framework: {{ framework | default(value="auto-detect") }}"#,
    ),
    (
        "quick-fix",
        "Quick Fix",
        "Get quick fixes for common code issues",
        "general",
        r#"You are a debugging expert. Analyze the following code and provide quick fixes for any issues.

For each issue found:
1. Describe the problem
2. Show the problematic code
3. Provide the fixed code
4. Explain why the fix works

Code to fix:
```
{{ code }}
```

Error message (if any): {{ error | default(value="none") }}"#,
    ),
];

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Get the prompts directory path, creating it if necessary.
pub fn get_prompts_dir() -> Option<std::path::PathBuf> {
    let home = dirs::home_dir()?;
    let prompts_dir = home.join(".mesoclaw").join("prompts");

    // Create directory if it doesn't exist
    if !prompts_dir.exists() {
        if std::fs::create_dir_all(&prompts_dir).is_err() {
            return None;
        }
    }

    Some(prompts_dir)
}

/// Ensure default skills exist in the prompts directory.
pub fn ensure_default_skills() -> bool {
    let Some(prompts_dir) = get_prompts_dir() else {
        return false;
    };

    // Check if directory is empty - if so, create default skills
    let Ok(entries) = std::fs::read_dir(&prompts_dir) else {
        return false;
    };

    let has_files = entries
        .flatten()
        .any(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false));

    if has_files {
        return true; // Already has skills, don't overwrite
    }

    // Create default skills
    for (id, name, description, category, template) in DEFAULT_SKILLS {
        let file_path = prompts_dir.join(format!("{id}.md"));
        let content = format!(
            "---\nid: {id}\nname: {name}\ndescription: {description}\ncategory: {category}\n---\n\n{template}"
        );
        if std::fs::write(&file_path, content).is_err() {
            return false;
        }
    }

    true
}

/// Create a new skill file.
pub fn create_skill_file(
    id: &str,
    name: &str,
    description: &str,
    category: &str,
    template: &str,
) -> Result<(), String> {
    let Some(prompts_dir) = get_prompts_dir() else {
        return Err("Could not access prompts directory".to_string());
    };

    // Sanitize id for filename (only alphanumeric and hyphens)
    let sanitized_id: String = id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect();

    let file_path = prompts_dir.join(format!("{sanitized_id}.md"));

    // Check if file already exists
    if file_path.exists() {
        return Err(format!("Skill '{sanitized_id}' already exists"));
    }

    let content = format!(
        "---\nid: {sanitized_id}\nname: {name}\ndescription: {description}\ncategory: {category}\n---\n\n{template}"
    );

    std::fs::write(&file_path, content).map_err(|e| format!("Failed to create skill file: {e}"))?;

    Ok(())
}

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

        // Ensure default skills exist
        ensure_default_skills();

        let Some(prompts_dir) = get_prompts_dir() else {
            return;
        };
        if !prompts_dir.exists() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(&prompts_dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.extension().map(|e| e == "md").unwrap_or(false) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(template) = parse_template(&content, &path) {
                map.insert(template.id.clone(), template);
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
    pub async fn render(&self, id: &str, vars: &HashMap<String, String>) -> Result<String, String> {
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
fn parse_template(content: &str, path: &Path) -> Option<PromptTemplate> {
    let stem = path.file_stem()?.to_string_lossy().to_string();

    if let Some(rest) = content.strip_prefix("---")
        && let Some(end) = rest.find("---")
    {
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

        let path_str = path.to_string_lossy().to_string();
        return Some(PromptTemplate {
            id,
            name,
            description,
            category,
            template: body,
            parameters: vec![],
            file_path: path_str,
        });
    }

    // No frontmatter — use the filename as id and name.
    Some(PromptTemplate {
        id: stem.clone(),
        name: stem,
        description: String::new(),
        category: "general".to_string(),
        template: content.to_string(),
        parameters: vec![],
        file_path: path.to_string_lossy().to_string(),
    })
}
