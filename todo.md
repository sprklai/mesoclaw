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
| ✅ | `src-tauri/src/gateway/routes.rs` | 39 | Wire `start_session` route to actual agent session manager (Phase 3) | High |
| ✅ | `src-tauri/src/gateway/routes.rs` | 57 | Return real sessions from agent session store (Phase 3) | High |
| ⏳ | `src-tauri/src/gateway/routes.rs` | 66 | Query real provider health from DB instead of stub (needs DbPool in GatewayState — Phase 3) | Medium |
| ✅ | `src-tauri/src/gateway/routes.rs` | 83–135 | Wire module routes (`list`, `health`) to `ModuleRegistry`; start/stop/reload remain 501 (need SidecarService lifecycle) | High |
| ⏳ | `src-tauri/src/services/notification_service.rs` | 199 | Call `tauri_plugin_notification` to show OS notifications (Phase 4 follow-up) | Medium |
| ✅ | `src-tauri/src/agent/agent_commands.rs` | 32 | Implement proper provider resolution from app state in `run_agent_command` | High |
| ✅ | `src-tauri/src/agent/agent_commands.rs` | 43 | Implement session cancellation via `CancellationToken` | High |
| ⏳ | `src-tauri/src/agent/session_router.rs` | 179 | Session router: complete Phase 4.3.3 wiring | Medium |
| ✅ | `src-tauri/src/agent/loop_.rs` | 244 | Implement full approval flow via EventBus (currently logs only) | High |
| ✅ | `src-tauri/src/scheduler/commands.rs` | 12–45 | Resolve managed `TokioScheduler` from app state in all scheduler IPC commands | Medium |
| ✅ | `src-tauri/src/scheduler/tokio_scheduler.rs` | 217 | Persist scheduled jobs to SQLite `scheduled_jobs` table — `add_job` upserts, `new_with_persistence` loads on startup | Low |
| ✅ | `src-tauri/src/scheduler/tokio_scheduler.rs` | 226 | Delete from `scheduled_jobs` SQLite table on `remove_job` | Low |
| ✅ | `src-tauri/src/scheduler/tokio_scheduler.rs` | 262 | Run heartbeat items via `AgentLoop` — wired when `AgentComponents` present, graceful skip otherwise | Medium |
| ✅ | `src-tauri/src/scheduler/tokio_scheduler.rs` | 266 | Run `AgentTurn` prompt through `AgentLoop` — wired when `AgentComponents` present | Medium |
| ✅ | `src-tauri/src/memory/commands.rs` | 17–41 | Resolve managed `InMemoryStore` from app state in all memory IPC commands | Medium |
| ⏳ | `src-tauri/src/modules/mod.rs` | 17 | Configure bundled sidecar binaries in `tauri.conf.json` | Low |
| ⏳ | `src-tauri/src/bin/cli.rs` | 424 | Implement `memory` CLI subcommand once memory service REST endpoint exists (Phase 3) | Low |
| ⏳ | `src-tauri/src/bin/cli.rs` | 429 | Wire `identity` CLI subcommand to identity REST endpoint (Phase 2.6) | Low |
| ⏳ | `src-tauri/src/bin/cli.rs` | 479 | Implement package-registry `install`/`remove` (Phase 6+) | Low |
| 🔄 | `src/stores/identityStore.ts` | 7 | Migrate identity store to gateway REST API — blocked: no `/api/v1/identity/*` routes on gateway yet | Medium |
| 🔄 | `src/stores/llm.ts` | 4 | Migrate provider listing to gateway — blocked: provider_status endpoint needs DbPool in GatewayState | Medium |
| 🔄 | `src/lib/tauri/identity/index.ts` | 4 | Migrate identity CRUD to gateway — blocked: no gateway identity endpoints yet | Medium |
| ✅ | `src/lib/gateway-client.ts` | 61 | `get_daemon_config_command` IPC reads daemon.pid (port) + daemon.token; `resolveDaemonConfig()` wired | Low |
| ✅ | `src/lib/gateway-client.ts` | 198 | Approval endpoint resolved: `sendApprovalResponse` correctly uses `approve_action_command` IPC (EventBus is source of truth) | Medium |
| ⏳ | `src-tauri/` | — | Phase 7.4.1–7.4.6, 7.4.8: `tauri ios init` + `tauri android init`, code signing, TestFlight / Google Play distribution — requires macOS + Xcode + Android SDK | High |
| ⏳ | `src/components/settings/MobileSettings.tsx` | — | Wire push notifications to `tauri-plugin-notification` once APNs/FCM signing is configured (Phase 7.4.4) | Medium |

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
| ✅ | `src-tauri/src/security/policy.rs` | 274 | `extract_executable` checked only first token — `FOO=1 rm …` bypassed block list | Security boundary unreliable | High |
| ✅ | `src-tauri/src/security/policy.rs` | 312 | `detect_injection` missed `\n`/`\r` — multiline commands evaded check | Security boundary unreliable | High |
| ✅ | `src-tauri/src/modules/sidecar_tool.rs` | 218 | `spawn_native_child` didn't set `current_dir` — scaffolded `"./<name>"` failed to resolve | Freshly created modules un-runnable | High |
| ✅ | `src-tauri/src/bin/cli.rs` | 366 | `daemon start` blocked the shell (foreground server) | Scripts/UX blocked until termination | Medium |
| ✅ | `src-tauri/src/channels/tauri_ipc.rs` | 75 | `event_to_channel_message` converted `AgentComplete` → inbound msg, creating feedback loop | Potential echo storm in agent loop | Medium |

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
| ✅ | `src-tauri/src/security/policy.rs` | 274–330 | Shell policy bypass via env-prefix and newline forms — fixed by patching `extract_executable` and `detect_injection` | High | High |

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
| 2026-02-18 | Security | `src-tauri/src/security/policy.rs` | Shell policy bypass: `extract_executable` now skips `VAR=value` tokens; `detect_injection` blocks `\n`/`\r` |
| 2026-02-18 | Bugfix | `src-tauri/src/modules/sidecar_tool.rs` | `SidecarTool` now stores `module_dir` and passes it as `current_dir` to `spawn_native_child` |
| 2026-02-18 | Bugfix | `src-tauri/src/modules/mod.rs` | Module registry passes discovered `path` as `module_dir` when constructing `SidecarTool` |
| 2026-02-18 | Bugfix | `src-tauri/src/gateway/routes.rs` | Stub endpoints `create_session`, `start_module`, `stop_module` now return 501 NOT_IMPLEMENTED instead of 201/202 |
| 2026-02-18 | Bugfix | `src-tauri/src/bin/cli.rs` | `daemon start` self-spawns with `--foreground` flag and returns to shell; no longer blocks |
| 2026-02-18 | Bugfix | `src-tauri/src/channels/tauri_ipc.rs` | `event_to_channel_message` returns `None` for `AgentComplete` — removes output→input feedback loop |
| 2026-02-18 | Feature | `src-tauri/src/channels/telegram.rs` | TelegramChannel: MarkdownV2 escaping, 4096-char split, exponential back-off, allow-list, 33 tests (Phase 7.1) |
| 2026-02-18 | Feature | `src/stores/channelStore.ts` | Zustand channel store; `ChannelList.tsx` + `TelegramConfig.tsx` settings UI (Phase 7.2) |
| 2026-02-18 | Feature | `src/hooks/useMobileSwipe.ts` | Swipe gesture hook; edge-right opens sidebar, swipe-left closes (Phase 7.3.1) |
| 2026-02-18 | Feature | `src/hooks/usePullToRefresh.ts` | Pull-to-refresh hook with `isPulling`/`isRefreshing` state (Phase 7.3.2) |
| 2026-02-18 | Feature | `src/hooks/useVirtualKeyboard.ts` | VisualViewport keyboard height tracker; sets `--keyboard-height` CSS var (Phase 7.3.3) |
| 2026-02-18 | Feature | `src/hooks/useHaptic.ts` | Haptic feedback via `navigator.vibrate()` with light/medium/heavy styles (Phase 7.3.4) |
| 2026-02-18 | Feature | `src/stores/offlineQueueStore.ts` | Persisted offline message queue with auto-flush on `window.online` (Phase 7.3.5) |
| 2026-02-18 | Feature | `src/stores/sidebarStore.ts` | Zustand store for mobile drawer open/close; wired to swipe gesture (Phase 7.3) |
| 2026-02-18 | Feature | `src/stores/mobileSettingsStore.ts` | localStorage-persisted mobile prefs: haptic toggle, push notifications, battery, background refresh (Phase 7.4.7) |
| 2026-02-18 | Feature | `src/components/settings/MobileSettings.tsx` | Mobile Settings tab: haptic toggle, push notification opt-in, Android/iOS guidance (Phase 7.4.7) |
| 2026-02-18 | Bugfix | `src-tauri/src/event_bus/tauri_bridge.rs` | Use `tauri::async_runtime::spawn` instead of `tokio::spawn` for Tauri runtime consistency |
| 2026-02-18 | CI/CD | `.github/workflows/ci.yml` | Unified CI pipeline: fmt + clippy + test + bun test on 3-platform matrix with Rust/Bun caching (Phase 8.1) |
| 2026-02-18 | CI/CD | `.github/workflows/release.yml` | Multi-platform signed release pipeline: 8 desktop configurations, macOS notarization, Windows Azure signing, updater keys (Phase 8.2) |
| 2026-02-18 | CI/CD | `.github/workflows/build-test.yml` | Unsigned test build workflow for manual verification across all platforms (Phase 8.2) |
| 2026-02-18 | CI/CD | `.github/workflows/mobile.yml` | iOS (TestFlight) and Android (Google Play Internal) build pipeline (Phase 8.3) |
| 2026-02-18 | CI/CD | `.github/workflows/labeler.yml` | PR auto-labeling via `actions/labeler@v5` (Phase 8.4) |
| 2026-02-18 | CI/CD | `.github/workflows/stale.yml` | Auto-close stale issues after 60+14 days (Phase 8.4) |
| 2026-02-18 | Config | `.github/dependabot.yml` | Weekly Cargo + npm/Bun + Actions dependency update PRs (Phase 8.4) |
| 2026-02-18 | Config | `.github/labeler.yml` | PR label mappings for 11 file-path categories (Phase 8.4) |
| 2026-02-18 | Docs | `.github/ISSUE_TEMPLATE/` | Bug report + feature request YAML forms + config disabling blank issues (Phase 8.5) |
| 2026-02-18 | Docs | `.github/pull_request_template.md` | Structured PR form: summary, change type, validation evidence, security impact (Phase 8.5) |
| 2026-02-18 | Docs | `CONTRIBUTING.md` | Full contributor guide: setup, collaboration tracks A/B/C, commit convention, extensibility examples (Phase 8.5) |
| 2026-02-18 | Docs | `SECURITY.md` | Security policy: GitHub Advisories reporting, 48h/1w/2w SLA, architecture summary (Phase 8.5) |
| 2026-02-18 | Docs | `.github/CODEOWNERS` | Auto-review routing for security/agent/CI paths (Phase 8.5) |
| 2026-02-18 | Docs | `CODE_OF_CONDUCT.md` | Contributor Covenant v2.1 — community standards and enforcement (Phase 8.5) |
