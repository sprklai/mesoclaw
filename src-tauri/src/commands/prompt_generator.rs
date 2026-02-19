use diesel::prelude::*;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::agent::agent_commands::resolve_active_provider;
use crate::ai::types::{CompletionRequest, Message};
use crate::database::DbPool;
use crate::database::models::generated_prompt::NewGeneratedPrompt;
use crate::database::schema::generated_prompts;

// ---------------------------------------------------------------------------
// Meta-prompts
// ---------------------------------------------------------------------------

const SKILL_META_PROMPT: &str = r#"You are an expert at creating AI skill prompt templates for the MesoClaw application.
Generate a skill file in Markdown with YAML frontmatter. Output ONLY the raw Markdown content.

Required frontmatter fields:
- id: kebab-case identifier
- name: Human-readable name
- description: One-sentence description
- category: one of (coding|writing|analysis|research|general)

After the frontmatter, write the system prompt body. Use {{ variable_name }} for Tera template variables that users will fill in.

Example format:
---
id: code-reviewer
name: Code Reviewer
description: Reviews code for bugs and improvements
category: coding
---
You are an expert code reviewer. Review the following code:

{{ code }}

Provide specific, actionable feedback on: bugs, performance, readability, and best practices.
"#;

const AGENT_META_PROMPT: &str = r#"You are an expert at creating AI agent configuration files for the MesoClaw application.
Generate an agent config in TOML format. Output ONLY the raw TOML content.

Required fields:
- id: kebab-case identifier
- name: Human-readable name
- description: What this agent does
- system_prompt: The agent's system prompt
- model: optional model override

Example format:
[agent]
id = "research-assistant"
name = "Research Assistant"
description = "Helps with research tasks"
system_prompt = """
You are a thorough research assistant. When given a topic...
"""
"#;

const SOUL_META_PROMPT: &str = r#"You are an expert at creating AI persona/identity files for the MesoClaw application.
Generate a soul/identity file in Markdown format. Output ONLY the raw Markdown content.

The file should describe the AI's personality, values, communication style, and behavioral guidelines.
It should be written in second person ("You are...") to directly address the AI.

Include sections for:
- Core identity and role
- Communication style
- Values and principles
- Behavioral guidelines
"#;

const CLAUDE_SKILL_META_PROMPT: &str = r#"You are an expert at creating Claude Code skill files.
Generate a skill file in Markdown format with YAML frontmatter. Output ONLY the raw Markdown content.

Required frontmatter fields:
- name: skill name (matches filename)
- description: When to use this skill (triggers)

The skill body should contain detailed instructions that Claude Code should follow when the skill is invoked.

Example:
---
name: my-skill
description: Use when the user wants to do X or Y. Triggered by keywords like "do X".
---

# My Skill

When this skill is activated:
1. First, do this...
2. Then, do that...
"#;

const GENERIC_META_PROMPT: &str = r#"You are an expert prompt engineer.
Generate a high-quality AI prompt based on the user's description.
Output ONLY the raw prompt text, ready to use directly.
"#;

// ---------------------------------------------------------------------------
// Return types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedArtifact {
    pub id: String,
    pub name: String,
    pub artifact_type: String,
    pub content: String,
    pub disk_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Generate an AI-powered prompt artifact via streaming.
#[tauri::command]
pub async fn generate_prompt_command(
    pool: tauri::State<'_, DbPool>,
    app: AppHandle,
    description: String,
    artifact_type: String,
    name: String,
    session_id: String,
) -> Result<GeneratedArtifact, String> {
    // 1. Resolve the active LLM provider.
    let provider = resolve_active_provider(&pool)?;

    // 2. Select meta-prompt based on artifact_type.
    let meta_prompt = match artifact_type.as_str() {
        "skill" => SKILL_META_PROMPT,
        "agent" => AGENT_META_PROMPT,
        "soul" => SOUL_META_PROMPT,
        "claude-skill" => CLAUDE_SKILL_META_PROMPT,
        "generic" => GENERIC_META_PROMPT,
        other => return Err(format!("Unknown artifact type: {other}")),
    };

    // 3. Build CompletionRequest.
    let messages = vec![Message::system(meta_prompt), Message::user(&description)];
    let completion_request = CompletionRequest::new("default", messages).with_max_tokens(4000);

    // 4. Stream the response, emitting tokens as Tauri events.
    let event_name = format!("prompt-gen-{session_id}");
    let mut stream = provider
        .stream(completion_request)
        .await
        .map_err(|e| format!("Failed to start stream: {e}"))?;

    let mut full_content = String::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                full_content.push_str(&chunk.delta);
                let _ = app.emit(
                    &event_name,
                    serde_json::json!({
                        "type": "token",
                        "content": full_content,
                    }),
                );
                if chunk.is_final {
                    break;
                }
            }
            Err(e) => {
                let _ = app.emit(
                    &event_name,
                    serde_json::json!({
                        "type": "error",
                        "error": e.to_string(),
                    }),
                );
                return Err(format!("Stream error: {e}"));
            }
        }
    }

    // Emit done event.
    let _ = app.emit(&event_name, serde_json::json!({ "type": "done" }));

    // 5. Build disk path based on artifact_type.
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    let disk_path: Option<String> = match artifact_type.as_str() {
        "skill" => {
            let p = home
                .join(".mesoclaw")
                .join("prompts")
                .join(format!("{name}.md"));
            Some(p.to_string_lossy().to_string())
        }
        "agent" => {
            let p = home
                .join(".mesoclaw")
                .join("agents")
                .join(format!("{name}.toml"));
            Some(p.to_string_lossy().to_string())
        }
        "soul" => {
            let p = home
                .join(".mesoclaw")
                .join("identity")
                .join(format!("{name}.md"));
            Some(p.to_string_lossy().to_string())
        }
        "claude-skill" => {
            let p = home
                .join(".claude")
                .join("plugins")
                .join(&name)
                .join("skills")
                .join(format!("{name}.md"));
            Some(p.to_string_lossy().to_string())
        }
        _ => None,
    };

    // 6. Write to disk if path is Some.
    if let Some(ref path_str) = disk_path {
        let path = std::path::Path::new(path_str);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {e}"))?;
        }
        std::fs::write(path, &full_content)
            .map_err(|e| format!("Failed to write artifact to disk: {e}"))?;
    }

    // 7. Insert into database.
    let artifact_id = uuid::Uuid::new_v4().to_string();
    let new_prompt = NewGeneratedPrompt {
        id: artifact_id.clone(),
        name: name.clone(),
        artifact_type: artifact_type.clone(),
        content: full_content.clone(),
        disk_path: disk_path.clone(),
        provider_id: None,
        model_id: None,
    };

    {
        let mut conn = pool
            .get()
            .map_err(|e| format!("Database connection failed: {e}"))?;
        diesel::insert_into(generated_prompts::table)
            .values(&new_prompt)
            .execute(&mut conn)
            .map_err(|e| format!("Failed to insert generated prompt: {e}"))?;
    }

    // 8. Reload the skill registry if this was a skill artifact.
    if artifact_type == "skill" {
        let registry = crate::prompts::get_or_init_registry().await;
        registry.load().await;
    }

    Ok(GeneratedArtifact {
        id: artifact_id,
        name,
        artifact_type,
        content: full_content,
        disk_path,
    })
}

/// List all previously generated prompt artifacts.
#[tauri::command]
pub async fn list_generated_prompts_command(
    pool: tauri::State<'_, DbPool>,
) -> Result<Vec<crate::database::models::generated_prompt::GeneratedPrompt>, String> {
    let mut conn = pool
        .get()
        .map_err(|e| format!("Database connection failed: {e}"))?;

    generated_prompts::table
        .order(generated_prompts::created_at.desc())
        .load::<crate::database::models::generated_prompt::GeneratedPrompt>(&mut conn)
        .map_err(|e| format!("Failed to list generated prompts: {e}"))
}

/// Delete a generated prompt artifact by ID.
#[tauri::command]
pub async fn delete_generated_prompt_command(
    pool: tauri::State<'_, DbPool>,
    id: String,
) -> Result<(), String> {
    let mut conn = pool
        .get()
        .map_err(|e| format!("Database connection failed: {e}"))?;

    diesel::delete(generated_prompts::table.filter(generated_prompts::id.eq(&id)))
        .execute(&mut conn)
        .map_err(|e| format!("Failed to delete generated prompt: {e}"))?;

    Ok(())
}
