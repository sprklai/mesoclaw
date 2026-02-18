# Tauri Plugin Baseline

> Purpose: define the minimum plugin baseline for MesoClaw, document the permission
> policy for each plugin, and record the rationale for inclusion or exclusion.
>
> **Status**: Phase 5.6 complete — all four previously-missing plugins added.
>
> References:
> - Sidecar docs: https://v2.tauri.app/develop/sidecar/
> - Plugin index: https://v2.tauri.app/plugin/

---

## Plugin Inventory

### Mandatory Plugins (all platforms)

These plugins are always initialised in `lib.rs::run()`.

| Plugin | Version | Rationale |
|--------|---------|-----------|
| `tauri-plugin-opener` | 2.5.x | Opens URLs/files in the default browser/app; required for OAuth redirect and external links |
| `tauri-plugin-log` | 2.7.x | Structured logging to console + platform log system; required for observability |
| `tauri-plugin-notification` | 2.3.x | Desktop notification delivery for heartbeats, agent completions, and approval requests |
| `tauri-plugin-stronghold` | 2.x | Encrypted secret storage backed by IOTA Stronghold; the only approved API key storage mechanism |
| `tauri-plugin-store` | 2.x | Persistent key-value store for non-sensitive app settings (theme, layout, onboarding state) |
| `tauri-plugin-shell` | 2.3.x | Permission-scoped shell/sidecar command execution for trusted bundled binaries |
| `tauri-plugin-updater` | 2.10.x | Signed auto-update delivery; required for NFR-5.11 (user-controlled updates) |

### Optional Plugins (desktop-only, `#[cfg(desktop)]`)

| Plugin | Version | Rationale |
|--------|---------|-----------|
| `tauri-plugin-window-state` | 2.4.x | Remembers window size/position across restarts |
| `tauri-plugin-autostart` | 2.5.x | Launch-at-login support; user-configurable, disabled by default |
| `tauri-plugin-single-instance` | 2.4.x | Prevents two desktop processes binding the same daemon port |
| `tauri-plugin-deep-link` | 2.4.x | Handles `mesoclaw://oauth/callback` URIs for OAuth flows |

---

## Permission Policy

All permissions follow the **least-privilege principle**: only the minimum set of
allow-rules needed for each feature is granted in `capabilities/default.json`.
No broad wildcard permissions (e.g. `shell:default`, `fs:default`) are granted.

### `shell` Plugin

Only `shell:allow-open` is granted. This allows opening URLs in the default browser
(e.g. OAuth consent pages) but **does not** allow arbitrary command execution from
the frontend.

Sidecar binaries for extension modules are executed via `tokio::process::Command`
in the Rust backend (see `src-tauri/src/modules/sidecar_tool.rs`), not through
the shell plugin, because they require complex lifecycle management (stdin/stdout
IPC, timeout, kill).

**Explicitly NOT granted:**
- `shell:allow-execute` — prevents frontend from running arbitrary commands
- `shell:allow-spawn` — prevents frontend from spawning processes

### `updater` Plugin

Grants `updater:allow-check` and `updater:allow-download-and-install`.

The update server URL must be set in `tauri.conf.json`:
```json
{
  "plugins": {
    "updater": {
      "endpoints": ["https://releases.mesoclaw.app/{{target}}/{{arch}}/{{current_version}}"],
      "pubkey": "<BASE64_PUBLIC_KEY>"
    }
  }
}
```

Update checks are **not** performed automatically on startup without explicit user
consent. The app checks for updates when the user navigates to Settings → About.

### `deep-link` Plugin

Grants `deep-link:default`.

The `mesoclaw://` URL scheme must be registered in `tauri.conf.json`:
```json
{
  "plugins": {
    "deep-link": {
      "mobile": [],
      "desktop": {
        "schemes": ["mesoclaw"]
      }
    }
  }
}
```

Deep-link handlers are registered in the frontend via `onOpenUrl()` from
`@tauri-apps/plugin-deep-link`. OAuth callback parameters are validated and
sanitised before use.

### `single-instance` Plugin

No frontend permissions required. The plugin is initialised with a callback that
logs the duplicate launch attempt. Future iteration: focus the main window on
duplicate launch detection.

---

## Sidecar-Specific Requirements

For bundled sidecars, implement all of the following:

1. Add sidecar binaries to `bundle.externalBin` in `src-tauri/tauri.conf.json`.
2. The `tauri-plugin-shell` plugin is now initialised (Phase 5.6).
3. Grant only the specific `shell:allow-execute` scope for each bundled binary,
   not a global execute permission.
4. Keep dynamic user-installed modules (`~/.mesoclaw/modules/*`) on backend-managed
   process paths — do not over-broaden shell plugin permissions for arbitrary execution.

---

## Adding a New Plugin

1. Add the crate to `Cargo.toml` under `[dependencies]`.
2. Initialise in `lib.rs::run()`. Use `#[cfg(desktop)]` for desktop-only plugins.
3. Add minimum required permissions to `capabilities/default.json`.
4. Update this document with the plugin entry and permission rationale.
5. If the plugin is optional, add a feature flag so it can be excluded from minimal builds.

---

## Security Considerations

- **Do not grant broad permissions** such as `fs:default` or `shell:default`. Always
  enumerate the specific allow-rules needed.
- **Review permissions after every Tauri upgrade.** Plugin permission identifiers
  change between minor versions; always re-check the plugin changelog.
- **Stronghold is the only approved secret store.** Do not store API keys or tokens
  in the `store` plugin (unencrypted JSON file) or in `localStorage`.
- **Updater keys must be rotated** if the private key is ever exposed. The public
  key embedded in `tauri.conf.json` must match the signing key used by release CI.

---

## Acceptance Criteria

1. All mandatory plugins initialise successfully at startup on desktop platforms.
2. `capabilities/default.json` grants only minimum required permissions for enabled plugins.
3. Bundled sidecars run from `externalBin` and fail closed when command scope is violated.
4. Single-instance behaviour is enforced (second launch logs and ignores).
5. Updater runtime checks and signed manifest validation are covered by smoke tests.
6. Deep-link URIs are validated before parameter extraction.
