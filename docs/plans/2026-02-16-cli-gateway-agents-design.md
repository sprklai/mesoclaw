# Design: CLI-First Architecture, Gateway, and Agent Orchestration

> Mesoclaw architectural redesign: from GUI-only app to CLI-first daemon with HTTP gateway.
> Brainstormed and approved: February 16, 2026

---

## Decisions

| Decision              | Choice                                                  | Rationale                                                   |
| --------------------- | ------------------------------------------------------- | ----------------------------------------------------------- |
| Primary identity      | CLI-first, GUI-optional                                 | Devs interact via terminal, non-devs via GUI                |
| CLI model             | Claude Code-style (REPL + one-shot)                     | Interactive sessions + Unix pipe integration                |
| Agent scope v1.0      | Single agent + tool delegation                          | Ship fast, extend to sub-agents via tool registration later |
| Agent extension point | Tools (spawn_agent as a Tool)                           | No agent loop refactoring needed for sub-agents             |
| Gateway protocol      | HTTP REST + WebSocket on 127.0.0.1:18790                | Standard, curl-debuggable, bearer token auth                |
| Build architecture    | Monolith with two binaries (cli + desktop)              | Single crate, simplest build, shared core                   |
| GUI relationship      | Embeds daemon in-process                                | One process for GUI users, daemon auto-starts               |
| Port strategy         | 127.0.0.1:18790, auto-increment 18791-18799 on conflict | Localhost only, PID+port in daemon.pid                      |
| API versioning        | /api/v1/ in URL path                                    | Breaking changes get v2, 6-month overlap                    |

---

## 1. Core Architecture

### Monolith with Two Entry Points

One Rust crate produces two binaries sharing 100% of core logic:

```
src-tauri/src/
├── lib.rs                 ← Core: daemon, agent, providers, memory, gateway, etc.
├── bin/
│   ├── cli.rs             ← CLI entry point (clap + rustyline REPL)
│   └── desktop.rs         ← Tauri GUI entry point (embeds daemon)
├── daemon/                ← Daemon lifecycle, state management
├── gateway/               ← HTTP REST + WebSocket (axum)
├── agent/                 ← Agent loop, tool execution
├── providers/             ← LLM providers (GenericProvider, ReliableProvider)
├── tools/                 ← Tool trait + registry + built-in tools
├── memory/                ← Hybrid search (vector + BM25)
├── security/              ← Policy, command validation, path traversal prevention
├── identity/              ← Markdown identity files
├── scheduler/             ← Heartbeat + cron
├── channels/              ← Telegram, webhook, Tauri IPC
├── event_bus/             ← Internal pub/sub (tokio::broadcast)
├── config/                ← TOML config + env overrides
├── prompts/               ← Template loading (replaces skills)
├── services/              ← Notifications, session router, boot sequence
└── database/              ← SQLite storage
```

### Startup Flow

**CLI mode** (`mesoclaw` binary):

1. Parse CLI args (clap)
2. Check `~/.mesoclaw/daemon.pid` — is daemon already running?
3. If no: start daemon in-process (gateway on 127.0.0.1:18790)
4. If yes: connect to existing daemon via HTTP/WebSocket
5. Execute command (one-shot) or enter interactive REPL

**GUI mode** (`mesoclaw-desktop` binary):

1. Start Tauri app
2. Start daemon in-process (gateway on 127.0.0.1:18790)
3. Write PID file so CLI can connect
4. Frontend connects to gateway (same as CLI)
5. Tauri IPC used ONLY for: window management, system tray, native notifications

**Key architectural change**: Frontend no longer uses Tauri `invoke()` for agent/memory/provider operations. It uses HTTP/WebSocket to the gateway, same as CLI. This ensures feature parity between CLI and GUI.

### Daemon Lifecycle

```
~/.mesoclaw/
├── daemon.pid          ← {"pid": 12345, "port": 18790, "started_at": "..."}
├── daemon.token        ← Random bearer token (mode 0600)
├── config.toml         ← User configuration
├── app.db              ← SQLite database
├── identity/           ← Agent personality files
├── memory/             ← Daily memory files
├── prompts/            ← Custom prompt templates
└── logs/               ← Audit + debug logs
```

---

## 2. Gateway API

### Port & Security

- Default: `127.0.0.1:18790` (localhost only, never 0.0.0.0)
- Bearer token auth: random token generated on startup, written to `daemon.token`
- Port conflict: auto-increment 18791-18799, configurable in config.toml
- CLI reads token file automatically
- GUI uses in-process calls (no token needed)
- API versioned at `/api/v1/` — breaking changes get `/api/v2/` with 6-month overlap

### REST Endpoints

```
Base: http://127.0.0.1:18790/api/v1
Auth: Authorization: Bearer <token from daemon.token>

Agent:
  POST   /agent/sessions                    Create session
  POST   /agent/sessions/{id}/messages      Send message (streaming response)
  DELETE /agent/sessions/{id}               Stop/cancel session
  GET    /agent/sessions                    List sessions
  GET    /agent/sessions/{id}               Session details
  GET    /agent/sessions/{id}/messages      Conversation history
  POST   /agent/sessions/{id}/approve       Approve pending tool execution

Providers:
  GET    /providers                         List providers
  POST   /providers/{id}/test               Test connection
  GET    /providers/models                  List models

Memory:
  GET    /memory/search?q=...&limit=10      Search memory
  POST   /memory                            Store entry
  DELETE /memory/{key}                      Forget entry
  GET    /memory/daily/{date?}              Daily memory

Identity:
  GET    /identity                          All identity files
  GET    /identity/{file}                   Specific file
  PUT    /identity/{file}                   Update file

Scheduler:
  GET    /scheduler/jobs                    List jobs
  POST   /scheduler/jobs                    Create job
  DELETE /scheduler/jobs/{id}               Delete job
  POST   /scheduler/jobs/{id}/toggle        Enable/disable

Channels:
  GET    /channels                          List + status
  POST   /channels/{name}/connect           Connect
  POST   /channels/{name}/disconnect        Disconnect

System:
  GET    /health                            Health check
  GET    /status                            Full status
```

### WebSocket Protocol

```
Endpoint: ws://127.0.0.1:18790/api/v1/ws
Auth: Bearer token in first message or query param

Client → Server:
  {"type": "subscribe", "events": ["agent.*", "scheduler.*"]}
  {"type": "message", "session_id": "abc123", "content": "analyze this"}
  {"type": "approve", "action_id": "xyz", "approved": true}
  {"type": "cancel", "session_id": "abc123"}

Server → Client:
  {"type": "agent.token", "session_id": "...", "token": "The"}
  {"type": "agent.tool_start", "session_id": "...", "tool": "read_file", "args": {...}}
  {"type": "agent.tool_result", "session_id": "...", "tool": "read_file", "success": true}
  {"type": "agent.approval_needed", "session_id": "...", "action_id": "...", "command": "rm temp.txt", "risk": "high"}
  {"type": "agent.complete", "session_id": "...", "summary": "..."}
  {"type": "scheduler.heartbeat", "job_id": "...", "results": [...]}
  {"type": "scheduler.cron_fired", "job_id": "...", "payload": "..."}
  {"type": "channel.message", "channel": "telegram", "from": "...", "content": "..."}
  {"type": "system.error", "module": "...", "message": "..."}
```

---

## 3. CLI Experience

### Command Structure

```bash
# Interactive REPL (default)
mesoclaw                                    # Start interactive session
mesoclaw -p "You are a Rust expert"         # Custom system prompt
mesoclaw --identity researcher              # Named identity profile
mesoclaw --resume <session-id>              # Resume previous session

# One-Shot
mesoclaw "analyze this Rust project"        # Run agent, print result, exit
mesoclaw "fix the failing test" --auto      # Full autonomy (no approvals)
mesoclaw "summarize" --provider ollama      # Override provider

# Pipe / Compose
cat schema.sql | mesoclaw "explain this"    # Stdin as context
git diff | mesoclaw "review this diff"      # Pipe anything in
mesoclaw "generate migration" --raw > out.sql  # Raw output for scripting
mesoclaw "find bugs" --json | jq '.issues'  # JSON output for tooling
mesoclaw "find issues" --raw | mesoclaw "fix these" --auto  # Agent chaining

# Watch Mode
mesoclaw watch ./src --prompt "review changes"              # Watch directory
mesoclaw watch . --glob "*.rs" --debounce 5s                # Filtered watch

# Agent Management
mesoclaw agent status                       # Show running sessions
mesoclaw agent stop <session-id>            # Stop a session
mesoclaw agent logs <session-id>            # Stream logs
mesoclaw agent list                         # All sessions (active + history)

# Daemon Management
mesoclaw daemon start                       # Start background daemon
mesoclaw daemon stop                        # Stop daemon
mesoclaw daemon status                      # PID, port, uptime, agents
mesoclaw daemon logs                        # Tail daemon logs

# Memory
mesoclaw memory search "database indexing"  # Search
mesoclaw memory store "uses PostgreSQL 16"  # Store fact
mesoclaw memory daily                       # Today's memory
mesoclaw memory daily 2026-02-15            # Specific day

# Identity
mesoclaw identity show                      # Current identity
mesoclaw identity edit soul                 # Open SOUL.md in $EDITOR
mesoclaw identity list                      # Available profiles

# Config
mesoclaw config show                        # Print config
mesoclaw config set provider.default anthropic
mesoclaw config set security.autonomy supervised

# Scheduler
mesoclaw schedule list                      # Jobs
mesoclaw schedule add --cron "0 9 * * MON" --prompt "weekly summary"
mesoclaw schedule remove <job-id>

# Channels
mesoclaw channel list                       # Status
mesoclaw channel connect telegram           # Connect (prompts for token)
mesoclaw channel disconnect telegram

# GUI
mesoclaw gui                                # Launch Tauri desktop app
```

### Interactive REPL Features

| Feature             | Implementation                                       |
| ------------------- | ---------------------------------------------------- |
| Streaming output    | Tokens render in real-time via WebSocket             |
| Tool approval       | `[y/N/always]` inline prompt for supervised mode     |
| Slash commands      | `/memory`, `/agent`, `/identity`, `/config`, `/quit` |
| Session persistence | Auto-save, resume with `--resume`                    |
| Piping              | Stdin detected, injected as context                  |
| Markdown rendering  | `termimad` crate for terminal markdown               |
| History             | `rustyline` for readline, Ctrl-R search              |
| Multi-line          | `\` continuation or `"""` blocks                     |

### Key CLI Flags

| Flag                | Description                                              |
| ------------------- | -------------------------------------------------------- |
| `--raw`             | Output only agent text (no spinners, no tool indicators) |
| `--json`            | Structured JSON output for scripting                     |
| `--auto`            | Full autonomy (skip approval prompts)                    |
| `--output <path>`   | Write response to file                                   |
| `--provider <name>` | Override default provider                                |
| `--model <name>`    | Override default model                                   |
| `--identity <name>` | Use named identity profile                               |
| `--port <port>`     | Connect to specific daemon instance                      |
| `--no-memory`       | Don't inject or store memory for this session            |

---

## 4. Agent Orchestration

### v1.0: Single Agent + Tool Delegation

Each session runs one agent loop. Multiple sessions can exist concurrently (main, heartbeat, cron, Telegram). All sessions share memory.

Tools at v1.0:

- `shell` — Execute shell commands (gated by SecurityPolicy)
- `file_read` — Read file contents (gated by SecurityPolicy)
- `file_write` — Write file contents (gated by SecurityPolicy)
- `file_list` — List directory contents
- `memory_store` — Store fact in memory
- `memory_recall` — Search memory
- `memory_forget` — Remove memory entry
- `web_search` — Search the web (future, P2)

### v1.x: Sub-Agent Spawning (Future — Designed For)

`spawn_agent` registered as a Tool in ToolRegistry. The main agent can call it like any other tool:

```json
{
  "name": "spawn_agent",
  "arguments": {
    "task": "analyze the auth module for security issues",
    "identity": "security-reviewer",
    "tools": ["file_read", "memory_recall"],
    "timeout": 300
  }
}
```

Implementation: creates an isolated agent session via AgentEngine, waits for completion, returns result as ToolResult. No agent loop changes needed.

### v2.0: Agent Swarm (Future — Designed For)

Multiple agents with event bus communication. Coordinator agent spawns workers, workers publish results to event bus, coordinator synthesizes.

Enabled by:

- AgentEngine already supports multiple concurrent sessions
- Event Bus already supports pub/sub between sessions
- Security Policy already scoped per session
- Identity system already supports named profiles (agent templates)

### Agent Templates (Future — Designed For)

Named profiles in `~/.mesoclaw/identity/profiles/`:

```
~/.mesoclaw/identity/profiles/
├── researcher/
│   ├── SOUL.md        ← "You are a research specialist..."
│   └── tools.toml     ← allowed_tools = ["web_search", "memory_store"]
├── coder/
│   ├── SOUL.md        ← "You are an expert programmer..."
│   └── tools.toml     ← allowed_tools = ["shell", "file_read", "file_write"]
└── reviewer/
    ├── SOUL.md        ← "You review code for bugs and security..."
    └── tools.toml     ← allowed_tools = ["file_read", "memory_recall"]
```

Used via: `mesoclaw --identity researcher` or `spawn_agent(identity: "coder")`.

---

## 5. What Changes in the Existing Plan

### New Phases/Tasks Required

| New Item                                 | Phase         | Description                                             |
| ---------------------------------------- | ------------- | ------------------------------------------------------- |
| **Restructure to lib + two binaries**    | Phase 0       | Move core to lib.rs, create bin/cli.rs + bin/desktop.rs |
| **Build gateway (axum)**                 | Phase 2       | HTTP REST + WebSocket server on localhost               |
| **Build CLI (clap + rustyline)**         | Phase 2       | Interactive REPL + one-shot commands                    |
| **Migrate frontend from IPC to HTTP/WS** | Phase 2       | Replace invoke() calls with fetch/WebSocket             |
| **Watch mode**                           | Phase 5       | File watcher + agent trigger                            |
| **Pipe/compose support**                 | Phase 0 (CLI) | --raw, --json, stdin detection                          |

### Modified Existing Items

| Existing Item               | Change                                       |
| --------------------------- | -------------------------------------------- |
| Event Bus (P1.8)            | Now also serves WebSocket event streaming    |
| Tauri IPC commands          | Reduced to window/tray/notification only     |
| All frontend invoke() calls | Migrated to HTTP/WebSocket gateway calls     |
| Phase 6 frontend tasks      | Now build against gateway API, not Tauri IPC |
| Channel Manager             | Runs inside daemon, not inside Tauri process |

### New Dependencies

```toml
# CLI
clap = { version = "4", features = ["derive"] }
rustyline = "14"               # REPL with history
termimad = "0.30"              # Terminal markdown rendering
indicatif = "0.17"             # Progress bars / spinners
crossterm = "0.28"             # Terminal control

# Gateway
axum = "0.8"                   # HTTP framework
axum-extra = "0.10"            # WebSocket support
tower-http = "0.6"             # CORS, auth middleware
tokio-tungstenite = "0.24"     # WebSocket
utoipa = "5"                   # OpenAPI spec generation

# Daemon
daemonize = "0.5"              # Background process (Linux/macOS)
```

---

## 6. Revised Phase Order

```
Phase 0: Slim Down + Responsive + CLI Restructure
  S4 Strict linting
  S2 Consolidate providers (async-openai)
  S1 Replace skills with prompt templates
  0.5 Responsive layout foundation
  NEW: Restructure to lib.rs + bin/cli.rs + bin/desktop.rs
  NEW: Basic CLI with clap (commands structure, no gateway yet)

Phase 1: Foundation
  1.1 Release profile optimization
  1.2 ReliableProvider wrapper

Phase 2: Core Infrastructure + Gateway
  2.1 Event Bus (tokio::broadcast)
  2.2 Tool trait + registry
  2.3 Security policy
  2.4 Identity system
  NEW: Gateway (axum HTTP + WebSocket)
  NEW: CLI REPL (rustyline, connects to gateway)
  NEW: Migrate frontend from IPC to gateway

Phase 3: Agent Intelligence
  3.1 Agent loop
  3.2 Memory system
  3.3 Daily memory files

Phase 4: Proactive
  4.1 Scheduler (heartbeat + cron)
  4.2 Desktop notifications
  4.3 Session management

Phase 5: Config, DX, & CLI Polish
  5.1 TOML configuration
  5.2 Provider router
  5.3 Prelude module
  5.4 Dual tool-call parser
  NEW: Watch mode
  NEW: Pipe/compose (--raw, --json, stdin)

Phase 6: Extensions & UI
  6.1 Channel trait + manager
  6.2 Boot sequence
  6.3 Frontend: Agent loop UI
  6.4 Frontend: Memory search UI
  6.5 Frontend: Identity & Scheduler UIs

Phase 7: Channels + Mobile (Post v1.0)
  7.1 Telegram channel
  7.2 Channel management UI
  7.3 Mobile-specific polish
  7.4 Tauri Mobile builds
```

**Critical path updated**: Slim Down + CLI Restructure → Gateway → Event Bus → Tools → Security → Identity → Agent Loop → Memory → Scheduler → Notifications → Telegram

---

_Design approved: February 16, 2026_
_References: docs/claw-ecosystem-analysis.md, docs/mesoclaw-gap-analysis.md_
