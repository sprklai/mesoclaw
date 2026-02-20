# UI/UX Gap Implementation Design

> **Created:** 2026-02-20
> **Status:** Approved
> **Source:** `docs/mesoclaw_gapopenclawUI.md` - Gap analysis against OpenClaw

## Overview

This design implements P0 (Critical) and P1 (High) UI/UX improvements identified in the gap analysis, following a backend-first sequential approach.

## Scope

| Priority | Feature | Effort |
|----------|---------|--------|
| P0 | Agent System Backend Integration | 2-3 days |
| P0 | Chat Session Persistence | 1-2 days |
| P1 | Command Palette (Cmd+K) | 1 day |
| P1 | Chat Commands (/status, /new, etc.) | 0.5 day |
| P1 | Keyboard Shortcuts | 0.5 day |

**Total Estimated Effort:** 5-7 days

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Frontend (React)                          │
├─────────────────────────────────────────────────────────────────┤
│  UI Layer                                                        │
│  ├─ agents.tsx          ← Wire to backend (no changes needed)   │
│  ├─ chat.tsx            ← Add session persistence + commands    │
│  ├─ CommandPalette.tsx  ← NEW: Global Cmd+K overlay             │
│  └─ chat-commands.ts    ← NEW: Slash command parser             │
├─────────────────────────────────────────────────────────────────┤
│  Store Layer                                                     │
│  ├─ agentConfigStore.ts ← Replace mock with invoke() calls      │
│  └─ chatSessionStore.ts ← NEW: Session state management         │
└───────────────────────────────┬─────────────────────────────────┘
                                │ Tauri IPC
┌───────────────────────────────▼─────────────────────────────────┐
│                        Backend (Rust)                            │
├─────────────────────────────────────────────────────────────────┤
│  Commands Layer                                                  │
│  ├─ agents.rs           ← NEW: Agent CRUD commands              │
│  └─ chat_sessions.rs    ← NEW: Session/message persistence      │
├─────────────────────────────────────────────────────────────────┤
│  Database Layer                                                  │
│  ├─ agents              ← Schema exists ✓                       │
│  ├─ agent_sessions      ← Schema exists ✓                       │
│  ├─ agent_runs          ← Schema exists ✓                       │
│  ├─ chat_sessions       ← Schema exists ✓                       │
│  └─ chat_messages       ← NEW: Migration needed                │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Agent Backend Commands

### Database Schema (Already Exists)

The following tables are already defined in `src-tauri/src/database/schema.rs`:

- `agents` - Agent configurations
- `agent_sessions` - Session groupings
- `agent_runs` - Individual execution instances

### New Files

#### `src-tauri/src/database/models/agent.rs`

Diesel models mapping to existing schema:

```rust
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
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
#[serde(rename_all = "camelCase")]
#[diesel(table_name = crate::database::schema::agents)]
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
#[serde(rename_all = "camelCase")]
#[diesel(table_name = crate::database::schema::agents)]
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
    pub updated_at: Option<String>,
}
```

#### `src-tauri/src/commands/agents.rs`

Agent CRUD and monitoring commands:

```rust
use crate::database::models::agent::{Agent, NewAgent, UpdateAgent};
use crate::database::schema::agents::dsl::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
pub fn list_agents_command(pool: State<DbPool>) -> Result<Vec<Agent>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agents
        .filter(is_active.eq(1))
        .load::<Agent>(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_agent_command(agent_id: String, pool: State<DbPool>) -> Result<Agent, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    agents
        .find(agent_id)
        .first::<Agent>(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_agent_command(request: CreateAgentRequest, pool: State<DbPool>) -> Result<Agent, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let new_agent = NewAgent {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        system_prompt: request.system_prompt,
        model_id: request.model_id,
        provider_id: request.provider_id,
        temperature: request.temperature.unwrap_or(0.7),
        max_tokens: request.max_tokens,
        tools_enabled: request.tools_enabled.unwrap_or(1),
        memory_enabled: request.memory_enabled.unwrap_or(1),
        workspace_path: None,
        is_active: 1,
        created_at: now.clone(),
        updated_at: now,
    };
    diesel::insert_into(agents)
        .values(&new_agent)
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    Ok(new_agent.into())
}

#[tauri::command]
pub fn update_agent_command(agent_id: String, request: UpdateAgentRequest, pool: State<DbPool>) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let update = UpdateAgent {
        name: request.name,
        description: request.description,
        system_prompt: request.system_prompt,
        model_id: request.model_id,
        provider_id: request.provider_id,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        tools_enabled: request.tools_enabled,
        memory_enabled: request.memory_enabled,
        workspace_path: request.workspace_path,
        is_active: request.is_active,
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    diesel::update(agents.find(agent_id))
        .set(&update)
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_agent_command(agent_id: String, pool: State<DbPool>) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    diesel::delete(agents.find(agent_id))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

### Frontend Store Updates

#### `src/stores/agentConfigStore.ts`

Replace mock implementations with backend calls:

```typescript
// Before (mock)
loadAgents: async () => {
  const agents: AgentConfig[] = [];
  set({ agents, isLoadingAgents: false });
}

// After (wired)
loadAgents: async () => {
  set({ isLoadingAgents: true, agentsError: null });
  try {
    const agents = await invoke<AgentConfig[]>("list_agents_command");
    set({ agents, isLoadingAgents: false });
  } catch (error) {
    const message = extractErrorMessage(error);
    set({ agentsError: message, isLoadingAgents: false });
  }
}
```

---

## Phase 2: Chat Session Persistence

### New Migration

#### `src-tauri/migrations/YYYY-MM-DD-HHMMSS_add_chat_messages/up.sql`

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

#### `src-tauri/migrations/YYYY-MM-DD-HHMMSS_add_chat_messages/down.sql`

```sql
DROP INDEX IF EXISTS idx_chat_messages_created;
DROP INDEX IF EXISTS idx_chat_messages_session;
DROP TABLE IF EXISTS chat_messages;
```

### New Files

#### `src-tauri/src/database/models/chat_session.rs`

```rust
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
```

#### `src-tauri/src/commands/chat_sessions.rs`

```rust
use crate::database::models::chat_session::{ChatSession, ChatMessage};
use crate::database::schema::{chat_sessions, chat_messages};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
pub fn list_sessions_command(pool: State<DbPool>) -> Result<Vec<ChatSession>, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    chat_sessions::table
        .order(chat_sessions::updated_at.desc())
        .limit(50)
        .load::<ChatSession>(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_session_command(
    provider_id: String,
    model_id: String,
    pool: State<DbPool>,
) -> Result<ChatSession, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    let session_key = format!("chat:{}:{}", provider_id, model_id);

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
        .first::<ChatSession>(&mut conn)
        .map_err(|e| e.to_string())
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
        .load::<ChatMessage>(&mut conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_message_command(
    session_id: String,
    role: String,
    content: String,
    pool: State<DbPool>,
) -> Result<ChatMessage, String> {
    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    diesel::insert_into(chat_messages::table)
        .values((
            chat_messages::id.eq(&id),
            chat_messages::session_id.eq(&session_id),
            chat_messages::role.eq(&role),
            chat_messages::content.eq(&content),
            chat_messages::created_at.eq(&created_at),
        ))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    // Update session updated_at
    diesel::update(chat_sessions::table.find(&session_id))
        .set(chat_sessions::updated_at.eq(&created_at))
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;

    chat_messages::table
        .find(&id)
        .first::<ChatMessage>(&mut conn)
        .map_err(|e| e.to_string())
}
```

### New Frontend Store

#### `src/stores/chatSessionStore.ts`

```typescript
import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

interface ChatSession {
  id: string;
  sessionKey: string;
  createdAt: string;
  updatedAt: string;
}

interface ChatMessage {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system";
  content: string;
  createdAt: string;
}

interface ChatSessionState {
  sessions: ChatSession[];
  activeSessionId: string | null;
  isLoading: boolean;
  error: string | null;

  loadSessions: () => Promise<void>;
  createSession: (providerId: string, modelId: string) => Promise<string>;
  loadSession: (sessionId: string) => Promise<void>;
  deleteSession: (sessionId: string) => Promise<void>;
}

export const useChatSessionStore = create<ChatSessionState>((set, get) => ({
  sessions: [],
  activeSessionId: null,
  isLoading: false,
  error: null,

  loadSessions: async () => {
    set({ isLoading: true, error: null });
    try {
      const sessions = await invoke<ChatSession[]>("list_sessions_command");
      set({ sessions, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  createSession: async (providerId: string, modelId: string) => {
    const session = await invoke<ChatSession>("create_session_command", {
      providerId,
      modelId,
    });
    set((state) => ({
      sessions: [session, ...state.sessions],
      activeSessionId: session.id,
    }));
    return session.id;
  },

  loadSession: async (sessionId: string) => {
    set({ activeSessionId: sessionId });
  },

  deleteSession: async (sessionId: string) => {
    await invoke("delete_session_command", { sessionId });
    set((state) => ({
      sessions: state.sessions.filter((s) => s.id !== sessionId),
      activeSessionId:
        state.activeSessionId === sessionId ? null : state.activeSessionId,
    }));
  },
}));
```

---

## Phase 3: Command Palette

### New File: `src/components/layout/CommandPalette.tsx`

```tsx
import { useState, useEffect } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  CommandDialog,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandShortcut,
} from "@/components/ui/command";
import { useChatSessionStore } from "@/stores/chatSessionStore";

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();
  const { createSession } = useChatSessionStore();

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((o) => !o);
      }
    };
    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, []);

  const handleNewChat = async () => {
    await createSession("default", "default");
    navigate("/chat");
    setOpen(false);
  };

  return (
    <CommandDialog open={open} onOpenChange={setOpen}>
      <CommandInput placeholder="Search commands..." />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>
        <CommandGroup heading="Navigation">
          <CommandItem onSelect={() => { navigate("/chat"); setOpen(false); }}>
            Chat
            <CommandShortcut>⌘/</CommandShortcut>
          </CommandItem>
          <CommandItem onSelect={() => { navigate("/agents"); setOpen(false); }}>
            Agents
          </CommandItem>
          <CommandItem onSelect={() => { navigate("/settings"); setOpen(false); }}>
            Settings
            <CommandShortcut>⌘,</CommandShortcut>
          </CommandItem>
        </CommandGroup>
        <CommandGroup heading="Actions">
          <CommandItem onSelect={handleNewChat}>
            New Chat
            <CommandShortcut>⌘N</CommandShortcut>
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
```

### Integration

Add to root layout (`src/routes/__root.tsx` or `src/App.tsx`):

```tsx
import { CommandPalette } from "@/components/layout/CommandPalette";

export function RootLayout() {
  return (
    <>
      {/* ... existing layout ... */}
      <CommandPalette />
    </>
  );
}
```

---

## Phase 4: Chat Commands System

### New File: `src/lib/chat-commands.ts`

```typescript
interface ChatCommand {
  description: string;
  execute: (ctx: ChatContext) => Promise<string>;
}

interface ChatContext {
  sessionId: string | null;
  model: string;
  providerId: string;
  messageCount: number;
  startNewSession: () => Promise<void>;
  clearMessages: () => void;
  exportConversation: () => void;
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
```

### Integration in `chat.tsx`

```tsx
import { CHAT_COMMANDS, parseCommand, isCommand } from "@/lib/chat-commands";

// In handleSubmit function:
const handleSubmit = async (message: string) => {
  if (isCommand(message)) {
    const parsed = parseCommand(message);
    if (parsed && CHAT_COMMANDS[parsed.command]) {
      const result = await CHAT_COMMANDS[parsed.command].execute({
        sessionId,
        model: selectedModel,
        providerId,
        messageCount: messages.length,
        startNewSession: handleNewSession,
        clearMessages: () => setMessages([]),
        exportConversation: handleExport,
      });
      addSystemMessage(result);
    } else {
      addSystemMessage(`Unknown command: ${parsed?.command}. Type /help for available commands.`);
    }
    return;
  }
  // Normal message handling...
};
```

---

## Phase 5: Keyboard Shortcuts

### New File: `src/hooks/useKeyboardShortcuts.ts`

```typescript
import { useHotkeys } from "react-hotkeys-hook";
import { useNavigate } from "@tanstack/react-router";
import { useChatSessionStore } from "@/stores/chatSessionStore";

export function useKeyboardShortcuts() {
  const navigate = useNavigate();
  const { createSession } = useChatSessionStore();

  // New Chat
  useHotkeys(
    "meta+n, ctrl+n",
    async () => {
      await createSession("default", "default");
      navigate("/chat");
    },
    { preventDefault: true }
  );

  // Settings
  useHotkeys(
    "meta+comma, ctrl+comma",
    () => navigate("/settings"),
    { preventDefault: true }
  );

  // Go to Chat
  useHotkeys(
    "meta+slash, ctrl+slash",
    () => navigate("/chat"),
    { preventDefault: true }
  );
}
```

### Shortcuts Reference

| Action | Mac | Windows/Linux |
|--------|-----|---------------|
| Command Palette | `Cmd+K` | `Ctrl+K` |
| New Chat | `Cmd+N` | `Ctrl+N` |
| Settings | `Cmd+,` | `Ctrl+,` |
| Go to Chat | `Cmd+/` | `Ctrl+/` |

---

## Files Summary

### New Files to Create

| File | Purpose |
|------|---------|
| `src-tauri/migrations/..._add_chat_messages/up.sql` | Messages table migration |
| `src-tauri/migrations/..._add_chat_messages/down.sql` | Rollback migration |
| `src-tauri/src/database/models/agent.rs` | Diesel models for agents |
| `src-tauri/src/database/models/chat_session.rs` | Diesel models for chat |
| `src-tauri/src/commands/agents.rs` | Agent CRUD commands |
| `src-tauri/src/commands/chat_sessions.rs` | Session persistence commands |
| `src/stores/chatSessionStore.ts` | Session state management |
| `src/components/layout/CommandPalette.tsx` | Global Cmd+K overlay |
| `src/lib/chat-commands.ts` | Slash command parser |
| `src/hooks/useKeyboardShortcuts.ts` | Global keyboard shortcuts |

### Files to Modify

| File | Changes |
|------|---------|
| `src-tauri/src/database/models/mod.rs` | Export new models |
| `src-tauri/src/commands/mod.rs` | Export new commands |
| `src-tauri/src/lib.rs` | Register new commands |
| `src/stores/agentConfigStore.ts` | Replace mock with invoke() |
| `src/routes/chat.tsx` | Add session persistence + command handling |
| `src/routes/__root.tsx` (or App.tsx) | Add CommandPalette + useKeyboardShortcuts |
| `src-tauri/src/database/schema.rs` | Regenerate after migration |

---

## Testing Checklist

### Agent Backend
- [ ] `list_agents_command` returns agents from database
- [ ] `create_agent_command` persists new agent
- [ ] `update_agent_command` modifies existing agent
- [ ] `delete_agent_command` removes agent

### Chat Persistence
- [ ] Messages persist across page refresh
- [ ] Session list shows recent chats
- [ ] Can resume previous session
- [ ] Delete session removes all messages

### Command Palette
- [ ] `Cmd+K` / `Ctrl+K` opens palette
- [ ] Navigation commands work
- [ ] New Chat action creates session

### Chat Commands
- [ ] `/status` shows session info
- [ ] `/new` starts fresh session
- [ ] `/clear` removes messages
- [ ] `/help` lists commands

### Keyboard Shortcuts
- [ ] `Cmd+N` creates new chat
- [ ] `Cmd+,` opens settings
- [ ] Shortcuts work on all pages
