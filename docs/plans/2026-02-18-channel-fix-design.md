# Channel Fix Design

**Date:** 2026-02-18
**Status:** Approved
**Scope:** Gap analysis items 5 and 6 — channel command stubs + Telegram runtime wiring

---

## Problem Summary

Three concrete blockers prevent channels from working at runtime:

1. `ChannelManager` is never added to Tauri managed state — created in `lib.rs:178`, passed into `BootSequence`, then lost. Channel IPC commands have no way to reach it.
2. `BootContext.message_rx` (the aggregated inbound receiver from all channels) is dropped silently after boot. Inbound messages from any channel are discarded.
3. `TelegramChannel` is never registered at runtime even when the `channels-telegram` feature is enabled.

Channel commands (`connect_channel_command`, `disconnect_channel_command`, `test_channel_connection_command`, `list_channels_command`) are all stubs because of blocker 1.

---

## Approach

**Approach B — Full channel fix** (chosen over Minimal/state-only and UserMessage-event approaches).

- Keep direct Tauri invoke as the sole inbound path for IPC (no `AppEvent::UserMessage` needed).
- `TauriIpcChannel::listen()` stays as-is — no-op by design for IPC; external channels (Telegram, webhook) use the mpsc path through `ChannelManager`.
- Full Telegram connect-from-UI (token keyring flow + 7.1.6 UI) is deferred; only silent boot registration when a token already exists is in scope here.

---

## Section 1 — State Wiring (`src-tauri/src/lib.rs`)

Add `app.manage(Arc::clone(&channel_mgr))` before the spawn block so all Tauri commands can access `ChannelManager` via `State<'_, Arc<ChannelManager>>`.

**Files:** `src-tauri/src/lib.rs` — 1 line added.

---

## Section 2 — Channel Commands (`src-tauri/src/commands/channels.rs`)

All four commands accept a new `State<'_, Arc<ChannelManager>>` parameter. Real implementations:

| Command | Before | After |
|---|---|---|
| `list_channels_command` | hardcoded 2-item vec | `mgr.channel_names()` + `mgr.health_all()` |
| `test_channel_connection_command` | always `false` | `mgr.health_all()[name]` |
| `disconnect_channel_command` | no-op log | `mgr.unregister(&name)` |
| `connect_channel_command` | always `Err(...)` | returns current channel health (full Telegram connect deferred to 7.1.6) |

**Files:** `src-tauri/src/commands/channels.rs` — ~40 lines changed.

---

## Section 3 — Telegram Boot Registration (`src-tauri/src/lib.rs`)

Under `#[cfg(feature = "channels-telegram")]`, inside the existing boot spawn block before `BootSequence::new()`, load the bot token from OS keyring and register `TelegramChannel` if a token is found. Silent skip if no token (user hasn't configured it yet).

```rust
#[cfg(feature = "channels-telegram")]
{
    let entry = keyring::Entry::new("mesoclaw", "telegram_bot_token")
        .map_err(|e| log::warn!(...));
    if let Ok(token) = entry.and_then(|e| e.get_password()) {
        let config = channels::TelegramConfig { token, ..Default::default() };
        let telegram = Arc::new(channels::TelegramChannel::new(config));
        if let Err(e) = channel_mgr_clone.register(telegram).await {
            log::warn!("boot: telegram channel registration failed: {e}");
        }
    } else {
        log::info!("boot: no telegram bot token found, channel not started");
    }
}
```

**Files:** `src-tauri/src/lib.rs` — ~15 lines added (feature-gated).

---

## Section 4 — Channel Router Task (`src-tauri/src/lib.rs`)

After `BootSequence::run()` succeeds, spawn a `channel_router` task that polls `BootContext.message_rx` and publishes each inbound `ChannelMessage` to the EventBus as `AppEvent::ChannelMessage`.

This makes all channel messages observable on the bus. The frontend can react to them immediately. A future "agent subscriber" (7.1.5) will subscribe to `EventType::ChannelMessage` and route to `AgentLoop` via `SessionRouter` — that's a separate design pending session-per-channel architecture.

**Files:** `src-tauri/src/lib.rs` — ~12 lines added.

---

## Out of Scope

- Full Telegram connect-from-UI (token input → keyring → live registration) — Phase 7.1.6
- `AppEvent::ChannelMessage` → AgentLoop routing subscriber (7.1.5) — requires SessionRouter + per-channel session context, separate design
- Webhook channel implementation — not in this codebase yet
- Module lifecycle commands (gap items 4, 9)
- Daily memory date listing (gap item 3)

---

## Files Changed

| File | Change |
|---|---|
| `src-tauri/src/lib.rs` | Add `app.manage()`, Telegram conditional registration, channel router task |
| `src-tauri/src/commands/channels.rs` | De-stub all 4 commands with real ChannelManager state |

---

## Testing

- `cargo test --lib` must pass (372 tests, zero failures)
- Manual: `list_channels_command` returns real channel names and health
- Manual: `test_channel_connection_command` returns real health boolean
- Manual: `disconnect_channel_command` actually removes a channel from ChannelManager
- `channels-telegram` feature: `cargo test --features channels-telegram --lib` must pass
