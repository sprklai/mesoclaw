# Mesoclaw Gap Analysis: Current State vs. Recommended Architecture

> Compares existing Mesoclaw codebase against patterns recommended in `docs/claw-ecosystem-analysis.md`.
> Reference: ZeroClaw (Rust), PicoClaw (Go), IronClaw (Rust), OpenClaw (TypeScript).
> **Reconciliation note (Feb 16, 2026)**: Architecture decisions in this document are partially superseded by `docs/plans/2026-02-16-cli-gateway-agents-design.md` and `docs/plans/2026-02-16-sidecar-modularity-design.md`. This file remains valuable for gap identification and implementation rationale.

---

## What Already Exists (Baseline Snapshot)

| Recommendation                                 | Current Status                                                                                         | Location                                          |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------- |
| `LLMProvider` trait                            | Already exists with `complete()`, `stream()`, `context_limit()`, `supports_tools()`, `provider_name()` | `src-tauri/src/ai/provider.rs`                    |
| Provider implementations                       | 3 implementations: OpenAI-compatible, Vercel AI Gateway, OpenRouter                                    | `src-tauri/src/ai/providers/`                     |
| `ApplicationAdapter` trait                     | Already exists - domain-agnostic adapter for skills engine                                             | `src-tauri/src/adapters/traits.rs`                |
| Skills system                                  | Fully built - loader, registry, selector, composer, executor, embedded skills                          | `src-tauri/src/skills/`                           |
| OS keyring credential storage                  | Already exists with `zeroize` for memory clearing                                                      | `src-tauri/src/services/credential_store.rs`      |
| Diesel ORM + SQLite                            | Already exists with r2d2 pool (10 connections), embedded migrations                                    | `src-tauri/src/database/`                         |
| Tauri IPC commands layer (legacy control path) | 18+ commands registered, clean `Result<T, String>` pattern                                             | `src-tauri/src/commands/`                         |
| `thiserror` typed errors                       | Already used across DbError, CredentialError, SkillError, AdapterError                                 | Multiple modules                                  |
| Streaming chat (SSE)                           | Already implemented - backend streams tokens via Tauri events                                          | `commands/streaming_chat.rs`                      |
| Frontend stores (Zustand)                      | 5 stores: LLM, Skills, AppSettings, Settings, Theme                                                    | `src/stores/`                                     |
| AI Elements components                         | Full component library - Conversation, PromptInput, Message, CodeBlock, Artifact, ModelSelector        | `src/components/ai-elements/`                     |
| Stronghold + Keyring dual security             | Already multi-layer - Stronghold (frontend), Keyring (backend), zeroize                                | `lib/keychain-storage.ts` + `credential_store.rs` |

---

## Architecture Overview (Gateway-Ready / Approach B)

Mesoclaw follows a **Gateway-Ready** architecture: trait-based abstractions for all subsystems, shipping with a localhost HTTP/WebSocket gateway as the default control plane, while Tauri IPC remains for desktop-native integrations (window/tray/notifications).

```
┌──────────────────────────────────────────────────────┐
│              Frontend (React/TypeScript)              │
│  Routes • Stores • Components • Desktop Notifications│
└────────────────────────┬─────────────────────────────┘
                         │ HTTP REST + WebSocket (localhost gateway)
┌────────────────────────▼─────────────────────────────┐
│            Event Bus / Message Router                 │
│  (EventBus trait → tokio::broadcast + gateway stream) │
├────────────┬─────────────┬───────────────────────────┤
│  Agent     │  Scheduler  │  Channel Manager          │
│  Loop      │  (heartbeat │  (tauri ipc, webhook,     │
│  (P0.3)    │   + cron)   │   (tauri, telegram, webhook) │
├────────────┴─────────────┴───────────────────────────┤
│            Core Services                              │
│  Providers • Memory • Tools • Security • Identity    │
├──────────────────────────────────────────────────────┤
│            Identity / Config Layer                    │
│  SOUL.md • USER.md • AGENTS.md • config.toml        │
│  memory/YYYY-MM-DD.md • SQLite + FTS5               │
└──────────────────────────────────────────────────────┘
```

**Key principle**: Every subsystem is a trait. Default implementations use gateway + Tokio-native mechanisms. Future implementations can use additional channels or platform SDKs without touching consumer code.

**Desktop vs Server distinction**: Unlike OpenClaw (server-based with remote control plane), Mesoclaw is desktop-first with a localhost gateway (`127.0.0.1`) and internal Event Bus routing.

---

## Changes Required

### Pre-P0 — Slim Down (Technical Debt Reduction)

> **Rationale**: Codebase audit (see `docs/moltis-microclaw-analysis.md`) found that 49% of backend code (4,429 of 9,152 lines) is over-engineered or duplicated. Adding 25 new features on top of this creates unmaintainable tech debt. Slim down first, grow second.

#### S1. Replace Skills System with Simple Prompt Templates

**Current state**: 2,631 lines across 10 files (loader.rs, registry.rs, selector.rs, composer.rs, executor.rs, types.rs, error.rs, settings.rs, state.rs, mod.rs). Custom YAML frontmatter parser, LLM-based skill selector, skill composition/inheritance engine, dynamic loading from 3 sources (embedded, local filesystem, remote).

**Problem**: This is 29% of the entire backend for a prompt template system. The LLM-based skill selector (326 lines) uses an AI call just to pick which prompt to use. The composer (283 lines) enables skill inheritance/composition that's never needed in practice.

**Replace with**:

- Prompt templates as `.md` files in `~/.mesoclaw/prompts/` (or embedded via `include_str!()`)
- `tera` or `handlebars` crate for variable substitution (1 dependency vs custom parser)
- Frontend selects prompts via simple dropdown (no LLM selection needed)
- Keep the `execute_skill_command` IPC signature — just simplify the backend

**Savings**: ~2,300 lines removed, 1 crate added (`tera`), faster startup (no LLM call for skill selection)

**Files to delete**: `src-tauri/src/skills/` (entire directory except `types.rs` if type definitions needed)
**Files to create**: `src-tauri/src/prompts/mod.rs`, `src-tauri/src/prompts/loader.rs` (~150 lines total)

---

#### S2. Consolidate AI Providers Using `async-openai`

**Current state**: 3 provider implementations totaling 1,798 lines — OpenAI-compatible (678), OpenRouter (566), Vercel AI Gateway (554). All three follow nearly identical patterns: build request → send HTTP → parse SSE stream → emit tokens. The differences are minor (base URL, auth header format, model name resolution).

**Problem**: 20% of the backend is copy-pasted HTTP request/response handling. When adding new providers, you'd copy the same pattern again. Bug fixes must be applied 3x.

**Replace with**:

- `async-openai` crate for OpenAI-compatible providers (handles streaming, tool calls, retries)
- Single `GenericProvider` struct parameterized by base URL + auth config
- Thin wrapper for non-OpenAI providers (Anthropic, Google) that maps to the common interface
- Keep `LLMProvider` trait — implementations just delegate to `async-openai` internally

**Savings**: ~1,200 lines removed, 1 crate added (`async-openai`)

**Files affected**:

- Delete: `src-tauri/src/ai/providers/openai_compatible.rs`, `openrouter.rs`, `vercel_gateway.rs`
- Create: `src-tauri/src/ai/providers/generic.rs` (~200 lines)
- Modify: `src-tauri/src/ai/providers/mod.rs` (update factory)

---

#### S3. Replace Diesel ORM with `rusqlite`

**Current state**: Diesel ORM with 3 crates (diesel, diesel_migrations, libsqlite3-sys), r2d2 connection pool (max 10 connections), code-generated schema.rs, 955 lines across 7 database files.

**Problem**: Diesel is a production-grade ORM designed for multi-database, multi-user server applications. Mesoclaw is a single-user desktop app using SQLite exclusively. The ORM adds:

- `schema.rs` code generation step
- Complex type conversions (FromSql/ToSql custom impls)
- Build-time dependency on libsqlite3-sys (C compilation)
- r2d2 connection pool (unnecessary for single-user SQLite)

**Replace with**:

- `rusqlite` with `bundled` feature (SQLite bundled, no system dependency)
- Direct SQL queries (SQLite queries are simple enough to not need an ORM)
- `rusqlite::Connection` wrapped in `Arc<Mutex<>>` (single connection sufficient for desktop)
- Keep migration files as `.sql` scripts, apply on startup

**Savings**: ~400 lines simplified, 2 crates removed (diesel, diesel_migrations), faster compilation, smaller binary

**Files affected**:

- Modify: `src-tauri/src/database/mod.rs`, all model files
- Delete: `src-tauri/src/database/schema.rs` (generated code)
- Modify: `src-tauri/Cargo.toml` (replace diesel with rusqlite)

**Note**: This is the most disruptive change. If Diesel is already well-tested and working, defer this to later. The skills and provider consolidation have better ROI.

---

#### S4. Adopt Strict Linting (from Moltis)

**Current state**: Default Clippy lints, no `deny` directives.

**Add to `src-tauri/Cargo.toml`**:

```toml
[lints.rust]
unsafe_code = "deny"

[lints.clippy]
unwrap_used = "warn"     # Escalate to "deny" once existing unwraps are fixed
expect_used = "warn"     # Escalate to "deny" once existing expects are fixed
```

**Savings**: 0 lines removed, but prevents future code bloat and panics in production. Moltis uses this config and it's one reason their code quality is higher.

**Files affected**: `src-tauri/Cargo.toml` only

---

### Slim-Down Summary

| Item                                     | Lines Saved | Crates Changed                         | Risk                                 |
| ---------------------------------------- | ----------- | -------------------------------------- | ------------------------------------ |
| S1. Simplify skills → prompt templates   | ~2,300      | +1 (tera), -0                          | Medium (rewrite existing feature)    |
| S2. Consolidate providers → async-openai | ~1,200      | +1 (async-openai), -0                  | Low (same trait, new implementation) |
| S3. Diesel → rusqlite                    | ~400        | +1 (rusqlite), -2 (diesel, migrations) | High (touches all DB code)           |
| S4. Strict linting                       | 0           | 0                                      | None                                 |
| **Total**                                | **~3,900**  | **net -0**                             |                                      |

**Recommended order**: S4 (linting) → S2 (providers) → S1 (skills) → S3 (diesel, defer if risky)

---

### P0 - Critical (Foundation)

#### 1. Add `ReliableProvider` Wrapper

**Gap**: No retry/fallback logic for LLM calls. If a provider fails, the error propagates directly to the frontend.

**What's needed**: A decorator struct wrapping `Arc<dyn LLMProvider>` that adds:

- Configurable retry count (default 3) with exponential backoff
- Fallback provider chain (e.g., try Anthropic → OpenAI → Ollama)
- Connection warmup (`warmup()` method added to trait)
- Timeout enforcement per attempt

**Pattern source**: ZeroClaw `src/providers/reliable.rs`

**Files affected**:

- `src-tauri/src/ai/provider.rs` - Add `warmup()` default method to `LLMProvider` trait
- New file: `src-tauri/src/ai/providers/reliable.rs` - `ReliableProvider` struct
- `src-tauri/src/ai/providers/mod.rs` - Wire up in provider factory

---

#### 2. Release Profile Optimization

**Gap**: No release optimizations in `Cargo.toml`. Default profile produces larger binaries than necessary.

**What's needed** in `src-tauri/Cargo.toml`:

```toml
[profile.release]
opt-level = "z"       # Optimize for size
lto = true            # Link-time optimization
codegen-units = 1     # Better optimization, slower compile
strip = true          # Remove debug symbols
panic = "abort"       # No unwind tables
```

**Pattern source**: ZeroClaw `Cargo.toml` — achieves 3.4 MB binary

**Files affected**: `src-tauri/Cargo.toml` only

**Impact**: Estimated 3-5x reduction in binary size with zero behavioral change.

---

#### 3. Add Agent Loop Module

**Gap**: No agent loop exists. Current chat is single-turn request/response only. The backend receives a message, calls the LLM once, streams the response, and stops. There is no autonomous multi-turn conversation with tool calling, no iterative reasoning.

**What's needed**: A new `agent/` module implementing:

- Multi-turn conversation manager (send message → LLM responds → if tool call, execute tool → feed result back → repeat)
- Tool call parsing (both XML-style `<tool_call>` and OpenAI-format `tool_calls` array)
- History trimming (cap at 50 messages to prevent context overflow)
- Max tool iterations per turn (cap at 10-20 to prevent runaway loops)
- Memory context injection (once memory system exists)
- Tauri event emission for each stage (tool execution, intermediate results)

**Pattern source**: ZeroClaw `src/agent/loop_.rs` (589 lines, 13 tests)

**Files affected**:

- New directory: `src-tauri/src/agent/`
- New files: `mod.rs`, `loop_.rs`, `tool_parser.rs`
- `src-tauri/src/lib.rs` - Add `pub mod agent;`
- `src-tauri/src/commands/` - New `agent_commands.rs` for IPC
- Frontend: New route or enhanced chat route for agent UI

**Dependencies on**: P1.4 (Tool trait), P1.6 (Security policy for safe tool execution)

---

### P1 - High Priority

#### 4. Add Tool Trait and Registry

**Gap**: The `ApplicationAdapter` trait has `execute_tool()` and `available_tools()`, but there is no standalone `Tool` trait for defining extensible, self-describing tools. Tools are tightly coupled to the adapter implementation rather than being independently registerable.

**What's needed**: A separate `Tool` trait (following ZeroClaw's pattern) with:

- `name() -> &str`
- `description() -> &str`
- `parameters_schema() -> serde_json::Value` (JSON Schema for LLM tool calling)
- `async execute(args: Value) -> Result<ToolResult>`
- `ToolRegistry` struct for dynamic tool registration and lookup
- Built-in tool implementations: shell command execution, file read/write, memory store/recall

**Pattern source**: ZeroClaw `src/tools/traits.rs` (50 lines per tool)

**Files affected**:

- New directory: `src-tauri/src/tools/`
- New files: `traits.rs`, `registry.rs`, `shell.rs`, `file_ops.rs`
- `src-tauri/src/lib.rs` - Add `pub mod tools;`
- `src-tauri/src/adapters/traits.rs` - Refactor `execute_tool()` to delegate to `ToolRegistry`

**Relationship to existing code**: The `ApplicationAdapter` trait already defines `ToolCall` and `ToolResult` types. The new `Tool` trait would provide the implementations that the adapter dispatches to.

---

#### 5. Add Memory/Knowledge System

**Gap**: The database layer handles only app data (settings, providers, chat sessions). There is no vector memory, no semantic search, no embedding storage, no knowledge persistence across conversations.

**What's needed**:

- `Memory` trait with `store()`, `recall()`, `forget()` methods
- SQLite-based vector storage (embeddings stored as BLOBs in a new table)
- FTS5 full-text search with BM25 scoring (SQLite extension, already bundled)
- Hybrid merge algorithm: `final_score = 0.7 * vector_similarity + 0.3 * bm25_score`
- LRU embedding cache (default 10,000 entries) using existing `lru` crate (already in Cargo.toml)
- Document chunker for splitting long inputs before embedding
- Memory hygiene: auto-archive >7 days, purge >30 days
- Memory categories: Core, Daily, Conversation, Custom

**Pattern source**: ZeroClaw `src/memory/` (sqlite.rs, embeddings.rs, chunker.rs, hygiene.rs)

**Files affected**:

- New directory: `src-tauri/src/memory/`
- New files: `traits.rs`, `sqlite.rs`, `embeddings.rs`, `chunker.rs`, `hygiene.rs`
- `src-tauri/src/lib.rs` - Add `pub mod memory;`
- `src-tauri/src/database/` - New Diesel migrations for:
  - `memory_entries` table (id, key, content, category, embedding BLOB, created_at)
  - FTS5 virtual table for keyword search
- New file: `src-tauri/src/commands/memory_commands.rs`
- Frontend: New memory search UI (store + route)

**Note**: The `lru` crate (v0.12) is already a dependency. The `reqwest` crate (already present) can call embedding APIs (OpenAI, Ollama).

---

#### 6. Add Security Policy Module

**Gap**: Credential storage is secure (keyring + zeroize), but there is no command validation, no path traversal prevention, no filesystem sandboxing. If the agent loop (P0.3) is added and it can execute shell commands via tools, operating without a security policy is dangerous.

**What's needed**:

- `SecurityPolicy` struct with configurable autonomy levels:
  - `ReadOnly` - Can only read files and run safe commands
  - `Supervised` - Requires user approval for medium/high risk
  - `Full` - All operations permitted (with rate limiting)
- Command risk classification:
  - Low: ls, cat, grep, git status (read-only)
  - Medium: git commit, npm install, mkdir (state-changing)
  - High: rm, sudo, curl, wget, chmod (destructive)
- Injection protection: block backticks, `$()`, `${}`, `>`, `>>`, pipe splitting
- Path traversal prevention: block `..`, null bytes, URL-encoded traversal, symlink escape
- Blocked system directories: `/etc`, `/root`, `/proc`, `/sys`, `/dev`, `~/.ssh`, `~/.gnupg`, `~/.aws`
- Rate limiting: sliding window, configurable actions/hour

**Pattern source**: ZeroClaw `src/security/policy.rs` (1,100+ lines, 130+ tests)

**Files affected**:

- New directory: `src-tauri/src/security/`
- New files: `mod.rs`, `policy.rs`
- `src-tauri/src/lib.rs` - Add `pub mod security;`
- Agent loop (P0.3) would call `SecurityPolicy::validate_command()` before executing any tool
- Frontend: Approval overlay UI for supervised mode

---

#### 7. Add Channel Trait

**Gap**: No channel abstraction for external sources. Current behavior is desktop-local only, with no unified adapter model for Telegram/webhook/future channels.

**What's needed**:

- `Channel` trait with:
  - `name() -> &str`
  - `async send(message, recipient) -> Result<()>`
  - `async listen(tx: mpsc::Sender<ChannelMessage>) -> Result<()>`
  - `health_check() -> bool`
- HTTP webhook channel implementation (using axum)
- WebSocket gateway (optional, for remote/LAN access)
- Channel manager for lifecycle (start, stop, health monitoring)
- Tauri WebView as a "channel" (wrapping existing IPC)

**Pattern source**: ZeroClaw `src/channels/traits.rs`, PicoClaw `pkg/channels/`

**Files affected**:

- New directory: `src-tauri/src/channels/`
- New files: `traits.rs`, `webhook.rs`, `mod.rs`
- `src-tauri/src/lib.rs` - Add `pub mod channels;`
- `src-tauri/Cargo.toml` - Add `axum` (0.8) + `tower-http` for gateway and webhook channel support

**Note**: This is lower priority than the agent loop, tools, and memory. Only needed if the app should accept input from sources other than the Tauri WebView.

---

#### 8. Add Event Bus / Message Router

**Gap**: No internal event routing between subsystems. The agent loop, scheduler, channels, and memory system have no way to communicate asynchronously. Currently, all communication flows through Tauri IPC commands (frontend → backend), with no backend-to-backend event routing.

**What's needed**:

- `EventBus` trait with:
  - `async publish(event: AppEvent) -> Result<()>`
  - `subscribe(event_type: EventType) -> broadcast::Receiver<AppEvent>`
  - `subscribe_filtered(filter: EventFilter) -> broadcast::Receiver<AppEvent>`
- `AppEvent` enum covering all subsystem events:
  - `ChannelMessage { session_key, channel, content }` — inbound from any channel
  - `AgentToolStart { tool_name, args }` / `AgentToolResult { tool_name, result }`
  - `HeartbeatTick { checks: Vec<CheckResult> }` — periodic monitoring results
  - `CronFired { job_id, payload }` — scheduled job triggered
  - `MemoryStored { key, category }` / `MemoryRecalled { query, results }`
  - `SystemEvent { kind, message }` — notifications, errors, lifecycle
  - `ApprovalNeeded { action, risk_level }` / `ApprovalResponse { approved }`
- Default implementation: `TokioBroadcastBus` using `tokio::sync::broadcast`
- Tauri bridge: automatically emit relevant events to frontend via `app_handle.emit()`
- Event filtering by type, channel, or session scope

**Pattern source**: ZeroClaw gateway event broadcasting, OpenClaw `broadcast()` with version-based filtering

**Files affected**:

- New directory: `src-tauri/src/event_bus/`
- New files: `traits.rs` (EventBus trait + AppEvent enum), `tokio_bus.rs` (default implementation), `tauri_bridge.rs` (forward events to WebView)
- `src-tauri/src/lib.rs` — Add `pub mod event_bus;`
- Agent loop, scheduler, channel manager all receive `Arc<dyn EventBus>` via dependency injection

**Design note**: The EventBus is the backbone that connects all new subsystems. The heartbeat publishes `HeartbeatTick` events, the agent loop publishes tool execution events, channels publish inbound messages, and the gateway/WebSocket layer forwards UI-relevant events to clients.

---

#### 9. Add Heartbeat / Scheduler System

**Gap**: No periodic background task execution. The app is entirely reactive — it only acts when the user sends a message. There is no heartbeat monitoring, no cron scheduling, no proactive behavior. OpenClaw's heartbeat is one of its most powerful features: the agent autonomously checks services, processes events, and sends notifications without user prompting.

**What's needed**:

- `Scheduler` trait with:
  - `async start() -> Result<()>`
  - `async stop() -> Result<()>`
  - `add_job(job: ScheduledJob) -> Result<JobId>`
  - `remove_job(job_id: JobId) -> Result<()>`
  - `list_jobs() -> Vec<ScheduledJob>`
- Two scheduling modes (following OpenClaw/ZeroClaw dual pattern):

  **Heartbeat** (approximate intervals, batched monitoring):
  - Default: every 30 minutes (configurable)
  - Reads `HEARTBEAT.md` for checklist of what to monitor
  - Runs in main session with full conversational continuity
  - Batches multiple checks into one LLM turn for efficiency
  - Use cases: check email, monitor calendar, review notifications, health checks
  - Error backoff: 30s → 1m → 5m → 15m → 60m (exponential, from OpenClaw)

  **Cron** (precise scheduling):
  - 5-field cron expressions with timezone support (e.g., `"0 9 * * MON-FRI"`)
  - One-shot jobs via `--at` flag (e.g., "remind me at 3pm")
  - Can run in isolated sessions (avoid polluting main history) or inject into main
  - Use cases: daily reports, weekly summaries, timed reminders

- `ScheduledJob` struct:
  ```rust
  struct ScheduledJob {
      id: JobId,
      schedule: Schedule,        // Cron expression or interval
      session_target: SessionTarget, // Main or Isolated
      payload: JobPayload,       // SystemEvent or AgentTurn
      enabled: bool,
      error_count: u32,
      next_run: Option<DateTime<Utc>>,
  }
  ```
- Default implementation: `TokioScheduler` using `tokio::time::interval` + cron expression parser
- Publishes events to EventBus: `CronFired`, `HeartbeatTick`
- Stuck detection: if a heartbeat run takes >120 seconds, flag as stuck (OpenClaw pattern)

**Pattern source**: OpenClaw `src/cron/service/timer.ts` (620 lines), ZeroClaw heartbeat + cron dual system

**Files affected**:

- New directory: `src-tauri/src/scheduler/`
- New files: `traits.rs`, `tokio_scheduler.rs`, `cron_parser.rs`, `mod.rs`
- `src-tauri/src/lib.rs` — Add `pub mod scheduler;`
- `src-tauri/Cargo.toml` — Add `cron` crate (lightweight cron expression parser)
- New file: `src-tauri/src/commands/scheduler_commands.rs` — IPC for managing jobs
- Frontend: Scheduler management UI (job list, create/edit/delete, status)

**Dependencies on**: P1.8 (Event Bus for publishing events), P1.10 (Identity files for HEARTBEAT.md)

**Alternatives considered**:

- `tokio-cron-scheduler` crate — heavier but feature-complete, good if we need complex scheduling
- `croner` crate — lightweight Rust cron parser, good for minimal approach
- Custom implementation — simple `tokio::time::interval` loop with cron matching

---

#### 10. Add Identity / Personality Files

**Gap**: No agent identity or personality system. The agent has no persistent character, no memory of user preferences across sessions, no configurable behavior guidelines. OpenClaw's markdown identity files are central to its user experience — they make the agent feel personal and consistent.

**What's needed**:

- `Identity` struct loaded from markdown files in `~/.mesoclaw/identity/`:

  | File           | Purpose                                           | When Loaded              |
  | -------------- | ------------------------------------------------- | ------------------------ |
  | `SOUL.md`      | Core personality, tone, boundaries, values        | Every session start      |
  | `USER.md`      | Who the user is, preferences, how to address them | Every session start      |
  | `AGENTS.md`    | Operating instructions, behavioral guidelines     | Every session start      |
  | `IDENTITY.md`  | Agent's name, avatar, description, voice settings | Bootstrap + settings UI  |
  | `HEARTBEAT.md` | Checklist for heartbeat monitoring runs           | Heartbeat execution only |
  | `BOOT.md`      | Startup checklist on app launch                   | App startup only         |
  | `TOOLS.md`     | Notes/guidance about available tools              | Every session start      |

- `IdentityLoader` service:
  - Reads markdown files from identity directory
  - Injects into system prompt construction: `build_system_prompt(identity: &Identity, memory: &[DailyMemory])`
  - File watcher for hot-reload when user edits files externally
  - Fallback to sensible defaults if files don't exist
  - Template support: ship default templates users can customize

- System prompt assembly order (following OpenClaw pattern):
  1. SOUL.md (personality foundation)
  2. AGENTS.md (behavioral guidelines)
  3. USER.md (user context)
  4. TOOLS.md (tool guidance)
  5. MEMORY.md (curated long-term memory)
  6. Today's daily memory + yesterday's daily memory
  7. Conversation history

**Pattern source**: OpenClaw workspace files (SOUL.md, USER.md, AGENTS.md, IDENTITY.md, HEARTBEAT.md, BOOT.md), ZeroClaw identity files

**Files affected**:

- New directory: `src-tauri/src/identity/`
- New files: `mod.rs`, `loader.rs`, `types.rs`, `defaults/` (default templates)
- `src-tauri/src/lib.rs` — Add `pub mod identity;`
- `src-tauri/src/commands/` — New `identity_commands.rs` (read/update identity files)
- Frontend: Identity editor UI in settings (markdown editor for each file)
- Agent loop (P0.3): inject identity into system prompt construction
- Scheduler (P1.9): load HEARTBEAT.md for heartbeat runs, BOOT.md on startup

**Design note**: These are plain markdown files, human-editable with any text editor. The UI provides a nice editing experience, but power users can edit files directly. This follows OpenClaw's philosophy of transparency — the agent's personality is not hidden in a database, it's a readable file you can version control.

---

### P2 - Medium Priority

#### 11. Switch Config to TOML with Env Overrides

**Gap**: All settings are stored in SQLite via Diesel. There's no human-editable config file. Users can't configure the app without the UI, can't use environment variables for CI/Docker, and there's no atomic save protection against crashes.

**What's needed**:

- TOML config file at `~/.mesoclaw/config.toml`
- Serde deserialization with `#[serde(default)]` for all fields
- Environment variable override support (e.g., `MESOCLAW_PROVIDER=anthropic`)
- Atomic save: write-to-temp → fsync → backup → atomic rename
- SQLite remains for runtime state (sessions, cache), but user-facing config moves to TOML
- Config watcher for hot-reload (optional)

**Pattern source**: ZeroClaw atomic config saves, PicoClaw env override pattern

**Files affected**:

- New directory: `src-tauri/src/config/`
- New files: `mod.rs`, `schema.rs`
- `src-tauri/Cargo.toml` - Add `toml` crate
- `src-tauri/src/services/settings.rs` - Refactor to read/write TOML alongside SQLite

---

#### 12. Add Provider Routing / Model Router

**Gap**: Provider selection is entirely manual. The user picks a provider and model in the UI. There's no automatic routing based on task type, no model aliasing, no cost-based selection, no smart fallback.

**What's needed**:

- Model route config (e.g., "use claude-3.5 for code tasks, gpt-4o for general")
- Router that selects provider based on:
  - Task type / skill category
  - Model alias resolution
  - Cost tiers
  - Availability (provider health check)
- Fallback chains configured in TOML

**Pattern source**: ZeroClaw `src/providers/router.rs`

**Files affected**:

- New file: `src-tauri/src/ai/providers/router.rs`
- `src-tauri/src/ai/providers/mod.rs` - Wire up in factory

---

#### 13. Add Prelude Module

**Gap**: No prelude module for clean imports. Every consumer must know exact module paths. IronClaw demonstrates this well with a focused prelude re-exporting commonly used types.

**What's needed**: A `prelude` module in `lib.rs` re-exporting:

- `LLMProvider`, `CompletionRequest`, `CompletionResponse`
- `ApplicationAdapter`, `AdapterError`
- `SkillError`, `SkillResult`
- `DbPool`, `DbError`
- Future: `Tool`, `ToolRegistry`, `Memory`, `SecurityPolicy`, `Channel`

**Pattern source**: IronClaw `src/lib.rs` prelude module

**Files affected**: `src-tauri/src/lib.rs` only

---

#### 14. Dual Tool-Call Parser

**Gap**: Current streaming chat only handles text responses. There's no parsing of tool call instructions from LLM responses, whether in OpenAI JSON format or XML format.

**What's needed**:

- Parse OpenAI-format `tool_calls` array from `CompletionResponse`
- Parse XML-format `<tool_call>{"name": "...", "arguments": {...}}</tool_call>` blocks
- Unified `ParsedToolCall` output type
- Integration with agent loop (P0.3)

**Pattern source**: ZeroClaw `src/agent/loop_.rs` (dual format support)

**Files affected**:

- Part of agent loop: `src-tauri/src/agent/tool_parser.rs`

---

#### 15. Add Proactive Desktop Notifications

**Gap**: No mechanism for the agent to proactively notify the user. Desktop apps have native notification capabilities (system tray, toast notifications, badges) that server-based OpenClaw doesn't have. This is a Tauri advantage we should leverage.

**What's needed**:

- Integration with Tauri notification plugin (`tauri-plugin-notification`)
- System tray icon with status indicators (idle, processing, notification pending)
- Notification types:
  - **Heartbeat alert**: "You have 3 unread emails" (from heartbeat monitoring)
  - **Cron reminder**: "Daily standup in 15 minutes" (from scheduled jobs)
  - **Agent completion**: "Task finished: database analysis complete" (from agent loop)
  - **Approval request**: "Agent wants to run `git push` — approve?" (from security policy)
- Click-to-open: clicking a notification opens the relevant chat/session
- Do Not Disturb mode: suppress non-critical notifications
- Notification preferences per category (configurable in settings)

**Pattern source**: OpenClaw proactive notifications via channel adapters (adapted for desktop context)

**Files affected**:

- New file: `src-tauri/src/services/notification_service.rs`
- `src-tauri/Cargo.toml` — Add `tauri-plugin-notification`
- `src-tauri/tauri.conf.json` — Add notification plugin permissions
- Frontend: Notification preferences in settings UI
- Scheduler (P1.9): trigger notifications from heartbeat/cron results

---

#### 16. Add Session Management / Routing

**Gap**: Chat sessions exist (SQLite `chat_sessions` table) but there's no structured session routing. Sessions aren't scoped by channel or context. There's no way to have parallel sessions (e.g., one for main chat, one for a cron job, one for a webhook handler).

**What's needed**:

- Structured session keys following OpenClaw pattern:
  ```
  {agent_id}:{scope}:{channel}:{peer}
  Examples:
  - "main:dm:tauri:user"           — Main desktop chat
  - "main:cron:daily-report"       — Scheduled job session
  - "main:heartbeat:check"         — Heartbeat monitoring session
  - "main:webhook:github:push"     — Webhook handler session
  - "isolated:task:analyze-db"     — Isolated task session
  ```
- Session router: resolves inbound messages to correct session based on channel + context
- Session isolation: cron/heartbeat can run in isolated sessions to avoid polluting main chat history
- Session compaction: truncate old messages while preserving summary (for long-running sessions)
- JSONL session logging for audit trail and replay

**Pattern source**: OpenClaw session key structure, ZeroClaw session management

**Files affected**:

- New file: `src-tauri/src/services/session_router.rs`
- `src-tauri/src/database/models/` — Extend chat_sessions with session_key, scope, channel fields
- New migration: add session routing columns
- Agent loop (P0.3): route conversations to correct session
- Scheduler (P1.9): create isolated sessions for cron/heartbeat

---

#### 17. Add Daily Memory Files

**Gap**: No daily memory logging. Conversations happen and are stored in SQLite, but there's no curated daily summary that persists as a readable file. OpenClaw's `memory/YYYY-MM-DD.md` pattern provides a human-readable daily journal that the agent reads on session start for continuity.

**What's needed**:

- Daily memory directory: `~/.mesoclaw/memory/`
- File pattern: `YYYY-MM-DD.md` (one file per day)
- Auto-generated daily summary at end of day (or configurable time)
- Agent reads today + yesterday on session start (injected into system prompt after identity files)
- MEMORY.md for curated long-term memory (manually maintained or agent-updated)
- Memory can be updated by:
  - Agent (via memory tool): `store_daily_memory("learned that user prefers dark mode")`
  - User (via UI): add/edit memory entries
  - Auto-summary: end-of-day LLM summarization of key interactions

**Pattern source**: OpenClaw `memory/YYYY-MM-DD.md`, ZeroClaw memory categories

**Files affected**:

- New file: `src-tauri/src/memory/daily.rs` — Daily file read/write
- `src-tauri/src/memory/traits.rs` — Extend Memory trait with `store_daily()`, `recall_daily(date)` methods
- `src-tauri/src/identity/loader.rs` — Include daily memory in system prompt construction
- Frontend: Daily memory view (timeline of daily files)

**Memory backend alternatives** (user can choose):

| Backend                            | Strengths                                             | When to Use                                 |
| ---------------------------------- | ----------------------------------------------------- | ------------------------------------------- |
| **SQLite + FTS5 + BM25** (default) | Zero dependencies, fast, embedded, proven             | Default for all users                       |
| **sqlite-vec**                     | Vector similarity search in SQLite                    | When embeddings needed without external API |
| **Markdown files**                 | Human-readable, version-controllable, editor-friendly | Power users, transparency                   |
| **Hybrid (SQLite + Markdown)**     | Best of both — structured search + readable files     | Recommended for production                  |
| **Qdrant** (future)                | Dedicated vector DB, scales to millions               | Large knowledge bases                       |
| **Turbopuffer** (future)           | Serverless vector search                              | Cloud-connected agents                      |

---

### P3 - Lower Priority

#### 18. Memory Hygiene / Auto-Archiving

**Gap**: No cleanup for old conversation sessions or cached data. Over time, the SQLite database will grow unbounded.

**What's needed**: Background Tokio task that:

- Archives conversation sessions older than 7 days
- Purges archived data older than 30 days
- Compacts SQLite database periodically

**Pattern source**: ZeroClaw `src/memory/hygiene.rs`

---

#### 19. Native Ollama Provider Optimizations

**Gap**: Ollama is discovered via `discover_ollama_models_command` but uses the generic `OpenAICompatibleProvider`. No Ollama-specific optimizations exist.

**What's needed**:

- Local connection detection (skip TLS, use localhost directly)
- Model pull status monitoring
- Warmup for local models (pre-load into GPU memory)

---

#### 20. WASM Extension System

**Gap**: No plugin/extension mechanism beyond the skills system. Skills are prompt templates, not executable code. IronClaw's WASM approach enables third-party tools and channels.

**What's needed**: WASM runtime (wasmtime) for loading and executing third-party tools in a sandbox.

**Pattern source**: IronClaw `src/sandbox/`, `src/extensions/`

---

#### 21. Frontend: Memory Search UI

**Gap**: No UI for searching or browsing the memory system (once P1.5 is built).

**What's needed**:

- New Zustand store: `memoryStore.ts`
- New route or panel for memory search
- Components: search input, result list, memory detail view
- IPC commands: `search_memory_command`, `store_memory_command`, `forget_memory_command`

---

#### 22. Frontend: Agent Loop UI

**Gap**: Current chat is simple request/response. No UI for the multi-turn agent loop (P0.3).

**What's needed**:

- Tool execution status display (which tool is running, what arguments)
- User approval overlay for supervised mode (P1.6)
- Multi-turn progress indicator
- Execution log panel (expandable)
- Cancel/abort button for running agent loops
- New Tauri event types: `agent-tool-start`, `agent-tool-result`, `agent-approval-needed`

---

#### 23. Add Boot Sequence

**Gap**: No structured startup sequence. The app launches and waits for user interaction. OpenClaw has a `BOOT.md` startup checklist that the agent executes on gateway restart — checking system health, loading context, and optionally greeting the user.

**What's needed**:

- `BootSequence` service that runs on app startup:
  1. Load identity files (SOUL.md, USER.md, etc.)
  2. Execute `BOOT.md` checklist (if exists)
  3. Load today's + yesterday's daily memory
  4. Warm up configured providers (ReliableProvider.warmup())
  5. Start scheduler (heartbeat + cron jobs)
  6. Start channel listeners (if configured)
  7. Emit `SystemEvent::Ready` to Event Bus
  8. Optionally send greeting notification

**Pattern source**: OpenClaw BOOT.md, ZeroClaw daemon startup

**Files affected**:

- New file: `src-tauri/src/services/boot.rs`
- `src-tauri/src/lib.rs` — Call boot sequence in Tauri setup hook

---

#### 24. Frontend: Scheduler Management UI

**Gap**: No UI for managing scheduled jobs, heartbeat configuration, or viewing cron history.

**What's needed**:

- New Zustand store: `schedulerStore.ts`
- Job list view: active/paused/completed jobs with next run time
- Create/edit job form: cron expression builder, payload editor
- Heartbeat config: interval, HEARTBEAT.md editor
- Job execution history with results
- IPC commands: `list_jobs_command`, `create_job_command`, `toggle_job_command`, `job_history_command`

---

#### 25. Frontend: Identity Editor UI

**Gap**: No UI for editing agent identity/personality files.

**What's needed**:

- Markdown editor for each identity file (SOUL.md, USER.md, AGENTS.md, etc.)
- Preview pane showing how the agent will interpret the file
- Template gallery: pre-built personality templates users can start from
- Located in Settings → Identity section
- IPC commands: `get_identity_file_command`, `update_identity_file_command`, `list_identity_templates_command`

---

## Dependency Changes

### New Cargo.toml Dependencies

```toml
# P2.8 - Config system
toml = "1"

# P1.7 - Channel system (for external channels beyond desktop gateway)
axum = "0.8"
tower-http = "0.6"

# P1.6 - Security (already partially covered)
# No new deps needed - uses existing std lib

# P1.5 - Memory system
# No new deps needed - uses planned rusqlite + existing lru, reqwest

# P1.9 - Scheduler system
cron = "0.12"           # Lightweight cron expression parser
# Alternative: croner = "2" (Rust-native, no C deps)

# P2.15 - Desktop notifications
tauri-plugin-notification = "2"
```

### Dependencies Already Present (Leverage These)

| Crate                  | Version | Use For                                                     |
| ---------------------- | ------- | ----------------------------------------------------------- |
| `lru`                  | 0.12    | Embedding cache (P1.5)                                      |
| `reqwest`              | 0.12    | Embedding API calls (P1.5)                                  |
| `tokio`                | 1       | Background tasks, async agent loop (P0.3)                   |
| `serde` + `serde_json` | 1       | Tool schemas, memory serialization                          |
| `sha2`                 | 0.10    | Content hashing for deduplication (P1.5)                    |
| `uuid`                 | 1.11    | Session/memory entry IDs                                    |
| `regex`                | 1.10    | Tool call parsing, injection detection (P1.6)               |
| `thiserror`            | 2       | Error types for new modules                                 |
| `async-trait`          | 0.1     | New trait definitions                                       |
| `tokio` (broadcast)    | 1       | Event Bus default implementation (P1.8)                     |
| `notify`               | —       | File watcher for identity hot-reload (P1.10, add if needed) |

---

## Effort Summary

| Priority  | Items           | New Files           | Modified Files   | Estimated Scope                      |
| --------- | --------------- | ------------------- | ---------------- | ------------------------------------ |
| **P0**    | 3 (items 1-3)   | ~5 new              | ~3 modified      | Foundation — must do first           |
| **P1**    | 7 (items 4-10)  | ~25 new, 7 new dirs | ~8 modified      | Core capabilities + OpenClaw parity  |
| **P2**    | 7 (items 11-17) | ~12 new             | ~8 modified      | Quality of life + proactive features |
| **P3**    | 8 (items 18-25) | ~12 new             | ~5 modified      | Future extensibility + UI polish     |
| **Total** | **25 items**    | **~54 new files**   | **~24 modified** |                                      |

---

## Recommended Implementation Order

```
Phase 0: Slim Down (Technical Debt)
  S4 Strict linting (1 file, zero risk)
    ↓
  S2 Consolidate AI providers with async-openai (low risk, ~1,200 lines saved)
    ↓
  S1 Replace skills with prompt templates (medium risk, ~2,300 lines saved)
    ↓
  S3 Diesel → rusqlite (high risk, defer if needed)

Phase 1: Foundation
  P0.2 Release profile optimization (1 file, immediate impact)
    ↓
  P0.1 ReliableProvider wrapper (depends on nothing)

Phase 2: Core Infrastructure
  P1.8 Event Bus (backbone for all new subsystems)
    ↓
  P1.4 Tool trait + registry (needed before agent loop)
    ↓
  P1.6 Security policy (needed before agent loop executes tools)
    ↓
  P1.10 Identity files (needed for system prompt construction)

Phase 3: Agent Intelligence
  P0.3 Agent loop (depends on tools + security + identity)
    ↓
  P1.5 Memory system (agent loop can use it for context)
    ↓
  P2.17 Daily memory files (extends memory system with daily pattern)

Phase 4: Proactive Behavior
  P1.9 Heartbeat / Scheduler (depends on Event Bus + Identity)
    ↓
  P2.15 Proactive notifications (depends on scheduler)
    ↓
  P2.16 Session management (routing for heartbeat/cron sessions)

Phase 5: Configuration & DX
  P2.11 TOML config (improves DX, independent)
    ↓
  P2.12 Provider router (enhances provider selection)
    ↓
  P2.13 Prelude module (clean imports for growing codebase)
    ↓
  P2.14 Dual tool-call parser (part of agent loop)

Phase 6: Channels & Extensions
  P1.7 Channel trait + manager (external input)
    ↓
  P3.23 Boot sequence (orchestrates startup)
    ↓
  P3.* (remaining items as needed)
```

**Critical path**: Slim Down → Release Profile → Event Bus → Tools → Security → Identity → Agent Loop → Memory → Scheduler → Notifications

Phase 0 (slim down) is now first. Starting from a 4,600-line codebase instead of 9,152 means every subsequent feature is easier to implement, test, and maintain.

---

## Conclusion

**Phase 0: Slim down first.** A codebase audit (see `docs/moltis-microclaw-analysis.md`) revealed that 49% of backend code is over-engineered or duplicated. The skills system alone is 29% of the backend (2,631 lines) for what should be simple prompt templates. Three AI provider implementations share 90% identical code. Diesel ORM adds unnecessary complexity for a single-user SQLite desktop app. Slimming from 9,152 → ~4,600 lines creates a clean foundation for the 25 new features below.

**Moltis as pattern reference.** While neither Moltis nor MicroClaw can be imported as dependencies (both are applications, not libraries), Moltis's memory system, agent loop, hook architecture, and strict linting provide excellent reference implementations for Mesoclaw's new modules.

**The existing foundation is strong.** The `LLMProvider` trait, skills system, credential store, streaming chat, and full frontend component library are well-built and align with best practices from all four analyzed projects.

**v2 additions bring OpenClaw feature parity** with a desktop-native architecture:

| OpenClaw Core Feature       | Mesoclaw Implementation                                                    | Status             |
| --------------------------- | -------------------------------------------------------------------------- | ------------------ |
| **Memory System**           | SQLite + FTS5 + BM25 hybrid search + daily markdown files + identity files | P1.5, P1.10, P2.17 |
| **Heartbeat**               | Tokio-based scheduler with dual heartbeat + cron modes                     | P1.9               |
| **Channel Adapters**        | Channel trait with Tauri IPC default, webhook/WS optional                  | P1.7               |
| **Skills Registry**         | Existing skills system (already built)                                     | Done               |
| **Gateway / Control Plane** | Event Bus trait (replaces WebSocket gateway for desktop context)           | P1.8               |
| **Proactive Notifications** | Desktop-native via Tauri notification plugin + system tray                 | P2.15              |
| **Session Routing**         | Structured session keys with scope/channel/peer routing                    | P2.16              |
| **Identity / Personality**  | Markdown files (SOUL.md, USER.md, AGENTS.md, etc.)                         | P1.10              |

The biggest architectural additions are:

1. **Event Bus** — The backbone replacing OpenClaw's WebSocket control plane, connecting all subsystems
2. **Heartbeat / Scheduler** — Proactive behavior without user prompting
3. **Identity files** — Agent personality and user context persistence
4. **Agent loop** — Multi-turn autonomous reasoning with tool calling
5. **Memory system** — Persistent knowledge with hybrid search + daily files

The **Gateway-Ready architecture** (Approach B) ensures all these features ship with lightweight Tauri-native implementations today, while trait abstractions keep the door open for WebSocket gateways, external channels, and platform integrations in the future.
