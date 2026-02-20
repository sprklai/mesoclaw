# Enhanced Logging System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add comprehensive logging across agents, channels, memory, chat, scheduler, and gateway modules with module category filtering in the logs viewer UI.

**Architecture:**
1. Backend: Add `log::info!`, `log::warn!`, `log::error!` calls throughout key modules using consistent prefixes like `[chat]`, `[agent:spawner]`, `[memory]` for easy filtering
2. Frontend: Extend the existing logs viewer with module category filter buttons (Agents, Channels, Memory, Chat, Scheduler, Gateway, System) that filter by the `target` field in `LogEntry`
3. The existing tracing infrastructure already captures the `target` field (Rust module path) - we just need to add UI controls and more logging

**Tech Stack:**
- Backend: Rust `log` crate, `tracing` subscriber (already configured)
- Frontend: React 19, TypeScript, TanStack Router, Tailwind CSS 4

---

## Task 1: Add Module Filter Type and Constants to Frontend

**Files:**
- Modify: `src/routes/logs.tsx`

**Step 1: Add LogModule type and MODULE_PREFIXES constant**

After the existing `LogLevel` type definition (around line 27), add:

```typescript
type LogModule = "ALL" | "AGENTS" | "CHANNELS" | "MEMORY" | "CHAT" | "SCHEDULER" | "GATEWAY" | "SYSTEM";

const MODULES: LogModule[] = ["ALL", "AGENTS", "CHANNELS", "MEMORY", "CHAT", "SCHEDULER", "GATEWAY", "SYSTEM"];

// Target prefixes for each module category
const MODULE_PREFIXES: Record<LogModule, string[]> = {
  AGENTS: ["agent::", "agents::", "spawner", "orchestrator"],
  CHANNELS: ["channels::", "telegram", "discord", "slack", "matrix"],
  MEMORY: ["memory::"],
  CHAT: ["chat::"],
  SCHEDULER: ["scheduler::"],
  GATEWAY: ["gateway::", "event_bus::"],
  SYSTEM: ["boot::", "lib::", "cli::"],
};
```

**Step 2: Add helper function to check if entry matches module**

After the `levelBadgeVariant` function (around line 61), add:

```typescript
function moduleMatches(entry: LogEntry, activeModule: LogModule): boolean {
  if (activeModule === "ALL") return true;
  const prefixes = MODULE_PREFIXES[activeModule] ?? [];
  const target = entry.target.toLowerCase();
  return prefixes.some(prefix => target.startsWith(prefix));
}
```

**Step 3: Run dev server to verify no TypeScript errors**

Run: `bun run dev`
Expected: Vite dev server starts without errors

**Step 4: Commit**

```bash
git add src/routes/logs.tsx
git commit -m "feat(logs): add module filter type and prefix constants"
```

---

## Task 2: Add Module Filter State to Component

**Files:**
- Modify: `src/routes/logs.tsx`

**Step 1: Add activeModule state**

Find the state declarations (around line 70) and add after `setAutoRefresh`:

```typescript
const [activeModule, setActiveModule] = useState<LogModule>("ALL");
```

**Step 2: Update filtered entries to include module filtering**

Find the `filtered` useMemo (around line 120) and update the filter logic:

```typescript
const filtered = useMemo(() => {
  return entries.filter((e) => {
    // Level filter
    if (activeLevel !== "ALL" && e.level.toUpperCase() !== activeLevel)
      return false;

    // Module filter
    if (!moduleMatches(e, activeModule))
      return false;

    // Search filter
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

**Step 3: Add module counts to useMemo**

Find the `counts` useMemo (around line 145) and add module counts after it:

```typescript
const moduleCounts = useMemo(() => {
  const map: Record<string, number> = { ALL: entries.length };
  for (const e of entries) {
    for (const [mod, prefixes] of Object.entries(MODULE_PREFIXES)) {
      if (moduleMatches(e, mod as LogModule)) {
        map[mod] = (map[mod] ?? 0) + 1;
      }
    }
  }
  return map;
}, [entries]);
```

**Step 4: Run dev server to verify**

Run: `bun run dev`
Expected: No TypeScript errors, logs page loads

**Step 5: Commit**

```bash
git add src/routes/logs.tsx
git commit -m "feat(logs): add module filter state and update filtering logic"
```

---

## Task 3: Add Module Filter Buttons to UI

**Files:**
- Modify: `src/routes/logs.tsx`

**Step 1: Add module filter buttons section**

Find the level filter buttons section (around line 163, the div with role="group") and add a new section after it:

```tsx
{/* Module filter buttons */}
<div className="flex flex-wrap gap-1" role="group" aria-label="Filter by module">
  {MODULES.map((mod) => (
    <button
      key={mod}
      type="button"
      onClick={() => setActiveModule(mod)}
      className={cn(
        "rounded-md px-3 py-1 text-xs font-medium transition-colors",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        activeModule === mod
          ? "bg-primary text-primary-foreground"
          : "bg-muted text-muted-foreground hover:bg-accent hover:text-accent-foreground",
      )}
      aria-pressed={activeModule === mod}
    >
      {mod}
      {mod !== "ALL" && moduleCounts[mod] != null && (
        <span className="ml-1.5 opacity-70">
          {moduleCounts[mod]}
        </span>
      )}
    </button>
  ))}
</div>
```

**Step 2: Test the UI**

Run: `bun run dev`
Navigate to `/logs`
Expected: See two rows of filter buttons - level filters (ALL, TRACE, DEBUG, INFO, WARN, ERROR) and module filters (ALL, AGENTS, CHANNELS, MEMORY, CHAT, SCHEDULER, GATEWAY, SYSTEM)

**Step 3: Verify filtering works**

1. Click different module buttons
2. Search should filter within the selected module
3. Level buttons should filter within the selected module
4. "ALL" module should show all logs

**Step 4: Commit**

```bash
git add src/routes/logs.tsx
git commit -m "feat(logs): add module filter buttons UI"
```

---

## Task 4: Add Logging to Chat Commands

**Files:**
- Modify: `src-tauri/src/commands/chat.rs`

**Step 1: Add log import at top**

```rust
use log::{info, debug};
```

**Step 2: Add logging to get_available_models_command**

```rust
#[tauri::command]
pub fn get_available_models_command() -> Vec<AvailableModel> {
    info!("[chat] fetching available models");
    let result = vec![
        // ... existing model definitions
    ];
    debug!("[chat] returning {} models", result.len());
    result
}
```

**Step 3: Run cargo check**

Run (from src-tauri directory): `cargo check`
Expected: No errors

**Step 4: Test the logging**

Run: `cd .. && bun run tauri dev`
Navigate to logs page, trigger any chat-related action
Expected: See `[chat]` prefixed logs in the logs viewer

**Step 5: Commit**

```bash
git add src-tauri/src/commands/chat.rs
git commit -m "feat(logs): add logging to chat commands"
```

---

## Task 5: Add Logging to Agent Spawner

**Files:**
- Modify: `src-tauri/src/agents/spawner.rs`

**Step 1: Add log import if not present**

Check if `use log::info;` exists, if not add at top with other imports.

**Step 2: Add logging to spawn method**

Find the `spawn` method (around line 253, after `#[tracing::instrument]`) and add logging at key points:

```rust
pub async fn spawn(
    &self,
    parent_session_key: &str,
    task: SubagentTask,
    system_prompt: &str,
) -> Result<SubagentResult, String> {
    let started_at = Utc::now();

    // Log spawn start
    info!(
        "[agent:spawner] spawning subagent task_id={} parent={} prompt_len={}",
        task.id,
        parent_session_key,
        task.prompt.len()
    );

    // Generate unique lane ID
    let lane_id = format!("lane-{}", Uuid::new_v4().simple());

    // ... existing code ...

    // Check spawn depth limit
    if spawn_depth > self.max_spawn_depth {
        warn!(
            "[agent:spawner] maximum spawn depth exceeded: {} > {}",
            spawn_depth, self.max_spawn_depth
        );
        return Err(format!(
            "Maximum spawn depth ({}) exceeded. Current depth: {}",
            self.max_spawn_depth, spawn_depth
        ));
    }

    // ... existing execution code ...

    let completed_at = Utc::now();

    // Log completion
    match &result {
        Ok(_) => {
            info!(
                "[agent:spawner] subagent completed lane_id={} duration_ms={} success=true",
                lane_id,
                (completed_at - started_at).num_milliseconds()
            );
        }
        Err(e) => {
            warn!(
                "[agent:spawner] subagent failed lane_id={} duration_ms={} error={}",
                lane_id,
                (completed_at - started_at).num_milliseconds(),
                e
            );
        }
    }

    // ... rest of existing code ...
}
```

**Step 3: Add warn import if needed**

If using `warn!`, ensure the import includes it: `use log::{info, warn};`

**Step 4: Run cargo check**

Run (from src-tauri directory): `cargo check`
Expected: No errors

**Step 5: Commit**

```bash
git add src-tauri/src/agents/spawner.rs
git commit -m "feat(logs): add logging to agent spawner"
```

---

## Task 6: Add Logging to Agent Orchestrator

**Files:**
- Modify: `src-tauri/src/agents/orchestrator.rs`

**Step 1: Add log import**

```rust
use log::{info, debug, warn};
```

**Step 2: Add logging to key orchestrator methods**

Find the main orchestration method (look for `#[tracing::instrument]`) and add:

```rust
info!("[agent:orchestrator] starting orchestration session={}", session_key);
// ... at the end of orchestration
info!("[agent:orchestrator] orchestration completed session={} steps={}", session_key, step_count);
```

**Step 3: Run cargo check**

Run (from src-tauri directory): `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src-tauri/src/agents/orchestrator.rs
git commit -m "feat(logs): add logging to agent orchestrator"
```

---

## Task 7: Add Logging to Memory Commands

**Files:**
- Modify: `src-tauri/src/memory/commands.rs`

**Step 1: Read the file to understand existing commands**

Run: `cat src-tauri/src/memory/commands.rs`

**Step 2: Add log import**

```rust
use log::{info, debug, warn};
```

**Step 3: Add logging to each command**

For each `#[tauri::command]` function, add:
- `info!` at entry with key parameters
- `info!` at exit with result summary
- `warn!` for errors

Example pattern:
```rust
#[tauri::command]
pub async fn memory_search_command(query: String, limit: usize) -> Result<Vec<MemoryEntry>, String> {
    info!("[memory] search query='{}' limit={}", query, limit);

    match search_memory(&query, limit).await {
        Ok(results) => {
            info!("[memory] search found {} results", results.len());
            Ok(results)
        }
        Err(e) => {
            warn!("[memory] search failed: {}", e);
            Err(e)
        }
    }
}
```

**Step 4: Run cargo check**

Run (from src-tauri directory): `cargo check`
Expected: No errors

**Step 5: Commit**

```bash
git add src-tauri/src/memory/commands.rs
git commit -m "feat(logs): add logging to memory commands"
```

---

## Task 8: Promote Memory Hygiene Debug Logs to Info

**Files:**
- Modify: `src-tauri/src/memory/hygiene.rs`

**Step 1: Find and replace debug! with info!**

The file already has commented-out example logging. Find the `log::debug!` calls (around line 193, 243) and change to `log::info!`:

```rust
// Change:
log::debug!("hygiene: archived {:?} → {:?}", path, dest);
// To:
info!("hygiene: archived {:?} → {:?}", path, dest);

// Change:
log::debug!("hygiene: purged {:?}", path);
// To:
info!("hygiene: purged {:?}", path);
```

**Step 2: Add log import if needed**

```rust
use log::info;
```

**Step 3: Run cargo check**

Run (from src-tauri directory): `cargo check`
Expected: No errors

**Step 4: Commit**

```bash
git add src-tauri/src/memory/hygiene.rs
git commit -m "feat(logs): promote hygiene logs from debug to info"
```

---

## Task 9: Verify All Filters Work Together

**Files:**
- No file changes - integration testing

**Step 1: Run the application**

Run: `bun run tauri dev`

**Step 2: Generate logs from each module**

1. Trigger a chat operation → should see `[chat]` logs
2. If agents are implemented, trigger an agent operation → should see `[agent:spawner]` logs
3. Send/receive a message through a channel → should see channel logs
4. Perform a memory search → should see `[memory]` logs
5. Check scheduler heartbeat → should see `[scheduler]` logs

**Step 3: Test each module filter**

1. Click "AGENTS" → only show agent-related logs
2. Click "CHANNELS" → only show channel logs
3. Click "MEMORY" → only show memory logs
4. Click "CHAT" → only show chat logs
5. Click "ALL" → show everything

**Step 4: Test combined filtering**

1. Select "MEMORY" module
2. Select "WARN" level
3. Expected: Only WARN and ERROR level logs from memory module

**Step 5: Test search with module filter**

1. Select "CHAT" module
2. Type "models" in search
3. Expected: Only chat logs containing "models"

**Step 6: Verify badge counts**

Each module button should show count of logs for that module.

**Step 7: Commit any UI tweaks if needed**

If minor adjustments were needed:

```bash
git add src/routes/logs.tsx
git commit -m "fix(logs): tweak filter UI after testing"
```

---

## Task 10: Update Documentation

**Files:**
- Modify: `README.md` or `docs/features/` if applicable

**Step 1: Document the new filter capability**

Add to the logs section of the README or features documentation:

```markdown
### Log Viewer

The logs viewer provides real-time application logs with filtering:

- **Level filters**: ALL, TRACE, DEBUG, INFO, WARN, ERROR
- **Module filters**: ALL, Agents, Channels, Memory, Chat, Scheduler, Gateway, System
- **Search**: Full-text search across messages
- **Auto-refresh**: Live log updates with pause/resume
- **Clear logs**: Clear the displayed logs (does not delete log files)
```

**Step 2: Commit documentation**

```bash
git add README.md
git commit -m "docs: document module filter feature in logs viewer"
```

---

## Testing Checklist

After completing all tasks, verify:

- [ ] Module filter buttons are visible in the logs page
- [ ] Clicking a module button filters logs correctly
- [ ] Badge counts show correct numbers per module
- [ ] Combined level + module filtering works
- [ ] Search works within selected module
- [ ] Chat operations produce `[chat]` logs
- [ ] Agent operations produce `[agent:spawner]` and `[agent:orchestrator]` logs
- [ ] Memory operations produce `[memory]` logs
- [ ] Channel operations show up under Channels filter
- [ ] Scheduler logs show up under Scheduler filter
- [ ] All existing functionality still works (no regressions)

---

## End of Implementation Plan

All tasks complete when:
1. All module filters work correctly
2. All key modules have appropriate logging
3. All tests pass
4. Documentation is updated
