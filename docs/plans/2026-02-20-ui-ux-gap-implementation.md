# UI/UX Gap Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement P0-P1 UI/UX improvements from OpenClaw gap analysis: agent backend, chat persistence, command palette, chat commands, and keyboard shortcuts.

**Architecture:** Backend-first approach with Diesel ORM models and Tauri IPC commands, then wire existing frontend stores to backend, finally add UI enhancements using existing cmdk components.

**Tech Stack:** Rust (Tauri 2, Diesel ORM), TypeScript (React 19, Zustand, cmdk, react-hotkeys-hook)

---

## Phase 1: Agent Backend Commands

### Task 1: Create Diesel Models for Agents

**Files:**
- Create: `src-tauri/src/database/models/agent.rs`
- Modify: `src-tauri/src/database/models/mod.rs`

**Step 1: Create agent model file**

Create `src-tauri/src/database/models/agent.rs`:

```rust
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::agents)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
    pub provider_id: String,
    pub temperature: f32,
    pub max_tokens: Option<i32>,
    pub tools_enabled: i32,
    pub memory_enabled: i32,
    pub workspace_path: Option<String>,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::agents)]
#[serde(rename_all = "camelCase")]
pub struct NewAgent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
    pub provider_id: String,
    pub temperature: f32,
    pub max_tokens: Option<i32>,
    pub tools_enabled: i32,
    pub memory_enabled: i32,
    pub workspace_path: Option<String>,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::agents)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgent {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub provider_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub tools_enabled: Option<i32>,
    pub memory_enabled: Option<i32>,
    pub workspace_path: Option<String>,
    pub is_active: Option<i32>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::agent_sessions)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub agent_id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::agent_runs)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: String,
    pub session_id: String,
    pub agent_id: String,
    pub parent_run_id: Option<String>,
    pub status: String,
    pub input_message: String,
    pub output_message: Option<String>,
    pub error_message: Option<String>,
    pub tokens_used: Option<i32>,
    pub duration_ms: Option<i32>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

// Request types for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentRequest {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
    pub provider_id: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub tools_enabled: Option<bool>,
    pub memory_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentRequest {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model_id: Option<String>,
    pub provider_id: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub tools_enabled: Option<bool>,
    pub memory_enabled: Option<bool>,
    pub workspace_path: Option<String>,
    pub is_active: Option<bool>,
}
```

**Step 2: Export models from mod.rs**

Add to `src-tauri/src/database/models/mod.rs`:

```rust
pub mod agent;
pub use agent::*;
```

**Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src-tauri/src/database/models/agent.rs src-tauri/src/database/models/mod.rs
git commit -m "feat(agents): add Diesel models for agents, sessions, and runs

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 2: Create Agent Commands

**Files:**
- Create: `src-tauri/src/commands/agents.rs`
- Modify: `src-tauri/src/commands/mod.rs`

**Step 1: Create agent commands file**

Create `src-tauri/src/commands/agents.rs`:

```rust
use crate::database::models::agent::{
    Agent, AgentSession, AgentRun, CreateAgentRequest, UpdateAgentRequest, NewAgent, UpdateAgent,
};
use crate::database::schema::{agents, agent_sessions, agent_runs};
use crate::DbPool;
use diesel::prelude::*;
use tauri::State;

// ─── Agent CRUD Commands ─────────────────────────────────────────────────────

#[tauri::command]
pub fn list_agents_command(pool: State<DbPool>) -> Result<Vec<Agent>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agents::table
        .filter(agents::is_active.eq(1))
        .select(Agent::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_agent_command(id: String, pool: State<DbPool>) -> Result<Agent, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agents::table
        .find(id)
        .select(Agent::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_agent_command(
    request: CreateAgentRequest,
    pool: State<DbPool>,
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
        tools_enabled: request.tools_enabled.map(|b| if b { 1 } else { 0 }).unwrap_or(1),
        memory_enabled: request.memory_enabled.map(|b| if b { 1 } else { 0 }).unwrap_or(1),
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

#[tauri::command]
pub fn update_agent_command(
    request: UpdateAgentRequest,
    pool: State<DbPool>,
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
        tools_enabled: request.tools_enabled.map(|b| if b { 1 } else { 0 }),
        memory_enabled: request.memory_enabled.map(|b| if b { 1 } else { 0 }),
        workspace_path: request.workspace_path,
        is_active: request.is_active.map(|b| if b { 1 } else { 0 }),
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

#[tauri::command]
pub fn delete_agent_command(id: String, pool: State<DbPool>) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    diesel::delete(agents::table.find(id))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Session Commands ────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_agent_sessions_command(
    agent_id: String,
    pool: State<DbPool>,
) -> Result<Vec<AgentSession>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agent_sessions::table
        .filter(agent_sessions::agent_id.eq(agent_id))
        .order(agent_sessions::created_at.desc())
        .select(AgentSession::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_recent_sessions_command(
    limit: i64,
    pool: State<DbPool>,
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

#[tauri::command]
pub fn list_active_runs_command(pool: State<DbPool>) -> Result<Vec<AgentRun>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agent_runs::table
        .filter(agent_runs::status.eq("running"))
        .select(AgentRun::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_run_details_command(run_id: String, pool: State<DbPool>) -> Result<AgentRun, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agent_runs::table
        .find(run_id)
        .select(AgentRun::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cancel_run_command(run_id: String, pool: State<DbPool>) -> Result<(), String> {
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
```

**Step 2: Export commands from mod.rs**

Add to `src-tauri/src/commands/mod.rs`:

```rust
pub mod agents;
```

**Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src-tauri/src/commands/agents.rs src-tauri/src/commands/mod.rs
git commit -m "feat(agents): add IPC commands for agent CRUD and monitoring

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 3: Register Agent Commands in lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add commands to invoke_handler**

Find the `invoke_handler` section in `src-tauri/src/lib.rs` and add the agent commands:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    crate::commands::agents::list_agents_command,
    crate::commands::agents::get_agent_command,
    crate::commands::agents::create_agent_command,
    crate::commands::agents::update_agent_command,
    crate::commands::agents::delete_agent_command,
    crate::commands::agents::list_agent_sessions_command,
    crate::commands::agents::list_recent_sessions_command,
    crate::commands::agents::list_active_runs_command,
    crate::commands::agents::get_run_details_command,
    crate::commands::agents::cancel_run_command,
])
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(agents): register agent commands in Tauri invoke handler

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 4: Wire Frontend Store to Backend

**Files:**
- Modify: `src/stores/agentConfigStore.ts`

**Step 1: Replace mock implementations with backend calls**

Update each function in `src/stores/agentConfigStore.ts`:

```typescript
// Replace loadAgents
loadAgents: async () => {
  set({ isLoadingAgents: true, agentsError: null });
  try {
    const agents = await invoke<AgentConfig[]>("list_agents_command");
    set({ agents, isLoadingAgents: false });
  } catch (error) {
    const message = extractErrorMessage(error);
    set({ agentsError: message, isLoadingAgents: false });
  }
},

// Replace createAgent
createAgent: async (request: CreateAgentRequest) => {
  set({ agentsError: null });
  try {
    const agent = await invoke<AgentConfig>("create_agent_command", { request });
    set((state) => ({
      agents: [...state.agents, agent],
    }));
    return agent;
  } catch (error) {
    const message = extractErrorMessage(error);
    set({ agentsError: message });
    throw new Error(message);
  }
},

// Replace updateAgent
updateAgent: async (request: UpdateAgentRequest) => {
  set({ agentsError: null });
  try {
    const updated = await invoke<AgentConfig>("update_agent_command", { request });
    set((state) => ({
      agents: state.agents.map((a) => (a.id === request.id ? updated : a)),
    }));
  } catch (error) {
    const message = extractErrorMessage(error);
    set({ agentsError: message });
    throw new Error(message);
  }
},

// Replace deleteAgent
deleteAgent: async (id: string) => {
  set({ agentsError: null });
  try {
    await invoke("delete_agent_command", { id });
    set((state) => ({
      agents: state.agents.filter((a) => a.id !== id),
      selectedAgentId: state.selectedAgentId === id ? null : state.selectedAgentId,
    }));
  } catch (error) {
    const message = extractErrorMessage(error);
    set({ agentsError: message });
    throw new Error(message);
  }
},

// Replace loadRecentSessions
loadRecentSessions: async (_agentId?: string) => {
  set({ isLoadingSessions: true, sessionsError: null });
  try {
    const sessions = await invoke<AgentSessionSummary[]>("list_recent_sessions_command", {
      limit: 50
    });
    set({ sessions, isLoadingSessions: false });
  } catch (error) {
    const message = extractErrorMessage(error);
    set({ sessionsError: message, isLoadingSessions: false });
  }
},

// Replace loadActiveRuns
loadActiveRuns: async () => {
  set({ runsError: null });
  try {
    const runs = await invoke<AgentRun[]>("list_active_runs_command");
    set({ activeRuns: runs });
  } catch (error) {
    const message = extractErrorMessage(error);
    set({ runsError: message });
  }
},

// Replace cancelRun
cancelRun: async (runId: string) => {
  set({ runsError: null });
  try {
    await invoke("cancel_run_command", { runId });
    set((state) => ({
      activeRuns: state.activeRuns.filter((r) => r.id !== runId),
    }));
  } catch (error) {
    const message = extractErrorMessage(error);
    set({ runsError: message });
    throw new Error(message);
  }
},
```

**Step 2: Verify TypeScript compilation**

Run: `bun run check`
Expected: No errors

**Step 3: Test manually**

Run: `bun run tauri dev`
- Navigate to Agents page
- Create a new agent
- Verify it persists after page refresh

**Step 4: Commit**

```bash
git add src/stores/agentConfigStore.ts
git commit -m "feat(agents): wire frontend store to backend commands

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Phase 2: Chat Session Persistence

### Task 5: Create Chat Messages Migration

**Files:**
- Create: `src-tauri/migrations/2026-02-20-220000_add_chat_messages/up.sql`
- Create: `src-tauri/migrations/2026-02-20-220000_add_chat_messages/down.sql`

**Step 1: Create migration directory**

Run: `cd src-tauri && diesel migration generate add_chat_messages`

**Step 2: Write up.sql**

```sql
-- Add messages table for chat session persistence
CREATE TABLE chat_messages (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_chat_messages_session ON chat_messages(session_id);
CREATE INDEX idx_chat_messages_created ON chat_messages(created_at);
```

**Step 3: Write down.sql**

```sql
DROP INDEX IF EXISTS idx_chat_messages_created;
DROP INDEX IF EXISTS idx_chat_messages_session;
DROP TABLE IF EXISTS chat_messages;
```

**Step 4: Run migration**

Run: `cd src-tauri && diesel migration run`
Expected: Migration successful

**Step 5: Regenerate schema**

Run: `cd src-tauri && diesel print-schema > src/database/schema.rs`

**Step 6: Commit**

```bash
git add src-tauri/migrations/2026-02-20-220000_add_chat_messages/ src-tauri/src/database/schema.rs
git commit -m "feat(chat): add chat_messages table for message persistence

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 6: Create Chat Session Models

**Files:**
- Modify: `src-tauri/src/database/models/mod.rs`
- Modify: `src-tauri/src/database/models/agent.rs` (add chat models here or create separate file)

**Step 1: Add chat models to models file**

Add to `src-tauri/src/database/models/agent.rs` or create new file:

```rust
// Chat Session Models

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::chat_sessions)]
#[serde(rename_all = "camelCase")]
pub struct ChatSession {
    pub id: String,
    pub session_key: String,
    pub agent: String,
    pub scope: String,
    pub channel: String,
    pub peer: String,
    pub created_at: String,
    pub updated_at: String,
    pub compaction_summary: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = crate::database::schema::chat_messages)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::database::schema::chat_messages)]
pub struct NewChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMessageRequest {
    pub session_id: String,
    pub role: String,
    pub content: String,
}
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 3: Commit**

```bash
git add src-tauri/src/database/models/
git commit -m "feat(chat): add Diesel models for chat sessions and messages

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 7: Create Chat Session Commands

**Files:**
- Create: `src-tauri/src/commands/chat_sessions.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create chat sessions commands**

Create `src-tauri/src/commands/chat_sessions.rs`:

```rust
use crate::database::models::agent::{
    ChatSession, ChatMessage, NewChatMessage, CreateSessionRequest, SaveMessageRequest,
};
use crate::database::schema::{chat_sessions, chat_messages};
use crate::DbPool;
use diesel::prelude::*;
use tauri::State;

#[tauri::command]
pub fn list_chat_sessions_command(pool: State<DbPool>) -> Result<Vec<ChatSession>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    chat_sessions::table
        .order(chat_sessions::updated_at.desc())
        .limit(50)
        .select(ChatSession::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_chat_session_command(id: String, pool: State<DbPool>) -> Result<ChatSession, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    chat_sessions::table
        .find(id)
        .select(ChatSession::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_chat_session_command(
    request: CreateSessionRequest,
    pool: State<DbPool>,
) -> Result<ChatSession, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    let session_key = format!("chat:{}:{}", request.provider_id, request.model_id);

    diesel::insert_into(chat_sessions::table)
        .values((
            chat_sessions::id.eq(&id),
            chat_sessions::session_key.eq(&session_key),
            chat_sessions::agent.eq("main"),
            chat_sessions::scope.eq("dm"),
            chat_sessions::channel.eq("tauri"),
            chat_sessions::peer.eq("user"),
            chat_sessions::created_at.eq(&now),
            chat_sessions::updated_at.eq(&now),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    chat_sessions::table
        .find(&id)
        .select(ChatSession::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_chat_session_command(id: String, pool: State<DbPool>) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    diesel::delete(chat_sessions::table.find(id))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn load_messages_command(
    session_id: String,
    pool: State<DbPool>,
) -> Result<Vec<ChatMessage>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    chat_messages::table
        .filter(chat_messages::session_id.eq(&session_id))
        .order(chat_messages::created_at.asc())
        .select(ChatMessage::as_select())
        .load(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_message_command(
    request: SaveMessageRequest,
    pool: State<DbPool>,
) -> Result<ChatMessage, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    let new_message = NewChatMessage {
        id: id.clone(),
        session_id: request.session_id,
        role: request.role,
        content: request.content,
        created_at: created_at.clone(),
    };

    diesel::insert_into(chat_messages::table)
        .values(&new_message)
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    // Update session updated_at
    diesel::update(chat_sessions::table.filter(
        chat_sessions::id.eq(&new_message.session_id)
    ))
    .set(chat_sessions::updated_at.eq(&created_at))
    .execute(&mut conn)
    .map_err(|e| e.to_string())?;

    chat_messages::table
        .find(id)
        .select(ChatMessage::as_select())
        .first(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_session_messages_command(
    session_id: String,
    pool: State<DbPool>,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    diesel::delete(chat_messages::table.filter(chat_messages::session_id.eq(&session_id)))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

**Step 2: Export from mod.rs**

Add to `src-tauri/src/commands/mod.rs`:

```rust
pub mod chat_sessions;
```

**Step 3: Register in lib.rs**

Add to invoke_handler:

```rust
crate::commands::chat_sessions::list_chat_sessions_command,
crate::commands::chat_sessions::get_chat_session_command,
crate::commands::chat_sessions::create_chat_session_command,
crate::commands::chat_sessions::delete_chat_session_command,
crate::commands::chat_sessions::load_messages_command,
crate::commands::chat_sessions::save_message_command,
crate::commands::chat_sessions::clear_session_messages_command,
```

**Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: No errors

**Step 5: Commit**

```bash
git add src-tauri/src/commands/
git commit -m "feat(chat): add IPC commands for chat session persistence

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 8: Create Chat Session Store

**Files:**
- Create: `src/stores/chatSessionStore.ts`

**Step 1: Create the store**

```typescript
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface ChatSession {
  id: string;
  sessionKey: string;
  agent: string;
  scope: string;
  channel: string;
  peer: string;
  createdAt: string;
  updatedAt: string;
  compactionSummary?: string;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system";
  content: string;
  createdAt: string;
}

interface ChatSessionState {
  sessions: ChatSession[];
  activeSessionId: string | null;
  messages: Map<string, ChatMessage[]>;
  isLoading: boolean;
  error: string | null;

  loadSessions: () => Promise<void>;
  createSession: (providerId: string, modelId: string) => Promise<string>;
  loadSession: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
  loadMessages: (sessionId: string) => Promise<void>;
  saveMessage: (role: "user" | "assistant" | "system", content: string) => Promise<void>;
  clearMessages: () => Promise<void>;
}

export const useChatSessionStore = create<ChatSessionState>((set, get) => ({
  sessions: [],
  activeSessionId: null,
  messages: new Map(),
  isLoading: false,
  error: null,

  loadSessions: async () => {
    set({ isLoading: true, error: null });
    try {
      const sessions = await invoke<ChatSession[]>("list_chat_sessions_command");
      set({ sessions, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  createSession: async (providerId: string, modelId: string) => {
    try {
      const session = await invoke<ChatSession>("create_chat_session_command", {
        request: { providerId, modelId },
      });
      set((state) => ({
        sessions: [session, ...state.sessions],
        activeSessionId: session.id,
        messages: new Map(state.messages).set(session.id, []),
      }));
      return session.id;
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
  },

  loadSession: async (sessionId: string) => {
    set({ activeSessionId: sessionId });
    await get().loadMessages(sessionId);
  },

  deleteSession: async (sessionId: string) => {
    try {
      await invoke("delete_chat_session_command", { id: sessionId });
      set((state) => {
        const newMessages = new Map(state.messages);
        newMessages.delete(sessionId);
        return {
          sessions: state.sessions.filter((s) => s.id !== sessionId),
          activeSessionId:
            state.activeSessionId === sessionId ? null : state.activeSessionId,
          messages: newMessages,
        };
      });
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
  },

  loadMessages: async (sessionId: string) => {
    try {
      const messages = await invoke<ChatMessage[]>("load_messages_command", {
        sessionId,
      });
      set((state) => {
        const newMessages = new Map(state.messages);
        newMessages.set(sessionId, messages);
        return { messages: newMessages };
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  saveMessage: async (role: "user" | "assistant" | "system", content: string) => {
    const { activeSessionId, messages } = get();
    if (!activeSessionId) return;

    try {
      const message = await invoke<ChatMessage>("save_message_command", {
        request: { sessionId: activeSessionId, role, content },
      });
      set((state) => {
        const newMessages = new Map(state.messages);
        const existing = newMessages.get(activeSessionId) || [];
        newMessages.set(activeSessionId, [...existing, message]);
        return { messages: newMessages };
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  clearMessages: async () => {
    const { activeSessionId } = get();
    if (!activeSessionId) return;

    try {
      await invoke("clear_session_messages_command", { sessionId: activeSessionId });
      set((state) => {
        const newMessages = new Map(state.messages);
        newMessages.set(activeSessionId, []);
        return { messages: newMessages };
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },
}));
```

**Step 2: Verify TypeScript compilation**

Run: `bun run check`
Expected: No errors

**Step 3: Commit**

```bash
git add src/stores/chatSessionStore.ts
git commit -m "feat(chat): add Zustand store for chat session management

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 9: Wire Chat Page to Session Store

**Files:**
- Modify: `src/routes/chat.tsx`

**Step 1: Import and use session store**

Add imports and integrate with existing chat state:

```typescript
import { useChatSessionStore } from "@/stores/chatSessionStore";

// In ChatPage component:
const {
  sessions,
  activeSessionId,
  loadSessions,
  createSession,
  loadSession,
  saveMessage,
  clearMessages,
} = useChatSessionStore();

// Load sessions on mount
useEffect(() => {
  loadSessions();
}, [loadSessions]);

// Auto-save messages with debounce
useEffect(() => {
  if (messages.length === 0 || !activeSessionId) return;

  const timeout = setTimeout(() => {
    // Save last message if not already saved
    const lastMessage = messages[messages.length - 1];
    saveMessage(lastMessage.role, lastMessage.content);
  }, 1000);

  return () => clearTimeout(timeout);
}, [messages, activeSessionId, saveMessage]);
```

**Step 2: Add session list UI (optional sidebar)**

Add a session list component or dropdown for switching sessions.

**Step 3: Test manually**

Run: `bun run tauri dev`
- Send messages in chat
- Refresh page
- Verify messages persist

**Step 4: Commit**

```bash
git add src/routes/chat.tsx
git commit -m "feat(chat): wire chat page to session persistence store

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Phase 3: Command Palette

### Task 10: Install react-hotkeys-hook (if needed)

**Files:**
- Modify: `package.json`

**Step 1: Install dependency**

Run: `bun add react-hotkeys-hook`

**Step 2: Commit**

```bash
git add package.json bun.lockb
git commit -m "chore: add react-hotkeys-hook for keyboard shortcuts

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 11: Create CommandPalette Component

**Files:**
- Create: `src/components/layout/CommandPalette.tsx`
- Create: `src/stores/commandPaletteStore.ts`

**Step 1: Create store for command palette state**

Create `src/stores/commandPaletteStore.ts`:

```typescript
import { create } from "zustand";

interface CommandPaletteState {
  isOpen: boolean;
  open: () => void;
  close: () => void;
  toggle: () => void;
}

export const useCommandPaletteStore = create<CommandPaletteState>((set) => ({
  isOpen: false,
  open: () => set({ isOpen: true }),
  close: () => set({ isOpen: false }),
  toggle: () => set((state) => ({ isOpen: !state.isOpen })),
}));
```

**Step 2: Create CommandPalette component**

Create `src/components/layout/CommandPalette.tsx`:

```tsx
import { useEffect } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  CommandDialog,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandShortcut,
  CommandSeparator,
} from "@/components/ui/command";
import { useCommandPaletteStore } from "@/stores/commandPaletteStore";
import { useChatSessionStore } from "@/stores/chatSessionStore";

import { Bot, MessageSquare, Settings, Plus, Home, Clock, Zap } from "@/lib/icons";

export function CommandPalette() {
  const { isOpen, close, toggle } = useCommandPaletteStore();
  const navigate = useNavigate();
  const { createSession, sessions } = useChatSessionStore();

  // Keyboard shortcut
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        toggle();
      }
    };
    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, [toggle]);

  const handleNewChat = async () => {
    await createSession("default", "default");
    navigate({ to: "/chat" });
    close();
  };

  const handleNavigate = (to: string) => {
    navigate({ to });
    close();
  };

  const handleSelectSession = async (sessionId: string) => {
    const { loadSession } = useChatSessionStore.getState();
    await loadSession(sessionId);
    navigate({ to: "/chat" });
    close();
  };

  return (
    <CommandDialog open={isOpen} onOpenChange={(open) => !open && close()}>
      <CommandInput placeholder="Search commands and navigate..." />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        <CommandGroup heading="Navigation">
          <CommandItem onSelect={() => handleNavigate("/chat")}>
            <MessageSquare className="mr-2 h-4 w-4" />
            Chat
            <CommandShortcut>⌘/</CommandShortcut>
          </CommandItem>
          <CommandItem onSelect={() => handleNavigate("/agents")}>
            <Bot className="mr-2 h-4 w-4" />
            Agents
          </CommandItem>
          <CommandItem onSelect={() => handleNavigate("/settings")}>
            <Settings className="mr-2 h-4 w-4" />
            Settings
            <CommandShortcut>⌘,</CommandShortcut>
          </CommandItem>
          <CommandItem onSelect={() => handleNavigate("/")}>
            <Home className="mr-2 h-4 w-4" />
            Home
          </CommandItem>
        </CommandGroup>

        <CommandSeparator />

        <CommandGroup heading="Actions">
          <CommandItem onSelect={handleNewChat}>
            <Plus className="mr-2 h-4 w-4" />
            New Chat
            <CommandShortcut>⌘N</CommandShortcut>
          </CommandItem>
          <CommandItem onSelect={() => handleNavigate("/agents")}>
            <Zap className="mr-2 h-4 w-4" />
            New Agent
          </CommandItem>
        </CommandGroup>

        {sessions.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading="Recent Chats">
              {sessions.slice(0, 5).map((session) => (
                <CommandItem
                  key={session.id}
                  onSelect={() => handleSelectSession(session.id)}
                >
                  <Clock className="mr-2 h-4 w-4" />
                  {session.sessionKey}
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
      </CommandList>
    </CommandDialog>
  );
}
```

**Step 3: Verify TypeScript compilation**

Run: `bun run check`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/layout/CommandPalette.tsx src/stores/commandPaletteStore.ts
git commit -m "feat(ui): add command palette component with Cmd+K shortcut

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 12: Integrate CommandPalette in Root Layout

**Files:**
- Modify: `src/routes/__root.tsx` (or `src/App.tsx`)

**Step 1: Add CommandPalette to layout**

```tsx
import { CommandPalette } from "@/components/layout/CommandPalette";

// In the root layout component, add:
<CommandPalette />
```

**Step 2: Test manually**

Run: `bun run tauri dev`
- Press Cmd+K / Ctrl+K
- Verify palette opens
- Navigate to different pages

**Step 3: Commit**

```bash
git add src/routes/__root.tsx
git commit -m "feat(ui): integrate CommandPalette in root layout

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Phase 4: Chat Commands

### Task 13: Create Chat Commands Library

**Files:**
- Create: `src/lib/chat-commands.ts`

**Step 1: Create the commands library**

```typescript
export interface ChatContext {
  sessionId: string | null;
  model: string;
  providerId: string;
  messageCount: number;
  startNewSession: () => Promise<void>;
  clearMessages: () => void;
  exportConversation: () => void;
}

export interface ChatCommand {
  description: string;
  execute: (ctx: ChatContext) => Promise<string>;
}

export const CHAT_COMMANDS: Record<string, ChatCommand> = {
  "/status": {
    description: "Show current session status",
    execute: async (ctx) => {
      const lines = [
        `Model: ${ctx.model}`,
        `Provider: ${ctx.providerId}`,
        `Messages: ${ctx.messageCount}`,
        `Session: ${ctx.sessionId ?? "New (unsaved)"}`,
      ];
      return lines.join("\n");
    },
  },
  "/new": {
    description: "Start a new conversation",
    execute: async (ctx) => {
      await ctx.startNewSession();
      return "Started new conversation";
    },
  },
  "/clear": {
    description: "Clear current conversation",
    execute: async (ctx) => {
      ctx.clearMessages();
      return "Conversation cleared";
    },
  },
  "/export": {
    description: "Export conversation to file",
    execute: async (ctx) => {
      ctx.exportConversation();
      return "Conversation exported";
    },
  },
  "/help": {
    description: "Show available commands",
    execute: async () => {
      const commands = Object.entries(CHAT_COMMANDS)
        .map(([cmd, c]) => `${cmd} - ${c.description}`)
        .join("\n");
      return `Available commands:\n${commands}`;
    },
  },
};

export function parseCommand(input: string): { command: string; args: string[] } | null {
  if (!input.startsWith("/")) return null;
  const parts = input.trim().split(/\s+/);
  return { command: parts[0].toLowerCase(), args: parts.slice(1) };
}

export function isCommand(input: string): boolean {
  return input.trim().startsWith("/");
}

export function getCommandSuggestions(input: string): string[] {
  const trimmed = input.trim().toLowerCase();
  if (!trimmed.startsWith("/")) return [];

  return Object.keys(CHAT_COMMANDS).filter((cmd) =>
    cmd.startsWith(trimmed)
  );
}
```

**Step 2: Commit**

```bash
git add src/lib/chat-commands.ts
git commit -m "feat(chat): add slash command parser and handlers

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 14: Wire Chat Commands to Chat Page

**Files:**
- Modify: `src/routes/chat.tsx`

**Step 1: Import and use chat commands**

```typescript
import { CHAT_COMMANDS, parseCommand, isCommand } from "@/lib/chat-commands";

// Add a system message type
interface MessageType {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  isStreaming?: boolean;
}

// In handleSubmit function:
const handleSubmit = async (message: string) => {
  // Check for slash command
  if (isCommand(message)) {
    const parsed = parseCommand(message);
    if (parsed && CHAT_COMMANDS[parsed.command]) {
      const result = await CHAT_COMMANDS[parsed.command].execute({
        sessionId: activeSessionId,
        model: selectedModel,
        providerId: selectedProviderId,
        messageCount: messages.length,
        startNewSession: handleNewSession,
        clearMessages: () => setMessages([]),
        exportConversation: handleExport,
      });
      // Add as system message
      setMessages((prev) => [
        ...prev,
        { id: nanoid(), role: "system", content: result },
      ]);
    } else {
      setMessages((prev) => [
        ...prev,
        {
          id: nanoid(),
          role: "system",
          content: `Unknown command: ${parsed?.command}. Type /help for available commands.`,
        },
      ]);
    }
    return;
  }

  // Normal message handling...
};

// Add export handler
const handleExport = () => {
  const content = messages
    .map((m) => `[${m.role}]: ${m.content}`)
    .join("\n\n");
  const blob = new Blob([content], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `chat-${new Date().toISOString().split("T")[0]}.txt`;
  a.click();
  URL.revokeObjectURL(url);
};
```

**Step 2: Test manually**

Run: `bun run tauri dev`
- Type `/help` in chat
- Type `/status` to see session info
- Type `/clear` to clear messages

**Step 3: Commit**

```bash
git add src/routes/chat.tsx
git commit -m "feat(chat): integrate slash commands in chat input

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Phase 5: Keyboard Shortcuts

### Task 15: Create Global Keyboard Shortcuts Hook

**Files:**
- Create: `src/hooks/useKeyboardShortcuts.ts`

**Step 1: Create the hook**

```typescript
import { useHotkeys } from "react-hotkeys-hook";
import { useNavigate } from "@tanstack/react-router";
import { useChatSessionStore } from "@/stores/chatSessionStore";
import { useCommandPaletteStore } from "@/stores/commandPaletteStore";

export function useKeyboardShortcuts() {
  const navigate = useNavigate();
  const createSession = useChatSessionStore((s) => s.createSession);
  const openCommandPalette = useCommandPaletteStore((s) => s.open);

  // New Chat: Cmd/Ctrl + N
  useHotkeys(
    "meta+n, ctrl+n",
    async (e) => {
      e.preventDefault();
      await createSession("default", "default");
      navigate({ to: "/chat" });
    },
    { enableOnFormTags: false }
  );

  // Settings: Cmd/Ctrl + ,
  useHotkeys(
    "meta+comma, ctrl+comma",
    (e) => {
      e.preventDefault();
      navigate({ to: "/settings" });
    },
    { enableOnFormTags: false }
  );

  // Go to Chat: Cmd/Ctrl + /
  useHotkeys(
    "meta+slash, ctrl+slash",
    (e) => {
      e.preventDefault();
      navigate({ to: "/chat" });
    },
    { enableOnFormTags: false }
  );

  // Go to Home: Cmd/Ctrl + H
  useHotkeys(
    "meta+h, ctrl+h",
    (e) => {
      e.preventDefault();
      navigate({ to: "/" });
    },
    { enableOnFormTags: false }
  );
}
```

**Step 2: Commit**

```bash
git add src/hooks/useKeyboardShortcuts.ts
git commit -m "feat(ui): add global keyboard shortcuts hook

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

### Task 16: Integrate Keyboard Shortcuts in Root Layout

**Files:**
- Modify: `src/routes/__root.tsx`

**Step 1: Call the hook in root layout**

```tsx
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";

// In the root component:
useKeyboardShortcuts();
```

**Step 2: Test all shortcuts**

Run: `bun run tauri dev`
- Cmd+N: New chat
- Cmd+,: Settings
- Cmd+/: Chat
- Cmd+K: Command palette

**Step 3: Commit**

```bash
git add src/routes/__root.tsx
git commit -m "feat(ui): integrate global keyboard shortcuts in root layout

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Final Verification

### Task 17: Run Full Test Suite

**Step 1: Run backend tests**

Run: `cd src-tauri && cargo test --lib`
Expected: All tests pass

**Step 2: Run frontend tests**

Run: `bun run test`
Expected: All tests pass

**Step 3: Run linting**

Run: `bunx ultracite check && cd src-tauri && cargo clippy`
Expected: No errors

**Step 4: Manual end-to-end test**

1. Create a new agent → verify it persists
2. Delete an agent → verify it's removed
3. Send chat messages → verify persistence after refresh
4. Use Cmd+K → verify command palette opens
5. Type `/help` in chat → verify commands list shows
6. Use Cmd+N → verify new chat starts

**Step 5: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: address test failures and lint issues

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| 1. Agent Backend | 1-4 | 2-3 hours |
| 2. Chat Persistence | 5-9 | 2-3 hours |
| 3. Command Palette | 10-12 | 1-2 hours |
| 4. Chat Commands | 13-14 | 0.5-1 hour |
| 5. Keyboard Shortcuts | 15-16 | 0.5 hour |
| 6. Verification | 17 | 0.5 hour |

**Total: 6-10 hours**
