# TauriClaw Product Requirements Document (PRD)

> Defines what TauriClaw is, who it's for, what it does, and how success is measured.
> Reference: `docs/claw-ecosystem-analysis.md`, `docs/tauriclaw-gap-analysis.md`

---

## 1. Product Vision

**TauriClaw** is a lightweight, privacy-first desktop AI agent that runs natively on macOS, Windows, and Linux. It combines the power of cloud LLMs (or local models via Ollama) with the security and responsiveness of a native application. Unlike browser-based AI tools, TauriClaw runs on your machine, stores data locally, and can act autonomously — reading files, executing commands, monitoring services, and learning from interactions over time.

**One-liner**: A personal AI agent that lives on your desktop, knows your context, and works for you — even when you're not watching.

---

## 2. Target Users

| Persona | Description | Primary Value |
|---------|-------------|---------------|
| **Solo Developer** | Works with codebases, databases, APIs daily. Wants an AI assistant that understands their project. | Autonomous code analysis, tool execution, persistent project knowledge. |
| **Team Lead** | Manages multiple projects and people. Needs summarization and monitoring. | Proactive heartbeat, scheduled summaries, notification routing. |
| **AI Enthusiast** | Experiments with different LLMs, prompts, and agent behaviors. | Multi-provider support, identity customization, extensibility. |
| **Privacy-Conscious User** | Needs AI assistance but won't send data to third-party servers. | Local Ollama support, on-device storage, no telemetry. |

---

## 3. Product Principles

1. **Desktop-native, not browser-wrapped** — Native OS integration (system tray, notifications, keyring). Tauri's 3-5 MB binary, not Electron's 200 MB.
2. **Privacy by default** — All data stored locally. API keys in OS keyring. No telemetry. Local LLM option via Ollama.
3. **Progressive trust** — Users start with read-only agent, graduate to supervised, then full autonomy at their own pace.
4. **Transparent personality** — Agent identity is plain markdown files the user can read and edit. No black-box behavior.
5. **Lean codebase** — Prefer one well-tested implementation over three similar ones. Use established crates over custom machinery. Target < 5,000 lines of Rust for core backend.

---

## 4. Functional Requirements

### FR-1: LLM Provider Management

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-1.1 | Support multiple LLM providers: OpenAI, Anthropic, Google AI, Groq, OpenRouter, Vercel AI Gateway | P0 | **Done** |
| FR-1.2 | Support local models via Ollama | P0 | **Done** |
| FR-1.3 | Store API keys in OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service) | P0 | **Done** |
| FR-1.4 | Streaming responses via Server-Sent Events | P0 | **Done** |
| FR-1.5 | Consolidate providers into single `GenericProvider` using `async-openai` crate | P0 | Planned (S2) |
| FR-1.6 | `ReliableProvider` wrapper with retry (3x exponential backoff) + fallback chain | P0 | Planned (P0.1) |
| FR-1.7 | Provider warmup (pre-establish TLS connections on startup) | P1 | Planned (P0.1) |
| FR-1.8 | Model routing: automatic provider selection based on task type or cost tier | P2 | Planned (P2.12) |
| FR-1.9 | Provider health monitoring with status exposed to frontend | P2 | Planned |

### FR-2: Agent Loop (Autonomous Multi-Turn Execution)

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-2.1 | Multi-turn conversation: message → LLM → tool call → execute → feed result → repeat | P0 | Planned (P0.3) |
| FR-2.2 | Dual tool-call parsing: OpenAI JSON format + XML format | P0 | Planned (P0.3) |
| FR-2.3 | Max iterations per turn (configurable, default 20) to prevent runaway loops | P0 | Planned (P0.3) |
| FR-2.4 | History trimming at 50 messages to prevent context overflow | P0 | Planned (P0.3) |
| FR-2.5 | Emit events for each stage (tool start, tool result, approval needed) via Event Bus | P1 | Planned (P0.3) |
| FR-2.6 | Cancel/abort running agent loop from frontend | P1 | Planned (P3.22) |
| FR-2.7 | Memory context injection into agent conversations | P1 | Planned (P1.5) |

### FR-3: Tool System

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-3.1 | `Tool` trait: name, description, JSON Schema parameters, async execute | P1 | Planned (P1.4) |
| FR-3.2 | `ToolRegistry`: dynamic registration, lookup by name, list all tools with schemas | P1 | Planned (P1.4) |
| FR-3.3 | Built-in tools: shell command execution, file read, file write, file list | P1 | Planned (P1.4) |
| FR-3.4 | Tool execution gated by Security Policy (FR-6) | P1 | Planned (P1.6) |
| FR-3.5 | Memory tools: store, recall, forget (once memory system exists) | P1 | Planned (P1.5) |
| FR-3.6 | WASM extension system for third-party tools | P3 | Planned (P3.20) |

### FR-4: Memory System

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-4.1 | `Memory` trait: store(key, content, category), recall(query, limit), forget(key) | P1 | Planned (P1.5) |
| FR-4.2 | SQLite-based vector storage (embeddings as BLOBs) | P1 | Planned (P1.5) |
| FR-4.3 | FTS5 full-text search with BM25 scoring | P1 | Planned (P1.5) |
| FR-4.4 | Hybrid search: `0.7 * vector_similarity + 0.3 * bm25_score` | P1 | Planned (P1.5) |
| FR-4.5 | LRU embedding cache (10,000 entries default) | P1 | Planned (P1.5) |
| FR-4.6 | Document chunking for long inputs before embedding | P1 | Planned (P1.5) |
| FR-4.7 | Memory categories: Core, Daily, Conversation, Custom | P1 | Planned (P1.5) |
| FR-4.8 | Daily memory files: `~/.tauriclaw/memory/YYYY-MM-DD.md` | P2 | Planned (P2.17) |
| FR-4.9 | Auto-generated daily summary at configurable time | P2 | Planned (P2.17) |
| FR-4.10 | Curated long-term memory: `MEMORY.md` (manually maintained) | P2 | Planned (P2.17) |
| FR-4.11 | Memory hygiene: auto-archive > 7 days, purge > 30 days | P3 | Planned (P3.18) |
| FR-4.12 | Frontend: memory search UI with results display | P3 | Planned (P3.21) |

### FR-5: Identity & Personality System

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-5.1 | Identity files in `~/.tauriclaw/identity/`: SOUL.md, USER.md, AGENTS.md, IDENTITY.md, TOOLS.md | P1 | Planned (P1.10) |
| FR-5.2 | System prompt assembly: SOUL → AGENTS → USER → TOOLS → MEMORY → daily → conversation | P1 | Planned (P1.10) |
| FR-5.3 | Hot-reload on file change (file watcher) | P1 | Planned (P1.10) |
| FR-5.4 | Default templates shipped with the app | P1 | Planned (P1.10) |
| FR-5.5 | HEARTBEAT.md: checklist for heartbeat monitoring runs | P1 | Planned (P1.10) |
| FR-5.6 | BOOT.md: startup checklist executed on app launch | P2 | Planned (P3.23) |
| FR-5.7 | Frontend: markdown editor for identity files in Settings | P3 | Planned (P3.25) |
| FR-5.8 | Template gallery: pre-built personality templates | P3 | Planned (P3.25) |

### FR-6: Security Policy

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-6.1 | Three autonomy levels: ReadOnly, Supervised, Full | P1 | Planned (P1.6) |
| FR-6.2 | Command risk classification: Low (read-only), Medium (state-changing), High (destructive) | P1 | Planned (P1.6) |
| FR-6.3 | User approval overlay for Supervised mode (approve/deny from frontend) | P1 | Planned (P1.6) |
| FR-6.4 | Path traversal prevention: block `..`, null bytes, symlink escape | P1 | Planned (P1.6) |
| FR-6.5 | Injection protection: block backticks, `$()`, `${}`, `>`, `>>`, pipes | P1 | Planned (P1.6) |
| FR-6.6 | Blocked system directories: /etc, /root, ~/.ssh, ~/.aws, ~/.gnupg | P1 | Planned (P1.6) |
| FR-6.7 | Rate limiting: sliding window, configurable actions/hour (default 20) | P1 | Planned (P1.6) |
| FR-6.8 | Audit trail: all tool executions logged with timestamp + result | P2 | Planned |

### FR-7: Event Bus

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-7.1 | `EventBus` trait: publish(event), subscribe(event_type), subscribe_filtered(filter) | P1 | Planned (P1.8) |
| FR-7.2 | `AppEvent` enum covering all subsystem events | P1 | Planned (P1.8) |
| FR-7.3 | Default implementation: `TokioBroadcastBus` using `tokio::sync::broadcast` | P1 | Planned (P1.8) |
| FR-7.4 | Tauri bridge: forward UI-relevant events to frontend via `app_handle.emit()` | P1 | Planned (P1.8) |
| FR-7.5 | Event filtering by type, channel, or session scope | P2 | Planned (P1.8) |

### FR-8: Scheduler (Heartbeat + Cron)

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-8.1 | `Scheduler` trait: start, stop, add_job, remove_job, list_jobs | P1 | Planned (P1.9) |
| FR-8.2 | Heartbeat mode: configurable interval (default 30 min), reads HEARTBEAT.md checklist | P1 | Planned (P1.9) |
| FR-8.3 | Cron mode: 5-field cron expressions with timezone support | P1 | Planned (P1.9) |
| FR-8.4 | One-shot timed jobs (e.g., "remind me at 3pm") | P2 | Planned (P1.9) |
| FR-8.5 | Error backoff: exponential 30s → 1m → 5m → 15m → 60m | P2 | Planned (P1.9) |
| FR-8.6 | Stuck detection: flag heartbeat runs > 120 seconds | P2 | Planned (P1.9) |
| FR-8.7 | Frontend: scheduler management UI (job list, create/edit, history) | P3 | Planned (P3.24) |

### FR-9: Desktop Notifications

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-9.1 | Native OS notifications via `tauri-plugin-notification` | P2 | Planned (P2.15) |
| FR-9.2 | System tray icon with status indicators (idle, processing, notification pending) | P2 | Planned (P2.15) |
| FR-9.3 | Notification types: heartbeat alert, cron reminder, agent completion, approval request | P2 | Planned (P2.15) |
| FR-9.4 | Click-to-open: notification opens relevant chat/session | P2 | Planned (P2.15) |
| FR-9.5 | Do Not Disturb mode | P3 | Planned |
| FR-9.6 | Per-category notification preferences | P3 | Planned |

### FR-10: Session Management

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-10.1 | Structured session keys: `{agent}:{scope}:{channel}:{peer}` | P2 | Planned (P2.16) |
| FR-10.2 | Session router: resolve inbound messages to correct session | P2 | Planned (P2.16) |
| FR-10.3 | Isolated sessions for cron/heartbeat (don't pollute main chat) | P2 | Planned (P2.16) |
| FR-10.4 | Session compaction: truncate old messages while preserving summary | P2 | Planned (P2.16) |

### FR-11: Configuration

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-11.1 | TOML config file at `~/.tauriclaw/config.toml` | P2 | Planned (P2.11) |
| FR-11.2 | Environment variable overrides (e.g., `TAURICLAW_PROVIDER=anthropic`) | P2 | Planned (P2.11) |
| FR-11.3 | Atomic config saves: write-to-temp → fsync → backup → rename | P2 | Planned (P2.11) |
| FR-11.4 | Config hot-reload on file change | P3 | Planned |

### FR-12: Channels (External Input)

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-12.1 | `Channel` trait: send, listen, health_check, name | P1 | Planned (P1.7) |
| FR-12.2 | Tauri IPC as default channel (wraps existing IPC) | P1 | Planned (P1.7) |
| FR-12.3 | `ChannelMessage` normalization: all channels produce same message struct | P1 | Planned (P1.7) |
| FR-12.4 | Channel Manager: lifecycle (start/stop/reconnect), health monitoring | P1 | Planned (P1.7) |
| FR-12.5 | **Telegram bot channel**: long-polling listener, MarkdownV2 responses, file/photo support | P1 | Planned (P7.1) |
| FR-12.6 | Telegram: `allowed_chat_ids` allowlist (only authorized chats can interact) | P1 | Planned (P7.1) |
| FR-12.7 | Telegram: bot token stored in OS keyring, never in config file | P1 | Planned (P7.1) |
| FR-12.8 | Telegram: long message splitting (4096 char limit per message) | P1 | Planned (P7.1) |
| FR-12.9 | Telegram: bot commands (`/start`, `/status`, `/cancel`) | P2 | Planned |
| FR-12.10 | Channel → session routing: each chat_id/phone gets its own session | P2 | Planned (P2.16) |
| FR-12.11 | Cross-channel memory: agent remembers context across all channels | P2 | Planned |
| FR-12.12 | Approval routing: tool approvals always go to desktop app, never via messaging channel | P1 | Planned |
| FR-12.13 | HTTP webhook listener (axum) for integrations (GitHub, etc.) | P2 | Planned (P1.7) |
| FR-12.14 | **WhatsApp channel** via WhatsApp Web protocol or Cloud API | P3 | Planned |
| FR-12.15 | **Discord bot channel** | P3 | Planned |
| FR-12.16 | WebSocket gateway for LAN/remote access | P3 | Planned |
| FR-12.17 | Frontend: Channel management UI (connect/disconnect, status, allowlist config) | P2 | Planned |

### FR-13: Responsive Design & Mobile Readiness

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-13.1 | Responsive layout system with 5 breakpoints: xs (<640), sm (640-768), md (768-1024), lg (1024-1280), xl (>1280) | P0 | Planned |
| FR-13.2 | Desktop: 3-column layout (sidebar + chat + detail panel) | P0 | Planned |
| FR-13.3 | Tablet: 2-column with collapsible sidebar | P0 | Planned |
| FR-13.4 | Mobile: single-column with bottom navigation bar | P0 | Planned |
| FR-13.5 | All interactive elements minimum 44x44px touch targets | P0 | Planned |
| FR-13.6 | Safe area support for mobile notch/home indicator (`env(safe-area-inset-*)`) | P1 | Planned |
| FR-13.7 | Virtual keyboard handling: chat area resizes when keyboard opens | P1 | Planned |
| FR-13.8 | Swipe gestures: right to open sidebar, left to close | P2 | Planned |
| FR-13.9 | Pull-to-refresh for loading older messages | P2 | Planned |
| FR-13.10 | Bottom sheet pattern for modals on mobile (instead of centered dialogs) | P1 | Planned |
| FR-13.11 | Approval overlay: centered modal on desktop, bottom sheet on mobile | P1 | Planned |
| FR-13.12 | Tauri Mobile compilation (iOS + Android) | P3 | Planned |
| FR-13.13 | Mobile push notifications via APNs (iOS) and FCM (Android) | P3 | Planned |
| FR-13.14 | Offline message queuing: queue messages when offline, send when reconnected | P3 | Planned |
| FR-13.15 | Dark mode respects `prefers-color-scheme` + manual toggle | P0 | **Done** |

### FR-14: CLI Interface

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-14.1 | Interactive REPL with streaming output, readline history, and slash commands | P0 | Planned |
| FR-14.2 | One-shot mode: `tauriclaw "prompt"` runs agent, prints result, exits | P0 | Planned |
| FR-14.3 | Stdin pipe support: `cat file \| tauriclaw "analyze"` | P0 | Planned |
| FR-14.4 | `--raw` flag: output only agent text (no UI chrome) for scripting | P0 | Planned |
| FR-14.5 | `--json` flag: structured JSON output | P1 | Planned |
| FR-14.6 | `--auto` flag: full autonomy (skip approval prompts) | P0 | Planned |
| FR-14.7 | Daemon management: `tauriclaw daemon start/stop/status/logs` | P0 | Planned |
| FR-14.8 | Agent management: `tauriclaw agent status/stop/logs/list` | P0 | Planned |
| FR-14.9 | Memory CLI: `tauriclaw memory search/store/daily` | P1 | Planned |
| FR-14.10 | Config CLI: `tauriclaw config show/set` | P1 | Planned |
| FR-14.11 | Watch mode: `tauriclaw watch ./src --prompt "review changes"` | P2 | Planned |
| FR-14.12 | Agent chaining: pipe output of one agent into another | P2 | Planned |
| FR-14.13 | Terminal markdown rendering (bold, code blocks, colors) | P1 | Planned |
| FR-14.14 | Session resume: `tauriclaw --resume session-id` | P1 | Planned |

### FR-15: Gateway / Control Plane

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-15.1 | HTTP REST API at 127.0.0.1:18790 with bearer token auth | P0 | Planned |
| FR-15.2 | WebSocket streaming for real-time events (tokens, tools, approvals) | P0 | Planned |
| FR-15.3 | Localhost-only binding (never 0.0.0.0) | P0 | Planned |
| FR-15.4 | Auto-generated bearer token in ~/.tauriclaw/daemon.token | P0 | Planned |
| FR-15.5 | Port auto-increment (18790-18799) on conflict | P1 | Planned |
| FR-15.6 | PID file at ~/.tauriclaw/daemon.pid with port info | P0 | Planned |
| FR-15.7 | API versioning at /api/v1/ with 6-month overlap for breaking changes | P1 | Planned |
| FR-15.8 | OpenAPI spec auto-generated from route definitions | P2 | Planned |
| FR-15.9 | GUI uses gateway (not Tauri IPC) for agent/memory/provider operations | P0 | Planned |

### FR-16: CI/CD & Release Automation

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-16.1 | GitHub Actions CI: lint + test + format on every PR for all 3 desktop platforms | P0 | Planned |
| FR-16.2 | Reusable build workflow supporting 8 desktop platform configurations | P0 | Planned |
| FR-16.3 | Automated release pipeline: draft GitHub Release + parallel builds + artifact upload | P0 | Planned |
| FR-16.4 | macOS code signing and notarization in CI | P1 | Planned |
| FR-16.5 | Windows code signing via Azure Trusted Signing in CI | P1 | Planned |
| FR-16.6 | Tauri auto-updater signing (private/public keypair) | P1 | Planned |
| FR-16.7 | Mobile build pipeline: iOS (TestFlight) + Android (Play Console internal testing) | P2 | Planned |
| FR-16.8 | Dependabot: weekly Cargo + GitHub Actions dependency updates | P1 | Planned |
| FR-16.9 | Automatic PR labeling by changed files | P2 | Planned |
| FR-16.10 | Stale issue/PR management (auto-close after 60+14 days) | P2 | Planned |

### FR-17: Community & Contribution

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-17.1 | Issue templates: bug report (YAML form with severity, component, reproduction steps) | P1 | Planned |
| FR-17.2 | Issue templates: feature request (YAML form with problem statement, proposed solution, acceptance criteria) | P1 | Planned |
| FR-17.3 | Pull request template with validation evidence, security impact, rollback plan | P1 | Planned |
| FR-17.4 | CONTRIBUTING.md with setup instructions, risk-based collaboration tracks (A/B/C), naming conventions | P1 | Planned |
| FR-17.5 | SECURITY.md with responsible disclosure process and response SLAs (48h ack, 1w assess, 2w fix critical) | P0 | Planned |
| FR-17.6 | CODEOWNERS for automated review routing (security paths → maintainer) | P1 | Planned |
| FR-17.7 | CODE_OF_CONDUCT.md (Contributor Covenant v2.1) | P1 | Planned |
| FR-17.8 | Blank issues disabled — all issues via templates or security policy | P2 | Planned |

### FR-18: Sidecar Module System

| ID | Requirement | Priority | Status |
|----|-------------|----------|--------|
| FR-18.1 | `SidecarModule` trait extending Tool: module_type, runtime, health_check, start, stop | P1 | Planned |
| FR-18.2 | Three module types: SidecarTool (on-demand), SidecarService (long-lived), McpServer (MCP protocol) | P1 | Planned |
| FR-18.3 | TOML manifest format for module definition (`~/.tauriclaw/modules/{name}/manifest.toml`) | P1 | Planned |
| FR-18.4 | Stdin/Stdout JSON protocol for SidecarTool communication | P1 | Planned |
| FR-18.5 | MCP (JSON-RPC over stdin/stdout) protocol for McpServer modules | P1 | Planned |
| FR-18.6 | HTTP REST protocol for SidecarService communication | P2 | Planned |
| FR-18.7 | Container runtime abstraction: Docker and Podman support via `ContainerRuntime` trait | P2 | Planned |
| FR-18.8 | Auto-detection priority: Podman → Docker → native fallback | P2 | Planned |
| FR-18.9 | Module discovery: scan `~/.tauriclaw/modules/` on startup, register in ToolRegistry | P1 | Planned |
| FR-18.10 | Module security: `allowed_paths`, `network` flag, `timeout_seconds`, `max_memory_mb` per manifest | P1 | Planned |
| FR-18.11 | Module CLI: `tauriclaw module list/install/remove/start/stop/health/reload/create` | P2 | Planned |
| FR-18.12 | Module management UI in Settings | P3 | Planned |
| FR-18.13 | Composio.dev integration via MCP server manifest (zero custom code) | P2 | Planned |
| FR-18.14 | Cargo feature flags for modular compilation (`sidecars`, `containers`, `mcp-client`) | P1 | Planned |
| FR-18.15 | Module scaffolding: `tauriclaw module create <name> --type tool --runtime python` | P2 | Planned |

---

## 5. Non-Functional Requirements

### NFR-1: Performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1.1 | Binary size (release build) | < 15 MB (target: < 10 MB with `opt-level = "z"` + LTO + strip) |
| NFR-1.2 | RAM usage (idle) | < 50 MB |
| NFR-1.3 | RAM usage (active agent loop) | < 150 MB |
| NFR-1.4 | App startup time | < 2 seconds to interactive UI |
| NFR-1.5 | Time to first token (streaming) | < 2 seconds (excluding LLM latency) |
| NFR-1.6 | Memory search latency (10K entries) | < 100ms |

### NFR-2: Security

| ID | Requirement |
|----|-------------|
| NFR-2.1 | API keys never written to disk (OS keyring only) |
| NFR-2.2 | API keys zeroized in memory after use |
| NFR-2.3 | No telemetry, analytics, or phone-home behavior |
| NFR-2.4 | All tool executions auditable via log |
| NFR-2.5 | `unsafe` code denied via Clippy lint |

### NFR-3: Reliability

| ID | Requirement |
|----|-------------|
| NFR-3.1 | LLM call failures handled gracefully (retry + fallback, not crash) |
| NFR-3.2 | Config file corruption handled (atomic saves, backup on write) |
| NFR-3.3 | SQLite WAL mode for crash recovery |
| NFR-3.4 | Agent loop has max iteration limit (prevent infinite loops) |

### NFR-4: Code Quality

| ID | Requirement |
|----|-------------|
| NFR-4.1 | Backend: < 5,000 lines of Rust after slim-down (from current 9,152) |
| NFR-4.2 | Backend: `unsafe_code = "deny"` in Cargo lints |
| NFR-4.3 | Backend: `unwrap_used = "warn"`, escalate to "deny" after cleanup |
| NFR-4.4 | Frontend: Ultracite (Biome) formatting enforced |
| NFR-4.5 | Backend: 120+ unit tests maintained and passing |

### NFR-5: Cross-Platform & Distribution

| ID | Requirement |
|----|-------------|
| NFR-5.1 | macOS: Universal binary (Apple Silicon aarch64 + Intel x86_64) as DMG installer |
| NFR-5.2 | macOS: Notarized and stapled for Gatekeeper |
| NFR-5.3 | Windows x64: MSI + NSIS installer, Azure Trusted Signing |
| NFR-5.4 | Windows ARM64: MSI + NSIS installer (native ARM, not emulated x86) |
| NFR-5.5 | Linux x64: AppImage (Ubuntu 24.04+), .deb (Ubuntu 22.04+), .rpm (Fedora) |
| NFR-5.6 | Linux ARM64: .deb + AppImage (Raspberry Pi, AWS Graviton) |
| NFR-5.7 | iOS: arm64 (all modern iPhones/iPads). Simulator: arm64 + x86_64 |
| NFR-5.8 | Android: arm64-v8a (primary), armeabi-v7a (legacy 32-bit), x86_64 (emulator) |
| NFR-5.9 | All desktop platforms: same feature set, same data format |
| NFR-5.10 | Mobile platforms: same frontend, adapted for touch/gestures/push notifications |
| NFR-5.11 | Tauri auto-updater with signed update manifests on all desktop platforms |
| NFR-5.12 | 32-bit x86 NOT supported (all modern OSes are 64-bit; not worth the build matrix complexity) |

#### Platform Architecture Summary

| OS | Architectures | Bundle Types | Code Signing |
|----|---------------|--------------|-------------|
| macOS | x86_64 + aarch64 (Universal) | DMG, APP | Apple notarization |
| Windows | x86_64, aarch64 | MSI, NSIS EXE | Azure Trusted Signing |
| Linux | x86_64, aarch64 | AppImage, DEB, RPM | None (GPG optional) |
| iOS | arm64 | IPA (TestFlight/App Store) | Apple distribution cert |
| Android | arm64-v8a, armeabi-v7a, x86_64 | AAB (Play Store), APK | Keystore signing |

---

## 6. Out of Scope (Explicitly Excluded)

| Item | Reason |
|------|--------|
| Community skill marketplace | Adds complexity without core value; revisit after v1.0 |
| pgvector / PostgreSQL backend | Desktop app — SQLite is sufficient and simpler |
| Full container orchestration (Kubernetes, Docker Compose) | Sidecar containers are tool runtimes, not orchestration platforms. Docker/Podman used only for isolated script execution |
| Multi-user / shared access | Single-user desktop app by design |
| Browser extension | Focus on native desktop experience |
| Real-time collaboration | Single-user app |
| WhatsApp/Discord/Slack channels | v1.0 ships Telegram only; others in v1.1+ |
| App Store / Play Store **public** distribution | v1.2 ships to TestFlight and Play Console internal testing; public store submission is post-v1.2 |

---

## 7. Success Metrics

| Metric | Target | How Measured |
|--------|--------|-------------|
| Setup-to-first-chat time | < 3 minutes | User testing |
| Binary size (release) | < 10 MB | CI build output |
| Backend LOC after slim-down | < 5,000 lines | `tokei` or `cloc` |
| Unit test count | > 150 (from current 120) | `cargo test --lib` |
| Agent loop: successful 5-step task | > 80% completion rate | Manual QA testing |
| Memory recall accuracy | > 70% relevant results in top 5 | Evaluation dataset |
| Heartbeat reliability | > 99% on-time execution | Log analysis |

---

## 8. Dependencies & Risks

### External Dependencies

| Dependency | Risk | Mitigation |
|------------|------|------------|
| LLM API availability | Provider outage breaks agent | ReliableProvider fallback chain (FR-1.6) |
| Ollama availability | Local model not running | Graceful detection + guidance to start Ollama |
| OS keyring access | Keyring locked or unavailable | Fallback to Stronghold encrypted storage |
| SQLite | Data corruption | WAL mode + atomic saves + backups |
| Container runtime (Docker/Podman) | Container not installed | Graceful fallback to native process execution with warning |
| MCP server availability | MCP server crashes or stalls | Health monitoring, auto-restart, timeout enforcement |

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Diesel → rusqlite migration breaks existing data | Medium | High | Defer to last; test with data migration script |
| Agent loop runaway (infinite tool calls) | Medium | Medium | Max iteration limit + rate limiting + cancel button |
| Memory search quality too low | Low | Medium | Tune hybrid weights, add evaluation dataset |
| Event Bus message ordering issues | Low | Low | Use sequence numbers, test concurrency |

---

## 9. Release Plan

| Release | Contents | Target |
|---------|----------|--------|
| **v0.5** (Slim Down) | S1-S4: Consolidate providers, simplify skills, strict linting. **Responsive layout foundation** (FR-13.1-13.5) | Phase 0 |
| **v0.6** (Foundation) | P0.1-P0.2: ReliableProvider, release profile optimization | Phase 1 |
| **v0.7** (Core Agent) | P1.4, P1.6, P1.8, P1.10: Tools, security, event bus, identity. **CI/CD pipeline** (FR-16.1-16.3). **Module system core** (FR-18.1-18.10, FR-18.14) | Phase 2 |
| **v0.8** (Intelligence) | P0.3, P1.5, P2.17: Agent loop, memory system, daily memory. **MCP client, container runtime** (FR-18.5, FR-18.7-18.8) | Phase 3 |
| **v0.9** (Proactive) | P1.9, P2.15, P2.16: Scheduler, notifications, sessions. **Telegram channel** (P7.1) | Phase 4 |
| **v1.0** (Complete) | P2.11-P2.14, P3.*: Config, routing, UI polish, boot sequence, channel management UI. **Contribution infrastructure** (FR-17). **Module management UI** (FR-18.11-18.12, FR-18.15) | Phase 5-6 |
| **v1.1** (Channels+) | WhatsApp, Discord channels. Mobile-specific polish (safe areas, gestures). **Code signing** (FR-16.4-16.6) | Phase 7 |
| **v1.2** (Mobile) | Tauri Mobile builds (iOS/Android/Tablets). Push notifications. TestFlight + Play Console. **Mobile CI** (FR-16.7) | Phase 7 |

---

*Document created: February 2026*
*References: docs/claw-ecosystem-analysis.md, docs/tauriclaw-gap-analysis.md*
