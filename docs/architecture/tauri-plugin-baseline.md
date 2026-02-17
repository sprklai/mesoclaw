# Tauri Plugin Baseline (v2)

> Purpose: define the minimum plugin baseline for Mesoclaw and close gaps between planned capabilities and current code.
> References:
>
> - Sidecar docs: https://v2.tauri.app/develop/sidecar/
> - Plugin index: https://v2.tauri.app/plugin/

---

## Current Plugin Inventory (from codebase)

Detected in `src-tauri/Cargo.toml` and `src-tauri/src/lib.rs`:

- `tauri-plugin-notification`
- `tauri-plugin-opener`
- `tauri-plugin-log`
- `tauri-plugin-window-state`
- `tauri-plugin-stronghold`
- `tauri-plugin-store`
- `tauri-plugin-autostart` (desktop target)

Capabilities currently configured in `src-tauri/capabilities/default.json` for the above set.

---

## Plugin Gaps vs Planned Architecture

### P1 (important)

1. `tauri-plugin-shell`

- Why: official sidecar/bundled binary execution path and command scoping.
- Required for: trusted bundled helper binaries and permission-scoped execution via capabilities.

2. `tauri-plugin-single-instance`

- Why: prevents duplicate GUI/daemon processes and port/token file contention.
- Required for: CLI + GUI mixed usage where one instance should own runtime services.

3. `tauri-plugin-updater`

- Why: runtime integration for signed updates (already planned in release docs).
- Required for: FR-16.6 and NFR-5.11 alignment.

### P2 (context-driven)

4. `tauri-plugin-deep-link`

- Why: OAuth and external callback flows (providers/channels/Composio style integrations).
- Required when: desktop app must capture custom URI callbacks.

5. `tauri-plugin-dialog`

- Why: safer native file/directory picking UX over ad-hoc paths in frontend.
- Required when: module install/import and local file workflows expand.

---

## Sidecar-Specific Requirements (Official Tauri Guidance)

For bundled sidecars, implement all of the following:

1. Add sidecar binaries to `bundle.externalBin` in `src-tauri/tauri.conf.json`.
2. Enable shell plugin in `src-tauri/Cargo.toml` and initialize in `src-tauri/src/lib.rs`.
3. Add least-privilege shell permissions to `src-tauri/capabilities/default.json` (allow only expected commands/paths).
4. Keep dynamic user-installed modules (`~/.mesoclaw/modules/*`) on backend-managed process/container paths; do not over-broaden shell plugin permissions for arbitrary execution.

---

## Implementation Plan Mapping

- `docs/implementation-plan.md`:
  - Task `2.8` includes sidecar packaging/permission alignment with official docs.
  - Task `5.6` covers plugin baseline hardening (`shell`, `single-instance`, `updater`, `deep-link`).

- `docs/test-plan.md`:
  - Task coverage addendum includes plugin baseline integration and permission-regression checks.

---

## Acceptance Criteria

1. All mandatory plugins are initialized successfully at startup on desktop platforms.
2. `capabilities/default.json` grants only minimum required permissions for enabled plugins.
3. Bundled sidecars run from `externalBin` and fail closed when command scope is violated.
4. Single-instance behavior is enforced and verified by integration test.
5. Updater runtime checks and signed manifest validation are covered by smoke tests.
