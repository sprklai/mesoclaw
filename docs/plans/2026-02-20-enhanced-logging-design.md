# Enhanced Logging System with Module Filters

**Date:** 2026-02-20
**Status:** Approved
**Author:** Claude Code

## Overview

Add comprehensive logging across all major modules (agents, channels, memory, chat, scheduler, gateway) and provide a UI to filter logs by module category alongside the existing level filters.

## Problem Statement

- Chat operations have **no logging** at all
- Agent and Memory modules have **minimal logging** (mostly debug level)
- No way to **filter logs by module** - only by log level
- Difficult to debug issues in specific subsystems

## Solution

1. Add structured logging to key modules
2. Add module category filter buttons to the logs viewer UI
3. Leverage existing `target` field in `LogEntry` for filtering

## Module Categories

| Category | Target Prefixes | Description |
|----------|-----------------|-------------|
| ALL | *(all)* | Show all logs |
| Agents | `agent::`, `agents::`, `spawner`, `orchestrator` | Spawner, orchestrator, config, workspace |
| Channels | `channels::`, `telegram`, `discord`, `slack`, `matrix` | All messaging channel operations |
| Memory | `memory::` | Store, embeddings, chunker, daily timeline, search |
| Chat | `chat::`, `commands::chat` | Chat sessions, streaming, AI interactions |
| Scheduler | `scheduler::` | Jobs, heartbeats, agent turns |
| Gateway | `gateway::`, `event_bus::` | WebSocket, auth, routing |
| System | `boot::`, `lib::`, `cli::` | Core initialization, system events |

## Backend Changes

### 1. `src-tauri/src/commands/chat.rs`

Add logging at command boundaries:
```rust
#[tauri::command]
pub fn get_available_models_command() -> Vec<AvailableModel> {
    log::info!("[chat] fetching available models");
    // ... existing code
    log::debug!("[chat] returning {} models", result.len());
    result
}
```

### 2. `src-tauri/src/agents/spawner.rs`

Add logging for lifecycle events:
```rust
pub async fn spawn(...) -> Result<SubagentResult, String> {
    log::info!("[agent:spawner] spawning subagent task_id={} parent={}", task.id, parent_session_key);
    // ... spawn logic
    log::info!("[agent:spawner] subagent completed lane_id={} success={} duration_ms={}",
        lane_id, success, result.duration_ms());
}
```

### 3. `src-tauri/src/agents/orchestrator.rs`

Add logging for orchestration decisions.

### 4. `src-tauri/src/memory/commands.rs`

Add logging for memory operations:
```rust
log::info!("[memory] search query: {}", query);
log::info!("[memory] found {} results", results.len());
```

### 5. `src-tauri/src/memory/hygiene.rs`

Promote `debug!` to `info!` for visibility.

## Frontend Changes

### UI Layout Update

Add module filter buttons below level filters:

```
┌─────────────────────────────────────────────────────────┐
│ [ALL] [TRACE] [DEBUG] [INFO] [WARN] [ERROR]            │ ← Level filters (existing)
│ [ALL] [Agents] [Channels] [Memory] [Chat] [Scheduler]  │ ← Module filters (new)
│       [Gateway] [System]                               │
├─────────────────────────────────────────────────────────┤
│ [Search...] [Clear] [Pause/Play] [Refresh]             │
└─────────────────────────────────────────────────────────┘
```

### State Updates

```typescript
type LogModule = "ALL" | "AGENTS" | "CHANNELS" | "MEMORY" | "CHAT" | "SCHEDULER" | "GATEWAY" | "SYSTEM";

const [activeModule, setActiveModule] = useState<LogModule>("ALL");
```

### Filtering Logic

```typescript
const MODULE_PREFIXES: Record<LogModule, string[]> = {
  AGENTS: ["agent::", "agents::", "spawner", "orchestrator"],
  CHANNELS: ["channels::", "telegram", "discord", "slack", "matrix"],
  MEMORY: ["memory::"],
  CHAT: ["chat::"],
  SCHEDULER: ["scheduler::"],
  GATEWAY: ["gateway::", "event_bus::"],
  SYSTEM: ["boot::", "lib::", "cli::"],
};

const moduleMatches = (entry: LogEntry): boolean => {
  if (activeModule === "ALL") return true;
  const prefixes = MODULE_PREFIXES[activeModule] ?? [];
  return prefixes.some(prefix => entry.target.toLowerCase().startsWith(prefix));
};
```

### Combined Filtering

```typescript
const filtered = useMemo(() => {
  return entries.filter((e) => {
    // Level filter (existing)
    if (activeLevel !== "ALL" && e.level.toUpperCase() !== activeLevel)
      return false;

    // Module filter (new)
    if (!moduleMatches(e)) return false;

    // Search filter (existing)
    if (search.trim()) {
      const q = search.toLowerCase();
      return (
        e.message.toLowerCase().includes(q) ||
        e.target.toLowerCase().includes(q) ||
        e.timestamp.toLowerCase().includes(q)
      );
    }

    return true;
  });
}, [entries, activeLevel, activeModule, search]);
```

## Implementation Order

1. Frontend: Add module filter UI and logic (can test with existing logs)
2. Backend: Add logging to `chat.rs`
3. Backend: Add logging to `agents/spawner.rs`
4. Backend: Add logging to `agents/orchestrator.rs`
5. Backend: Add logging to `memory/commands.rs`
6. Backend: Promote hygiene logs to `info!`
7. Test: Verify filters work with new logs

## Files Modified

| File | Type | Changes |
|------|------|---------|
| `src/routes/logs.tsx` | Frontend | Add module filter UI + filtering logic |
| `src-tauri/src/commands/chat.rs` | Backend | Add `log::info!` calls |
| `src-tauri/src/agents/spawner.rs` | Backend | Add `log::info!` calls |
| `src-tauri/src/agents/orchestrator.rs` | Backend | Add `log::info!` calls |
| `src-tauri/src/memory/commands.rs` | Backend | Add `log::info!` calls |
| `src-tauri/src/memory/hygiene.rs` | Backend | Promote `debug!` to `info!` |
