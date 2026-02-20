//! Agent Database IPC Commands
//!
//! Commands for managing AI agents, sessions, and runs via Tauri IPC.
//! These commands interact with the database models defined in
//! `src-tauri/src/database/models/agent.rs`.
//!
//! Note: These are distinct from the agent runtime commands in
//! `src-tauri/src/agent/commands.rs` which handle agent execution.

use crate::database::models::agent::{
    Agent, AgentRun, AgentSession, CreateAgentRequest, UpdateAgentRequest, NewAgent, UpdateAgent,
};
use crate::database::schema::{agents, agent_sessions, agent_runs};
use crate::database::utils::bool_to_int;
use crate::database::DbPool;
use diesel::prelude::*;
use tauri::State;

// ─── Agent CRUD Commands ─────────────────────────────────────────────────────

/// List all active agents from the database
#[tauri::command]
pub fn list_db_agents_command(pool: State<'_, DbPool>) -> Result<Vec<Agent>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agents::table
        .filter(agents::is_active.eq(1))
        .select(Agent::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

/// Get a single agent by ID from the database
#[tauri::command]
pub fn get_db_agent_command(id: String, pool: State<'_, DbPool>) -> Result<Agent, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agents::table
        .find(id)
        .select(Agent::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

/// Create a new agent in the database
#[tauri::command]
pub fn create_db_agent_command(
    request: CreateAgentRequest,
    pool: State<'_, DbPool>,
) -> Result<Agent, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

    let new_agent = NewAgent {
        id: id.clone(),
        name: request.name,
        description: request.description,
        system_prompt: request.system_prompt,
        model_id: request.model_id,
        provider_id: request.provider_id,
        temperature: request.temperature.unwrap_or(0.7),
        max_tokens: request.max_tokens,
        tools_enabled: request.tools_enabled.map(bool_to_int).unwrap_or(1),
        memory_enabled: request.memory_enabled.map(bool_to_int).unwrap_or(1),
        workspace_path: None,
        is_active: 1,
        created_at: now.clone(),
        updated_at: now,
    };

    diesel::insert_into(agents::table)
        .values(&new_agent)
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    agents::table
        .find(id)
        .select(Agent::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

/// Update an existing agent in the database
#[tauri::command]
pub fn update_db_agent_command(
    request: UpdateAgentRequest,
    pool: State<'_, DbPool>,
) -> Result<Agent, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    let update = UpdateAgent {
        name: request.name,
        description: request.description,
        system_prompt: request.system_prompt,
        model_id: request.model_id,
        provider_id: request.provider_id,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        tools_enabled: request.tools_enabled.map(bool_to_int),
        memory_enabled: request.memory_enabled.map(bool_to_int),
        workspace_path: request.workspace_path,
        is_active: request.is_active.map(bool_to_int),
        updated_at: now,
    };

    diesel::update(agents::table.find(&request.id))
        .set(&update)
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    agents::table
        .find(request.id)
        .select(Agent::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

/// Delete an agent by ID from the database
#[tauri::command]
pub fn delete_db_agent_command(id: String, pool: State<'_, DbPool>) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    diesel::delete(agents::table.find(id))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Session Commands ────────────────────────────────────────────────────────

/// List all sessions for a specific agent from the database
#[tauri::command]
pub fn list_db_agent_sessions_command(
    agent_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<AgentSession>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agent_sessions::table
        .filter(agent_sessions::agent_id.eq(agent_id))
        .order(agent_sessions::created_at.desc())
        .select(AgentSession::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

/// List recent sessions across all agents from the database
#[tauri::command]
pub fn list_recent_db_sessions_command(
    limit: i64,
    pool: State<'_, DbPool>,
) -> Result<Vec<AgentSession>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agent_sessions::table
        .order(agent_sessions::updated_at.desc())
        .limit(limit)
        .select(AgentSession::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

// ─── Run Commands ────────────────────────────────────────────────────────────

/// List all currently active runs from the database
#[tauri::command]
pub fn list_active_db_runs_command(pool: State<'_, DbPool>) -> Result<Vec<AgentRun>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agent_runs::table
        .filter(agent_runs::status.eq("running"))
        .select(AgentRun::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

/// Get detailed information about a specific run from the database
#[tauri::command]
pub fn get_db_run_details_command(run_id: String, pool: State<'_, DbPool>) -> Result<AgentRun, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agent_runs::table
        .find(run_id)
        .select(AgentRun::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

/// Cancel an active run in the database
#[tauri::command]
pub fn cancel_db_run_command(run_id: String, pool: State<'_, DbPool>) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    diesel::update(agent_runs::table.find(run_id))
        .set((
            agent_runs::status.eq("cancelled"),
            agent_runs::completed_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}
