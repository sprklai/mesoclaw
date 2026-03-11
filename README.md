# MesoClaw

<p align="center">
  <img src="crates/mesoclaw-desktop/icons/icon.png" alt="MesoClaw" width="180" />
</p>

<h1 align="center">A lightweight, secure, local-first AI agent for desktop, CLI, and daemon.</h1>

<p align="center">
  Built with Rust + Tauri 2. Privacy-first alternative to <a href="https://github.com/openclaw/openclaw">OpenClaw</a>. &lt;20MB native binary.<br>
  <a href="https://mesoclaw.sprklai.com">https://mesoclaw.sprklai.com</a>
</p>

<!-- Row 1: CI & Release -->
<p align="center">
  <a href="https://github.com/sprklai/mesoclaw/actions/workflows/release.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/sprklai/mesoclaw/release.yml?style=flat-square&label=release" alt="Release Build" />
  </a>
  <a href="https://github.com/sprklai/mesoclaw/actions/workflows/ci.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/sprklai/mesoclaw/ci.yml?style=flat-square&label=CI" alt="CI" />
  </a>
  <a href="https://github.com/sprklai/mesoclaw/releases/latest">
    <img src="https://img.shields.io/github/v/release/sprklai/mesoclaw?style=flat-square" alt="Latest Release" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT License" />
  </a>
</p>

<!-- Row 2: Tech Stack -->
<p align="center">
  <img src="https://img.shields.io/badge/Rust-2024%20Edition-orange?style=flat-square&logo=rust" alt="Rust 2024 Edition" />
  <img src="https://img.shields.io/badge/Tauri-2-24C8D8?style=flat-square&logo=tauri&logoColor=white" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Svelte-5-FF3E00?style=flat-square&logo=svelte&logoColor=white" alt="Svelte 5" />
  <img src="https://img.shields.io/badge/TypeScript-5-3178C6?style=flat-square&logo=typescript&logoColor=white" alt="TypeScript" />
  <img src="https://img.shields.io/badge/SQLite-rusqlite%20%2B%20sqlite--vec-003B57?style=flat-square&logo=sqlite&logoColor=white" alt="SQLite + rusqlite + sqlite-vec" />
</p>

<!-- Row 3: App Modes & Platform -->
<p align="center">
  <img src="https://img.shields.io/badge/mode-Desktop%20GUI-8B5CF6?style=flat-square" alt="Desktop GUI" />
  <img src="https://img.shields.io/badge/mode-CLI-4B5563?style=flat-square" alt="CLI" />
  <img src="https://img.shields.io/badge/mode-Daemon-1F2937?style=flat-square" alt="Daemon" />
  <img src="https://img.shields.io/badge/Windows-0078D4?style=flat-square&logo=windows&logoColor=white" alt="Windows" />
  <img src="https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white" alt="macOS" />
  <img src="https://img.shields.io/badge/Linux-FCC624?style=flat-square&logo=linux&logoColor=black" alt="Linux" />
  <img src="https://img.shields.io/badge/ARM64-CLI-6C757D?style=flat-square&logo=arm&logoColor=white" alt="ARM64 CLI" />
</p>

<!-- Row 4: Quality & i18n -->
<p align="center">
  <img src="https://img.shields.io/badge/tests-1000%20Rust%20%2B%2039%20JS-success?style=flat-square" alt="1000 Rust + 39 JS Tests" />
  <img src="https://img.shields.io/badge/i18n-EN-blue?style=flat-square" alt="English" />
</p>

<!-- Row 5: Community -->
<p align="center">
  <a href="https://github.com/sprklai/mesoclaw/stargazers">
    <img src="https://img.shields.io/github/stars/sprklai/mesoclaw?style=flat-square" alt="GitHub Stars" />
  </a>
  <a href="https://github.com/sprklai/mesoclaw/issues">
    <img src="https://img.shields.io/github/issues/sprklai/mesoclaw?style=flat-square" alt="GitHub Issues" />
  </a>
  <a href="https://github.com/sprklai/mesoclaw/pulls">
    <img src="https://img.shields.io/badge/PRs-welcome-brightgreen?style=flat-square" alt="PRs Welcome" />
  </a>
</p>

---

## Why MesoClaw?

- **Native desktop app** -- Tauri 2 + Svelte 5 GUI, not a browser tab or Electron wrapper
- **<20MB binary** with full GUI -- no Node.js runtime, no JVM, just a native Rust binary
- **Plugin system in ANY language** -- JSON-RPC protocol, write plugins in Python, Go, JS, or anything that speaks stdio
- **Self-evolving skills** -- agent learns your preferences and proposes skill changes with human-in-the-loop approval
- **Semantic memory** -- SQLite FTS5 + vector embeddings for intelligent recall across conversations
- **Scheduled autonomy** -- cron-driven agent tasks that run unattended
- **Autonomous reasoning** -- multi-step agent loops with configurable continuation strategies
- **Privacy-first** -- zero telemetry, all data local, OS keyring for credentials

---

## How It Compares

| Category | MesoClaw | OpenClaw | ZeroClaw |
|----------|----------|----------|----------|
| **Language** | Rust | TypeScript | Rust |
| **Desktop GUI** | Tauri 2 + Svelte 5 | -- | -- |
| **CLI** | clap | -- | -- |
| **Headless Daemon** | axum (90 routes) | Node.js | 3.4MB daemon |
| **AI Providers** | 18 via rig-core | Multi-model | 22+ |
| **Built-in Tools** | 15 | 100+ AgentSkills | Tool orchestration |
| **Plugin System** | JSON-RPC (any language) | AgentSkills | Trait-based adapters |
| **Memory** | SQLite FTS5 + vectors | File-based | Built-in |
| **Embeddings** | OpenAI + local FastEmbed | -- | -- |
| **Self-Evolution** | LearnTool + SkillProposals | -- | -- |
| **Autonomous Reasoning** | ReasoningEngine | -- | -- |
| **Scheduler** | Cron + one-shot | -- | -- |
| **Channels** | Telegram, Slack, Discord | WhatsApp, Signal, Telegram, Discord | Telegram, Discord, WhatsApp |
| **Binary Size** | <20MB (native w/ GUI) | Node.js runtime | 3.4MB |
| **Privacy** | 100% local, zero telemetry | Local, model-agnostic | 100% local |
| **License** | MIT | Open source | Open source |
| **Tests** | 1000 Rust + 39 JS | -- | -- |

---

## Features

- **18 AI providers** via rig-core (OpenAI, Anthropic, Google, Ollama, and more)
- **Tool calling** with 15 built-in tools (13 base + 2 feature-gated) via DashMap-backed ToolRegistry: websearch, sysinfo, shell, file read/write/list/search, patch, process, learn, skill_proposal, memory, config + feature-gated channel_send, scheduler
- **Plugin system** -- external process plugins via JSON-RPC 2.0 protocol, installable from git or local paths, with automatic tool and skill registration
- **Autonomous reasoning** -- ReasoningEngine with ContinuationStrategy for multi-step autonomous agent loops
- **Context-aware agent** -- 3-tier adaptive context injection (Full/Minimal/Summary) with hash-based cache invalidation
- **Self-evolving framework** -- agent learns user preferences and proposes skill changes with human-in-the-loop approval
- **Streaming responses** via WebSocket
- **Semantic memory** with SQLite FTS5 + vector embeddings (sqlite-vec), OpenAI and local FastEmbed embedding providers
- **Soul / Persona system** -- 3 identity files (SOUL/IDENTITY/USER.md) with dynamic prompt composition
- **Skills system** -- bundled + user markdown skills loaded into agent context (Claude Code model)
- **Progressive user learning** -- SQLite-backed observations with category filtering, confidence scoring, and privacy controls
- **Secure credentials** via OS keyring with zeroize memory protection
- **Messaging channels** -- Telegram, Slack, Discord with lifecycle hooks (typing indicators, status messages) and end-to-end channel router pipeline (feature-gated, trait-based with DashMap registry)
- **Cron scheduler** -- automated recurring tasks with real payload execution (Notify, AgentTurn, Heartbeat, SendViaChannel)
- **Notifications** -- desktop OS notifications (tauri-plugin-notification) + web toast notifications (svelte-sonner) via WebSocket push
- **Cross-platform** -- Linux, macOS, Windows, ARM (Raspberry Pi)

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust 2024 edition |
| Async | Tokio |
| AI | rig-core |
| Database | rusqlite + sqlite-vec |
| Gateway | axum (HTTP + WebSocket) |
| Frontend | Svelte 5 + SvelteKit + shadcn-svelte + Tailwind CSS |
| Desktop | Tauri 2 |
| CLI | clap |
| Plugins | JSON-RPC 2.0 external processes |
| Channels | Telegram (teloxide), Slack, Discord (serenity) -- feature-gated |
| Content | serde_yaml (YAML frontmatter parsing) |
| i18n | paraglide-js (compile-time, tree-shakeable) |
| Mobile | Tauri 2 (iOS + Android) -- future release |
| TUI | ratatui -- future release |

---

## Architecture

### System Architecture

```mermaid
graph TD
    subgraph Clients["Clients"]
        Desktop[Desktop] & Mobile["Mobile<br>#40;future#41;"] & CLI[CLI] & TUI["TUI<br>#40;future#41;"] & Daemon[Daemon]
        Web["Frontend<br>Svelte 5"]
    end

    subgraph Core["mesoclaw-core"]
        BootEntry["boot.rs<br>init_services"]

        subgraph App["Application Layer"]
            Gateway["Gateway<br>axum :18981"]
            AI["AI Engine<br>rig-core"]
            Context["Context Engine<br>3-tier injection"]
            DB["Database<br>rusqlite + sqlite-vec"]
        end

        subgraph Domain["Domain Layer"]
            Identity["Identity<br>SoulLoader"]
            Skills["Skills<br>SkillRegistry"]
            UserL["User Profile<br>UserLearner"]
            Channels["Channels"]
            PluginReg["Plugins<br>PluginRegistry"]
        end

        subgraph Support["Support Layer"]
            Tools["Agent Tools"]
            Security["Security"]
            Creds2["Credentials"]
            Config["Config"]
            EventBus["EventBus"]
        end
    end

    Desktop -->|embedded gateway| Gateway
    Mobile & CLI & TUI & Daemon --> Gateway
    Web -->|HTTP/WS| Gateway

    BootEntry --> Gateway & DB & EventBus
    Gateway --> AI & DB & Context
    Gateway --> Identity & Skills & UserL & Channels & PluginReg
    AI --> Tools & Security & DB
    AI --> Identity & Skills
    Context --> DB & Identity & UserL & Skills

    style Clients fill:#2196F3,color:#fff
    style App fill:#4CAF50,color:#fff
    style Domain fill:#FF9800,color:#fff
    style Support fill:#9E9E9E,color:#fff
```

### Crate Dependency Graph

```mermaid
graph TD
    desktop[mesoclaw-desktop] --> core[mesoclaw-core]
    mobile["mesoclaw-mobile<br>#40;future#41;"] -.-> core
    cli[mesoclaw-cli]
    cli --> reqwest["reqwest<br>#40;HTTP client#41;"]
    cli --> tungstenite["tokio-tungstenite<br>#40;WS#41;"]
    tui["mesoclaw-tui<br>#40;future#41;"] -.-> core
    daemon[mesoclaw-daemon] --> core

    core --> axum["axum<br>#40;gateway#41;"]
    core --> rusqlite["rusqlite<br>#40;database#41;"]
    core --> rigcore["rig-core<br>#40;AI#41;"]
    core --> tokio["tokio<br>#40;async#41;"]
    core --> keyring["keyring<br>#40;credentials#41;"]
    core --> serdeyaml["serde_yaml<br>#40;YAML frontmatter#41;"]
    core -.-> teloxide["teloxide<br>#40;Telegram, feature-gated#41;"]
    core -.-> serenity["serenity<br>#40;Discord, feature-gated#41;"]
```

### Chat Request Flow

```mermaid
sequenceDiagram
    participant U as User
    participant G as Gateway (axum)
    participant AI as AI Engine (rig-core)
    participant M as Memory (sqlite-vec)
    participant LLM as LLM Provider
    participant T as Tools

    U->>G: Send message (REST/WS)
    G->>M: Query relevant context
    M-->>G: Context results
    G->>AI: Dispatch with context + tools
    AI->>LLM: Stream prompt

    loop Tool calling loop
        LLM-->>AI: Response (may include tool calls)
        alt Tool call detected
            AI->>T: Execute tool
            T-->>AI: Tool result
            AI->>LLM: Feed result back
        end
    end

    LLM-->>AI: Final response
    AI-->>G: Stream tokens
    G-->>U: Stream via WS
    G->>M: Store conversation
```

### Startup Sequence

```mermaid
sequenceDiagram
    participant App as Application
    participant Cfg as Config
    participant DB as SQLite
    participant Cred as Keyring
    participant AI as AI Providers
    participant Plug as Plugins
    participant GW as Gateway

    App->>Cfg: Parse CLI args + load TOML
    App->>App: Initialize tracing
    App->>DB: Open/create database + migrations
    App->>Cred: Initialize credential store
    App->>AI: Register providers + load API keys
    App->>AI: Register agent tools
    App->>Plug: Scan plugins directory + register tools/skills
    App->>GW: Start axum server (:18981)

    alt Desktop
        App->>App: Open Tauri window
    else CLI
        App->>App: Enter REPL loop
    else TUI (future release)
        App->>App: Render ratatui UI
    else Daemon
        App->>App: Wait for connections
    end
```

### WebSocket Message Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server

    C->>S: WS Connect /ws/chat
    C->>S: { type: "chat", content: "hello" }
    Note over S: Query memory + prompt + LLM
    S-->>C: { type: "token", content: "Hi" }
    S-->>C: { type: "token", content: " there" }
    S-->>C: { type: "tool_call", name: "websearch" }
    S-->>C: { type: "tool_result", result: "..." }
    S-->>C: { type: "done" }
```

### Feature Flag Composition

```mermaid
graph TD
    Daemon[mesoclaw-daemon] --> Default[default]
    Daemon --> Ch["--features channels"]
    Daemon --> Sc["--features scheduler"]
    Daemon --> Wd["--features web-dashboard"]

    Default --> GW["mesoclaw-core/gateway"]
    GW --> Axum[axum + tower-http]
    Ch --> ChCore[mesoclaw-core/channels]
    Sc --> ScCore[mesoclaw-core/scheduler]
    Wd --> WdCore[mesoclaw-core/web-dashboard]
    WdCore --> GW
```

---

## Project Structure

```
mesoclaw/
├── Cargo.toml              # Workspace root (5 members)
├── CLAUDE.md               # AI assistant instructions
├── README.md               # This file
├── scripts/
│   └── build.sh            # Cross-platform build script
├── docs/
│   ├── architecture.md     # Detailed architecture diagrams
│   ├── phases.md           # Implementation phase details
│   ├── processes.md        # Process flow diagrams
│   ├── api-reference.md    # All 90 REST/WS routes
│   ├── configuration.md    # All 60+ config fields
│   ├── cli-reference.md    # CLI command reference
│   ├── deployment.md       # Deployment guide
│   └── development.md      # Development guide
├── plans/
│   ├── phase1_core_foundation.md  # Phase 1 implementation plan
│   ├── phase2_ai_integration.md   # Phase 2 implementation plan
│   ├── phase3_gateway_server.md   # Phase 3 implementation plan
│   ├── phase4_agent_intelligence.md # Phase 4 implementation plan
│   ├── phase5_combined.md         # Phase 5 implementation plan
│   ├── phase6_frontend.md         # Phase 6 implementation plan
│   └── phase9_plugin_architecture.md # Phase 9 plugin plan
├── tests/
│   ├── phase1_core_foundation.md  # Phase 1 test plan + results
│   ├── phase2_ai_integration.md   # Phase 2 test plan + results
│   ├── phase3_gateway_server.md   # Phase 3 test plan + results
│   ├── phase4_agent_intelligence.md # Phase 4 test plan + results
│   ├── phase5_combined.md         # Phase 5 test plan + results
│   ├── phase6_frontend.md         # Phase 6 test plan + results
│   └── phase9_plugin_architecture.md # Phase 9 test plan
├── crates/
│   ├── mesoclaw-core/      # Shared library (NO Tauri dependency)
│   ├── mesoclaw-desktop/   # Tauri 2.10 shell (macOS, Windows, Linux)
│   ├── mesoclaw-mobile/    # Tauri 2 shell (iOS, Android) (future release)
│   ├── mesoclaw-cli/       # clap CLI
│   ├── mesoclaw-tui/       # ratatui TUI (future release)
│   └── mesoclaw-daemon/    # Headless daemon
└── web/                    # Svelte 5 SPA frontend (shared by desktop + mobile)
```

---

## Getting Started

### Prerequisites

- **Rust** 1.85+ (2024 edition support)
- **Bun** (for frontend development)
- **SQLite3** development libraries

#### Platform-specific

**Linux (Debian/Ubuntu):**
```bash
sudo apt install libsqlite3-dev libwebkit2gtk-4.1-dev libappindicator3-dev \
  librsvg2-dev patchelf libssl-dev
```

**macOS:**
```bash
brew install sqlite3
```

**Windows:**
```powershell
# SQLite is bundled via rusqlite's "bundled" feature -- no extra install needed
```

### Build & Run

```bash
# Check everything compiles
cargo check --workspace

# Run tests
cargo test --workspace

# Lint
cargo clippy --workspace

# Start the daemon
cargo run -p mesoclaw-daemon

# Start the CLI
cargo run -p mesoclaw-cli -- chat

# Start the TUI
cargo run -p mesoclaw-tui

# Start the desktop app (dev mode with hot reload)
cd crates/mesoclaw-desktop && cargo tauri dev

# Start the desktop app connecting to external daemon
MESOCLAW_GATEWAY_URL=http://localhost:18981 cd crates/mesoclaw-desktop && cargo tauri dev

# Frontend dev server (hot reload)
cd web && bun run dev
```

### Building Executables

#### Native builds (current platform)

```bash
./scripts/build.sh --target native                  # Debug build
./scripts/build.sh --target native --release         # Release (optimized, smallest binary)
./scripts/build.sh --target native --release --crates "mesoclaw-daemon mesoclaw-cli"  # Specific crates only
./scripts/build.sh --target native --release --all-features  # With all features
```

Output goes to `dist/native/release/`.

#### Tauri desktop app (with GUI)

```bash
./scripts/build.sh --tauri --release                 # Release bundle (.deb/.AppImage, .dmg, .msi)
./scripts/build.sh --tauri --release --bundle deb,appimage  # Specific bundle formats
./scripts/build.sh --dev                             # Dev mode (Vite + Tauri hot reload)
```

#### Cross-compilation

```bash
./scripts/build.sh --list-targets                    # Show all available targets

# Linux targets
./scripts/build.sh --target linux-x86 --release --install-toolchain
./scripts/build.sh --target linux-arm64 --release --install-toolchain
./scripts/build.sh --target linux-armv7 --release --install-toolchain   # Raspberry Pi
./scripts/build.sh --target linux-musl --release --install-toolchain    # Static binary

# macOS (must run on macOS)
./scripts/build.sh --target macos-x86 --release      # Intel
./scripts/build.sh --target macos-arm --release       # Apple Silicon
./scripts/build.sh --target macos-universal --release  # Universal (x86_64 + ARM via lipo)

# Windows (from Linux)
./scripts/build.sh --target windows --release --install-toolchain

# All targets at once
./scripts/build.sh --target all --release --install-toolchain
```

**Cross-compilation prerequisites (Linux):**

```bash
sudo apt install gcc-aarch64-linux-gnu      # ARM64
sudo apt install gcc-arm-linux-gnueabihf    # ARMv7
sudo apt install gcc-mingw-w64-x86-64       # Windows
```

#### Docker-based cross-compilation (no local cross-compilers needed)

```bash
./scripts/build.sh --target linux-arm64 --release --docker
./scripts/build.sh --target windows --release --docker
```

#### Build profiles

| Profile | Flag | Use Case |
|---------|------|----------|
| `debug` | *(default)* | Development |
| `release` | `--release` | Production (full LTO, smallest binary) |
| `ci-release` | `--profile ci-release` | CI builds (thin LTO, faster compile) |
| `release-fast` | `--profile release-fast` | Profiling (thin LTO + debug info) |

> **Note:** Tauri desktop builds cannot cross-compile -- each platform must build on its native OS. Use the [GitHub Actions CI workflow](.github/workflows/ci.yml) for automated multi-platform Tauri builds.

See [scripts/build.sh](scripts/build.sh) for full options.

---

## Feature Flags

```bash
cargo build -p mesoclaw-daemon                          # Core only (gateway + ai + keyring)
cargo build -p mesoclaw-daemon --features local-embeddings  # + local FastEmbed ONNX embeddings
cargo build -p mesoclaw-daemon --features channels      # + channel core traits + registry
cargo build -p mesoclaw-daemon --features channels-telegram  # + Telegram (teloxide)
cargo build -p mesoclaw-daemon --features channels-slack     # + Slack
cargo build -p mesoclaw-daemon --features channels-discord   # + Discord (serenity)
cargo build -p mesoclaw-daemon --features scheduler     # + cron jobs
cargo build -p mesoclaw-daemon --features api-docs      # + Scalar UI + OpenAPI spec at /api-docs
cargo build -p mesoclaw-daemon --features web-dashboard # + embedded web UI
cargo build -p mesoclaw-daemon --all-features           # Everything
```

---

## Testing

```bash
cargo test --workspace                    # All tests
cargo test -p mesoclaw-core               # Core only
cargo test -p mesoclaw-core -- memory     # Memory module
cargo test -p mesoclaw-core -- db         # Database module
cd web && bun run test                    # Frontend tests
```

---

## Configuration

MesoClaw uses a TOML configuration file. Paths are resolved via `directories::ProjectDirs::from("com", "sprklai", "mesoclaw")`:

| OS | Config File | Database File |
|---|---|---|
| **Linux** | `~/.config/mesoclaw/config.toml` | `~/.local/share/mesoclaw/mesoclaw.db` |
| **macOS** | `~/Library/Application Support/com.sprklai.mesoclaw/config.toml` | `~/Library/Application Support/com.sprklai.mesoclaw/mesoclaw.db` |
| **Windows** | `%APPDATA%\sprklai\mesoclaw\config\config.toml` | `%APPDATA%\sprklai\mesoclaw\data\mesoclaw.db` |

Example `config.toml` (flat structure, all fields optional with defaults):

```toml
gateway_host = "127.0.0.1"
gateway_port = 18981
log_level = "info"
# data_dir = "/custom/data/path"       # Override default data directory
# db_path = "/custom/path/mesoclaw.db" # Override database file path
identity_name = "MesoClaw"
identity_description = "AI-powered assistant"
default_provider = "openai"
default_model = "gpt-4o"
security_autonomy_level = "supervised"  # supervised | autonomous | strict
max_tool_retries = 3
# gateway_auth_token = "your-secret-token"  # Optional bearer token for auth
# agent_max_turns = 20                       # Max tool-calling turns per request
# agent_max_continuations = 5               # Max autonomous reasoning turns
# embedding_provider = "none"               # none | openai | local
# plugins_dir = "/custom/plugins/path"      # Override default plugins directory
# plugin_auto_update = false                # Auto-update git-sourced plugins
```

## CLI Commands

```bash
mesoclaw daemon start|stop|status     # Manage the daemon process
mesoclaw chat [--session ID] [--model M]  # Interactive WS streaming chat
mesoclaw run "prompt" [--session] [--model]  # Single prompt, print response
mesoclaw memory search "query" [--limit N] [--offset N]  # Search memories
mesoclaw memory add <key> <content>   # Add memory entry
mesoclaw memory remove <key>          # Remove memory entry
mesoclaw config show                  # Show current config
mesoclaw config set <key> <value>     # Set a config value
mesoclaw key set <provider> <key>     # Set API key
mesoclaw key remove <provider>        # Remove API key
mesoclaw key list                     # List stored keys
mesoclaw provider list                # List AI providers
mesoclaw provider test <id>           # Test provider connection
mesoclaw provider add <id> <name> <base_url>  # Add custom provider
mesoclaw provider remove <id>         # Remove user-defined provider
mesoclaw provider default <provider> <model>  # Set default model
mesoclaw embedding activate <provider>       # Activate embeddings (openai/local)
mesoclaw embedding deactivate                # Deactivate embeddings
mesoclaw embedding status                    # Show embedding provider status
mesoclaw embedding test                      # Test embedding generation
mesoclaw embedding reindex                   # Re-embed all memories
mesoclaw plugin list                         # List installed plugins
mesoclaw plugin install <source> [--local]   # Install from git URL or local path
mesoclaw plugin remove <name>                # Remove a plugin
mesoclaw plugin update <name>                # Update a plugin
mesoclaw plugin enable <name>                # Enable a plugin
mesoclaw plugin disable <name>               # Disable a plugin
mesoclaw plugin info <name>                  # Show plugin details
```

Global options: `--host`, `--port`, `--token` (or `MESOCLAW_TOKEN` env var)

## Gateway Routes (73 base + 17 feature-gated = 90 total)

| Group | Routes | Description |
|-------|--------|-------------|
| Health | `GET /health` | Health check (no auth) |
| Sessions & Chat | `POST /sessions`, `GET /sessions`, `GET/PUT/DELETE /sessions/{id}`, `POST /sessions/{id}/generate-title`, `GET/POST /sessions/{id}/messages`, `POST /chat` | Chat sessions and messaging |
| Memory | `POST /memory`, `GET /memory`, `GET/PUT/DELETE /memory/{key}` | Semantic memory CRUD |
| Config | `GET /config`, `PUT /config` | Configuration management |
| Credentials | `POST/GET /credentials`, `DELETE /credentials/{key}`, `GET /credentials/{key}/value`, `GET /credentials/{key}/exists` | Credential management (keyring) |
| Providers & Models | `GET/POST /providers`, `GET /providers/with-key-status`, `GET/PUT /providers/default`, `GET/PUT/DELETE /providers/{id}`, `POST /providers/{id}/test`, `POST /providers/{id}/models`, `DELETE /providers/{id}/models/{model_id}`, `GET /models` | Multi-provider AI management |
| Tools | `GET /tools`, `POST /tools/{name}/execute` | Tool listing and execution |
| System | `GET /system/info` | System information |
| Identity | `GET /identity`, `GET/PUT /identity/{name}`, `POST /identity/reload` | Persona management |
| Skills | `GET /skills`, `GET/PUT/DELETE /skills/{id}`, `POST /skills`, `POST /skills/reload` | Skill CRUD |
| Skill Proposals | `GET /skills/proposals`, `POST /skills/proposals/{id}/approve`, `POST /skills/proposals/{id}/reject`, `DELETE /skills/proposals/{id}` | Self-evolving skill management |
| User | `GET/POST/DELETE /user/observations`, `GET/DELETE /user/observations/{key}`, `GET /user/profile` | User learning + privacy |
| Embeddings | `GET /embeddings/status`, `POST /embeddings/test`, `POST /embeddings/embed`, `POST /embeddings/download`, `POST /embeddings/reindex` | Semantic memory embedding management |
| Plugins | `GET /plugins`, `POST /plugins/install`, `GET/DELETE /plugins/{name}`, `PUT /plugins/{name}/toggle`, `POST /plugins/{name}/update`, `GET/PUT /plugins/{name}/config` | Plugin management (install, remove, enable/disable, config) |
| Channels | `POST /channels/{name}/test` (always), `GET /channels`, `GET /channels/{name}/status`, `POST /channels/{name}/send`, `POST /channels/{name}/connect/disconnect`, `GET /channels/{name}/health`, `POST /channels/{name}/message`, `GET /channels/sessions`, `GET /channels/sessions/{id}/messages` (feature-gated) | Messaging channels |
| Scheduler | `GET/POST /scheduler/jobs`, `PUT /scheduler/jobs/{id}/toggle`, `DELETE /scheduler/jobs/{id}`, `GET /scheduler/jobs/{id}/history`, `GET /scheduler/status` (feature-gated) | Cron job management |
| WebSocket | `GET /ws/chat`, `GET /ws/notifications` | Streaming chat + notification push |
| API Docs | `GET /api-docs`, `GET /api-docs/openapi.json` | Interactive Scalar UI + OpenAPI 3.1 spec (feature-gated: `api-docs`) |

---

## Documentation

Detailed documentation lives in the `docs/` directory:

- [CLI Reference](docs/cli-reference.md) -- All commands, options, shell completions, recipes
- [API Reference](docs/api-reference.md) -- All 90 REST/WS routes with request/response schemas
- [Configuration](docs/configuration.md) -- All 60+ config.toml fields with types and defaults
- [Deployment Guide](docs/deployment.md) -- Native, Docker, systemd, Raspberry Pi, reverse proxy
- [Development Guide](docs/development.md) -- Prerequisites, building, testing, how-to guides
- [Architecture](docs/architecture.md) -- System diagrams, crate dependencies, project structure
- [Implementation Phases](docs/phases.md) -- Phase gate protocol, checklist, phase details
- [Process Flows](docs/processes.md) -- Chat request, startup, error handling, WebSocket flows
- [Changelog](CHANGELOG.md) -- Release history

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines. Quick summary:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Write tests first, then implement
4. Ensure `cargo test --workspace` and `cargo clippy --workspace -- -D warnings` pass
5. Submit a pull request

---

## License

MIT
