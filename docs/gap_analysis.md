# MesoClaw — Gap Analysis & Implementation Checklist

> **Analysis date:** 2026-02-19
> **Based on:** `docs/product-requirements.md`, `docs/user-journey.md`, full codebase exploration (3 parallel agents)
>
> **Purpose:** Track implementation of every gap between the PRD and the actual codebase. Check items off as they are completed.

---

## P1 — Must Implement First (Core product quality + security)

- [ ] **GAP-1: Memory Persistence** (Priority: P1)
  > FR-4.2, FR-4.3, FR-4.8, FR-4.9, FR-4.10, FR-4.11
  >
  > **Problem:** `src-tauri/src/memory/store.rs` uses a `HashMap`-backed in-memory store. Memory is **lost on every app restart**. The PRD requires SQLite persistence with vector BLOBs and FTS5 full-text search. Embeddings use a mock hash-based approach (not real embeddings).
  - [ ] Add Diesel migration: `create_memories` table (`id`, `key`, `content`, `category`, `embedding BLOB`, `created_at`, `last_accessed`) + FTS5 virtual table `memories_fts(content)`
  - [ ] Rewrite `MemoryStore` in `src-tauri/src/memory/store.rs` to use SQLite connection pool — `store()` → INSERT/UPDATE, `recall()` → FTS5 query + cosine similarity on BLOBs, `forget()` → DELETE
  - [ ] Replace mock embeddings in `embeddings.rs`: call active provider's embedding endpoint (OpenAI `text-embedding-3-small`, Ollama `nomic-embed-text`) — add `embed()` method to `LLMProvider` trait or create `EmbeddingProvider` trait
  - [ ] Wire `daily.rs` summarization: each day at configurable time, run agent to summarize conversation memories into `~/.mesoclaw/memory/YYYY-MM-DD.md`
  - [ ] Wire `hygiene.rs` as a cron job in the scheduler (daily at 2am): archive >7 days, purge >30 days from SQLite
  - [ ] Add `MEMORY.md` curated file support in identity loader (injected into system prompt)

- [ ] **GAP-2: GUI Uses Gateway Instead of Tauri IPC** (Priority: P1)
  > FR-15.9
  >
  > **Problem:** The PRD states "GUI uses gateway (not Tauri IPC) for agent/memory/provider operations." Frontend uses `invoke()` from `@tauri-apps/api/core` for nearly all operations. CLI and GUI have diverged data paths.
  - [ ] Audit `src-tauri/src/gateway/routes.rs` — list all endpoints, cross-check against store needs
  - [ ] Add missing gateway endpoints for: agent sessions (POST/GET/DELETE), memory CRUD, provider config
  - [ ] Update `src/stores/agentStore.ts` to POST to gateway + subscribe to WebSocket `ws://localhost:18790/api/v1/ws` for streaming events
  - [ ] Update `src/stores/memoryStore.ts` to use REST calls instead of `invoke()`
  - [ ] Keep Tauri IPC only for desktop-specific operations (settings, keychain, window, file browser)

- [ ] **GAP-3: Security Policy Completeness** (Priority: P1)
  > FR-6.4, FR-6.5, FR-6.6, FR-6.7, FR-6.8
  >
  > **Problem:** `src-tauri/src/security/policy.rs` exists but specific PRD protections may be incomplete: path traversal, injection, blocked dirs, rate limiting, and audit trail.
  - [ ] Add unit tests for each protection type in `policy.rs` to discover what's missing
  - [ ] Implement path traversal checks: regex for `..`, `\0`, symlink resolution via `std::fs::canonicalize()`
  - [ ] Implement injection protection: check shell args for backtick, `$(`, `${`, `>`, `>>`, `|` characters
  - [ ] Add blocked directory list check: resolve canonical path, check prefix against deny list (`/etc`, `/root`, `~/.ssh`, `~/.aws`, `~/.gnupg`)
  - [ ] Add sliding window rate limiter: `Arc<Mutex<VecDeque<Instant>>>` per session, configurable `max_per_hour` (default 20)
  - [ ] Create `tool_audit_log` table migration and `AuditTrail` service (`src-tauri/src/security/audit.rs`)
  - [ ] Call `audit.log(tool, args, result)` after every tool execution in agent loop

- [ ] **GAP-5: Agent Loop — XML Tool Parsing + Memory Injection** (Priority: P1)
  > FR-2.2, FR-2.7
  >
  > **Problem:** `tool_parser.rs` may only handle OpenAI JSON tool calls. Models without native tool calling use XML: `<tool_call><name>shell</name><args>{"cmd":"ls"}</args></tool_call>`. Memory context injection not automatic.
  - [ ] Read `src-tauri/src/agent/tool_parser.rs` — verify if XML branch exists; if not, add regex-based XML parser for `<tool_call>` tag format
  - [ ] In `AgentLoop::run()`, before first LLM call: call `memory.recall(session_topic, 5)` to retrieve relevant memories
  - [ ] Prepend recalled memories as a `system` message: `"Relevant context from memory:\n{memories}"`
  - [ ] After agent session completes, call `memory.store(session_summary, category=Conversation)` to persist the interaction

- [ ] **GAP-11: Community GitHub Files** (Priority: P1)
  > FR-17.1–FR-17.8
  >
  > **Problem:** GitHub community files are missing. Required before the project can accept open-source contributions.
  - [ ] Create `.github/ISSUE_TEMPLATE/bug_report.yml` — YAML form: severity, component, reproduction steps
  - [ ] Create `.github/ISSUE_TEMPLATE/feature_request.yml` — YAML form: problem statement, proposed solution, acceptance criteria
  - [ ] Create `.github/ISSUE_TEMPLATE/config.yml` — `blank_issues_enabled: false`
  - [ ] Create `.github/pull_request_template.md` — summary, validation evidence, security impact, rollback plan
  - [ ] Create `CONTRIBUTING.md` — setup instructions, risk-based tracks (A/B/C), naming conventions
  - [ ] Create `SECURITY.md` — responsible disclosure, SLAs (48h ack, 1w assess, 2w fix critical)
  - [ ] Create `CODEOWNERS` — `src-tauri/src/security/` → @maintainer, etc.
  - [ ] Create `CODE_OF_CONDUCT.md` — Contributor Covenant v2.1

---

## P2 — Next Sprint

- [ ] **GAP-6: HEARTBEAT.md and BOOT.md Checklist Execution** (Priority: P2)
  > FR-5.5, FR-5.6
  >
  > **Problem:** It's unclear if heartbeat jobs read and execute `HEARTBEAT.md` as a checklist. `BOOT.md` (startup checklist) may not be read by the boot sequence.
  - [ ] Read `src-tauri/src/scheduler/heartbeat.rs` — if HEARTBEAT.md reading is missing, add: parse markdown checklist items, pass as agent prompt in isolated session
  - [ ] Read `src-tauri/src/services/boot.rs` — if BOOT.md execution is missing, add: after identity load, read BOOT.md, run agent with checklist as prompt
  - [ ] Add default HEARTBEAT.md and BOOT.md templates to bundled app resources

- [ ] **GAP-7: Session Compaction with Summarization** (Priority: P2)
  > FR-10.4
  >
  > **Problem:** Without session compaction, long conversations will hit context window limits. The PRD requires truncating old messages while preserving a summary.
  - [ ] When `conversation_history.len() > max_history`, take oldest N messages and run a summarization LLM call: `"Summarize this conversation excerpt"`
  - [ ] Replace truncated messages with a single `system` message: `"Earlier conversation summary: {summary}"`
  - [ ] Persist compaction summary to memory store (category: Conversation)

- [ ] **GAP-8: Notifications — Click-to-Open, DND, Per-Category** (Priority: P2)
  > FR-9.4, FR-9.5, FR-9.6
  >
  > **Problem:** `notification_service.rs` sends notifications but no deep link action, no DND mode, no per-category preferences.
  - [ ] Add `action_url` to notification calls: `"mesoclaw://session/{session_id}"` using Tauri deep link
  - [ ] Register deep link handler in `lib.rs` to navigate window to relevant route on click
  - [ ] Add DND schedule to `AppSettings` (`start_hour`, `end_hour`): check before sending notifications
  - [ ] Add per-category flags to settings (heartbeat, cron_reminder, agent_complete, approval_request)
  - [ ] Add UI in `src/components/settings/AppSettingsTab.tsx` for DND time range + per-category toggles

- [ ] **GAP-9: Responsive Layout Verification** (Priority: P2)
  > FR-13.1–FR-13.11
  >
  > **Problem:** Mobile components exist but the 5-breakpoint layout system, safe area insets, virtual keyboard handling, swipe gestures, pull-to-refresh, and bottom sheets may be missing.
  - [ ] Audit each route for Tailwind breakpoint classes (xs/sm/md/lg/xl) — ensure consistent application
  - [ ] Add safe area inset CSS: `padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left)`
  - [ ] Add virtual keyboard handler in `src/routes/chat.tsx`: listen for `visualViewport.resize`, adjust chat input height
  - [ ] Add swipe gesture in `src/components/layout/ResponsiveSidebar.tsx`: use touch events for right-swipe to open
  - [ ] Create `src/components/ui/bottom-sheet.tsx` component with translate-y animation and drag handle
  - [ ] Update `src/components/agent/ApprovalOverlay.tsx`: use BottomSheet when `window.innerWidth < 768px`
  - [ ] Add pull-to-refresh in message list: detect overscroll, trigger `loadOlderMessages()`

- [ ] **GAP-10: Module System — MCP Client + Container Runtime** (Priority: P2)
  > FR-18.5, FR-18.7, FR-18.8
  >
  > **Problem:** The `mcp-client` feature is flagged but MCP JSON-RPC protocol is not implemented. Container runtime (Docker/Podman) is feature-flagged but has no implementation.
  - [ ] Create `src-tauri/src/modules/mcp_client.rs`: spawn process from manifest `command`, write JSON-RPC `tools/list` to stdin, parse response, register tools in ToolRegistry
  - [ ] For `tools/call`: serialize args, write to stdin, read result from stdout
  - [ ] Create `src-tauri/src/modules/container_runtime.rs`: try `podman` then `docker` via `which`, implement `run(image, mounts, env, cmd)`, `stop(id)`, status check
  - [ ] Wire container modules: when `manifest.runtime.type == "docker"`, use ContainerRuntime instead of direct process spawn
  - [ ] Composio integration follows automatically once MCP client works (zero custom code per PRD)

- [ ] **GAP-14: Config System — Env Vars, Atomic Saves, Hot-Reload** (Priority: P2)
  > FR-11.2, FR-11.3, FR-11.4
  >
  > **Problem:** TOML config exists but environment variable overrides, atomic saves, and config hot-reload on file change may not be implemented.
  - [ ] After TOML parse, iterate `std::env::vars()` for `MESOCLAW_*` prefix and override matching config fields
  - [ ] Atomic save: write to `config.toml.tmp`, fsync, copy old to `config.toml.bak`, rename tmp → `config.toml`
  - [ ] Add `notify` watcher on config file path in boot sequence, broadcast `AppEvent::ConfigChanged` on change

---

## P3 — Backlog / Pre-Release

- [ ] **GAP-4: CLI Flags and Completeness** (Priority: P3)
  > FR-14.4, FR-14.5, FR-14.6, FR-14.11, FR-14.12, FR-14.13, FR-14.14
  >
  > **Problem:** `src-tauri/src/cli.rs` has all subcommands but `--raw`, `--json`, `--auto`, `--resume`, watch mode, agent chaining, and terminal markdown may not be implemented.
  - [ ] Search `cli.rs` for `--raw`, `--json`, `--auto` in clap arg definitions — add any missing
  - [ ] `--raw` mode: suppress spinner/headers, print only final agent text to stdout
  - [ ] `--json` mode: wrap agent response in `{"response": "...", "tool_calls": [...]}` JSON
  - [ ] `--auto` mode: set SecurityPolicy to `Full` for the session duration
  - [ ] `--resume session-id`: look up session from gateway, append to existing conversation
  - [ ] `watch` subcommand: use `notify` crate to watch path, trigger agent on file change (debounced 500ms)
  - [ ] Terminal markdown: add `termimad` crate for pretty REPL output (bold headings, code fences, colors)
  - [ ] Agent chaining: detect piped stdin (`!stdin.is_terminal()`), treat as additional context

- [ ] **GAP-12: Gateway — Port Auto-Increment, API Versioning, OpenAPI** (Priority: P3)
  > FR-15.5, FR-15.7, FR-15.8
  >
  > **Problem:** Port auto-increment (18790–18799 on bind conflict) unclear; API versioning policy not enforced; no OpenAPI spec.
  - [ ] Wrap bind call in loop trying ports 18790–18799; write chosen port to PID file
  - [ ] Add `X-API-Version: v1` response header to all routes
  - [ ] Add `utoipa` crate (feature-gated `openapi`) for OpenAPI spec generation
  - [ ] Expose `/api/v1/openapi.json` endpoint

- [ ] **GAP-13: Mobile Builds — iOS/Android** (Priority: P3)
  > FR-13.12, FR-13.13, FR-16.7
  >
  > **Problem:** Mobile compilation is blocked on platform signing setup. Code and workflows exist (`.github/workflows/mobile.yml`) but builds cannot run without signing credentials.
  - [ ] Set up Apple Developer account + create App ID `com.mesoclaw.app`
  - [ ] Create iOS distribution certificate + provisioning profile → store in GitHub Secrets
  - [ ] Create Android keystore → store in GitHub Secrets (`ANDROID_KEYSTORE_BASE64`, etc.)
  - [ ] Configure entitlements for push notifications (APNs/FCM)
  - [ ] Add APNs/FCM token registration to frontend on mobile startup
  - [ ] Update `notification_service.rs` to route push through APNs/FCM when on mobile

---

## Verification Commands

```bash
# GAP-1: Memory persistence
cd src-tauri && cargo test --lib memory::
# Store memory, restart app, search — results must persist

# GAP-3: Security policy
cd src-tauri && cargo test --lib security::
# Attempt path traversal in shell tool arg, verify it's blocked

# GAP-4: CLI flags
cargo run --bin mesoclaw -- "hello" --raw   # Only text output
cargo run --bin mesoclaw -- "hello" --json  # JSON output

# GAP-11: Community files
ls .github/ISSUE_TEMPLATE/
cat CONTRIBUTING.md | head -20

# GAP-2: Gateway routing
curl -H "Authorization: Bearer $(cat ~/.mesoclaw/daemon.token)" \
  http://localhost:18790/api/v1/sessions
# Verify frontend agentStore uses fetch() not invoke() for agent calls

# GAP-10: Module MCP
mesoclaw module create test-mcp --type mcp
# Verify tools/list JSON-RPC call works
```

---

## Summary Table

| Gap | Priority | Area | Status |
|-----|----------|------|--------|
| GAP-1: Memory persistence | P1 | Backend: `memory/` | ☐ |
| GAP-2: GUI → Gateway | P1 | Frontend: `stores/` + Backend: `gateway/` | ☐ |
| GAP-3: Security policy | P1 | Backend: `security/` | ☐ |
| GAP-5: Agent XML + memory inject | P1 | Backend: `agent/` | ☐ |
| GAP-11: Community GitHub files | P1 | Repo: `.github/` | ☐ |
| GAP-6: HEARTBEAT + BOOT checklists | P2 | Backend: `scheduler/`, `services/` | ☐ |
| GAP-7: Session compaction | P2 | Backend: `agent/` | ☐ |
| GAP-8: Notification enhancements | P2 | Backend + Frontend | ☐ |
| GAP-9: Responsive layout | P2 | Frontend: `components/layout/` | ☐ |
| GAP-10: MCP client + container | P2 | Backend: `modules/` | ☐ |
| GAP-14: Config env/atomic/hot-reload | P2 | Backend: `config/` | ☐ |
| GAP-4: CLI flags | P3 | Backend: `cli.rs` | ☐ |
| GAP-12: Gateway polish | P3 | Backend: `gateway/` | ☐ |
| GAP-13: Mobile builds | P3 | CI/CD + external accounts | ☐ |

---

_Analysis date: 2026-02-19_
_Based on: docs/product-requirements.md, docs/user-journey.md, full codebase exploration (3 parallel agents)_
