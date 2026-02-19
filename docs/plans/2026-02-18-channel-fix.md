# Channel Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire `ChannelManager` into Tauri state, de-stub all four channel IPC commands, register `TelegramChannel` at boot under the `channels-telegram` feature flag, and prevent inbound `message_rx` from being silently dropped.

**Architecture:** 4 sequential tasks across 2 files. Task 1 (state wiring) must land before Task 2 (commands) because commands depend on state. Task 3 (Telegram boot) and Task 4 (channel router) both modify `lib.rs` and should come after Task 1. All changes are additive and non-breaking to existing invoke paths.

**Tech Stack:** Rust 2024, Tauri 2, tokio mpsc/broadcast, `keyring` crate (already in `Cargo.toml`), `channels-telegram` Cargo feature flag.

---

## Task 1: Add ChannelManager to Tauri managed state

**Files:**
- Modify: `src-tauri/src/lib.rs` (line ~178, 1 line added)

**Context:** `channel_mgr` is `Arc::new(channels::ChannelManager::new())` at `lib.rs:178` but is only cloned into an async spawn block and passed to `BootSequence`. It is never passed to `app.manage()`, so zero Tauri commands can access it via `State<>`.

---

**Step 1: Add `app.manage()` call**

In `src-tauri/src/lib.rs`, after the line `let channel_mgr = Arc::new(channels::ChannelManager::new());`, add exactly one line:

```rust
let channel_mgr = Arc::new(channels::ChannelManager::new());
app.manage(Arc::clone(&channel_mgr));   // ← ADD THIS
let ipc_ch = Arc::new(channels::TauriIpcChannel::new(Arc::clone(&bus_boot)));
```

No other changes in this step.

---

**Step 2: Verify it compiles**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

Expected: `Finished dev [unoptimized + debuginfo] target(s)`

---

**Step 3: Run the full test suite**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -5
```

Expected: `test result: ok. 372 passed; 0 failed`

---

**Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix(channels): expose ChannelManager via Tauri managed state"
```

---

## Task 2: De-stub channel IPC commands

**Files:**
- Modify: `src-tauri/src/commands/channels.rs` (full rewrite — ~70 lines)

**Context:** All four commands are stubs that ignore the `ChannelManager`. With Task 1 done, they can now accept `State<'_, Arc<ChannelManager>>` and delegate to real methods:

| Command | Before | After |
|---|---|---|
| `list_channels_command` | hardcoded 2-item vec | `mgr.channel_names()` + `mgr.health_all()` |
| `test_channel_connection_command` | always `false` | `mgr.health_all()[name]` |
| `disconnect_channel_command` | no-op log | `mgr.unregister(&name)` |
| `connect_channel_command` | always `Err(...)` | returns current health (full Telegram connect is 7.1.6) |

---

**Step 1: Write tests for the underlying logic**

Append the following `#[cfg(test)]` block to the end of `src-tauri/src/commands/channels.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::{Channel, ChannelManager, ChannelMessage};
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    struct FakeChannel {
        id: String,
        healthy: bool,
    }

    #[async_trait]
    impl Channel for FakeChannel {
        fn name(&self) -> &str {
            &self.id
        }
        async fn send(&self, _: &str, _: Option<&str>) -> Result<(), String> {
            Ok(())
        }
        async fn listen(&self, _: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
            Ok(())
        }
        async fn health_check(&self) -> bool {
            self.healthy
        }
    }

    fn healthy(name: &str) -> Arc<dyn Channel> {
        Arc::new(FakeChannel { id: name.to_string(), healthy: true })
    }

    fn unhealthy(name: &str) -> Arc<dyn Channel> {
        Arc::new(FakeChannel { id: name.to_string(), healthy: false })
    }

    #[tokio::test]
    async fn list_channels_returns_all_with_health() {
        let mgr = Arc::new(ChannelManager::new());
        mgr.register(healthy("tauri-ipc")).await.unwrap();
        mgr.register(unhealthy("telegram")).await.unwrap();

        let health = mgr.health_all().await;
        let mut names = mgr.channel_names().await;
        names.sort();
        let result: Vec<ChannelStatusPayload> = names
            .into_iter()
            .map(|name| {
                let connected = health.get(&name).copied().unwrap_or(false);
                ChannelStatusPayload { name, connected, error: None }
            })
            .collect();

        assert_eq!(result.len(), 2);
        let ipc = result.iter().find(|p| p.name == "tauri-ipc").unwrap();
        assert!(ipc.connected);
        let tg = result.iter().find(|p| p.name == "telegram").unwrap();
        assert!(!tg.connected);
    }

    #[tokio::test]
    async fn health_check_returns_true_for_healthy_channel() {
        let mgr = Arc::new(ChannelManager::new());
        mgr.register(healthy("tauri-ipc")).await.unwrap();
        let health = mgr.health_all().await;
        let result = health.get("tauri-ipc").copied()
            .ok_or_else(|| "Channel 'tauri-ipc' not found".to_string());
        assert_eq!(result, Ok(true));
    }

    #[tokio::test]
    async fn health_check_returns_err_for_unknown_channel() {
        let mgr = Arc::new(ChannelManager::new());
        let health = mgr.health_all().await;
        let result: Result<bool, String> = health.get("ghost").copied()
            .ok_or_else(|| "Channel 'ghost' not found".to_string());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn disconnect_unregisters_channel() {
        let mgr = Arc::new(ChannelManager::new());
        mgr.register(healthy("tauri-ipc")).await.unwrap();
        assert_eq!(mgr.len().await, 1);
        assert!(mgr.unregister("tauri-ipc").await);
        assert!(mgr.is_empty().await);
    }

    #[tokio::test]
    async fn disconnect_returns_false_for_unknown_channel() {
        let mgr = Arc::new(ChannelManager::new());
        assert!(!mgr.unregister("ghost").await);
    }
}
```

---

**Step 2: Run the tests (they should pass — they test ChannelManager behavior)**

```bash
cd src-tauri && cargo test --lib commands::channels::tests 2>&1 | tail -10
```

Expected: `test result: ok. 5 passed; 0 failed`

If any fail, fix the test assertion before proceeding.

---

**Step 3: Rewrite the four commands**

Replace the entire content of `src-tauri/src/commands/channels.rs` with the following (the test module from Step 1 stays at the bottom):

```rust
//! Tauri IPC commands for the Channels settings panel.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::channels::ChannelManager;

// ─── ChannelStatusPayload ─────────────────────────────────────────────────────

/// Status payload returned to the frontend.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStatusPayload {
    /// Channel name (e.g. `"telegram"`, `"tauri-ipc"`).
    pub name: String,
    /// Whether the channel is currently healthy.
    pub connected: bool,
    /// Optional human-readable error message.
    pub error: Option<String>,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Return the current health status for the named channel.
///
/// For a full Telegram connect flow (load token → register → start listener),
/// see Phase 7.1.6 (token management UI).
#[tauri::command]
pub async fn connect_channel_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<ChannelStatusPayload, String> {
    let health = mgr.health_all().await;
    let connected = health
        .get(&name)
        .copied()
        .ok_or_else(|| format!("Channel '{name}' not found"))?;
    Ok(ChannelStatusPayload { name, connected, error: None })
}

/// Unregister the named channel from the channel manager.
#[tauri::command]
pub async fn disconnect_channel_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<(), String> {
    if mgr.unregister(&name).await {
        Ok(())
    } else {
        Err(format!("Channel '{name}' not found"))
    }
}

/// Return `true` if the named channel's health check passes.
#[tauri::command]
pub async fn test_channel_connection_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<bool, String> {
    let health = mgr.health_all().await;
    health
        .get(&name)
        .copied()
        .ok_or_else(|| format!("Channel '{name}' not found"))
}

/// List all registered channels with their connection status.
#[tauri::command]
pub async fn list_channels_command(
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<Vec<ChannelStatusPayload>, String> {
    let health = mgr.health_all().await;
    let mut names = mgr.channel_names().await;
    names.sort(); // deterministic order for UI
    Ok(names
        .into_iter()
        .map(|name| {
            let connected = health.get(&name).copied().unwrap_or(false);
            ChannelStatusPayload { name, connected, error: None }
        })
        .collect())
}
```

Then paste the `#[cfg(test)] mod tests { ... }` block from Step 1 at the end of the file.

---

**Step 4: cargo check**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

Expected: `Finished dev`

If you see `error[E0425]: cannot find value 'mgr'` or similar, check that `use tauri::State` and `use crate::channels::ChannelManager` are present at the top.

---

**Step 5: Run the full test suite**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -5
```

Expected: `test result: ok. 377 passed; 0 failed` (5 new tests)

---

**Step 6: Commit**

```bash
git add src-tauri/src/commands/channels.rs
git commit -m "fix(channels): de-stub channel commands with real ChannelManager state"
```

---

## Task 3: Register TelegramChannel at boot (channels-telegram feature)

**Files:**
- Modify: `src-tauri/src/lib.rs` (inside the existing `tauri::async_runtime::spawn` block)

**Context:** Even with `channels-telegram` enabled, `TelegramChannel` is never registered. This step conditionally registers it at boot when a bot token exists in the OS keyring. No token → silent skip (user hasn't configured it yet).

Key types:
- `channels::telegram::TelegramConfig::new(token: impl Into<String>) -> TelegramConfig`
- `channels::telegram::TelegramChannel::new(config: TelegramConfig) -> TelegramChannel`
- `keyring::Entry::new("mesoclaw", "telegram_bot_token")` — same pattern as LLM API keys

---

**Step 1: Establish a baseline test run with the feature**

```bash
cd src-tauri && cargo test --features channels-telegram --lib 2>&1 | tail -5
```

Note the passing count. This is your baseline.

---

**Step 2: Add Telegram conditional registration in `lib.rs`**

Inside the `tauri::async_runtime::spawn(async move { ... })` block in `src-tauri/src/lib.rs`, add the following block **after** the `channel_mgr_clone.register(ipc_ch)` call and **before** the `BootSequence::new(...)` call:

```rust
// Conditionally register Telegram if a bot token is in the OS keyring.
#[cfg(feature = "channels-telegram")]
{
    match keyring::Entry::new("mesoclaw", "telegram_bot_token")
        .and_then(|e| e.get_password())
    {
        Ok(token) if !token.is_empty() => {
            let config = channels::telegram::TelegramConfig::new(token);
            let telegram = Arc::new(channels::telegram::TelegramChannel::new(config));
            if let Err(e) = channel_mgr_clone.register(telegram).await {
                log::warn!("boot: telegram channel registration failed: {e}");
            } else {
                log::info!("boot: telegram channel registered");
            }
        }
        Ok(_) => log::info!("boot: telegram token empty, channel not started"),
        Err(e) => log::info!("boot: no telegram token in keyring ({e}), channel not started"),
    }
}
```

The full spawn block should now read (abbreviated):

```rust
tauri::async_runtime::spawn(async move {
    if let Err(e) = channel_mgr_clone.register(ipc_ch).await {
        log::warn!("boot: failed to register tauri-ipc channel: {e}");
    }

    // ← INSERT Telegram block here

    match services::boot::BootSequence::new(bus_boot_clone, channel_mgr_clone) {
        Ok(seq) => match seq.run().await {
            Ok(ctx) => {
                log::info!("boot: sequence complete; {} channel handle(s)", ctx.channel_handles.len());
            }
            ...
        },
        ...
    }
});
```

---

**Step 3: cargo check with feature**

```bash
cd src-tauri && cargo check --features channels-telegram 2>&1 | tail -5
```

Expected: `Finished dev`

If you see `error[E0433]: failed to resolve: use of undeclared crate or module 'keyring'`, add `extern crate keyring;` at the top of `lib.rs` or use the full path `::keyring::Entry`. The `keyring` crate is in `Cargo.toml:105`.

---

**Step 4: Run tests with feature flag**

```bash
cd src-tauri && cargo test --features channels-telegram --lib 2>&1 | tail -5
```

Expected: same passing count as baseline (no new tests — boot wiring is integration-level behavior, no keyring in unit tests)

---

**Step 5: Confirm without feature also still passes**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -5
```

Expected: same as before (feature-gated block is excluded)

---

**Step 6: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "fix(channels): register TelegramChannel at boot when token is in keyring"
```

---

## Task 4: Spawn channel router to prevent message_rx drop

**Files:**
- Modify: `src-tauri/src/lib.rs` (the `Ok(ctx) =>` arm, ~15 lines changed/added)
- Modify: `src-tauri/src/services/boot.rs` (add 1 test)

**Context:** `BootContext.message_rx` is the aggregated `mpsc::Receiver<ChannelMessage>` from `ChannelManager::start_all()`. Currently it falls out of scope at the end of `Ok(ctx) => { log::info!(...) }` and is dropped — all inbound channel messages are discarded. Fix: spawn a `channel_router` task that drains the receiver and publishes each message as `AppEvent::ChannelMessage` on the `EventBus`.

---

**Step 1: Write a test for the router logic in `services/boot.rs`**

Append this test to the `#[cfg(test)] mod tests` block at the bottom of `src-tauri/src/services/boot.rs`:

```rust
#[tokio::test]
async fn channel_messages_reach_event_bus_via_router() {
    use crate::channels::{Channel, ChannelManager, ChannelMessage};
    use crate::event_bus::{AppEvent, EventBus, TokioBroadcastBus};
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    // A test channel that sends exactly one message then exits listen().
    struct OneShot;

    #[async_trait]
    impl Channel for OneShot {
        fn name(&self) -> &str { "test-oneshot" }
        async fn send(&self, _: &str, _: Option<&str>) -> Result<(), String> { Ok(()) }
        async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
            let msg = ChannelMessage::new("test-oneshot", "hello from router test");
            let _ = tx.send(msg).await;
            Ok(())
        }
        async fn health_check(&self) -> bool { true }
    }

    let bus = Arc::new(TokioBroadcastBus::new());
    let mut bus_rx = bus.subscribe();

    let mgr = Arc::new(ChannelManager::new());
    mgr.register(Arc::new(OneShot)).await.unwrap();

    let (mut message_rx, _handles) = mgr.start_all(8).await;

    // Simulate the router task: drain message_rx → publish AppEvent::ChannelMessage.
    let bus_clone = Arc::clone(&bus) as Arc<dyn EventBus>;
    tokio::spawn(async move {
        while let Some(msg) = message_rx.recv().await {
            let _ = bus_clone.publish(AppEvent::ChannelMessage {
                channel: msg.channel,
                from:    msg.sender.unwrap_or_default(),
                content: msg.content,
            });
        }
    });

    // Give the task a tick to run.
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Drain bus events and look for AppEvent::ChannelMessage.
    let mut found = false;
    while let Ok(evt) = bus_rx.try_recv() {
        if matches!(evt, AppEvent::ChannelMessage { .. }) {
            found = true;
            break;
        }
    }
    assert!(found, "AppEvent::ChannelMessage should have been published to the EventBus");
}
```

---

**Step 2: Run the new test to confirm it passes**

```bash
cd src-tauri && cargo test --lib services::boot::tests::channel_messages_reach_event_bus_via_router -- --nocapture 2>&1 | tail -10
```

Expected: `test result: ok. 1 passed; 0 failed`

If the test hangs, add `#[tokio::test(flavor = "multi_thread")]` to the test.

---

**Step 3: Add `bus_for_router` clone before the spawn block in `lib.rs`**

In `src-tauri/src/lib.rs`, find the two existing clone lines before `tauri::async_runtime::spawn`:

```rust
let channel_mgr_clone = Arc::clone(&channel_mgr);
let bus_boot_clone = Arc::clone(&bus_boot);
tauri::async_runtime::spawn(async move {
```

Add a third clone for the router:

```rust
let channel_mgr_clone = Arc::clone(&channel_mgr);
let bus_boot_clone = Arc::clone(&bus_boot);
let bus_for_router = Arc::clone(&bus_boot);    // ← ADD THIS
tauri::async_runtime::spawn(async move {
```

---

**Step 4: Replace the `Ok(ctx) =>` arm to spawn the router**

Find this arm inside the spawn block:

```rust
Ok(ctx) => {
    log::info!("boot: sequence complete; {} channel handle(s)", ctx.channel_handles.len());
    // Background scheduler keeps running until the process exits.
}
```

Replace it with:

```rust
Ok(ctx) => {
    log::info!(
        "boot: sequence complete; {} channel handle(s)",
        ctx.channel_handles.len()
    );
    // Drain inbound channel messages → EventBus so subscribers can observe them.
    // A future agent-routing subscriber (7.1.5) will act on AppEvent::ChannelMessage.
    tokio::spawn(async move {
        let mut rx = ctx.message_rx;
        while let Some(msg) = rx.recv().await {
            let _ = bus_for_router.publish(event_bus::AppEvent::ChannelMessage {
                channel: msg.channel,
                from:    msg.sender.unwrap_or_default(),
                content: msg.content,
            });
        }
        log::info!("channel_router: message_rx closed");
    });
}
```

Note: `bus_for_router` is captured by the inner `tokio::spawn` closure. `event_bus::AppEvent` is the module path used elsewhere in this file.

---

**Step 5: cargo check**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

Expected: `Finished dev`

Common errors:
- `use of moved value: bus_for_router` → make sure `bus_for_router` is cloned before the outer spawn (Step 3), not inside it.
- `event_bus::AppEvent` not found → try `crate::event_bus::AppEvent` instead.

---

**Step 6: Run the full test suite**

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -5
```

Expected: `test result: ok. 378 passed; 0 failed` (1 new test in boot.rs)

---

**Step 7: Run with Telegram feature**

```bash
cd src-tauri && cargo test --features channels-telegram --lib 2>&1 | tail -5
```

Expected: all pass (same count + Telegram-specific tests)

---

**Step 8: Final clippy check**

```bash
cd src-tauri && cargo clippy -- -D warnings 2>&1 | tail -10
```

Expected: no warnings.

---

**Step 9: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/services/boot.rs
git commit -m "fix(channels): spawn channel router to prevent message_rx drop"
```

---

## Final Verification

Run all three checks before marking complete:

```bash
cd src-tauri && cargo test --lib 2>&1 | tail -3
cd src-tauri && cargo test --features channels-telegram --lib 2>&1 | tail -3
cd src-tauri && cargo clippy -- -D warnings 2>&1 | tail -5
```

All must show zero failures and zero warnings.

---

## What This Does NOT Fix (Separate Tickets)

- **7.1.5** — `AppEvent::ChannelMessage` → AgentLoop routing subscriber (requires SessionRouter + per-channel session management)
- **7.1.6** — Telegram connect-from-UI (token input field → keyring store → live channel registration)
- **Gap item 3** — `list_daily_memory_dates_command` not implemented
- **Gap item 4** — Module IPC commands missing
