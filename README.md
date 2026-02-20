# Mesoclaw

<p align="center">
  <img src="public/mesoclaw.png" alt="Mesoclaw" width="100%" />
</p>

<p align="center">
  <strong>A lightweight, privacy-first desktop AI agent. Runs on your machine. Works for you.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Tauri-2.0-blue?style=flat-square&logo=tauri" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Rust-2024-orange?style=flat-square&logo=rust" alt="Rust 2024" />
  <img src="https://img.shields.io/badge/React-19-61dafb?style=flat-square&logo=react" alt="React 19" />
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT License" />
</p>

---

Mesoclaw is a native desktop AI agent built with Tauri 2 (Rust backend, React frontend). It connects to cloud LLMs or local Ollama models, stores everything locally, and can act autonomously — reading files, running commands, monitoring services, and learning from interactions over time.

**One-liner**: A personal AI agent that lives on your desktop, knows your context, and works for you — even when you're not watching.

Unlike browser-based AI tools, Mesoclaw is a real native application: 3–15 MB binary, native OS integration (system tray, keychain, notifications), no telemetry, and full offline support via Ollama.

## The Claw Ecosystem

Mesoclaw draws architectural lessons from the broader "claw" family of AI agents:

| Project                                          | Language         | Target               | RAM             | Binary           |
| ------------------------------------------------ | ---------------- | -------------------- | --------------- | ---------------- |
| [OpenClaw](https://github.com/openclaw/openclaw) | TypeScript       | Server/Desktop       | >1 GB           | ~28 MB + Node.js |
| [PicoClaw](https://github.com/sipeed/picoclaw)   | Go               | Edge ($10 SBCs)      | <10 MB          | ~8 MB            |
| [IronClaw](https://github.com/nearai/ironclaw)   | Rust             | NEAR AI agents       | Moderate        | Single binary    |
| [ZeroClaw](https://github.com/openagen/zeroclaw) | Rust             | Resource-constrained | <5 MB           | ~3.4 MB          |
| **Mesoclaw**                                     | **Rust + React** | **Native desktop**   | **<50 MB idle** | **<15 MB**       |

Mesoclaw combines **ZeroClaw's trait-based security architecture**, **PicoClaw's minimalism**, and **OpenClaw's feature breadth** — wrapped in a native Tauri desktop shell for macOS, Windows, and Linux.

---

## Quick Start

### Prerequisites

- **Bun** ≥ 1.3
- **Rust** stable toolchain
- Tauri v2 system requirements for your OS ([platform setup docs](https://v2.tauri.app/start/prerequisites/))
- An API key for any supported LLM provider (or [Ollama](https://ollama.ai) for fully local inference)

### Install and Run

```bash
git clone https://github.com/sprklai/mesoclaw.git
cd mesoclaw

bun install
bun run tauri:dev        # Full stack dev (hot reload frontend + Rust backend)
```

### Useful Scripts

```bash
# Frontend
bun run dev              # Vite dev server only
bun run build            # Frontend production build
bun run test             # Frontend unit tests
bun run lint             # Biome lint check
bunx ultracite fix       # Auto-format with Biome

# Backend
bun run cargo:check      # Quick compile check
bun run cargo:build      # Debug build
bun run cargo:build:release  # Release build (optimized)

# Full stack
bun run tauri:dev        # Dev with hot reload
bun run tauri build      # Production bundle (macOS/Windows/Linux)
```

---

## Features

### Current (v0.x)

- **Multi-provider LLM support** — OpenAI, Anthropic, Google AI, Groq, Ollama, Vercel AI Gateway, OpenRouter
- **Streaming responses** via Server-Sent Events
- **Secure API key storage** — OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service). Keys never touch disk.
- **Prompt template system** — Filesystem-based skill templates in `~/.mesoclaw/prompts/`
- **Gateway-first architecture** — HTTP REST + WebSocket control plane at `127.0.0.1:18790`
- **Telegram channel** — Bot-based inbound/outbound messaging with allowed-chat allowlist
- **Event Bus** — `tokio::broadcast` backbone connecting all subsystems
- **Scheduler** — Heartbeat and cron job support with configurable intervals
- **Identity system** — Editable markdown files define the agent's personality and behavior
- **Sidecar module system** — Extend via native processes, Docker containers, or MCP servers
- **Dark/light mode** — Respects `prefers-color-scheme` with manual override

### Roadmap

| Version | Milestone    | Key Features                                                                        |
| ------- | ------------ | ----------------------------------------------------------------------------------- |
| v0.6    | Foundation   | `ReliableProvider` retry/fallback, release profile optimization                     |
| v0.7    | Core Agent   | Tool system, security policy, event bus, identity system, sidecar modules           |
| v0.8    | Intelligence | Multi-turn agent loop, memory system (vector + BM25), MCP client, container runtime |
| v0.9    | Proactive    | Scheduler, native notifications, system tray, session management                    |
| v1.0    | Complete     | Config system, channel management UI, boot sequence, CLI REPL                       |
| v1.1    | Channels+    | Telegram polish, WhatsApp (TBD), channel management UX                              |
| v1.2    | Mobile       | Tauri Mobile (iOS + Android), push notifications, TestFlight                        |

See `docs/implementation-plan.md` for the full 49-task execution plan.

---

## Modes of Interaction

Mesoclaw is **gateway-centric**: the daemon is the product. All clients talk to the same local HTTP API at `127.0.0.1:18790`. No special client is required.

### Desktop GUI (current)

The Tauri window is the primary interface. It connects to the gateway over Tauri IPC and REST, giving you:

- Chat with any configured LLM provider
- Provider and API key management (Settings → Providers)
- Identity file editor (Settings → Identity)
- Tool approval overlay for supervised actions
- Memory timeline and search

### Gateway REST API (current)

The daemon exposes a full HTTP API. Any HTTP client can interact with Mesoclaw while the desktop app is running. A bearer token is auto-generated at `~/.mesoclaw/daemon.token` on first launch.

```bash
# Read the token
TOKEN=$(cat ~/.mesoclaw/daemon.token)
BASE="http://127.0.0.1:18790"

# Health check (no auth required)
curl "$BASE/api/v1/health"
# → {"status":"ok","service":"mesoclaw-daemon"}

# List active providers
curl -H "Authorization: Bearer $TOKEN" "$BASE/api/v1/providers"

# Create a chat session
curl -X POST "$BASE/api/v1/sessions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"channel":"user","system_prompt":"You are a helpful assistant."}'

# List sessions
curl -H "Authorization: Bearer $TOKEN" "$BASE/api/v1/sessions"

# List registered sidecar modules
curl -H "Authorization: Bearer $TOKEN" "$BASE/api/v1/modules"

# Reload modules from disk (hot-reload)
curl -X POST -H "Authorization: Bearer $TOKEN" "$BASE/api/v1/modules"

# Start/stop a module
curl -X POST -H "Authorization: Bearer $TOKEN" "$BASE/api/v1/modules/python-analyst/start"

# Read an identity file
curl -H "Authorization: Bearer $TOKEN" "$BASE/api/v1/identity/SOUL"

# Update an identity file (hot-reload, no restart needed)
curl -X PUT "$BASE/api/v1/identity/USER" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content":"# User\nName: Alice\nPrefers concise answers."}'

# WebSocket event stream
wscat -H "Authorization: Bearer $TOKEN" -c "ws://127.0.0.1:18790/api/v1/ws"
```

### CLI REPL (planned — v1.0)

A command-line REPL that connects to the same gateway as the desktop app. You will be able to chat with the agent, inspect sessions, and manage configuration — all from the terminal, without the desktop window. Planned for v1.0.

---

## Autonomy Modes

The security policy controls how freely the agent can execute tools on your behalf. Configure it in `~/.mesoclaw/config.toml`:

```toml
[security]
autonomy = "supervised"   # readOnly | supervised | full
rate_limit = 20           # max actions per hour (applies to "full" only)
```

| Mode                     | Shell commands                         | File writes       | Network           | Approval required             |
| ------------------------ | -------------------------------------- | ----------------- | ----------------- | ----------------------------- |
| `readOnly`               | Read-only (`ls`, `cat`, `grep`, `git`) | No                | No                | Medium/High always denied     |
| `supervised` _(default)_ | Read-only auto-approved                | Requires approval | Requires approval | Medium and High risk          |
| `full`                   | All (rate-limited)                     | Yes               | Yes               | Never — subject to rate limit |

**Risk classification:**

| Risk    | Examples                                   | `supervised` behavior        |
| ------- | ------------------------------------------ | ---------------------------- |
| Low     | `ls`, `cat`, `grep`, `git`, `echo`         | Auto-approved                |
| Medium  | `mkdir`, `cp`, `mv`, `cargo`, `npm`, `pip` | Requires user approval       |
| High    | `curl`, `wget`, `chmod`, unknown binaries  | Requires user approval       |
| Blocked | `rm`, `sudo`, `dd`, `mkfs`, `shutdown`     | Always denied — in all modes |

Shell injection patterns (backticks, `$()`, pipes, redirects, `;`, `&&`, `||`) are always blocked regardless of mode.

The approval UI appears as an overlay in the desktop app. Tool calls via the REST API in `supervised`/`readOnly` mode will receive a `needs_approval` response and block until the user acts in the desktop window.

---

## Architecture

Mesoclaw is built on a **gateway-centric architecture**: the daemon is the product, and both the desktop GUI and CLI are thin clients connecting to the same local gateway API.

```
                          mesoclaw-core (lib.rs)
              ┌──────────────────────────────────────┐
              │           Daemon + Gateway            │
              │     (axum HTTP + WebSocket on         │
              │      127.0.0.1:18790)                 │
              │                                       │
              │  Agent · Providers · Memory · Tools   │
              │  Security · Identity · Scheduler      │
              │  Channels · Event Bus · Config        │
              └───────────────┬───────────────────────┘
                              │ HTTP REST + WebSocket
              ┌───────────────┼───────────────────────┐
              │               │                       │
        ┌─────▼─────┐  ┌──────▼─────┐  ┌─────────────▼─┐
        │    CLI    │  │  Tauri GUI  │  │  curl/scripts │
        │  (REPL)   │  │  (React)    │  │  (any client) │
        └───────────┘  └────────────┘  └───────────────┘
```

### Backend Module Map

```
src-tauri/src/
├── gateway/         REST endpoints + WebSocket event stream + auth + daemon lifecycle
├── agent/           Multi-turn conversation loop + dual tool-call parser (JSON + XML)
├── providers/       LLMProvider trait + GenericProvider + ReliableProvider retry/fallback
├── tools/           Tool trait + ToolRegistry + built-in shell/file tools
├── memory/          SQLite vector storage + FTS5/BM25 hybrid search + chunking + embeddings
├── security/        SecurityPolicy (ReadOnly / Supervised / Full autonomy) + rate limiting
├── channels/        Channel trait + TauriIPC + Telegram + webhook listener
├── event_bus/       AppEvent enum + tokio::broadcast + Tauri frontend bridge
├── scheduler/       Heartbeat + cron jobs + stuck detection
├── identity/        SOUL.md / USER.md / AGENTS.md loader + system prompt assembly
├── modules/         SidecarTool / SidecarService / McpServer + container runtime abstraction
├── config/          TOML config + env variable overrides + atomic saves
├── prompts/         Filesystem-based markdown prompt templates + Tera rendering
├── services/        Credential store + notifications + session router + boot sequence
└── database/        SQLite connection management + Diesel ORM models
```

### Frontend Component Map

```
src/
├── routes/          Chat · Settings · Memory · Root layout (TanStack Router, file-based)
├── stores/          agentStore · providerStore · memoryStore · schedulerStore · identityStore
├── components/
│   ├── ui/          Base UI primitives (shadcn-style, Base UI components)
│   ├── ai-elements/ Conversation · Message · PromptInput (AI SDK Elements)
│   ├── agent/       ToolExecutionStatus · ApprovalOverlay · AgentProgress
│   ├── memory/      MemorySearch · DailyTimeline
│   └── settings/    ProviderConfig · SecurityConfig · IdentityEditor
└── lib/
    ├── gateway-client.ts  Typed REST + WebSocket client
    └── ws-events.ts       WebSocket event subscription helpers
```

---

## Security

Mesoclaw adopts a layered security model inspired by ZeroClaw's 6-layer defense, adapted for the desktop context:

| Layer                | Scope           | Implementation                                                                                           |
| -------------------- | --------------- | -------------------------------------------------------------------------------------------------------- |
| **1. Credentials**   | API keys        | OS keyring only. Never written to disk. Zeroized in memory after use.                                    |
| **2. Autonomy**      | Agent actions   | Three levels: `ReadOnly` (query-only), `Supervised` (approve medium/high risk), `Full` (rate-limited)    |
| **3. Filesystem**    | File access     | Workspace-restricted. Blocks `/etc`, `/root`, `~/.ssh`, `~/.aws`, `~/.gnupg`. Path traversal prevention. |
| **4. Injection**     | Command strings | Blocks backticks, `$()`, `${}`, `>`, `>>`, pipe splitting from LLM-provided inputs                       |
| **5. Rate Limiting** | Tool execution  | Sliding window, 20 actions/hour default, configurable per autonomy level                                 |
| **6. Audit Trail**   | Logging         | All tool executions logged with timestamp, args, and result in `~/.mesoclaw/logs/audit.jsonl`            |

**Privacy guarantees:**

- No telemetry or analytics. No phone-home behavior.
- All data stored locally in `~/.mesoclaw/` or the OS app data directory.
- API keys never logged, never written to config files, zeroized from memory after use.
- Local LLM inference available via Ollama (zero data leaves your machine).

---

## Sidecar Module System

Mesoclaw extends its capabilities through sidecar modules — external processes that the agent can invoke as tools:

| Module Type      | Protocol          | Use Case                                |
| ---------------- | ----------------- | --------------------------------------- |
| `SidecarTool`    | stdin/stdout JSON | On-demand scripts (Python, Node, shell) |
| `SidecarService` | HTTP REST         | Long-lived background services          |
| `McpServer`      | JSON-RPC (MCP)    | Any MCP-compatible tool server          |

Modules are defined with TOML manifests and discovered automatically at startup:

```toml
# ~/.mesoclaw/modules/python-analyst/manifest.toml
[module]
name = "python-analyst"
description = "Run Python data analysis scripts"
type = "sidecar_tool"

[runtime]
command = "python3"
args = ["main.py"]

[tool]
input_schema = { query = "string" }

[security]
allowed_paths = ["~/Documents/data"]
network = false
timeout_seconds = 30
```

Container isolation (Docker or Podman) is supported for sandboxed execution.

---

## Identity & Personality

The agent's behavior is controlled by plain markdown files you can read and edit:

```
~/.mesoclaw/identity/
├── SOUL.md          Core personality, values, and hard limits
├── USER.md          User preferences, context, and working style
├── AGENTS.md        Operating instructions and agent role definition
├── IDENTITY.md      Agent name, description, and avatar
├── HEARTBEAT.md     Checklist the agent runs on every heartbeat
├── BOOT.md          Startup checklist executed on app launch
└── TOOLS.md         Guidance on when and how to use each tool
```

The agent assembles these into its system prompt in order: SOUL → AGENTS → USER → TOOLS → MEMORY → daily context → conversation history. Hot-reload via file watcher — edit and save, no restart needed.

---

## Messaging Channels

Mesoclaw supports multiple inbound channels beyond the desktop UI:

| Channel             | Status | Notes                                                                     |
| ------------------- | ------ | ------------------------------------------------------------------------- |
| Desktop (Tauri IPC) | Done   | Default channel, always available                                         |
| Telegram Bot        | v1.1   | Long-polling, MarkdownV2, file/photo support, `allowed_chat_ids` security |
| HTTP Webhook        | v1.0   | axum listener for GitHub, Zapier, and other integrations                  |
| WhatsApp            | Future | Post-v1.1                                                                 |
| Discord             | Future | Post-v1.1                                                                 |

Security note: tool approvals always route to the desktop app. They are never granted through external messaging channels.

---

## Tech Stack

| Layer             | Technology                  | Purpose                                         |
| ----------------- | --------------------------- | ----------------------------------------------- |
| Desktop shell     | Tauri 2                     | Native window, system tray, notifications, IPC  |
| Frontend          | React 19 + TypeScript       | UI rendering                                    |
| Build tool        | Vite                        | Frontend bundling, HMR                          |
| Routing           | TanStack Router             | File-based client-side routing                  |
| State             | Zustand                     | Lightweight stores                              |
| Styling           | Tailwind CSS 4              | Utility-first CSS                               |
| Backend           | Rust 2024                   | Performance, memory safety                      |
| Async             | Tokio                       | Async I/O, background tasks, scheduling         |
| LLM client        | async-openai                | OpenAI-compatible API calls                     |
| Database          | SQLite (rusqlite)           | App data, vector storage, FTS5 full-text search |
| Secrets           | OS Keyring + zeroize        | Secure credential storage                       |
| Templates         | Tera                        | Prompt template rendering                       |
| HTTP server       | axum                        | Gateway/control plane, webhook listener         |
| Sidecar protocol  | stdin/stdout JSON + MCP     | Module communication                            |
| Container runtime | bollard (Docker/Podman API) | Container-based module isolation                |
| Logging           | tracing                     | Structured async logging                        |
| Package manager   | Bun                         | Frontend dependency management                  |

---

## Platform Support

| OS      | Architecture                   | Installer        | Code Signing            |
| ------- | ------------------------------ | ---------------- | ----------------------- |
| macOS   | Apple Silicon (aarch64)        | DMG, APP         | Apple notarization      |
| macOS   | Intel (x86_64)                 | DMG, APP         | Apple notarization      |
| macOS   | Universal binary               | DMG              | Apple notarization      |
| Windows | x64                            | MSI, NSIS EXE    | Azure Trusted Signing   |
| Windows | ARM64                          | MSI, NSIS EXE    | Azure Trusted Signing   |
| Linux   | x64 (Ubuntu 22.04+)            | .deb             | —                       |
| Linux   | x64 (Ubuntu 24.04+)            | AppImage, .rpm   | —                       |
| Linux   | ARM64                          | .deb, AppImage   | —                       |
| iOS     | arm64                          | IPA (TestFlight) | Apple distribution cert |
| Android | arm64-v8a, armeabi-v7a, x86_64 | AAB, APK         | Keystore                |

32-bit x86 is not supported on any platform.

---

## Configuration

Mesoclaw uses TOML configuration with environment variable overrides:

```toml
# ~/.mesoclaw/config.toml

[provider]
default = "anthropic"           # Override: MESOCLAW_PROVIDER=anthropic

[agent]
max_iterations = 20             # Max tool calls per agent turn
history_limit = 50              # Max messages before trimming

[security]
autonomy = "supervised"         # readOnly | supervised | full
rate_limit = 20                 # Actions per hour

[scheduler]
heartbeat_interval_mins = 30    # Heartbeat frequency

[modules]
preferred_runtime = "auto"      # auto | native | podman | docker
```

Config saves are atomic (write-to-temp → fsync → backup → rename) to prevent corruption on crash.

---

## Branding & Renaming

Product naming is centralized in `branding.config.json`. To white-label or rename:

```bash
# 1) Edit branding values
$EDITOR branding.config.json

# 2) Sync branding across all files
bun run branding:sync

# 3) Verify backend still compiles
bun run cargo:check
```

Fields you can change: `productName`, `slug`, `reverseDomain`, `mainWindowTitle`, `keychainService`, `skillsConfigDirName`, and more.

---

## Development

### Testing

```bash
# Backend (Rust)
cd src-tauri
cargo test --lib                                    # All unit tests
cargo test --lib gateway::                          # Gateway module only
cargo test --lib -- --nocapture                     # With output

# Frontend
bun run test                  # Run once
bun run test:watch            # Watch mode
bun run test:coverage         # Coverage report
```

### Database

```bash
cd src-tauri
diesel migration run          # Apply pending migrations
diesel migration revert       # Rollback last migration
diesel migration refresh      # Rebuild schema (destructive)
```

App database: Tauri app-local data directory (`app_local_data_dir`), typically under your OS application data path.

### Code Quality

```bash
bunx ultracite fix            # Auto-format (Biome)
bunx ultracite check          # Lint only
cargo clippy -- -D warnings   # Rust lint
cargo fmt                     # Rust format
```

---

## Storage Layout

```
~/.mesoclaw/
├── config.toml               User configuration
├── app.db                    SQLite database (sessions, messages, memory vectors)
├── daemon.token              Auto-generated gateway bearer token
├── daemon.pid                Gateway PID + port
├── identity/                 Agent personality files (editable markdown)
│   ├── SOUL.md
│   ├── USER.md
│   ├── AGENTS.md
│   ├── IDENTITY.md
│   ├── HEARTBEAT.md
│   ├── BOOT.md
│   └── TOOLS.md
├── memory/                   Daily memory summaries
│   ├── MEMORY.md             Curated long-term memory
│   └── YYYY-MM-DD.md         Per-day summaries
├── prompts/                  Custom prompt templates (*.md)
├── modules/                  User-installed sidecar modules
│   └── {name}/manifest.toml
└── logs/
    └── audit.jsonl           Tool execution audit trail
```

---

## Documentation Index

| Document                          | Description                                                    |
| --------------------------------- | -------------------------------------------------------------- |
| `docs/product-requirements.md`    | Full PRD — requirements, personas, release plan                |
| `docs/architecture-diagram.md`    | Detailed system architecture with data flow diagrams           |
| `docs/implementation-plan.md`     | 49-task execution plan across 8 phases                         |
| `docs/claw-ecosystem-analysis.md` | Comparative analysis of OpenClaw, PicoClaw, IronClaw, ZeroClaw |
| `docs/mesoclaw-gap-analysis.md`   | Gap analysis between current state and target architecture     |
| `docs/plans/`                     | Detailed design documents for each subsystem                   |
| `src/CLAUDE.md`                   | Frontend code standards (React/TypeScript)                     |
| `src-tauri/CLAUDE.md`             | Backend code standards (Rust)                                  |

---

## Release

```bash
# Check version status
./scripts/release.sh status

# Create a release
./scripts/release.sh patch    # 0.0.1 → 0.0.2
./scripts/release.sh minor    # 0.0.2 → 0.1.0
./scripts/release.sh major    # 0.1.0 → 1.0.0
```

This syncs versions, creates a release commit, and triggers GitHub Actions for cross-platform builds. See `docs/RELEASING.md` for code signing setup.

---

## License

MIT
