# Gap Analysis: Code vs Planning Docs

Date: 2026-02-19

Scope compared:
- `docs/index.md`
- `docs/implementation-plan.md`
- `todo.md`
- Current codebase under `src-tauri/` and `src/`

## A) Implementation Gaps (Code Work Needed)

1. Agent session cancellation is not operational end-to-end.
- Evidence: `start_agent_session_command` returns only response text, no session id (`src-tauri/src/agent/agent_commands.rs:117`).
- Evidence: cancel API requires `session_id` (`src-tauri/src/agent/agent_commands.rs:178`).
- Evidence: cancellation flag is not consumed by `AgentLoop` (`src-tauri/src/agent/loop_.rs:183`).
- Fix needed: expose cancellable session handle, plumb cancellation token/flag into loop checks, and guarantee cleanup on all early-return paths.

2. Memory and scheduler IPC commands exist but are not registered in Tauri invoke handler.
- Evidence: commands implemented in `src-tauri/src/memory/commands.rs` and `src-tauri/src/scheduler/commands.rs`.
- Evidence: invoke list in `src-tauri/src/lib.rs:249` does not include these command functions.
- Evidence: frontend relies on them (`src/stores/memoryStore.ts:75`, `src/stores/schedulerStore.ts:101`).
- Fix needed: add command registration entries in `tauri::generate_handler!`.

3. Missing backend command for daily memory date listing.
- Evidence: frontend calls `list_daily_memory_dates_command` (`src/stores/memoryStore.ts:97`).
- Evidence: no command implementation exists in `src-tauri/src/memory/commands.rs`.
- Fix needed: implement and register `list_daily_memory_dates_command`.

4. Module management frontend calls non-existent IPC commands.
- Evidence: frontend calls `list_modules_command`, `start_module_command`, `stop_module_command`, `create_module_command` (`src/stores/moduleStore.ts:111`, `src/stores/moduleStore.ts:124`, `src/stores/moduleStore.ts:139`, `src/stores/moduleStore.ts:164`).
- Evidence: no matching `#[tauri::command]` implementations were found in backend command modules.
- Fix needed: add/ wire module IPC commands or update frontend to use actual gateway endpoints.

5. Channel management commands are still stubs.
- Evidence: explicit TODO/stub behavior in `src-tauri/src/commands/channels.rs:24`.
- Fix needed: connect these commands to real channel manager state, Telegram config persistence, and health checks.

6. Telegram channel exists but is not wired into app runtime lifecycle.
- Evidence: Telegram implementation present (`src-tauri/src/channels/telegram.rs`).
- Evidence: no runtime registration path for Telegram channel in startup wiring (`src-tauri/src/lib.rs`, `src-tauri/src/services/boot.rs`).
- Fix needed: conditional registration/startup under `channels-telegram` feature with config/key management.

7. Gateway provider status endpoint remains stubbed.
- Evidence: placeholder response in `src-tauri/src/gateway/routes.rs:78`.
- Fix needed: query and return real provider health/status from DB/runtime state.

8. Gateway WebSocket inbound command handling is not implemented.
- Evidence: TODO in `src-tauri/src/gateway/ws.rs:53`.
- Fix needed: parse and route incoming WS commands (approval/session/control) with validation.

9. Module lifecycle routes remain unimplemented (501).
- Evidence: `start_module`, `stop_module`, `reload_modules` return 501 in `src-tauri/src/gateway/routes.rs:130`, `src-tauri/src/gateway/routes.rs:149`, `src-tauri/src/gateway/routes.rs:172`.
- Fix needed: integrate SidecarService/registry lifecycle orchestration.

10. Notification service is not actually dispatching OS notifications.
- Evidence: TODO in `src-tauri/src/services/notification_service.rs:173` and `src-tauri/src/services/notification_service.rs:193`.
- Evidence: no startup wiring found for `NotificationService` in `src-tauri/src/lib.rs`.
- Fix needed: initialize service at startup and call notification plugin APIs.

11. Scheduler lifecycle is inconsistent (double start path).
- Evidence: scheduler started in boot sequence (`src-tauri/src/services/boot.rs:124`).
- Evidence: second scheduler started and managed in app setup (`src-tauri/src/lib.rs:204`).
- Fix needed: single ownership model for scheduler startup and shutdown.

12. Scheduler runtime state mutations are not persisted after execution.
- Evidence: in-memory updates for `error_count` and `next_run` at `src-tauri/src/scheduler/tokio_scheduler.rs:357`.
- Evidence: persistence currently done on add/remove path (`src-tauri/src/scheduler/tokio_scheduler.rs:392`).
- Fix needed: persist post-execution job state updates.

13. Updater plugin is declared but not runtime-integrated.
- Evidence: dependency present (`src-tauri/Cargo.toml:90`).
- Evidence: no updater plugin initialization/use in `src-tauri/src/lib.rs`.
- Fix needed: add runtime updater flow or remove/clarify scope.

14. Mobile build/distribution prerequisites are incomplete in repository state.
- Evidence: `mobile.yml` expects mobile build targets (`.github/workflows/mobile.yml:36`, `.github/workflows/mobile.yml:101`).
- Evidence: no generated iOS/Android project artifacts under `src-tauri/gen` beyond schemas.
- Fix needed: initialize mobile targets and document/signing prerequisites.

## B) Documentation / Status Gaps (Docs Need Updates)

15. `docs/index.md` has stale status pointers.
- Evidence: marks Phase 2 as active (`docs/index.md:41`) while implementation plan marks Phase 2 complete (`docs/implementation-plan.md:400`).
- Evidence: “START HERE” and “Active Work” still point to Phase 0 in-progress (`docs/index.md:92`, `docs/index.md:244`).
- Update needed: align phase status and active-work links with current reality.

16. `docs/implementation-plan.md` contains over-claimed completion statements.
- Evidence: Phase 3 checkpoint says SQLite-backed memory (`docs/implementation-plan.md:479`) while current memory store is in-memory (`src-tauri/src/memory/store.rs:1`).
- Evidence: multiple later checkpoint claims assume runtime wiring not yet complete (channels/module lifecycle/notifications/mobile distribution).
- Update needed: revise checkpoints to “implemented partially” where applicable and list explicit remaining items.

17. `docs/implementation-plan.md` workflow/task mismatch.
- Evidence: plan includes `pr-hygiene.yml` (`docs/implementation-plan.md:949`) but workflow file is absent under `.github/workflows/`.
- Evidence: plan checkpoint claims `bunx ultracite check` in CI (`docs/implementation-plan.md:973`) but `ci.yml` does not run it.
- Update needed: either implement missing workflow/checks or correct the documentation.

18. `todo.md` has inaccurate completion statuses.
- Evidence: cancellation marked done (`todo.md:25`) but cancellation flow is incomplete (see item 1).
- Evidence: scheduler/memory IPC marked done (`todo.md:28`, `todo.md:33`) but commands are not in invoke handler.
- Update needed: set these back to pending/in-progress and add newly identified gaps (double scheduler, post-run persistence, missing module/memory commands).

## C) Suggested Priority Order

1. Fix runtime correctness blockers: items 1, 2, 3, 4.
2. Fix orchestration and reliability: items 10, 11, 12.
3. Complete gateway/channel/module integration: items 5, 6, 7, 8, 9.
4. Reconcile CI/mobile/tooling claims: items 13, 14, 17.
5. Align project docs and tracker status: items 15, 16, 18.
