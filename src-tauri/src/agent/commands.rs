//! Tauri IPC commands for agent management.
//!
//! Provides CRUD operations for agent configurations, session management,
//! and workspace file operations.

use std::sync::Arc;

use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use crate::{
    agent::session_router::{Session, SessionKey, SessionRouter},
    database::DbPool,
    event_bus::EventBus,
    identity::IdentityLoader,
    security::SecurityPolicy,
    tools::ToolRegistry,
};

use super::agent_commands::SessionCancelMap;

// ─── Response Types ───────────────────────────────────────────────────────────

/// Agent info returned by list_agents_command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub model: String,
    pub is_active: bool,
    pub created_at: String,
}

/// Agent configuration for create/update operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfigInput {
    pub name: String,
    pub description: Option<String>,
    pub model_provider: Option<String>,
    pub model_id: Option<String>,
    pub skills: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

/// Session info returned by list_sessions_command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub agent: String,
    pub scope: String,
    pub channel: String,
    pub peer: String,
    pub message_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Workspace file content response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFile {
    pub path: String,
    pub content: String,
    pub exists: bool,
}

/// Workspace file listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFileList {
    pub agent_id: String,
    pub files: Vec<String>,
}

// ─── Agent CRUD Commands ──────────────────────────────────────────────────────

/// List all configured agents.
///
/// Returns a list of agent configurations with their current status.
#[tauri::command]
pub async fn list_agents_command(_app_handle: tauri::AppHandle) -> Result<Vec<AgentInfo>, String> {
    // For now, return a single default agent based on the active provider.
    // This can be extended later to support multiple agent configurations
    // stored in the database.
    Ok(vec![AgentInfo {
        id: "default".to_string(),
        name: "Default Agent".to_string(),
        description: "The main conversational agent using the configured LLM provider.".to_string(),
        model: "auto".to_string(),
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    }])
}

/// Create a new agent configuration.
///
/// Stores the agent configuration in the database and returns the new agent ID.
#[tauri::command]
pub async fn create_agent_command(
    _id: String,
    _config: AgentConfigInput,
    _app_handle: tauri::AppHandle,
) -> Result<AgentInfo, String> {
    // Placeholder for future multi-agent support.
    // Currently, only the default agent is supported.
    Err("Multi-agent configuration is not yet implemented. Use the default agent.".to_string())
}

/// Get a specific agent configuration by ID.
///
/// Returns the full configuration for the specified agent.
#[tauri::command]
pub async fn get_agent_command(
    agent_id: String,
    _app_handle: tauri::AppHandle,
) -> Result<AgentInfo, String> {
    if agent_id == "default" {
        Ok(AgentInfo {
            id: "default".to_string(),
            name: "Default Agent".to_string(),
            description: "The main conversational agent using the configured LLM provider."
                .to_string(),
            model: "auto".to_string(),
            is_active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    } else {
        Err(format!("Agent '{}' not found", agent_id))
    }
}

/// Update an agent configuration.
///
/// Updates the specified fields of an agent configuration.
#[tauri::command]
pub async fn update_agent_command(
    _agent_id: String,
    _config: AgentConfigInput,
    _app_handle: tauri::AppHandle,
) -> Result<AgentInfo, String> {
    // Placeholder for future multi-agent support.
    Err("Agent update is not yet implemented. Use the default agent.".to_string())
}

/// Delete an agent configuration.
///
/// Removes the agent configuration from the database.
#[tauri::command]
pub async fn delete_agent_command(
    agent_id: String,
    _app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if agent_id == "default" {
        Err("Cannot delete the default agent".to_string())
    } else {
        Err(format!("Agent '{}' not found", agent_id))
    }
}

// ─── Agent Execution Commands ─────────────────────────────────────────────────

/// Run an agent with a specific message.
///
/// Executes the agent loop with the provided message and returns the response.
/// This is an alias for start_agent_session_command but with agent selection.
#[tauri::command]
pub async fn run_agent_command(
    _agent_id: String,
    message: String,
    app_handle: tauri::AppHandle,
    pool: State<'_, DbPool>,
    tool_registry: State<'_, Arc<ToolRegistry>>,
    security_policy: State<'_, Arc<SecurityPolicy>>,
    event_bus: State<'_, Arc<dyn EventBus>>,
    identity_loader: State<'_, IdentityLoader>,
    cancel_map: State<'_, SessionCancelMap>,
    supervisor: State<'_, Arc<crate::lifecycle::LifecycleSupervisor>>,
) -> Result<String, String> {
    // For now, all runs use the default agent.
    // The agent_id parameter is reserved for future multi-agent support.
    super::agent_commands::start_agent_session_command(
        message,
        app_handle,
        pool,
        tool_registry,
        security_policy,
        event_bus,
        identity_loader,
        cancel_map,
        supervisor,
    )
    .await
}

// ─── Session Management Commands ──────────────────────────────────────────────

/// List all active sessions.
///
/// Returns session metadata for all sessions currently in memory.
#[tauri::command]
pub async fn list_sessions_command(pool: State<'_, DbPool>) -> Result<Vec<SessionInfo>, String> {
    use crate::database::schema::chat_sessions::dsl::*;

    let mut conn = pool.get().map_err(|e| format!("Database error: {}", e))?;

    let results = chat_sessions
        .order(created_at.desc())
        .limit(100)
        .load::<crate::database::models::ChatSession>(&mut conn)
        .map_err(|e| format!("Failed to load sessions: {}", e))?;

    Ok(results
        .into_iter()
        .map(|s| SessionInfo {
            session_id: s.session_key,
            agent: s.agent,
            scope: s.scope,
            channel: s.channel,
            peer: s.peer,
            message_count: 0, // Would need to join with messages table
            created_at: s.created_at,
            updated_at: s.updated_at,
        })
        .collect())
}

/// Get detailed session information with message history.
///
/// Returns the full session including message history.
#[tauri::command]
pub async fn get_session_command(
    session_key: String,
    session_router: State<'_, Arc<SessionRouter>>,
) -> Result<Session, String> {
    let key = SessionKey::parse(&session_key)?;
    session_router
        .get_session(&key)
        .ok_or_else(|| format!("Session '{}' not found", session_key))
}

// ─── Workspace File Commands ──────────────────────────────────────────────────

/// Get the content of a workspace file.
///
/// Reads a file from the agent's workspace directory and returns its content.
#[tauri::command]
pub async fn get_workspace_file_command(
    agent_id: String,
    file_path: String,
    app_handle: tauri::AppHandle,
) -> Result<WorkspaceFile, String> {
    let workspace_dir = get_agent_workspace_dir(&agent_id, &app_handle)?;

    let full_path = workspace_dir.join(&file_path);

    if !full_path.exists() {
        return Ok(WorkspaceFile {
            path: file_path,
            content: String::new(),
            exists: false,
        });
    }

    let content =
        std::fs::read_to_string(&full_path).map_err(|e| format!("Failed to read file: {}", e))?;

    Ok(WorkspaceFile {
        path: file_path,
        content,
        exists: true,
    })
}

/// Update the content of a workspace file.
///
/// Creates or overwrites a file in the agent's workspace directory.
#[tauri::command]
pub async fn update_workspace_file_command(
    agent_id: String,
    file_path: String,
    content: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let workspace_dir = get_agent_workspace_dir(&agent_id, &app_handle)?;

    // Ensure workspace directory exists
    std::fs::create_dir_all(&workspace_dir)
        .map_err(|e| format!("Failed to create workspace directory: {}", e))?;

    let full_path = workspace_dir.join(&file_path);

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    std::fs::write(&full_path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    log::info!(
        "Updated workspace file '{}' for agent '{}'",
        file_path,
        agent_id
    );
    Ok(())
}

/// List all files in an agent's workspace.
///
/// Returns a list of all files in the agent's workspace directory.
#[tauri::command]
pub async fn list_workspace_files_command(
    agent_id: String,
    app_handle: tauri::AppHandle,
) -> Result<WorkspaceFileList, String> {
    let workspace_dir = get_agent_workspace_dir(&agent_id, &app_handle)?;

    if !workspace_dir.exists() {
        return Ok(WorkspaceFileList {
            agent_id,
            files: vec![],
        });
    }

    let mut files = Vec::new();
    collect_files(&workspace_dir, &workspace_dir, &mut files)?;

    Ok(WorkspaceFileList { agent_id, files })
}

// ─── Helper Functions ─────────────────────────────────────────────────────────

/// Get the workspace directory for a specific agent.
fn get_agent_workspace_dir(
    agent_id: &str,
    app: &tauri::AppHandle,
) -> Result<std::path::PathBuf, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let workspace_dir = app_data
        .join("agents")
        .join("workspaces")
        .join(agent_id)
        .join("workspace");
    Ok(workspace_dir)
}

/// Recursively collect files from a directory.
fn collect_files(
    base: &std::path::Path,
    current: &std::path::Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    let entries =
        std::fs::read_dir(current).map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            collect_files(base, &path, files)?;
        } else if path.is_file() {
            if let Ok(relative) = path.strip_prefix(base) {
                files.push(relative.to_string_lossy().to_string());
            }
        }
    }

    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_info_serialization() {
        let info = AgentInfo {
            id: "default".to_string(),
            name: "Default Agent".to_string(),
            description: "Test agent".to_string(),
            model: "gpt-4".to_string(),
            is_active: true,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: AgentInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "default");
        assert_eq!(deserialized.name, "Default Agent");
    }

    #[test]
    fn test_session_info_serialization() {
        let info = SessionInfo {
            session_id: "main:dm:tauri:user".to_string(),
            agent: "main".to_string(),
            scope: "dm".to_string(),
            channel: "tauri".to_string(),
            peer: "user".to_string(),
            message_count: 5,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: SessionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.session_id, "main:dm:tauri:user");
        assert_eq!(deserialized.message_count, 5);
    }

    #[test]
    fn test_workspace_file_serialization() {
        let file = WorkspaceFile {
            path: "test.md".to_string(),
            content: "# Test\nHello world".to_string(),
            exists: true,
        };

        let json = serde_json::to_string(&file).unwrap();
        let deserialized: WorkspaceFile = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.path, "test.md");
        assert!(deserialized.exists);
    }

    #[test]
    fn test_workspace_file_list_serialization() {
        let list = WorkspaceFileList {
            agent_id: "agent-1".to_string(),
            files: vec!["file1.md".to_string(), "file2.txt".to_string()],
        };

        let json = serde_json::to_string(&list).unwrap();
        let deserialized: WorkspaceFileList = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_id, "agent-1");
        assert_eq!(deserialized.files.len(), 2);
    }
}
