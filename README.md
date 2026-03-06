# MesoClaw

An AI-powered multi-interface application built with Rust, producing five binaries from a single codebase: Desktop, Mobile, CLI, TUI, and Daemon.

---

## Overview

MesoClaw is a Rust workspace that delivers AI assistant capabilities across multiple interfaces. All business logic lives in a shared core library (`mesoclaw-core`), while each binary crate is a thin shell that adapts the core to its specific interface.

```mermaid
graph TB
    Desktop["Desktop<br>#40;Tauri 2 + Svelte 5#41;"] --> Core[mesoclaw-core<br>shared logic]
    Mobile["Mobile<br>#40;Tauri 2 iOS + Android#41;"] --> Core
    CLI["CLI<br>#40;clap#41;"] --> Core
    TUI["TUI<br>#40;ratatui#41;"] --> Core
    Daemon["Daemon<br>#40;headless server#41;"] --> Core
```

## Features

- **18 AI providers** via rig-core (OpenAI, Anthropic, Google, Ollama, and more)
- **Tool calling** with built-in websearch, sysinfo, and file search (ripgrep)
- **Streaming responses** via WebSocket
- **Semantic memory** with SQLite FTS5 + vector embeddings (sqlite-vec)
- **Soul / Persona system** -- markdown-defined personalities with hot-reload
- **Skills / Prompt templates** -- reusable templates with parameter substitution (comrak + Tera)
- **Progressive user learning** -- agent learns preferences over conversations with privacy controls
- **Secure credentials** via OS keyring with zeroize memory protection
- **Messaging channels** -- Telegram, Discord, Slack, Matrix, Signal, WhatsApp (openclaw-channels)
- **Cron scheduler** -- automated recurring tasks
- **Cross-platform** -- Linux, macOS, Windows, ARM (Raspberry Pi), iOS, Android

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
| Mobile | Tauri 2 (iOS + Android) |
| CLI | clap |
| TUI | ratatui |
| Channels | openclaw-channels (6 adapters) |
| Content | comrak (markdown) + serde_yaml (frontmatter) + Tera (templating) |
| i18n | paraglide-js (compile-time, tree-shakeable) |

---

## Architecture

### System Architecture

```mermaid
graph TB
    subgraph "User Interfaces"
        Desktop["Desktop<br>#40;Tauri 2#41;"]
        Mobile["Mobile<br>#40;Tauri 2#41;"]
        CLI["CLI<br>#40;clap#41;"]
        TUI["TUI<br>#40;ratatui#41;"]
        Web["Web Frontend<br>#40;Svelte 5#41;"]
    end

    subgraph "mesoclaw-core"
        subgraph "Application Layer"
            Gateway["Gateway<br>axum REST + WS<br>:18981"]
            AI["AI Engine<br>rig-core<br>18 providers"]
            Storage["Storage<br>rusqlite + sqlite-vec<br>FTS5 + vectors"]
        end
        subgraph "Domain Layer"
            Identity["Identity / Soul<br>personas + hot-reload"]
            Skills["Skills<br>prompt templates + Tera"]
            UserProfile["User Profile<br>progressive learning"]
        end
        subgraph "Support Layer"
            Agent["Agent System<br>tool registry"]
            Creds["Credentials<br>keyring + zeroize"]
            Config["Config<br>TOML + env"]
            Channels["Channels<br>openclaw-channels"]
        end
    end

    Desktop -->|Rust API| Gateway
    Mobile -->|Rust API| Gateway
    CLI -->|Rust API| Gateway
    TUI -->|Rust API| Gateway
    Web -->|HTTP/WS| Gateway
    Gateway --> AI
    Gateway --> Storage
    Gateway --> Identity
    Gateway --> Skills
    Gateway --> UserProfile
    AI --> Agent
    AI --> Creds
    AI --> Storage
```

### Crate Dependency Graph

```mermaid
graph TD
    desktop[mesoclaw-desktop] --> core[mesoclaw-core]
    mobile[mesoclaw-mobile] --> core
    cli[mesoclaw-cli] --> core
    tui[mesoclaw-tui] --> core
    daemon[mesoclaw-daemon] --> core

    core --> axum["axum<br>#40;gateway#41;"]
    core --> rusqlite["rusqlite<br>#40;database#41;"]
    core --> rigcore["rig-core<br>#40;AI#41;"]
    core --> tokio["tokio<br>#40;async#41;"]
    core --> keyring["keyring<br>#40;credentials#41;"]
    core --> comrak["comrak<br>#40;markdown#41;"]
    core --> openclaw["openclaw-channels<br>#40;messaging#41;"]
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
    participant GW as Gateway

    App->>Cfg: Parse CLI args + load TOML
    App->>App: Initialize tracing
    App->>DB: Open/create database + migrations
    App->>Cred: Initialize credential store
    App->>AI: Register providers + load API keys
    App->>AI: Register agent tools
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
├── Cargo.toml              # Workspace root (6 members)
├── CLAUDE.md               # AI assistant instructions
├── README.md               # This file
├── scripts/
│   └── build.sh            # Cross-platform build script
├── docs/
│   ├── architecture.md     # Detailed architecture diagrams
│   ├── phases.md           # Implementation phase details
│   └── processes.md        # Process flow diagrams
├── plans/
│   └── phase1_core_foundation.md  # Detailed implementation plan
├── tests/
│   ├── phase1_core_foundation.md  # Test plan + results
│   ├── phase2_ai_integration.md   # (planned)
│   └── ...
├── crates/
│   ├── mesoclaw-core/      # Shared library (NO Tauri dependency)
│   ├── mesoclaw-desktop/   # Tauri 2 shell (macOS, Windows, Linux)
│   ├── mesoclaw-mobile/    # Tauri 2 shell (iOS, Android)
│   ├── mesoclaw-cli/       # clap CLI
│   ├── mesoclaw-tui/       # ratatui TUI
│   └── mesoclaw-daemon/    # Headless daemon
└── web/                    # Svelte 5 frontend (shared by desktop + mobile)
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

# Start the desktop app
cd web && bun install && bun run build && cd ..
cd crates/mesoclaw-desktop && cargo tauri dev

# Frontend dev server (hot reload)
cd web && bun run dev
```

### Cross-Platform Builds

```bash
./scripts/build.sh --target native            # Current OS
./scripts/build.sh --target native --release   # Release mode
./scripts/build.sh --target linux-x86          # Linux x86_64
./scripts/build.sh --target linux-arm          # Linux aarch64 (RPi)
./scripts/build.sh --target macos-x86          # macOS Intel
./scripts/build.sh --target macos-arm          # macOS Apple Silicon
./scripts/build.sh --target windows            # Windows x86_64
./scripts/build.sh --target all                # All platforms
./scripts/build.sh --list-targets              # Show available targets
```

See [scripts/build.sh](scripts/build.sh) for full options.

---

## Feature Flags

```bash
cargo build -p mesoclaw-daemon                          # Core only
cargo build -p mesoclaw-daemon --features channels      # + messaging channels
cargo build -p mesoclaw-daemon --features scheduler     # + cron jobs
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
```

## Gateway Routes (~40)

All routes are prefixed with `/api/v1/`:

| Group | Routes | Description |
|-------|--------|-------------|
| Sessions & Chat | `POST /sessions`, `GET /sessions`, `GET /sessions/:id/messages`, `POST /chat` (SSE), `GET /ws` (WS) | Chat sessions and streaming |
| Providers | `GET /providers`, `PUT /providers/:id` | AI provider configuration |
| Memory | `GET /memory`, `POST /memory`, `GET /memory/search`, `GET /memory/daily/:date`, `DELETE /memory/:key` | Semantic memory CRUD |
| Identity / Soul | `GET /identity`, `PUT /identity`, `GET /identity/personas` | Persona management |
| Skills | `GET /skills`, `GET /skills/:id`, `POST /skills`, `PUT /skills/:id`, `DELETE /skills/:id` | Prompt template CRUD |
| User Profile | `GET /user`, `PUT /user`, `GET /user/observations`, `DELETE /user/observations/:id`, `POST /user/reset` | User learning + privacy |
| Scheduler | `GET /schedule/jobs`, `POST /schedule/jobs`, `PUT /schedule/jobs/:id`, `DELETE /schedule/jobs/:id` | Cron job management |
| System | `GET /health`, `GET /config`, `PUT /config`, `POST /approval/:action_id` | System administration |

---

## Documentation

Detailed documentation lives in the `docs/` and `plans/` directories:

- [Architecture](docs/architecture.md) -- System diagrams, crate dependencies, project structure
- [Implementation Phases](docs/phases.md) -- Phase gate protocol, checklist, phase details
- [Process Flows](docs/processes.md) -- Chat request, startup, error handling, WebSocket flows
- [Phase 1 Plan](plans/phase1_core_foundation.md) -- Detailed implementation plan for core foundation

### Implementation Status

| Phase | Steps | Status | Tests |
|-------|-------|--------|-------|
| Phase 1: Core Foundation | 1-4 | Complete | 16/16 passing |
| Phase 2: AI Integration | 5-7 | Not started | -- |
| Phase 3: Gateway Server | 8-10 | Not started | -- |
| Phase 4: Agent Intelligence | 10a-10c | Not started | -- |
| Phase 5: Binary Shells | 11-12 | Not started | -- |
| Phase 6: Frontend | 13 | Not started | -- |
| Phase 7: Desktop & Mobile | 14, 14b | Not started | -- |
| Phase 8: Channels & Scheduler | 15-16 | Not started | -- |
| Phase 9: TUI & Cross-Compilation | 17-18 | Not started | -- |
| Phase 10: CI/CD & Quality | 19-20 | Not started | -- |
| Phase 11: Documentation & Community | 21-22 | Not started | -- |

---

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Follow the phase gate protocol in [docs/phases.md](docs/phases.md)
4. Write tests first, then implement
5. Ensure `cargo test --workspace` and `cargo clippy --workspace` pass
6. Submit a pull request

---

## License

MIT
