---
sidebar_position: 6
title: Development
slug: /development
---

# Zenii Development Guide

This document covers setting up a development environment, building from source, running tests, and contributing to Zenii.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Repository Setup](#repository-setup)
- [Building](#building)
- [Testing](#testing)
- [Frontend Development](#frontend-development)
- [Desktop Development](#desktop-development)
- [How-to Guides](#how-to-guides)
- [Code Style](#code-style)
- [Debugging](#debugging)

---

## Prerequisites

### Required

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.85+ (edition 2024) | Backend compilation |
| Bun | latest | Frontend package manager and bundler |
| SQLite dev libs | 3.x | Database (bundled via rusqlite, but dev headers needed for sqlite3 CLI) |

### Platform-Specific

**Linux (Debian/Ubuntu)**:

```bash
sudo apt install build-essential pkg-config libsqlite3-dev libssl-dev
# For desktop (Tauri) development:
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

**Linux (Fedora)**:

```bash
sudo dnf install gcc pkg-config sqlite-devel openssl-devel
# For desktop (Tauri) development:
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel
```

**macOS**:

```bash
# Xcode command line tools (includes SQLite)
xcode-select --install
# Bun
curl -fsSL https://bun.sh/install | bash
```

**Windows**:

- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with "Desktop development with C++"
- Install [Bun](https://bun.sh) for Windows
- SQLite is bundled via `rusqlite` with the `bundled` feature

### Optional

| Tool | Purpose |
|------|---------|
| `cargo-tauri` | Desktop app development (`cargo install tauri-cli`) |
| `websocat` | WebSocket debugging |
| `sqlite3` | Database inspection |

---

## Repository Setup

```bash
# Clone
git clone https://github.com/sprklai/zenii.git
cd zenii

# Verify Rust workspace compiles
cargo check --workspace

# Run all backend tests
cargo test --workspace

# Install frontend dependencies
cd web && bun install

# Build frontend
bun run build

# Run frontend tests
bun run test
```

### Workspace Structure

```
crates/
├── zenii-core/       All business logic (library crate)
├── zenii-daemon/     Headless daemon (thin binary)
├── zenii-cli/        CLI client (thin binary, HTTP client to daemon)
├── zenii-tui/        TUI client (thin binary)
├── zenii-desktop/    Tauri 2 desktop shell (thin binary)
web/                     SvelteKit frontend (SPA, shared by desktop + web)
scripts/                 Build and utility scripts
docs/                    Architecture, deployment, and process documentation
```

All business logic lives in `zenii-core`. Binary crates are thin shells (under 100 lines each).

---

## Building

### All Binary Targets

```bash
# Debug build (all binaries)
cargo build --workspace

# Release build (optimized, LTO)
cargo build --workspace --release

# Single binary
cargo build -p zenii-daemon
cargo build -p zenii-cli
cargo build -p zenii-tui
```

### Feature Flags

Features are defined on `zenii-core` and flow through to binary crates:

| Feature | Description | Default |
|---------|-------------|---------|
| `gateway` | HTTP+WebSocket gateway (axum) | Yes |
| `ai` | AI agent (rig-core) | Yes |
| `keyring` | OS keyring credential storage | Yes |
| `local-embeddings` | Local FastEmbed ONNX embedding provider | No |
| `channels` | Channel messaging framework | No |
| `channels-telegram` | Telegram bot adapter | No |
| `channels-slack` | Slack bot adapter | No |
| `channels-discord` | Discord bot adapter | No |
| `scheduler` | Cron job scheduler | No |
| `web-dashboard` | Web dashboard (implies gateway) | No |

```bash
# Core only (no optional features)
cargo build -p zenii-daemon

# With channels
cargo build -p zenii-daemon --features channels

# With specific channel adapters
cargo build -p zenii-daemon --features channels-telegram,channels-discord

# With scheduler
cargo build -p zenii-daemon --features scheduler

# Everything
cargo build -p zenii-daemon --all-features
```

### Build Script

The `scripts/build.sh` script handles cross-compilation and packaging:

```bash
# Native debug build
./scripts/build.sh

# Native release build
./scripts/build.sh --release

# Cross-compile for Linux ARM64
./scripts/build.sh --target linux-arm64 --release

# CI release profile (faster than full release)
./scripts/build.sh --target native --profile ci-release

# Build Tauri desktop app
./scripts/build.sh --tauri --release

# List all available targets
./scripts/build.sh --list-targets
```

### Cargo Profiles

| Profile | LTO | Opt Level | Codegen Units | Use Case |
|---------|-----|-----------|---------------|----------|
| `debug` | No | 0 | default | Development |
| `release` | Full | z (size) | 1 | Production binaries |
| `ci-release` | Thin | s (size) | 16 | CI builds (faster) |
| `release-fast` | Thin | z | default + debug | Profiling |

---

## Testing

### Backend Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p zenii-core

# Run a specific test
cargo test -p zenii-core -- test_name

# Run tests with output
cargo test --workspace -- --nocapture

# Run integration tests that require credentials (skipped by default)
cargo test --workspace -- --ignored
```

### Frontend Tests

```bash
cd web
bun run test          # Run Vitest tests
bun run test:watch    # Watch mode
bun run check         # Svelte type checking
```

### Linting

```bash
# Rust linting
cargo clippy --workspace

# Clippy with all features
cargo clippy --workspace --all-features

# Frontend linting
cd web && bun run check
```

### Test Conventions

- Unit tests live in the same file as the code (`#[cfg(test)]` module)
- Integration tests use `#[cfg(test)]` modules in the same file
- Use `tempfile` crate for filesystem tests
- Use `tower::ServiceExt::oneshot()` for gateway handler tests
- Mock external APIs; never call real APIs in unit tests
- Test both success and failure paths
- Tests requiring real credentials use `#[ignore]`

---

## Frontend Development

The frontend is a SvelteKit SPA (Single Page Application) using Svelte 5 runes, shadcn-svelte components, and Tailwind CSS v4.

### Dev Server

```bash
cd web
bun install        # Install dependencies (first time)
bun run dev        # Start Vite dev server on http://localhost:18971
```

The frontend expects the Zenii daemon running on `http://localhost:18981`. Start the daemon in another terminal:

```bash
cargo run -p zenii-daemon
```

### Build

```bash
cd web
bun run build      # Production build (adapter-static, outputs to build/)
bun run preview    # Preview production build locally
```

### Key Frontend Paths

| Path | Description |
|------|-------------|
| `web/src/routes/` | SvelteKit routes (/, /chat, /memory, /settings, /schedule) |
| `web/src/lib/api/` | HTTP client and WebSocket utilities |
| `web/src/lib/stores/` | Svelte 5 rune-based stores |
| `web/src/lib/components/` | shadcn-svelte UI components |
| `web/src/lib/i18n/` | Internationalization (paraglide-js) |

---

## Desktop Development

The desktop app uses Tauri 2 wrapping the SvelteKit frontend.

### Dev Mode

```bash
# Start both Vite dev server and Tauri dev window
./scripts/build.sh --dev
```

This launches the Vite dev server on port 18971 and opens a Tauri window pointing to it. Hot module replacement (HMR) works for frontend changes. Rust changes trigger a recompile.

### Build Desktop App

```bash
# Debug build
./scripts/build.sh --tauri

# Release build
./scripts/build.sh --tauri --release

# With specific bundle format
./scripts/build.sh --tauri --release --bundle deb,appimage
```

### Desktop-Specific Code

| File | Description |
|------|-------------|
| `crates/zenii-desktop/src/main.rs` | Tauri app entry point |
| `crates/zenii-desktop/src/commands.rs` | IPC commands (close_to_tray, show_window, etc.) |
| `crates/zenii-desktop/src/tray.rs` | System tray setup |
| `web/src/lib/tauri.ts` | `isTauri` detection + invoke wrappers |

---

## How-to Guides

### Add a New Agent Tool

1. Create a new file in `crates/zenii-core/src/tools/`:

```rust
use async_trait::async_trait;
use serde_json::Value;
use crate::tools::Tool;
use crate::Result;

pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Description of what my tool does" }
    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": { "type": "string", "description": "The input parameter" }
            },
            "required": ["input"]
        })
    }
    async fn call(&self, args: Value) -> Result<Value> {
        let input = args["input"].as_str().unwrap_or_default();
        // Tool logic here
        Ok(serde_json::json!({ "result": input }))
    }
}
```

2. Register in `crates/zenii-core/src/boot.rs`:

```rust
tools.register(Arc::new(MyTool));
```

3. Add tests in the same file under `#[cfg(test)]`.

### Add a Gateway Route

1. Create a handler in `crates/zenii-core/src/gateway/handlers/`:

```rust
use axum::extract::State;
use axum::Json;
use std::sync::Arc;
use crate::gateway::state::AppState;

pub async fn my_handler(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
```

2. Add the route in `crates/zenii-core/src/gateway/routes.rs`:

```rust
.route("/my-endpoint", get(handlers::my_handler))
```

3. Add tests using `tower::ServiceExt::oneshot()`.

### Add a Channel Adapter

1. Implement the `Channel`, `ChannelLifecycle`, and `ChannelSender` traits in `crates/zenii-core/src/channels/`:

```rust
use async_trait::async_trait;
use crate::channels::traits::{Channel, ChannelLifecycle, ChannelSender, ChannelStatus};
use crate::Result;

pub struct MyChannel { /* ... */ }

#[async_trait]
impl Channel for MyChannel {
    fn name(&self) -> &str { "my_channel" }
    fn status(&self) -> ChannelStatus { /* ... */ }
}

#[async_trait]
impl ChannelLifecycle for MyChannel {
    async fn start(&self) -> Result<()> { /* ... */ }
    async fn stop(&self) -> Result<()> { /* ... */ }
}

#[async_trait]
impl ChannelSender for MyChannel {
    async fn send(&self, message: &crate::channels::message::ChannelMessage) -> Result<()> {
        /* ... */
    }
}
```

2. Add a feature flag in `crates/zenii-core/Cargo.toml`:

```toml
channels-mychannel = ["channels", "dep:my-crate"]
```

3. Register in the `ChannelRegistry` during boot.

### Add a Configuration Field

1. Add the field to `AppConfig` in `crates/zenii-core/src/config/schema.rs`:

```rust
pub struct AppConfig {
    // ...existing fields...
    pub my_new_field: u32,
}
```

2. Add the default value in `impl Default for AppConfig`:

```rust
my_new_field: 42,
```

3. The `#[serde(default)]` on `AppConfig` ensures backwards compatibility -- existing config files without this field will use the default value.

---

## Code Style

### Rust Conventions

- **Naming**: `snake_case` for functions, variables, modules; `PascalCase` for types
- **Error handling**: Use `ZeniiError` enum (thiserror). Never `Result<T, String>` or `.unwrap()` in production code
- **Async**: `tokio::sync` primitives only. Never `std::sync::Mutex` in async paths
- **Logging**: `tracing` macros (`info!`, `warn!`, `error!`, `debug!`). Never `println!`
- **SQLite**: All operations via `spawn_blocking`. rusqlite is sync
- **Imports**: Group as std, external crates, internal modules (blank line separated)
- **No magic numbers**: All tunables go in `AppConfig` with defaults in `schema.rs`
- **Testing**: `#[cfg(test)]` in same file, `tempfile` for FS tests

### TypeScript/Svelte Conventions

- **Naming**: `camelCase` for variables/functions, `PascalCase` for components
- **State**: Svelte 5 `$state` runes, max 1 `$effect` per component
- **Styling**: Tailwind CSS v4, shadcn-svelte components
- **Dark mode**: Use `bg-background text-foreground` on `<select>` elements
- **Real-time**: WebSocket for streaming, never polling

### Credential Key Naming

Colon-separated namespacing:
- AI provider keys: `api_key:{provider_id}` (e.g., `api_key:openai`)
- Channel credentials: `channel:{channel_id}:{field}` (e.g., `channel:telegram:token`)

---

## Debugging

### Log Files

All binaries write daily-rotated diagnostic logs to `{data_dir}/logs/`:

| Binary | Log File Pattern | Notes |
|---|---|---|
| Daemon | `daemon.log.YYYY-MM-DD` | Uses `init_tracing()` from zenii-core |
| Desktop | `desktop.log.YYYY-MM-DD` | Uses `init_tracing()` from zenii-core |
| CLI | `cli.log.YYYY-MM-DD` | Lightweight file-only logging (no stderr) |
| TUI | `tui.log.YYYY-MM-DD` | Lightweight file-only logging (no stderr, since TUI owns the terminal) |

Default log directory by OS:
- **Linux**: `~/.local/share/zenii/logs/`
- **macOS**: `~/Library/Application Support/com.sprklai.zenii/logs/`
- **Windows**: `C:\Users\{user}\AppData\Roaming\sprklai\zenii\logs\`

Override with `log_dir` in `config.toml`. Old log files are cleaned up automatically based on `log_keep_days` (default: 30 days).

### Log Levels

```bash
# Set via environment variable (overrides config)
RUST_LOG=debug cargo run -p zenii-daemon

# Per-module filtering
RUST_LOG=warn,zenii_core::gateway=debug cargo run -p zenii-daemon

# Trace SQL queries
RUST_LOG=zenii_core::db=trace cargo run -p zenii-daemon
```

### WebSocket Debugging

Use `websocat` to connect to the WebSocket endpoint:

```bash
# Install websocat
cargo install websocat

# Connect to chat WebSocket (replace token)
websocat "ws://localhost:18981/ws?token=your-token"

# Send a chat message
{"session_id": "test", "content": "Hello"}
```

### SQLite CLI

Inspect the database directly:

```bash
# Open the main database
sqlite3 ~/.local/share/zenii/zenii.db

# List tables
.tables

# Check schema version
SELECT * FROM migrations;

# View sessions
SELECT id, title, created_at FROM sessions ORDER BY created_at DESC LIMIT 10;

# Check WAL mode
PRAGMA journal_mode;
```

### Common Issues

| Issue | Cause | Fix |
|-------|-------|-----|
| "Failed to initialize keyring" | No keyring daemon on Linux | Install `gnome-keyring` or `kwallet`, or the daemon falls back to in-memory store |
| "Address already in use" | Port 18981 occupied | Stop the other process or change `gateway_port` in config |
| "ZENII_TOKEN not set" | CLI requires auth token | Set `gateway_auth_token` in config or `ZENII_TOKEN` env var |
| Frontend shows "Connection refused" | Daemon not running | Start daemon: `cargo run -p zenii-daemon` |
| "entity not found" on Tauri dev | Frontend not built | Run `cd web && bun install && bun run build` first |
