# Zenii

<p align="center">
  <img src="crates/zenii-desktop/icons/icon.png" alt="Zenii" width="180" />
</p>

<h1 align="center">20 megabytes. AI everywhere.</h1>

<p align="center">
  Install one binary. Now your scripts have <strong>AI memory</strong>. Your cron jobs <strong>reason</strong>. Your Telegram bot <strong>thinks</strong>.<br>
  A private AI backend for everything on your machine — native desktop app, plugins in <strong>any language</strong>, and an API your <code>curl</code> can call. Powered by Rust.<br>
  <a href="https://zenii.sprklai.com">https://zenii.sprklai.com</a>
</p>

<!-- Row 1: CI & Release -->
<p align="center">
  <a href="https://github.com/sprklai/zenii/actions/workflows/release.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/sprklai/zenii/release.yml?style=flat-square&label=release" alt="Release Build" />
  </a>
  <a href="https://github.com/sprklai/zenii/actions/workflows/ci.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/sprklai/zenii/ci.yml?style=flat-square&label=CI" alt="CI" />
  </a>
  <a href="https://github.com/sprklai/zenii/releases/latest">
    <img src="https://img.shields.io/github/v/release/sprklai/zenii?style=flat-square" alt="Latest Release" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT License" />
  </a>
  <a href="https://docs.zenii.sprklai.com">
    <img src="https://img.shields.io/badge/docs-zenii-blue?style=flat-square" alt="Documentation" />
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
  <img src="https://img.shields.io/badge/tests-1206%20Rust%20%2B%20JS-success?style=flat-square" alt="1206 Rust + JS Tests" />
  <img src="https://img.shields.io/badge/i18n-EN-blue?style=flat-square" alt="English" />
</p>

<!-- Row 5: Community -->
<p align="center">
  <a href="https://github.com/sprklai/zenii/stargazers">
    <img src="https://img.shields.io/github/stars/sprklai/zenii?style=flat-square" alt="GitHub Stars" />
  </a>
  <a href="https://github.com/sprklai/zenii/issues">
    <img src="https://img.shields.io/github/issues/sprklai/zenii?style=flat-square" alt="GitHub Issues" />
  </a>
  <a href="https://github.com/sprklai/zenii/pulls">
    <img src="https://img.shields.io/badge/PRs-welcome-brightgreen?style=flat-square" alt="PRs Welcome" />
  </a>
</p>

---

> *"ChatGPT is a tab you open. Zenii is a capability your machine gains."*

<!-- DEMO GIF HERE — 30-second terminal recording showing:
  1. curl health check
  2. Create session
  3. Send chat message with tool call
  4. Response streams back
  See go2market/Mar17_Launch/04_demo_scripts.md for exact script
-->
<p align="center">
  <img src="docs/assets/demo.gif" alt="Zenii terminal demo" width="720" />
</p>

## Quick Start

```bash
# Download (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | bash

# Start the daemon
zenii-daemon &

# Your first AI request
curl -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"session_id": "hello", "prompt": "What can you do?"}'
```

Or use the desktop app, CLI, or TUI — they all talk to the same backend.

---

## Why Zenii?

| Your pain | How Zenii fixes it |
|-----------|-------------------|
| Context resets every AI session | Semantic memory persists across sessions and survives restarts |
| AI can't do things, only talk | 16 built-in tools: web search, file ops, shell, scheduling |
| Locked into one AI provider | 18 providers, switch with one config change |
| AI tools are cloud-only | 100% local, zero telemetry, OS keyring for secrets |
| "Works on my machine" for AI | Same binary on macOS, Linux, Windows — desktop, CLI, or daemon |
| Plugin systems require learning a framework | JSON-RPC over stdio — write plugins in Python, Go, JS, or anything |
| AI doesn't learn your patterns | Self-evolving skills with human-in-the-loop approval |
| AI can't run tasks while you sleep | Built-in cron scheduler for autonomous recurring tasks |

## What Zenii is NOT

- Not a chatbot wrapper — it's a full API backend with 96 routes
- Not Electron — native Tauri 2, under 20 MB
- Not a framework you learn — it's infrastructure you call via `curl`
- Not cloud-dependent — runs fully offline with Ollama
- Not opinionated about your stack — any language, any tool, JSON over HTTP

---

## What Can I Automate?

```bash
# Schedule a daily morning briefing
curl -X POST http://localhost:18981/scheduler/jobs \
  -H "Content-Type: application/json" \
  -d '{"name":"briefing","schedule":{"Cron":{"expr":"0 9 * * *"}},"payload":{"AgentTurn":{"prompt":"Summarize system status and news"}}}'

# Store knowledge the AI should remember
curl -X POST http://localhost:18981/memory \
  -H "Content-Type: application/json" \
  -d '{"key":"deploy", "content":"Production DB is on port 5433, deploy via ssh prod"}'

# Ask a question that uses stored memory
curl -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"session_id":"ops", "prompt":"How do I deploy to production?"}'

# List what tools the agent has
curl http://localhost:18981/tools | jq '.[].name'

# Send a message via Telegram
curl -X POST http://localhost:18981/channels/telegram/send \
  -H "Content-Type: application/json" \
  -d '{"content":"Deploy complete", "recipient":"123456"}'
```

---

## How It Compares

| | **Zenii** | OpenClaw | ZeroClaw | PicoClaw | Open Interpreter | Khoj | Gemini CLI |
|---|---|---|---|---|---|---|---|
| **Category** | **AI backend** | Chat agent | Minimal daemon | Edge AI assistant | Code REPL | Document brain | Terminal AI |
| **Stars** | New | 210k+ | 20k+ | 24.8k | 62.6k | 32.8k | 97.8k |
| **Language** | **Rust** | TypeScript | Rust | Go | Python | Python/TS | TypeScript |
| **Binary** | **<20 MB (w/ GUI)** | ~100 MB+ | ~3.4 MB | <10 MB RAM | N/A (Python) | N/A (Docker) | N/A (npm) |
| **Desktop GUI** | **Native (Tauri 2)** | -- | -- | Web console | -- | Browser | -- |
| **API Routes** | **96 REST+WS** | Chat endpoint | Daemon endpoint | Webhook gateway | -- | -- | -- |
| **Plugins** | **Any language** | JS only | Rust only | Tool-based | -- | -- | -- |
| **Memory** | **FTS5 + vectors** | File-based | Basic | Workspace logs | -- | Doc search | -- |
| **Self-Evolution** | **Human-approved** | Autonomous | -- | Agent-generated | -- | -- | -- |
| **Scheduling** | **Cron + one-shot** | Cron | -- | Built-in | -- | Automations | -- |
| **Offline** | **Ollama** | Ollama | Ollama | DuckDuckGo | LiteLLM | Optional | No |
| **License** | **MIT** | Open source | Open source | MIT | AGPL-3.0 | AGPL-3.0 | Apache 2.0 |

**No other project has ALL of these simultaneously**: native desktop GUI, 96-route REST/WS API, plugins in any language, semantic vector memory, self-evolution with human approval, under 20 MB binary, and MIT licensed.

---

## The Self-Evolution Story

Most AI tools are static — they do exactly what they did on day one. OpenClaw self-modifies without asking. Zenii takes a third path:

1. **Zenii observes** your patterns and preferences over time
2. **Zenii proposes** skill modifications ("I notice you always want code reviews on Fridays. Want me to schedule that?")
3. **You approve or reject** — like a PR from your AI
4. **Zenii learns** — approved changes become permanent skills

Your AI gets smarter. You stay in control. No surprises.

---

## Features

- **Self-evolving agent** — proposes skill changes based on your patterns, learns only with your approval
- **Plugin system** — write plugins in Python, Go, JS, or any language. A plugin is any program that speaks JSON-RPC 2.0 over stdio (~15 lines of Python)
- **96 API routes** — full REST + WebSocket gateway. Interactive docs at `localhost:18981/api-docs`
- **18 AI providers** via rig-core (OpenAI, Anthropic, Google, Ollama, and more)
- **16 built-in tools** — websearch, sysinfo, shell, file ops, memory, config, scheduling, channels
- **Semantic memory** — SQLite FTS5 + vector embeddings, persists across sessions and restarts
- **Native desktop app** — Tauri 2 + Svelte 5, under 20 MB, not Electron
- **Compact prompts** — plugin-based prompt strategy with ~65% token reduction
- **Token usage tracking** — date-rotated JSONL logs for cost visibility
- **Messaging channels** — Telegram, Slack, Discord (feature-gated)
- **Cron scheduler** — automated recurring AI tasks
- **6-layer security** — OS keyring, autonomy levels, FS sandbox, injection detection, rate limits, audit trail
- **Cross-platform** — Linux, macOS, Windows, ARM (Raspberry Pi)

<!-- Detailed feature descriptions below for SEO / deep readers -->

<details>
<summary><strong>Full feature details</strong> (click to expand)</summary>

- **Tool calling** with 16 built-in tools (14 base + 2 feature-gated) via DashMap-backed ToolRegistry: websearch, sysinfo, shell, file read/write/list/search, patch, process, learn, skill_proposal, memory, config, agent_self + feature-gated channel_send, scheduler
- **Plugin system** -- external process plugins via JSON-RPC 2.0 protocol, installable from git or local paths, with automatic tool and skill registration. Managed via CLI, Web/Desktop UI, and TUI. See [zenii-plugins](https://github.com/sprklai/zenii-plugins) for official community plugins
- **Autonomous reasoning** -- ReasoningEngine with tool-aware ContinuationStrategy and per-request tool call deduplication cache
- **Context-driven auto-discovery** -- keyword-based domain detection (Channels/Scheduler/Skills/Tools) filters context injection and agent rules to only relevant domains per query
- **Self-evolving agent** -- AgentSelfTool (`agent_notes`) for agent-writable behavioral rules by category, stored in DB and auto-injected into context; SkillProposalTool for human-in-the-loop skill evolution
- **Model capability validation** -- `supports_tools` pre-check prevents tool-calling errors with incompatible models
- **Context-aware agent** -- 3-tier adaptive context injection (Full/Minimal/Summary) with hash-based cache invalidation
- **Efficient prompt system** -- plugin-based prompt strategy with CompactStrategy (~65% token reduction), 6 built-in plugins, and token budget trimming
- **Onboarding wizard** -- multi-step first-run setup across Desktop (2-step wizard), CLI (`zenii setup` interactive flow), and TUI (4-step overlay modal) collecting AI provider selection, API key, default model, and user profile (name, location, timezone)
- **LLM-based auto fact extraction** -- automatically extracts structured facts (preferences, knowledge, context, workflow) from conversations via a configurable LLM, persisted to user observations for progressive learning
- **User location awareness** -- timezone and location injected into agent context for location-sensitive queries (weather, events, news)
- **OpenAPI interactive docs** -- Scalar UI at `/api-docs` + OpenAPI 3.1 JSON spec (feature-gated `api-docs`, built with utoipa)
- **Streaming responses** via WebSocket
- **Semantic memory** with SQLite FTS5 + vector embeddings (sqlite-vec), OpenAI and local FastEmbed embedding providers
- **Soul / Persona system** -- 3 identity files (SOUL/IDENTITY/USER.md) with dynamic prompt composition
- **Skills system** -- bundled + user markdown skills loaded into agent context (Claude Code model)
- **Progressive user learning** -- SQLite-backed observations with category filtering, confidence scoring, and privacy controls
- **Tool permission system** -- per-surface, risk-based tool permissions with 3 risk levels (Low/Medium/High), surface-specific overrides, and settings UI
- **Secure credentials** via OS keyring with zeroize memory protection
- **Messaging channels** -- Telegram, Slack, Discord with lifecycle hooks (typing indicators, status messages) and end-to-end channel router pipeline (feature-gated, trait-based with DashMap registry)
- **Cron scheduler** -- automated recurring tasks with real payload execution (Notify, AgentTurn, Heartbeat, SendViaChannel)
- **Notifications** -- desktop OS notifications (tauri-plugin-notification) + web toast notifications (svelte-sonner) via WebSocket push
- **Cross-platform** -- Linux, macOS, Windows, ARM (Raspberry Pi)

</details>

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
| TUI | ratatui |

---

<details>
<summary><strong>Architecture</strong> (click to expand)</summary>

### System Architecture

<p align="center">
  <img src="docs/assets/system-architecture.png" alt="Zenii System Architecture" width="720" />
</p>

### 6 Layers of Defense

<p align="center">
  <img src="docs/assets/6-layers-of-defense.png" alt="Zenii 6 Layers of Defense" width="720" />
</p>

### Crate Dependency Graph

```mermaid
graph TD
    desktop[zenii-desktop] --> core[zenii-core]
    mobile["zenii-mobile<br>#40;future#41;"] -.-> core
    cli[zenii-cli]
    cli --> reqwest["reqwest<br>#40;HTTP client#41;"]
    cli --> tungstenite["tokio-tungstenite<br>#40;WS#41;"]
    tui[zenii-tui] --> core
    daemon[zenii-daemon] --> core

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
    participant Ctx as Context Engine
    participant Plug as Plugins
    participant GW as Gateway

    App->>Cfg: Parse CLI args + load TOML
    App->>App: Initialize tracing
    App->>DB: Open/create database + migrations
    App->>Cred: Initialize credential store
    App->>AI: Register providers + load API keys
    App->>AI: Register 14 base + 2 feature-gated agent tools
    App->>Ctx: Init ContextEngine + BootContext (OS, location, timezone)
    App->>Plug: Scan plugins directory + register tools/skills
    App->>GW: Start axum server (:18981)

    alt Desktop
        App->>App: Open Tauri window
    else CLI
        App->>App: Enter REPL loop
    else TUI
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
    Daemon[zenii-daemon] --> Default[default]
    Daemon --> Ch["--features channels"]
    Daemon --> Sc["--features scheduler"]
    Daemon --> Wd["--features web-dashboard"]

    Default --> GW["zenii-core/gateway"]
    GW --> Axum[axum + tower-http]
    Ch --> ChCore[zenii-core/channels]
    Sc --> ScCore[zenii-core/scheduler]
    Wd --> WdCore[zenii-core/web-dashboard]
    WdCore --> GW
```

</details>

---

## Project Structure

```
zenii/
├── Cargo.toml              # Workspace root (5 members)
├── CLAUDE.md               # AI assistant instructions
├── README.md               # This file
├── scripts/
│   └── build.sh            # Cross-platform build script
├── docs/
│   ├── architecture.md     # Detailed architecture diagrams
│   ├── processes.md        # Process flow diagrams
│   ├── api-reference.md    # All 96 REST/WS routes
│   ├── configuration.md    # All 70+ config fields
│   ├── cli-reference.md    # CLI command reference
│   ├── deployment.md       # Deployment guide
│   └── development.md      # Development guide
├── crates/
│   ├── zenii-core/      # Shared library (NO Tauri dependency)
│   ├── zenii-desktop/   # Tauri 2.10 shell (macOS, Windows, Linux)
│   ├── zenii-mobile/    # Tauri 2 shell (iOS, Android) (future release)
│   ├── zenii-cli/       # clap CLI
│   ├── zenii-tui/       # ratatui TUI
│   └── zenii-daemon/    # Headless daemon
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
cargo run -p zenii-daemon

# Start the CLI
cargo run -p zenii-cli -- chat

# Start the TUI
cargo run -p zenii-tui

# Start the desktop app (dev mode with hot reload)
cd crates/zenii-desktop && cargo tauri dev

# Start the desktop app connecting to external daemon
ZENII_GATEWAY_URL=http://localhost:18981 cd crates/zenii-desktop && cargo tauri dev

# Frontend dev server (hot reload)
cd web && bun run dev
```

### Building Executables

#### Native builds (current platform)

```bash
./scripts/build.sh --target native                  # Debug build
./scripts/build.sh --target native --release         # Release (optimized, smallest binary)
./scripts/build.sh --target native --release --crates "zenii-daemon zenii-cli"  # Specific crates only
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
cargo build -p zenii-daemon                          # Core only (gateway + ai + keyring)
cargo build -p zenii-daemon --features local-embeddings  # + local FastEmbed ONNX embeddings
cargo build -p zenii-daemon --features channels      # + channel core traits + registry
cargo build -p zenii-daemon --features channels-telegram  # + Telegram (teloxide)
cargo build -p zenii-daemon --features channels-slack     # + Slack
cargo build -p zenii-daemon --features channels-discord   # + Discord (serenity)
cargo build -p zenii-daemon --features scheduler     # + cron jobs
cargo build -p zenii-daemon --features api-docs      # + Scalar UI + OpenAPI spec at /api-docs
cargo build -p zenii-daemon --features web-dashboard # + embedded web UI
cargo build -p zenii-daemon --all-features           # Everything
```

---

## Testing

```bash
cargo test --workspace                    # All tests
cargo test -p zenii-core               # Core only
cargo test -p zenii-core -- memory     # Memory module
cargo test -p zenii-core -- db         # Database module
cd web && bun run test                    # Frontend tests
```

---

## Configuration

Zenii uses a TOML configuration file. Paths are resolved via `directories::ProjectDirs::from("com", "sprklai", "zenii")`:

| OS | Config File | Database File |
|---|---|---|
| **Linux** | `~/.config/zenii/config.toml` | `~/.local/share/zenii/zenii.db` |
| **macOS** | `~/Library/Application Support/com.sprklai.zenii/config.toml` | `~/Library/Application Support/com.sprklai.zenii/zenii.db` |
| **Windows** | `%APPDATA%\sprklai\zenii\config\config.toml` | `%APPDATA%\sprklai\zenii\data\zenii.db` |

Example `config.toml` (flat structure, all fields optional with defaults):

```toml
gateway_host = "127.0.0.1"
gateway_port = 18981
log_level = "info"
# data_dir = "/custom/data/path"       # Override default data directory
# db_path = "/custom/path/zenii.db" # Override database file path
identity_name = "Zenii"
identity_description = "AI-powered assistant"
default_provider = "anthropic"
default_model = "claude-sonnet-4-6"
security_autonomy_level = "supervised"  # supervised | autonomous | strict
max_tool_retries = 3
# gateway_auth_token = "your-secret-token"  # Optional bearer token for auth
# agent_max_turns = 4                        # Max tool-calling turns per request
# agent_max_continuations = 1               # Max autonomous reasoning turns
# tool_dedup_enabled = true                 # Deduplicate identical tool calls per request
# embedding_provider = "none"               # none | openai | local
# user_name = "John"                        # Display name for greetings
# user_timezone = "America/New_York"        # IANA timezone (auto-detected on first run)
# user_location = "New York, US"            # User location for context-aware queries
# plugins_dir = "/custom/plugins/path"      # Override default plugins directory
# plugin_auto_update = false                # Auto-update git-sourced plugins
```

## CLI Commands

```bash
zenii setup                        # First-run onboarding wizard (provider, API key, model, channels, profile)
zenii daemon start|stop|status     # Manage the daemon process
zenii chat [--session ID] [--model M]  # Interactive WS streaming chat
zenii run "prompt" [--session] [--model]  # Single prompt, print response
zenii memory search "query" [--limit N] [--offset N]  # Search memories
zenii memory add <key> <content>   # Add memory entry
zenii memory remove <key>          # Remove memory entry
zenii config show                  # Show current config
zenii config set <key> <value>     # Set a config value
zenii key set <provider> <key>     # Set API key
zenii key remove <provider>        # Remove API key
zenii key list                     # List stored keys
zenii provider list                # List AI providers
zenii provider test <id>           # Test provider connection
zenii provider add <id> <name> <base_url>  # Add custom provider
zenii provider remove <id>         # Remove user-defined provider
zenii provider default <provider> <model>  # Set default model
zenii embedding activate <provider>       # Activate embeddings (openai/local)
zenii embedding deactivate                # Deactivate embeddings
zenii embedding status                    # Show embedding provider status
zenii embedding test                      # Test embedding generation
zenii embedding reindex                   # Re-embed all memories
zenii plugin list                         # List installed plugins
zenii plugin install <source> [--local] [--all]  # Install from git, monorepo subdir, or local path
zenii plugin remove <name>                # Remove a plugin
zenii plugin update <name>                # Update a plugin
zenii plugin enable <name>                # Enable a plugin
zenii plugin disable <name>               # Disable a plugin
zenii plugin info <name>                  # Show plugin details
```

Global options: `--host`, `--port`, `--token` (or `ZENII_TOKEN` env var)

## Gateway Routes (79 base + 17 feature-gated = 96 total)

| Group | Routes | Description |
|-------|--------|-------------|
| Health | `GET /health` | Health check (no auth) |
| Sessions & Chat | `POST /sessions`, `GET /sessions`, `GET/PUT/DELETE /sessions/{id}`, `POST /sessions/{id}/generate-title`, `GET/POST /sessions/{id}/messages`, `POST /chat` | Chat sessions and messaging |
| Memory | `POST /memory`, `GET /memory`, `GET/PUT/DELETE /memory/{key}` | Semantic memory CRUD |
| Config | `GET /config`, `PUT /config`, `GET /config/file` | Configuration management |
| Setup | `GET /setup/status` | First-run onboarding status |
| Credentials | `POST/GET /credentials`, `DELETE /credentials/{key}`, `GET /credentials/{key}/value`, `GET /credentials/{key}/exists` | Credential management (keyring) |
| Providers & Models | `GET/POST /providers`, `GET /providers/with-key-status`, `GET/PUT /providers/default`, `GET/PUT/DELETE /providers/{id}`, `POST /providers/{id}/test`, `POST /providers/{id}/models`, `DELETE /providers/{id}/models/{model_id}`, `GET /models` | Multi-provider AI management |
| Tools | `GET /tools`, `POST /tools/{name}/execute` | Tool listing and execution |
| Permissions | `GET /permissions`, `GET /permissions/{surface}`, `PUT/DELETE /permissions/{surface}/{tool}` | Per-surface tool permissions |
| System | `GET /system/info` | System information |
| Identity | `GET /identity`, `GET/PUT /identity/{name}`, `POST /identity/reload` | Persona management |
| Skills | `GET /skills`, `GET/PUT/DELETE /skills/{id}`, `POST /skills`, `POST /skills/reload` | Skill CRUD |
| Skill Proposals | `GET /skills/proposals`, `POST /skills/proposals/{id}/approve`, `POST /skills/proposals/{id}/reject`, `DELETE /skills/proposals/{id}` | Self-evolving skill management |
| User | `GET/POST/DELETE /user/observations`, `GET/DELETE /user/observations/{key}`, `GET /user/profile` | User learning + privacy |
| Embeddings | `GET /embeddings/status`, `POST /embeddings/test`, `POST /embeddings/embed`, `POST /embeddings/download`, `POST /embeddings/reindex` | Semantic memory embedding management |
| Plugins | `GET /plugins`, `POST /plugins/install`, `GET/DELETE /plugins/{name}`, `PUT /plugins/{name}/toggle`, `POST /plugins/{name}/update`, `GET/PUT /plugins/{name}/config` | Plugin management (install, remove, enable/disable, config) |
| Channels | `POST /channels/{name}/test` (always); `GET /channels`, `GET /channels/{name}/status`, `POST /channels/{name}/send`, `POST /channels/{name}/connect`, `POST /channels/{name}/disconnect`, `GET /channels/{name}/health`, `POST /channels/{name}/message`, `GET /channels/sessions`, `GET /channels/sessions/{id}/messages` (feature-gated) | Messaging channels |
| Scheduler | `GET/POST /scheduler/jobs`, `PUT /scheduler/jobs/{id}/toggle`, `DELETE /scheduler/jobs/{id}`, `GET /scheduler/jobs/{id}/history`, `GET /scheduler/status` (feature-gated) | Cron job management |
| WebSocket | `GET /ws/chat`, `GET /ws/notifications` | Streaming chat + notification push |
| API Docs | `GET /api-docs`, `GET /api-docs/openapi.json` | Interactive Scalar UI + OpenAPI 3.1 spec (feature-gated: `api-docs`) |

---

## Documentation

**[docs.zenii.sprklai.com](https://docs.zenii.sprklai.com)** -- Full documentation site

- [Installation & Usage](https://docs.zenii.sprklai.com/installation-and-usage) -- Get up and running
- [CLI Reference](https://docs.zenii.sprklai.com/cli-reference) -- All commands, options, shell completions, recipes
- [API Reference](https://docs.zenii.sprklai.com/api-reference) -- All 96 REST/WS routes with request/response schemas
- [Configuration](https://docs.zenii.sprklai.com/configuration) -- All 70+ config.toml fields with types and defaults
- [Deployment Guide](https://docs.zenii.sprklai.com/deployment) -- Native, Docker, systemd, Raspberry Pi, reverse proxy
- [Development Guide](https://docs.zenii.sprklai.com/development) -- Prerequisites, building, testing, how-to guides
- [Architecture](https://docs.zenii.sprklai.com/architecture) -- System diagrams, crate dependencies, project structure
- [Process Flows](https://docs.zenii.sprklai.com/processes) -- Chat request, startup, error handling, WebSocket flows
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

<!-- ## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=sprklai/zenii&type=Date)](https://star-history.com/#sprklai/zenii&Date) -->

---

## Disclaimer

Zenii uses large language models (LLMs) to generate responses and can execute system-level actions (shell commands, file operations) on your behalf. LLM outputs may be inaccurate, incomplete, or inappropriate. System actions run with your user permissions. Always review AI-suggested actions before confirming. Use at your own risk.

## License

MIT
