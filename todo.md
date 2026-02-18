# Technical Debt Tracker

This file tracks incomplete features, mocks, and technical debt across the codebase.

**Status Legend:**
- ⏳ Pending - Not yet started
- 🚧 In Progress - Currently being worked on
- ✅ Done - Completed and verified
- 🔄 Blocked - Waiting on dependencies

---

## TODO - Features Not Yet Implemented

| Status | File | Line | Description | Priority |
|--------|------|------|-------------|----------|
| ⏳ | `src-tauri/src/channels/tauri_ipc.rs` | 46 | Subscribe to `AgentTurn` events from EventBus once a user-message event is added to `AppEvent` (Phase 6+) | Medium |
| ⏳ | `src-tauri/src/gateway/ws.rs` | 53 | Parse incoming WebSocket commands from client (Phase 2.6) | Medium |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 39 | Wire `start_session` route to actual agent session manager (Phase 3) | High |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 57 | Return real sessions from agent session store (Phase 3) | High |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 66 | Query real provider health instead of returning stub (Phase 3) | Medium |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 83–135 | Wire all module routes (`list`, `health`, `start`, `stop`, `reload`) to `ModuleRegistry` (Phase 6) | High |
| ⏳ | `src-tauri/src/services/notification_service.rs` | 199 | Call `tauri_plugin_notification` to show OS notifications (Phase 4 follow-up) | Medium |
| ⏳ | `src-tauri/src/agent/agent_commands.rs` | 32 | Implement proper provider resolution from app state in `run_agent_command` | High |
| ⏳ | `src-tauri/src/agent/agent_commands.rs` | 43 | Implement session cancellation via `CancellationToken` | High |
| ⏳ | `src-tauri/src/agent/session_router.rs` | 179 | Session router: complete Phase 4.3.3 wiring | Medium |
| ⏳ | `src-tauri/src/agent/loop_.rs` | 244 | Implement full approval flow via EventBus (currently logs only) | High |
| ⏳ | `src-tauri/src/scheduler/commands.rs` | 12–45 | Resolve managed `TokioScheduler` from app state in all scheduler IPC commands | Medium |
| ⏳ | `src-tauri/src/scheduler/tokio_scheduler.rs` | 217 | Persist scheduled jobs to SQLite `scheduled_jobs` table (Phase 4.1.11) | Low |
| ⏳ | `src-tauri/src/scheduler/tokio_scheduler.rs` | 226 | Delete from `scheduled_jobs` SQLite table on `remove_job` | Low |
| ⏳ | `src-tauri/src/scheduler/tokio_scheduler.rs` | 262 | Run heartbeat items via `AgentLoop` (Phase 3 follow-up) | Medium |
| ⏳ | `src-tauri/src/scheduler/tokio_scheduler.rs` | 266 | Run `AgentTurn` prompt through `AgentLoop` (Phase 3 follow-up) | Medium |
| ⏳ | `src-tauri/src/memory/commands.rs` | 17–41 | Resolve managed `InMemoryStore` from app state in all memory IPC commands | Medium |
| ⏳ | `src-tauri/src/modules/mod.rs` | 17 | Configure bundled sidecar binaries in `tauri.conf.json` | Low |
| ⏳ | `src-tauri/src/bin/cli.rs` | 424 | Implement `memory` CLI subcommand once memory service REST endpoint exists (Phase 3) | Low |
| ⏳ | `src-tauri/src/bin/cli.rs` | 429 | Wire `identity` CLI subcommand to identity REST endpoint (Phase 2.6) | Low |
| ⏳ | `src-tauri/src/bin/cli.rs` | 479 | Implement package-registry `install`/`remove` (Phase 6+) | Low |
| ⏳ | `src/stores/identityStore.ts` | 7 | Migrate identity store to gateway REST API `/api/v1/identity/*` (Phase 3) | Medium |
| ⏳ | `src/stores/llm.ts` | 4 | Migrate provider listing and session creation to gateway REST API (Phase 3) | Medium |
| ⏳ | `src/lib/tauri/identity/index.ts` | 4 | Migrate identity CRUD to gateway REST API (Phase 3) | Medium |
| ⏳ | `src/lib/gateway-client.ts` | 61 | Add dedicated `get_daemon_config_command` (Phase 2.7) | Low |
| ⏳ | `src/lib/gateway-client.ts` | 198 | Add dedicated approval endpoint to gateway REST API | Medium |

---

## MOCK - Temporary Mock Implementations

| Status | File | Line | Description | Replace With | Priority |
|--------|------|------|-------------|--------------|----------|
| ⏳ | `src-tauri/src/gateway/routes.rs` | 66 | `provider_status` returns hardcoded `{"status":"ok"}` | Real provider health probe | Medium |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 88 | `list_modules` returns empty `[]` until `ModuleRegistry` is wired | `ModuleRegistry::list()` | High |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 103 | `module_health` returns stubbed `{"healthy":true}` | `SidecarModule::health_check()` | High |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 112 | `start_module` returns `{"started":true}` stub | `ModuleRegistry::start(id)` | High |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 124 | `stop_module` returns `{"stopped":true}` stub | `ModuleRegistry::stop(id)` | High |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 135 | `reload_modules` returns `{"reloaded":true}` stub | `ModuleRegistry::discover()` | Medium |

---

## FIXME - Known Bugs & Issues

| Status | File | Line | Description | Impact | Priority |
|--------|------|------|-------------|--------|----------|
| | | | | | |

---

## HACK - Temporary Workarounds

| Status | File | Line | Description | Proper Solution | Priority |
|--------|------|------|-------------|-----------------|----------|
| | | | | | |

---

## PERF - Performance Optimizations

| Status | File | Line | Description | Expected Improvement | Priority |
|--------|------|------|-------------|---------------------|----------|
| | | | | | |

---

## SECURITY - Security Considerations

| Status | File | Line | Description | Risk Level | Priority |
|--------|------|------|-------------|------------|----------|
| | | | | | |

---

## REFACTOR - Code That Needs Refactoring

| Status | File | Line | Description | Reason | Priority |
|--------|------|------|-------------|--------|----------|
| ⏳ | `src-tauri/src/agent/agent_commands.rs` | 12 | Full agent session wiring (Phase 3 follow-up) — commands are stubs | Phase 3 agent loop integration | High |

---

## Usage Guidelines

1. **Adding new items**: Add a new row to the appropriate section when you add a marker comment in code
2. **File path**: Use relative path from project root (e.g., `src/components/Auth.tsx`)
3. **Line number**: Reference the line number for easy navigation
4. **Priority**: Use High/Medium/Low based on impact and urgency
5. **Status**: Update status as work progresses
6. **Completion**: Move to "Done" section or remove when fully addressed

## Completed Items (Archive)

| Date | Category | File | Description |
|------|----------|------|-------------|
| 2026-02-18 | Feature | `src-tauri/src/config/loader.rs` | TOML config loader with env overrides (Phase 5.1) |
| 2026-02-18 | Feature | `src-tauri/src/ai/providers/router.rs` | `ModelRouter` — cost-tier routing and alias resolution (Phase 5.2) |
| 2026-02-18 | Feature | `src-tauri/src/lib.rs` | Prelude module with re-exports for all major subsystems (Phase 5.3) |
| 2026-02-18 | Bugfix | `src-tauri/src/agent/tool_parser.rs` | JSON parser used `?` in loop — now `continue` on bad entries (Phase 5.4) |
| 2026-02-18 | Feature | `src-tauri/src/agent/tool_parser.rs` | `has_partial_tool_call()` for streaming detection (Phase 5.4) |
| 2026-02-18 | Feature | `src-tauri/src/bin/cli.rs` | `module` CLI subcommand (Phase 5.5) |
| 2026-02-18 | Feature | `src-tauri/src/gateway/routes.rs` | Module gateway route stubs (Phase 5.5) |
| 2026-02-18 | Feature | `src-tauri/Cargo.toml` | Tauri plugin baseline: shell, single-instance, updater, deep-link (Phase 5.6) |
| 2026-02-18 | Feature | `src-tauri/src/channels/traits.rs` | `Channel` async-trait, `ChannelMessage`, `ChannelEvent` (Phase 6.1) |
| 2026-02-18 | Feature | `src-tauri/src/channels/tauri_ipc.rs` | `TauriIpcChannel` wrapping `EventBus` (Phase 6.1) |
| 2026-02-18 | Feature | `src-tauri/src/channels/manager.rs` | `ChannelManager` — register/send/start_all/health_all (Phase 6.1) |
| 2026-02-18 | Feature | `src-tauri/src/services/boot.rs` | `BootSequence` — 9-step ordered startup with `SystemReady` (Phase 6.2) |
