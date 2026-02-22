# Mesoclaw Architecture Diagram

> Complete system architecture for Mesoclaw — a desktop AI agent built on Tauri 2.
> Reference: `docs/claw-ecosystem-analysis.md`, `docs/mesoclaw-gap-analysis.md`

---

### System Overview (CLI-First)

```
                              mesoclaw-core (lib.rs)
                    ┌──────────────────────────────────────┐
                    │           Daemon + Gateway            │
                    │     (axum HTTP + WebSocket on         │
                    │      127.0.0.1:18790)                 │
                    │                                       │
                    │  Agent · Providers · Memory · Tools   │
                    │  Security · Identity · Scheduler      │
                    │  Channels · Event Bus · Config        │
                    └───────────────┬───────────────────────┘
                                    │ HTTP REST + WebSocket
                    ┌───────────────┼───────────────────────┐
                    │               │                       │
              ┌─────▼─────┐  ┌─────▼──────┐  ┌────────────▼─┐
              │    CLI    │  │  Tauri GUI  │  │  curl/scripts │
              │ (clap +   │  │ (React +    │  │  (any HTTP    │
              │  rustyline │  │  WebSocket) │  │   client)     │
              │  REPL)    │  │             │  │               │
              └───────────┘  └────────────┘  └───────────────┘
              bin/cli.rs     bin/desktop.rs    External
```

**Key principle**: The daemon is the product. CLI and GUI are both thin clients connecting to the same gateway API. This ensures 100% feature parity.

---

## High-Level System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Desktop Application                          │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                 FRONTEND (React 19 / TypeScript)              │  │
│  │                                                               │  │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │  │
│  │  │  Chat   │  │ Settings │  │  Memory  │  │  Scheduler   │  │  │
│  │  │  Route  │  │  Route   │  │  Search  │  │  Management  │  │  │
│  │  └────┬────┘  └────┬─────┘  └────┬─────┘  └──────┬───────┘  │  │
│  │       │             │             │               │          │  │
│  │  ┌────▼─────────────▼─────────────▼───────────────▼───────┐  │  │
│  │  │              Zustand Stores Layer                       │  │  │
│  │  │  agentStore · providerStore · memoryStore              │  │  │
│  │  │  schedulerStore · identityStore · settingsStore        │  │  │
│  │  └────────────────────────┬───────────────────────────────┘  │  │
│  │                           │                                  │  │
│  │  ┌────────────────────────▼───────────────────────────────┐  │  │
│  │  │      Gateway Client Layer (HTTP + WebSocket)           │  │  │
│  │  │  fetch("/api/v1/*") + ws://127.0.0.1:18790/api/v1/ws  │  │  │
│  │  │  Tauri IPC only for window/tray/native notifications   │  │  │
│  │  └────────────────────────┬───────────────────────────────┘  │  │
│  └───────────────────────────┼───────────────────────────────────┘  │
│                              │                                      │
│  ════════════════════════════╪══════════════════════════════════════ │
│         Local Gateway Transport (WebView ↔ 127.0.0.1 API)           │
│  ════════════════════════════╪══════════════════════════════════════ │
│                              │                                      │
│  ┌───────────────────────────▼───────────────────────────────────┐  │
│  │                  BACKEND (Rust / Tauri 2)                     │  │
│  │                                                               │  │
│  │  ┌───────────────────────────────────────────────────────┐   │  │
│  │  │          Gateway API Layer (REST + WebSocket)         │   │  │
│  │  │  agent · providers · memory · scheduler · identity    │   │  │
│  │  │  modules · channels · system                           │   │  │
│  │  └───────────────────────┬───────────────────────────────┘   │  │
│  │                          │                                    │  │
│  │  ┌───────────────────────▼───────────────────────────────┐   │  │
│  │  │                   Event Bus                            │   │  │
│  │  │         (tokio::broadcast + Tauri emit)                │   │  │
│  │  │  Connects all subsystems asynchronously                │   │  │
│  │  └──┬──────┬──────┬──────┬──────┬──────┬─────────────────┘   │  │
│  │     │      │      │      │      │      │                      │  │
│  │     ▼      ▼      ▼      ▼      ▼      ▼                      │  │
│  │  ┌─────┐┌─────┐┌─────┐┌─────┐┌─────┐┌──────┐                │  │
│  │  │Agent││Tools││Memry││Sched││Ident││Secur │                │  │
│  │  │Loop ││     ││     ││uler ││ity  ││ity   │                │  │
│  │  └──┬──┘└──┬──┘└──┬──┘└──┬──┘└──┬──┘└──┬───┘                │  │
│  │     │      │      │      │      │      │                      │  │
│  │  ┌──▼──────▼──────▼──────▼──────▼──────▼───────────────────┐  │  │
│  │  │                Core Services                             │  │  │
│  │  │  LLM Providers · Credential Store · Config Manager       │  │  │
│  │  │  Notification Service · Boot Sequence · Session Router   │  │  │
│  │  └──────────────────────────┬──────────────────────────────┘  │  │
│  │                             │                                  │  │
│  │  ┌──────────────────────────▼──────────────────────────────┐  │  │
│  │  │              Storage Layer                               │  │  │
│  │  │  SQLite (app data + FTS5 + vector BLOBs)                │  │  │
│  │  │  Filesystem (identity .md + memory .md + config .toml)  │  │  │
│  │  │  OS Keyring (API keys — never on disk)                  │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    System Integration                         │  │
│  │  System Tray · Native Notifications · File System Access      │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
   ┌──────────┐    ┌────────────┐    ┌────────────┐    ┌──────────┐    ┌──────────┐
   │ LLM APIs │    │  Webhooks  │    │   Ollama   │    │ Telegram │    │ WhatsApp │
   │ (Remote) │    │ (Optional) │    │  (Local)   │    │ Bot API  │    │ (Future) │
   └──────────┘    └────────────┘    └────────────┘    └──────────┘    └──────────┘
```

---

## Backend Module Architecture

```
src-tauri/src/
│
├── lib.rs                           # Module tree + daemon/gateway setup + prelude
├── bin/
│   ├── cli.rs                       # CLI entry point
│   └── desktop.rs                   # Tauri desktop entry point
│
├── gateway/                         # ── Primary Control Plane ──
│   ├── routes.rs                    #   REST endpoints
│   ├── ws.rs                        #   WebSocket event stream + commands
│   ├── auth.rs                      #   Bearer token auth middleware
│   └── daemon.rs                    #   lifecycle (port, pid, token)
│
├── agent/                           # ── Agent Loop (P0.3) ──
│   ├── mod.rs                       #   Public API
│   ├── loop_.rs                     #   Multi-turn conversation manager
│   │                                #     message → LLM → tool call? → execute → repeat
│   │                                #     Max 10-20 iterations per turn
│   │                                #     History trimming at 50 messages
│   └── tool_parser.rs              #   Dual format parser (OpenAI JSON + XML)
│
├── providers/                       # ── LLM Provider System ──
│   ├── traits.rs                    #   LLMProvider trait (complete/stream/warmup)
│   ├── generic.rs                   #   GenericProvider via async-openai (replaces 3 files)
│   ├── anthropic.rs                 #   Anthropic adapter (different API format)
│   ├── reliable.rs                  #   ReliableProvider wrapper (retry + fallback)
│   ├── router.rs                    #   Model routing (task-based provider selection)
│   └── mod.rs                       #   Factory: config → Arc<dyn LLMProvider>
│
├── tools/                           # ── Tool System (P1.4) ──
│   ├── traits.rs                    #   Tool trait: name/desc/schema/execute
│   ├── registry.rs                  #   ToolRegistry: dynamic tool registration + lookup
│   ├── shell.rs                     #   Shell command execution tool
│   ├── file_ops.rs                  #   File read/write/list tools
│   └── mod.rs                       #   Built-in tool registration
│
├── memory/                          # ── Memory System (P1.5) ──
│   ├── traits.rs                    #   Memory trait: store/recall/forget
│   ├── sqlite.rs                    #   SQLite backend (vector BLOBs + FTS5/BM25)
│   ├── embeddings.rs                #   Embedding generation (OpenAI/Ollama APIs)
│   ├── chunker.rs                   #   Document splitting for long inputs
│   ├── daily.rs                     #   Daily memory files (YYYY-MM-DD.md)
│   ├── hygiene.rs                   #   Auto-archive (7d) + purge (30d)
│   └── mod.rs                       #   Hybrid search: 0.7*vector + 0.3*bm25
│
├── security/                        # ── Security Policy (P1.6) ──
│   ├── mod.rs                       #   Public API
│   └── policy.rs                    #   SecurityPolicy struct
│                                    #     3 autonomy levels: ReadOnly/Supervised/Full
│                                    #     Command risk classification (Low/Medium/High)
│                                    #     Path traversal prevention
│                                    #     Injection protection
│                                    #     Rate limiting (sliding window)
│
├── channels/                        # ── Channel System (P1.7) ──
│   ├── traits.rs                    #   Channel trait: send/listen/health_check
│   ├── tauri_ipc.rs                 #   Default: wraps existing Tauri IPC as a channel
│   ├── webhook.rs                   #   HTTP webhook listener (axum)
│   └── mod.rs                       #   Channel manager (lifecycle, health)
│
├── lifecycle/                       # ── Resource Lifecycle Management ──
│   ├── supervisor.rs                #   Centralized resource supervisor
│   ├── state_registry.rs            #   Resource state tracking
│   ├── health_monitor.rs            #   Heartbeat tracking + stuck detection
│   ├── storage.rs                   #   SQLite persistence for crash recovery
│   ├── manager.rs                   #   Unified SessionRouter + StateRegistry
│   ├── events.rs                    #   Tauri event emission
│   ├── recovery_engine.rs           #   Transfer + preserve recovery
│   ├── escalation_manager.rs        #   Tiered escalation (retry/fallback/user)
│   ├── handlers/                    #   Resource-type handlers
│   │   ├── agent.rs                 #     Agent session lifecycle
│   │   ├── channel.rs               #     Channel connection lifecycle
│   │   ├── tool.rs                  #     Tool execution lifecycle
│   │   └── scheduler.rs             #     Scheduler job lifecycle
│   └── mod.rs                       #   Plugin registry + re-exports
│
├── event_bus/                       # ── Event Bus (P1.8) ──
│   ├── traits.rs                    #   EventBus trait + AppEvent enum
│   │                                #     Events: ChannelMessage, AgentToolStart,
│   │                                #     AgentToolResult, HeartbeatTick, CronFired,
│   │                                #     MemoryStored, ApprovalNeeded, SystemEvent
│   ├── tokio_bus.rs                 #   Default impl: tokio::sync::broadcast
│   └── tauri_bridge.rs             #   Forward UI-relevant events to frontend
│
├── scheduler/                       # ── Scheduler (P1.9) ──
│   ├── traits.rs                    #   Scheduler trait: start/stop/add_job/list_jobs
│   ├── tokio_scheduler.rs           #   Default impl: tokio intervals + cron matching
│   ├── cron_parser.rs               #   5-field cron expression parser
│   └── mod.rs                       #   Dual mode: heartbeat (intervals) + cron (precise)
│
├── identity/                        # ── Identity System (P1.10) ──
│   ├── types.rs                     #   Identity struct, file definitions
│   ├── loader.rs                    #   Load .md files, hot-reload via file watcher
│   ├── defaults/                    #   Default template files
│   │   ├── SOUL.md
│   │   ├── USER.md
│   │   └── AGENTS.md
│   └── mod.rs                       #   System prompt assembly
│
├── config/                          # ── Configuration (P2.11) ──
│   ├── mod.rs                       #   Config loading + env override
│   └── schema.rs                    #   TOML schema with serde defaults
│
├── prompts/                         # ── Prompt Templates (replaces skills/) ──
│   ├── mod.rs                       #   Template loading + variable substitution
│   └── loader.rs                    #   Load .md templates from embedded + filesystem
│
├── services/                        # ── Core Services ──
│   ├── credential_store.rs          #   (existing) OS keyring integration
│   ├── notification_service.rs      #   Desktop notifications via Tauri plugin
│   ├── session_router.rs            #   Session key routing (main/cron/heartbeat)
│   ├── boot.rs                      #   Startup sequence orchestration
│   └── settings.rs                  #   (existing) App settings persistence
│
├── modules/                        # ── Sidecar Module System ──
│   ├── mod.rs                      #   ModuleRegistry, discovery, lifecycle
│   ├── manifest.rs                 #   TOML manifest parsing + validation
│   ├── sidecar_tool.rs             #   On-demand process spawning
│   ├── sidecar_service.rs          #   Long-lived HTTP service management
│   ├── mcp_client.rs               #   MCP protocol client (JSON-RPC)
│   ├── container/                  #   Container runtime abstraction
│   │   ├── mod.rs                  #     ContainerRuntime trait, auto-detect
│   │   ├── docker.rs               #     DockerRuntime (bollard)
│   │   └── podman.rs               #     PodmanRuntime (bollard)
│   └── protocol/                   #   Communication protocols
│       ├── mod.rs                  #     Protocol trait
│       ├── stdio_json.rs           #     Stdin/Stdout JSON protocol
│       ├── mcp.rs                  #     MCP JSON-RPC protocol
│       └── http.rs                 #     HTTP client for services
│
└── database/                        # ── Storage Layer ──
    ├── mod.rs                       #   Connection management
    ├── models/                      #   Data models
    └── migrations/                  #   SQL migration scripts
```

---

## Data Flow Diagrams

### Single-Turn Chat (Existing)

```
User types message
       │
       ▼
┌──────────────┐   POST /api/v1/agent/sessions/{id}/messages  ┌───────────────┐
│   Frontend   │ ──────────────────────────────────────────────▶│ Gateway API   │
│  PromptInput │                                                │ (REST)        │
└──────────────┘                                                └──────┬────────┘
                                                                        │
       ┌────────────────────────────────────────────────────────────────▼───────┐
       │                            Agent + Provider                             │
       │  ReliableProvider → GenericProvider → Remote API                        │
       │  (retry 3x)        (async-openai)    (OpenAI/Anthropic/Ollama)          │
       └───────────────────────────────────────────┬──────────────────────────────┘
                                                   │ token/tool events
       ┌───────────────────────────────────────────▼──────────────────────────────┐
       │                   WebSocket Stream /api/v1/ws                             │
       │  agent.token · agent.tool_start · agent.tool_result · agent.complete     │
       └───────────────────────────────────────────┬──────────────────────────────┘
                                                   │
       ┌───────────────────────────────────────────▼──────┐
       │  Frontend: Conversation component                 │
       │  Renders tokens and tool states incrementally     │
       └───────────────────────────────────────────────────┘
```

### Multi-Turn Agent Loop (New — P0.3)

```
User sends complex request
       │
       ▼
┌──────────────────────────────────────────────────────────────────┐
│                        AGENT LOOP                                │
│                                                                  │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │ 1. Build │───▶│ 2. Call  │───▶│ 3. Parse │───▶│ 4. Tool  │  │
│  │ Context  │    │   LLM    │    │ Response │    │  Call?   │  │
│  │          │    │          │    │          │    │          │  │
│  │ Identity │    │ Reliable │    │ Dual     │    │ Yes: ──────────┐
│  │ Memory   │    │ Provider │    │ Parser   │    │ No: Done │  │  │
│  │ History  │    │          │    │ (JSON+   │    └──────────┘  │  │
│  │ Tools    │    │          │    │  XML)    │                   │  │
│  └──────────┘    └──────────┘    └──────────┘                   │  │
│                                                                  │  │
│  ┌──────────────────────────────────────────────────────────┐   │  │
│  │ 5. Security Check                                        │◀──┘  │
│  │    SecurityPolicy.validate(tool_name, args, risk_level)  │      │
│  │    ReadOnly → deny if write  │  Supervised → ask user    │      │
│  │    Full → allow with rate limit                          │      │
│  └────────────────────┬─────────────────────────────────────┘      │
│                       │ approved                                    │
│  ┌────────────────────▼─────────────────────────────────────┐      │
│  │ 6. Execute Tool                                          │      │
│  │    ToolRegistry.execute(tool_name, args)                 │      │
│  │    Emit: AgentToolStart → AgentToolResult (via EventBus) │      │
│  └────────────────────┬─────────────────────────────────────┘      │
│                       │ result                                      │
│  ┌────────────────────▼─────────┐                                  │
│  │ 7. Append to History         │                                  │
│  │    tool_result → messages[]  │──── Loop back to step 2          │
│  │    iteration++ (max 20)      │    (until no more tool calls     │
│  └──────────────────────────────┘     or max iterations reached)   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
       │ final response
       ▼
┌──────────────────────────────────────┐
│  8. Store in Memory                  │
│     Key findings → Memory.store()    │
│     Daily summary → daily.md append  │
└──────────────────────────────────────┘
```

### Heartbeat / Scheduler Flow (New — P1.9)

```
App startup
    │
    ▼
┌────────────────────────────────────────────────────┐
│  Boot Sequence                                      │
│  1. Load identity files                             │
│  2. Warm up providers                               │
│  3. Start scheduler                                 │
│  4. Emit SystemEvent::Ready                         │
└────────────────────┬───────────────────────────────┘
                     │
                     ▼
┌────────────────────────────────────────────────────┐
│  Scheduler (tokio background tasks)                 │
│                                                     │
│  ┌──────────────────────┐  ┌─────────────────────┐ │
│  │  Heartbeat Timer     │  │  Cron Jobs          │ │
│  │  every 30 min        │  │  "0 9 * * MON-FRI"  │ │
│  │  (configurable)      │  │  (per-job schedule)  │ │
│  └──────────┬───────────┘  └──────────┬──────────┘ │
│             │                         │             │
│             ▼                         ▼             │
│  ┌────────────────────────────────────────────────┐ │
│  │  Event Bus: publish(HeartbeatTick / CronFired) │ │
│  └────────────────────┬───────────────────────────┘ │
└───────────────────────┼─────────────────────────────┘
                        │
          ┌─────────────┼─────────────┐
          ▼             ▼             ▼
   ┌────────────┐ ┌──────────┐ ┌───────────┐
   │ Agent Loop │ │  Notif   │ │  Tauri    │
   │ (isolated  │ │ Service  │ │  Bridge   │
   │  session)  │ │ (toast)  │ │ (→ UI)    │
   └────────────┘ └──────────┘ └───────────┘
```

---

## Event Bus Architecture (P1.8)

The Event Bus is the backbone connecting all subsystems. It replaces OpenClaw's WebSocket control plane for a desktop context.

```
                        ┌──────────────────┐
                        │    Event Bus     │
                        │ (tokio broadcast)│
                        └────────┬─────────┘
                                 │
         ┌───────────┬───────────┼───────────┬───────────┐
         │           │           │           │           │
    ┌────▼────┐ ┌────▼────┐ ┌───▼────┐ ┌────▼────┐ ┌───▼──────┐
    │  Agent  │ │Scheduler│ │Channel │ │ Memory  │ │  Tauri   │
    │  Loop   │ │         │ │Manager │ │ System  │ │  Bridge  │
    └─────────┘ └─────────┘ └────────┘ └─────────┘ └──────────┘

Publishes:          Publishes:       Publishes:       Publishes:       Subscribes:
AgentToolStart     HeartbeatTick    ChannelMessage   MemoryStored     All events →
AgentToolResult    CronFired                         MemoryRecalled   emit to frontend
ApprovalNeeded
```

### Event Types

```rust
enum AppEvent {
    // Agent events
    AgentToolStart { session_id, tool_name, args },
    AgentToolResult { session_id, tool_name, result, duration },
    AgentComplete { session_id, summary },
    ApprovalNeeded { action, risk_level, timeout },
    ApprovalResponse { approved, action },

    // Scheduler events
    HeartbeatTick { checks: Vec<CheckResult> },
    CronFired { job_id, payload },

    // Channel events
    ChannelMessage { session_key, channel, content },

    // Memory events
    MemoryStored { key, category },
    MemoryRecalled { query, result_count },

    // System events
    SystemReady,
    SystemError { module, message },
    ProviderHealthChange { provider, healthy },
}
```

---

## Security Architecture (P1.6)

Adapted from ZeroClaw's 6-layer model for desktop context:

```
┌─────────────────────────────────────────────────────┐
│  Layer 1: Credential Security                        │
│  OS Keyring (keyring crate) + zeroize memory clear  │
│  API keys never touch disk or logs                   │
├─────────────────────────────────────────────────────┤
│  Layer 2: Command Validation                         │
│  3 autonomy levels:                                  │
│    ReadOnly  → read commands only                    │
│    Supervised → approve medium/high risk             │
│    Full      → all ops with rate limiting            │
│  Command risk: Low (ls,cat) / Med (git) / High (rm) │
├─────────────────────────────────────────────────────┤
│  Layer 3: Filesystem Sandboxing                      │
│  Workspace-restricted access                         │
│  Blocked: /etc, /root, ~/.ssh, ~/.aws, ~/.gnupg     │
│  Path traversal prevention (.., null bytes, symlink) │
├─────────────────────────────────────────────────────┤
│  Layer 4: Injection Protection                       │
│  Block: backticks, $(), ${}, >, >>, pipe splitting  │
│  Sanitize all user/LLM-provided command strings      │
├─────────────────────────────────────────────────────┤
│  Layer 5: Rate Limiting                              │
│  Sliding window: 20 actions/hour default             │
│  Configurable per autonomy level                     │
├─────────────────────────────────────────────────────┤
│  Layer 6: Audit Trail                                │
│  All tool executions logged with timestamp + result  │
│  Security events emitted to Event Bus                │
└─────────────────────────────────────────────────────┘
```

---

## Storage Architecture

```
~/.mesoclaw/                          # Application data directory
├── config.toml                        # User configuration (TOML + env overrides)
├── app.db                             # SQLite database
│   ├── chat_sessions                  #   Session metadata
│   ├── chat_messages                  #   Conversation history
│   ├── memory_entries                 #   Vector embeddings + metadata
│   ├── memory_fts                     #   FTS5 virtual table for keyword search
│   ├── scheduled_jobs                 #   Cron/heartbeat job definitions
│   └── app_settings                   #   Runtime settings
├── identity/                          # Agent personality (markdown)
│   ├── SOUL.md                        #   Core personality + boundaries
│   ├── USER.md                        #   User preferences + context
│   ├── AGENTS.md                      #   Operating instructions
│   ├── IDENTITY.md                    #   Agent name, avatar, description
│   ├── HEARTBEAT.md                   #   Heartbeat monitoring checklist
│   ├── BOOT.md                        #   Startup checklist
│   └── TOOLS.md                       #   Tool usage guidance
├── memory/                            # Daily memory (markdown)
│   ├── MEMORY.md                      #   Curated long-term memory
│   ├── 2026-02-15.md                  #   Yesterday's summary
│   └── 2026-02-16.md                  #   Today's summary
├── prompts/                           # Custom prompt templates
│   └── *.md                           #   User-created prompt files
├── modules/                          # Sidecar modules (user-installed)
│   └── {module-name}/
│       ├── manifest.toml             #   Module definition
│       └── ...                       #   Module code/config
├── module-cache/                     # Container image cache metadata
└── logs/                              # Audit + debug logs
    └── audit.jsonl                    #   Tool execution audit trail
```

---

## Messaging Channel Architecture

```
                    External Messaging Platforms
                    ────────────────────────────
                    │              │              │
             ┌──────▼──────┐ ┌────▼─────┐ ┌─────▼──────┐
             │  Telegram   │ │ WhatsApp │ │  Discord   │
             │  Bot API    │ │ Web API  │ │  Bot API   │
             │  (P7.1)     │ │ (Future) │ │  (Future)  │
             └──────┬──────┘ └────┬─────┘ └─────┬──────┘
                    │             │              │
             ┌──────▼─────────────▼──────────────▼──────┐
             │          Channel Manager                  │
             │  Lifecycle: start/stop/reconnect          │
             │  Health monitoring per channel             │
             │  Message format normalization              │
             └────────────────────┬─────────────────────┘
                                  │ ChannelMessage
             ┌────────────────────▼─────────────────────┐
             │              Event Bus                    │
             │  Routes messages to correct session       │
             └────────────────────┬─────────────────────┘
                                  │
                    ┌─────────────┼─────────────┐
                    ▼             ▼             ▼
             ┌───────────┐ ┌──────────┐ ┌────────────┐
             │ Session   │ │  Agent   │ │  Approval  │
             │ Router    │ │  Loop    │ │  (Desktop) │
             └───────────┘ └──────────┘ └────────────┘
```

### Telegram Channel Detail (P7.1)

```
┌─────────────────────────────────────────────────────────────┐
│  TelegramChannel                                             │
│                                                              │
│  ┌──────────────┐     ┌────────────────┐                    │
│  │ Long-Polling  │────▶│ Message Parser │                    │
│  │ Listener      │     │ (text, photo,  │                    │
│  │ (tokio task)  │     │  document,     │                    │
│  │               │     │  voice, cmd)   │                    │
│  └──────────────┘     └───────┬────────┘                    │
│                               │                              │
│                    ┌──────────▼──────────┐                   │
│                    │ Normalize to        │                   │
│                    │ ChannelMessage {    │                   │
│                    │   channel: "telegram"│                  │
│                    │   peer: chat_id     │                   │
│                    │   content: text     │                   │
│                    │   attachments: []   │                   │
│                    │ }                    │                   │
│                    └──────────┬──────────┘                   │
│                               │ → EventBus                   │
│                                                              │
│  ┌──────────────────────────────────────┐                   │
│  │ send(message, recipient)             │                   │
│  │  → Format markdown for Telegram      │                   │
│  │  → Split long messages (4096 char)   │                   │
│  │  → POST /sendMessage to Bot API      │                   │
│  └──────────────────────────────────────┘                   │
│                                                              │
│  Config:                                                     │
│    bot_token: String  (stored in OS keyring)                │
│    allowed_chat_ids: Vec<i64>  (security: who can talk)     │
│    polling_timeout: u32  (default: 30s)                      │
│    parse_mode: "MarkdownV2"                                  │
└─────────────────────────────────────────────────────────────┘
```

### Channel Security Model

```
Inbound message from Telegram/WhatsApp
           │
           ▼
┌─────────────────────────────────┐
│  1. Channel Authentication      │
│     Telegram: allowed_chat_ids  │
│     WhatsApp: allowed_numbers   │
│     Unknown sender → reject     │
├─────────────────────────────────┤
│  2. Session Routing             │
│     telegram:{chat_id} → session│
│     Each chat = separate session│
├─────────────────────────────────┤
│  3. Agent Loop + Security Policy│
│     Same rules as desktop       │
│     Approvals → Desktop only    │
│     (never approve via channel) │
├─────────────────────────────────┤
│  4. Response Formatting         │
│     Adapt output per channel    │
│     Telegram: MarkdownV2        │
│     WhatsApp: basic formatting  │
└─────────────────────────────────┘
```

---

## Responsive Layout Architecture (Mobile-Ready)

The frontend uses a responsive layout system designed for Tauri Mobile from day one.

```
Desktop (>1024px)                    Tablet (768-1024px)
┌────────┬───────────────┬────────┐  ┌────────┬──────────────┐
│Sidebar │   Chat Area   │ Detail │  │Sidebar │  Chat Area   │
│        │               │ Panel  │  │(toggle)│              │
│ Nav    │  Messages     │ Memory │  │        │  Messages    │
│ Chans  │  Input        │ Tools  │  │        │  Input       │
│ Jobs   │               │ Agent  │  │        │              │
│        │               │ Status │  │        │              │
└────────┴───────────────┴────────┘  └────────┴──────────────┘

Mobile (<768px)
┌──────────────────┐
│    Chat Area     │
│                  │
│   Messages       │
│                  │
│                  │
├──────────────────┤
│   Input          │
├──────────────────┤
│ 🏠  💬  🔍  ⚙️  │  ← Bottom navigation bar
└──────────────────┘
```

### Breakpoint System

| Breakpoint  | Name | Layout              | Columns                 | Navigation                            |
| ----------- | ---- | ------------------- | ----------------------- | ------------------------------------- |
| >1280px     | `xl` | Full 3-panel        | Sidebar + Chat + Detail | Persistent sidebar                    |
| 1024-1280px | `lg` | 2-panel + overlay   | Sidebar + Chat          | Persistent sidebar, detail as overlay |
| 768-1024px  | `md` | 2-panel collapsible | Toggle sidebar + Chat   | Hamburger menu                        |
| 640-768px   | `sm` | Single + drawer     | Chat only               | Bottom nav + drawer                   |
| <640px      | `xs` | Single compact      | Chat only               | Bottom nav + drawer                   |

### Component Responsive Behavior

| Component            | Desktop                          | Tablet                           | Mobile                            |
| -------------------- | -------------------------------- | -------------------------------- | --------------------------------- |
| **Sidebar**          | Always visible, 256px wide       | Collapsible (hamburger), overlay | Hidden, drawer from left          |
| **Chat messages**    | Full width with max-width 800px  | Full width                       | Full width, compact spacing       |
| **PromptInput**      | Bottom of chat area              | Bottom of chat area              | Fixed bottom with safe area inset |
| **Settings**         | Tabbed panel in sidebar or route | Full-screen route                | Full-screen route                 |
| **Tool execution**   | Inline in chat + detail panel    | Inline in chat                   | Inline in chat, expandable        |
| **Approval overlay** | Centered modal                   | Centered modal                   | Bottom sheet                      |
| **Memory search**    | Detail panel (right)             | Overlay panel                    | Full-screen route                 |
| **Scheduler**        | Settings tab                     | Full-screen route                | Full-screen route                 |
| **System tray**      | Native OS tray                   | N/A (mobile)                     | N/A (mobile)                      |
| **Notifications**    | Desktop native toast             | Mobile native push (APNs/FCM)    | Mobile native push                |

### Tailwind CSS 4 Responsive Patterns

```tsx
// Layout container — responsive grid
<div className="grid grid-cols-1 md:grid-cols-[256px_1fr] xl:grid-cols-[256px_1fr_320px]">
  <Sidebar className="hidden md:block" />
  <ChatArea />
  <DetailPanel className="hidden xl:block" />
</div>

// Mobile bottom nav — only visible on small screens
<nav className="fixed bottom-0 inset-x-0 md:hidden flex justify-around
               bg-background border-t safe-area-pb">
  <NavItem icon={Home} label="Chat" />
  <NavItem icon={MessageSquare} label="Channels" />
  <NavItem icon={Search} label="Memory" />
  <NavItem icon={Settings} label="Settings" />
</nav>

// Chat input — safe area for mobile notch/home indicator
<div className="sticky bottom-0 pb-safe">
  <PromptInput />
</div>
```

### Mobile-Specific Considerations

| Concern                                | Solution                                                      |
| -------------------------------------- | ------------------------------------------------------------- |
| **Safe areas** (notch, home indicator) | `env(safe-area-inset-*)` via Tailwind `safe-area-*` utilities |
| **Virtual keyboard**                   | `visualViewport` API to resize chat on keyboard open          |
| **Touch targets**                      | Minimum 44x44px for all interactive elements                  |
| **Swipe gestures**                     | Swipe right → open sidebar. Swipe left → close sidebar        |
| **Pull to refresh**                    | Pull down in chat → load older messages                       |
| **Haptic feedback**                    | Tauri mobile APIs for button press feedback                   |
| **Dark mode**                          | `prefers-color-scheme` + manual toggle (already exists)       |
| **Offline mode**                       | Queue messages locally, send when back online                 |

---

## CI/CD Pipeline Architecture

### Build Matrix

```
                              GitHub Actions
                    ┌──────────────────────────────────────┐
                    │                                       │
   CI (Every PR)    │  ┌─────────┐ ┌─────────┐ ┌────────┐ │
                    │  │ Ubuntu  │ │  macOS  │ │Windows │ │
                    │  │ clippy  │ │ clippy  │ │clippy  │ │
                    │  │ test    │ │ test    │ │test    │ │
                    │  │ fmt     │ │ fmt     │ │fmt     │ │
                    │  └─────────┘ └─────────┘ └────────┘ │
                    │                                       │
   Release          │  ┌───────────────────────────────┐   │
   (Manual)         │  │     Create Draft Release      │   │
                    │  └──────────────┬────────────────┘   │
                    │                 │                      │
                    │  ┌──────────────▼────────────────┐   │
                    │  │    Parallel Build Matrix       │   │
                    │  │                                │   │
                    │  │  macOS aarch64  (sign+notarize)│   │
                    │  │  macOS x86_64   (sign+notarize)│   │
                    │  │  macOS Universal (lipo)        │   │
                    │  │  Windows x64    (Azure sign)   │   │
                    │  │  Windows ARM64  (Azure sign)   │   │
                    │  │  Ubuntu 22.04   (.deb)         │   │
                    │  │  Ubuntu 24.04   (AppImage+RPM) │   │
                    │  │  Linux ARM64    (.deb+AppImage)│   │
                    │  └──────────────┬────────────────┘   │
                    │                 │                      │
                    │  ┌──────────────▼────────────────┐   │
                    │  │  Upload All Artifacts to       │   │
                    │  │  GitHub Release                │   │
                    │  └───────────────────────────────┘   │
                    │                                       │
   Mobile           │  ┌─────────┐      ┌─────────────┐   │
   (Manual)         │  │   iOS   │      │   Android   │   │
                    │  │ arm64   │      │ arm64-v8a   │   │
                    │  │→TestFlght│      │ armeabi-v7a │   │
                    │  │         │      │ x86_64      │   │
                    │  │         │      │→Play Console│   │
                    │  └─────────┘      └─────────────┘   │
                    └──────────────────────────────────────┘
```

### Platform Support Matrix

| Platform | Architecture  | Target Triple               | Bundle         | Signing               |
| -------- | ------------- | --------------------------- | -------------- | --------------------- |
| macOS    | Apple Silicon | `aarch64-apple-darwin`      | DMG, APP       | Apple notarization    |
| macOS    | Intel         | `x86_64-apple-darwin`       | DMG, APP       | Apple notarization    |
| macOS    | Universal     | `lipo` of both              | DMG, APP       | Apple notarization    |
| Windows  | x64           | `x86_64-pc-windows-msvc`    | MSI, NSIS      | Azure Trusted Signing |
| Windows  | ARM64         | `aarch64-pc-windows-msvc`   | MSI, NSIS      | Azure Trusted Signing |
| Linux    | x64 (22.04)   | `x86_64-unknown-linux-gnu`  | DEB            | —                     |
| Linux    | x64 (24.04)   | `x86_64-unknown-linux-gnu`  | AppImage, RPM  | —                     |
| Linux    | ARM64         | `aarch64-unknown-linux-gnu` | DEB, AppImage  | —                     |
| iOS      | arm64         | `aarch64-apple-ios`         | IPA            | Apple distribution    |
| Android  | arm64-v8a     | `aarch64-linux-android`     | AAB, APK       | Keystore              |
| Android  | armeabi-v7a   | `armv7-linux-androideabi`   | AAB, APK       | Keystore              |
| Android  | x86_64        | `x86_64-linux-android`      | APK (emulator) | Debug                 |

**Not supported**: 32-bit x86 on any platform. All modern OSes are 64-bit.

### Contribution Workflow

```
Contributor                    Automation                    Maintainer
──────────                    ──────────                    ──────────
Fork + Branch                      │
     │                             │
     ▼                             │
Open Issue                         │
(bug_report.yml or                 │
 feature_request.yml)              │
     │                             │
     ▼                             │
Create PR                         │
(pull_request_template.md)        │
     │                             ▼
     │                    ┌─────────────────┐
     │                    │  Auto-label     │
     │                    │  (by files)     │
     │                    │                 │
     │                    │  CI pipeline    │
     │                    │  (lint+test+fmt)│
     │                    │                 │
     │                    │  Size label     │
     │                    │  (XS/S/M/L/XL) │
     │                    └────────┬────────┘
     │                             │
     │                             ▼                    ┌──────────────┐
     │                    CI Pass? ──── No ───────────▶ │ Fix & repush │
     │                       │                          └──────────────┘
     │                      Yes
     │                       │                          ┌──────────────┐
     │                       ▼                    ┌────▶│ Code Review  │
     │               ┌───────────────┐            │     │ (risk-based) │
     │               │ Risk routing  │────────────┘     │ A: 1 reviewer│
     │               │ A / B / C     │                  │ B: 1 + test  │
     │               └───────────────┘                  │ C: 2-pass    │
     │                                                  └──────┬───────┘
     │                                                         │
     ▼                                                         ▼
  Approved? ◄──────────────────────────────────────── Squash Merge
     │
    Done
```

---

## Sidecar Module Architecture

### Module System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     ToolRegistry (existing)                     │
├─────────────────────────────────────────────────────────────────┤
│  Built-in Tools          │  Sidecar Modules                    │
│  ├─ shell                │  ├─ SidecarTool (on-demand)         │
│  ├─ file_read            │  │   Protocol: stdin/stdout JSON    │
│  ├─ file_write           │  ├─ SidecarService (long-lived)     │
│  ├─ file_list            │  │   Protocol: HTTP REST             │
│  ├─ memory_store         │  └─ McpServer (MCP protocol)        │
│  ├─ memory_recall        │      Protocol: JSON-RPC stdin/stdout │
│  ├─ memory_forget        │                                      │
│  └─ spawn_agent          │  Container Runtime Abstraction       │
│                          │  Podman → Docker → Native fallback   │
└─────────────────────────────────────────────────────────────────┘
```

### Module Execution Flow

```
Agent requests sidecar tool
        │
        ▼
   ToolRegistry.get(tool_name)
        │
        ▼
   SidecarModule resolved
        │
   ┌────┴────────────┬───────────────────┐
   │ SidecarTool     │ SidecarService    │ McpServer
   │ (on-demand)     │ (long-lived)      │ (long-lived)
   │                 │                    │
   ▼                 ▼                    ▼
┌────────────┐  ┌──────────────┐  ┌──────────────┐
│ Check      │  │ HTTP POST to │  │ JSON-RPC     │
│ runtime:   │  │ /execute     │  │ tools/call   │
│ native?    │  │ endpoint     │  │ to MCP server│
│ container? │  └──────────────┘  └──────────────┘
└─────┬──────┘
      │
 ┌────┴────────┐
 │ native      │ container
 │             │
 ▼             ▼
Process      ContainerRuntime
.spawn()     .run(config)
stdin/stdout  stdin/stdout
JSON          JSON (inside container)
```

### Container Runtime Architecture

```
┌─────────────────────────────────────────────────┐
│           ContainerRuntime Trait                  │
│  is_available() · pull_image() · run()           │
│  stop() · exec()                                 │
├─────────────────┬───────────────────────────────┤
│  PodmanRuntime   │  DockerRuntime               │
│  (preferred)     │  (fallback)                  │
│  rootless        │  requires daemon             │
│  podman socket   │  docker socket               │
│  bollard crate   │  bollard crate               │
└─────────────────┴───────────────────────────────┘
                    │
         Auto-detection priority:
         1. podman binary → PodmanRuntime
         2. docker binary → DockerRuntime
         3. neither → native fallback (warn)
         4. config.toml override
```

### Module Manifest & Discovery

```
~/.mesoclaw/
├── modules/                          ← User-installed modules
│   ├── python-analyst/
│   │   ├── manifest.toml            ← [module] name, type, desc
│   │   ├── main.py                  ←   [runtime] command, args
│   │   └── requirements.txt         ←   [tool] input_schema
│   ├── node-transformer/            ←   [security] paths, network, timeout
│   │   ├── manifest.toml
│   │   └── index.js
│   └── composio-tools/
│       └── manifest.toml            ← MCP server (no code, just config)
├── module-cache/                     ← Container image cache metadata
└── config.toml                       ← [modules] preferred_runtime
```

### Module Security Model

```
┌─────────────────────────────────────────────────────┐
│  Sidecar Security Layers                             │
├─────────────────────────────────────────────────────┤
│  Layer 1: Manifest Security Constraints              │
│    allowed_paths, network flag, timeout, memory limit│
├─────────────────────────────────────────────────────┤
│  Layer 2: Container Isolation (if container runtime) │
│    --network=none, --memory limit, volume mounts     │
├─────────────────────────────────────────────────────┤
│  Layer 3: SecurityPolicy Integration                 │
│    Per-identity tools.toml controls module access     │
├─────────────────────────────────────────────────────┤
│  Layer 4: Input Validation                           │
│    Manifest input_schema enforced before execution    │
├─────────────────────────────────────────────────────┤
│  Layer 5: Audit Trail                                │
│    ModuleToolStart/ModuleToolResult → audit.jsonl     │
└─────────────────────────────────────────────────────┘
```

### Composio.dev Integration via MCP

```
Mesoclaw                      Composio MCP Server
┌──────────────┐              ┌──────────────────┐
│ MCP Client   │──initialize──▶│ composio-core    │
│ (mcp_client  │              │ mcp-server        │
│  .rs)        │◀─capabilities─│                  │
│              │              │                    │
│              │──tools/list──▶│ Discovers:        │
│              │◀─tool_list────│ gmail_send        │
│              │              │ github_create_issue│
│              │              │ slack_post         │
│              │              │ notion_update      │
│ Agent calls  │              │ ... 500+ tools     │
│ mcp:composio │              │                    │
│ :gmail_send  │──tools/call──▶│                   │
│              │◀─result───────│ Handles OAuth,    │
│              │              │ API calls, tokens  │
└──────────────┘              └──────────────────┘
```

---

## Frontend Component Architecture

```
src/
├── routes/                            # TanStack Router (file-based)
│   ├── index.tsx                      #   Main chat interface
│   ├── settings.tsx                   #   Settings (providers, identity, scheduler)
│   ├── memory.tsx                     #   Memory search + browse
│   └── __root.tsx                     #   Root layout (sidebar, tray, notifications)
│
├── stores/                            # Zustand state management
│   ├── agentStore.ts                  #   Agent loop state, tool execution status
│   ├── providerStore.ts              #   LLM provider config, health status
│   ├── memoryStore.ts                #   Memory search state, results
│   ├── schedulerStore.ts             #   Job list, execution history
│   ├── identityStore.ts             #   Identity file contents
│   ├── settingsStore.ts             #   App settings
│   └── themeStore.ts                #   Theme (light/dark)
│
├── components/
│   ├── ui/                           #   Base UI primitives (shadcn-style)
│   ├── ai-elements/                  #   Chat components (Conversation, Message, etc.)
│   ├── agent/                        #   Agent loop UI
│   │   ├── ToolExecutionStatus.tsx   #     Tool name + args + spinner
│   │   ├── ApprovalOverlay.tsx       #     Security approval dialog
│   │   └── AgentProgress.tsx         #     Multi-turn progress indicator
│   ├── memory/                       #   Memory UI
│   │   ├── MemorySearch.tsx          #     Search input + results
│   │   └── DailyTimeline.tsx         #     Daily memory file browser
│   ├── scheduler/                    #   Scheduler UI
│   │   ├── JobList.tsx               #     Active/paused/completed jobs
│   │   └── CronBuilder.tsx           #     Visual cron expression builder
│   ├── identity/                     #   Identity UI
│   │   └── IdentityEditor.tsx        #     Markdown editor for identity files
│   └── settings/                     #   Settings panels
│       ├── ProviderConfig.tsx
│       ├── SecurityConfig.tsx
│       └── NotificationConfig.tsx
│
└── lib/
    ├── gateway-client.ts             #   Typed REST + WebSocket client
    └── ws-events.ts                  #   WebSocket event subscription helpers
```

---

## Technology Stack Summary

| Layer                  | Technology                                 | Purpose                                               |
| ---------------------- | ------------------------------------------ | ----------------------------------------------------- |
| **Desktop Shell**      | Tauri 2                                    | Native window, IPC bridge, system tray, notifications |
| **Frontend Framework** | React 19                                   | UI rendering                                          |
| **Build Tool**         | Vite                                       | Frontend bundling, HMR                                |
| **Routing**            | TanStack Router                            | File-based routing                                    |
| **State**              | Zustand                                    | Lightweight stores                                    |
| **Styling**            | Tailwind CSS 4                             | Utility-first CSS                                     |
| **Backend Language**   | Rust 2024                                  | Performance, safety                                   |
| **Async Runtime**      | Tokio                                      | Async I/O, background tasks, scheduling               |
| **LLM Client**         | async-openai                               | OpenAI-compatible API calls                           |
| **Database**           | SQLite (rusqlite)                          | App data, vector storage, FTS5                        |
| **Secrets**            | OS Keyring + zeroize                       | Secure credential storage                             |
| **Templates**          | Tera                                       | Prompt template rendering                             |
| **Serialization**      | serde + serde_json + toml                  | Data serialization                                    |
| **Error Handling**     | thiserror + anyhow                         | Typed + ad-hoc errors                                 |
| **Logging**            | tracing                                    | Structured logging                                    |
| **HTTP**               | reqwest (rustls)                           | Embedding API calls                                   |
| **HTTP Server**        | axum                                       | Gateway/control plane + webhook listener              |
| **Sidecar Protocol**   | Stdin/Stdout JSON + MCP (JSON-RPC)         | Module communication                                  |
| **Container Runtime**  | bollard (Docker/Podman API)                | Container-based module isolation                      |
| **Manifest Format**    | TOML                                       | Module definition and configuration                   |
| **CI/CD**              | GitHub Actions                             | Multi-platform builds, testing, releases              |
| **Package Manager**    | Bun                                        | Frontend dependency management                        |
| **Code Signing**       | Apple Notarization + Azure Trusted Signing | macOS + Windows binary signing                        |
| **Mobile Build**       | Tauri Mobile (iOS/Android)                 | Same codebase compiles to mobile                      |

---

_Document created: February 2026_
_References: docs/claw-ecosystem-analysis.md, docs/mesoclaw-gap-analysis.md_
