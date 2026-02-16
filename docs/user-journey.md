# TauriClaw User Journey

> Maps the end-to-end user experience from first launch to daily power usage.
> Reference: `docs/claw-ecosystem-analysis.md`, `docs/tauriclaw-gap-analysis.md`

---

## Journey Overview

```
CLI Install ──▶ First Command ──▶ REPL Sessions ──▶ Pipe/Watch ──▶ Power Use
     │                                                                  │
     └──▶ GUI Launch ──▶ Setup Wizard ──▶ Chat ──▶ Agent ──▶ Daily Use ─┤
                        (non-developer path)                             │
                                                                         ├──▶ Channels (Telegram)
                                                                         ├──▶ Mobile & Tablet
                                                                         ├──▶ Sidecar Modules (Python, Node.js, MCP)
                                                                         └──▶ Contributing (Issues, PRs)
```

---

## Stage 0: CLI Quick Start (Developer Path)

**User Goal**: Install and start using TauriClaw from the terminal immediately.

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 0.1 | `cargo install tauriclaw` or downloads binary | Single binary installed to PATH | Release binary |
| 0.2 | `tauriclaw` | Daemon starts, interactive REPL opens. Prompts for provider + API key on first run. | CLI (clap + rustyline) |
| 0.3 | `tauriclaw "explain this codebase"` | One-shot: agent analyzes, streams result, exits | Gateway + Agent Loop |
| 0.4 | `cat schema.sql \| tauriclaw "review this"` | Piped input as context, agent responds | Pipe support |
| 0.5 | `tauriclaw gui` | Launches Tauri desktop app (connects to running daemon) | Tauri GUI |

**Success Criteria**: Developer goes from install to first agent response in < 60 seconds.

**Two paths from here**:
- **Developers**: Stay in CLI. Use REPL, pipe, watch mode. Stages 2-6 via terminal.
- **Non-developers**: Run `tauriclaw gui` once. Use GUI exclusively. Same agent, same memory.

---

## Stage 1: First Launch & Onboarding

**User Goal**: Install TauriClaw and get it running with minimal friction.

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 1.1 | Downloads installer (DMG/MSI/AppImage) | Platform-native installer runs | Tauri bundler |
| 1.2 | Launches app for first time | Boot sequence executes: loads defaults, creates `~/.tauriclaw/` directory structure | Boot Sequence (P3.23) |
| 1.3 | Sees welcome screen | App presents setup wizard — no blank slate | Frontend: onboarding route |
| 1.4 | Prompted: "Choose your AI provider" | Dropdown: OpenAI, Anthropic, Google AI, Groq, Ollama (local), OpenRouter | Provider config UI |
| 1.5 | Enters API key | Key stored in OS keyring (macOS Keychain / Windows Credential Manager / Linux Secret Service). Never touches disk. | `credential_store.rs` |
| 1.6 | Optionally names the agent | Creates `IDENTITY.md` with agent name, writes `SOUL.md` from selected template | Identity Files (P1.10) |
| 1.7 | Setup complete | App transitions to main chat interface. Provider warmup runs in background. | `ReliableProvider.warmup()` |

**Success Criteria**: User goes from download to first chat in < 3 minutes.

**Failure Recovery**: If API key is invalid, inline error with "Test Connection" button. If Ollama selected but not installed, link to Ollama download page.

---

## Stage 2: First Conversation

**User Goal**: Ask a question and get a useful response. Build trust in the tool.

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 2.1 | Types a message in chat input | Message captured by PromptInput component | Frontend: AI Elements |
| 2.2 | Presses Enter / Send | Message sent via Tauri IPC to backend | `commands/streaming_chat.rs` |
| 2.3 | Sees streaming response | Tokens stream back via Tauri events, rendered incrementally in Conversation component | SSE streaming |
| 2.4 | Response completes | Full message rendered with markdown formatting, code blocks with syntax highlighting | AI Elements: Message, CodeBlock |
| 2.5 | Asks follow-up | Conversation history maintained in session. Context carries forward. | Session state |

**Key Metric**: Time to first token < 2 seconds (excluding cold-start LLM latency).

**What's different from ChatGPT/Claude web**: This is a local desktop app — no browser tab management, native OS integration, data stays on device.

---

## Stage 3: Discovering the Agent Loop

**User Goal**: Graduate from single-turn Q&A to autonomous multi-step task execution.

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 3.1 | Asks a complex question: "Analyze my project's database schema" | Agent loop activates — LLM decides it needs tools | Agent Loop (P0.3) |
| 3.2 | Sees tool execution indicator | UI shows: "Running: `read_file schema.sql`" with spinner | Agent Loop UI (P3.22) |
| 3.3 | Security prompt appears (first time) | "Agent wants to read `/path/to/schema.sql` — Allow?" with Always/Once/Deny options | Security Policy (P1.6) |
| 3.4 | Approves action | Tool executes, result fed back to LLM, agent continues reasoning | Tool Registry (P1.4) |
| 3.5 | Agent runs 3-4 tool calls autonomously | Progress indicator shows each step. User can watch or minimize to system tray. | Event Bus (P1.8) |
| 3.6 | Agent completes | Final analysis presented with structured output. Memory stores key findings. | Memory System (P1.5) |

**Trust Building**: The approval overlay (step 3.3) is critical. Users must feel in control before they trust autonomous execution. The three autonomy levels (ReadOnly → Supervised → Full) let users gradually increase trust.

**Autonomy Progression**:
```
Week 1:  ReadOnly     — Agent can only read files, user approves everything
Week 2:  Supervised   — Agent executes low-risk commands, asks for medium/high
Month 2: Full         — Agent operates freely with rate limiting
```

---

## Stage 4: Personalizing the Agent

**User Goal**: Make the agent feel like *their* assistant — personalized, context-aware, persistent.

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 4.1 | Opens Settings → Identity | Sees markdown editor for SOUL.md, USER.md, AGENTS.md | Identity Editor UI (P3.25) |
| 4.2 | Edits SOUL.md: "You are concise and technical. Never use emojis." | Agent personality changes immediately (hot-reload) | Identity Loader (P1.10) |
| 4.3 | Edits USER.md: "I'm a backend developer. I work with Rust and PostgreSQL." | Agent tailors responses to user's expertise level | Identity Loader (P1.10) |
| 4.4 | Notices agent remembers yesterday's conversation | Daily memory files (`~/.tauriclaw/memory/2026-02-16.md`) persist key facts | Daily Memory (P2.17) |
| 4.5 | Searches past knowledge: "What did we discuss about indexing?" | Memory search returns relevant past interactions with hybrid vector + keyword scoring | Memory Search UI (P3.21) |

**Key Insight**: Identity files are plain markdown. Power users edit them in their text editor of choice. The UI is a convenience, not a requirement.

---

## Stage 5: Proactive Agent Behavior

**User Goal**: Agent works for the user even when they're not actively chatting.

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 5.1 | Configures heartbeat: "Check my GitHub notifications every 30 minutes" | Creates entry in HEARTBEAT.md, scheduler starts periodic job | Scheduler (P1.9) |
| 5.2 | Minimizes app to system tray | App continues running in background. Tray icon shows status. | System Tray |
| 5.3 | Receives desktop notification: "3 new GitHub notifications, 1 requires review" | Notification via Tauri plugin. Click opens relevant chat session. | Notifications (P2.15) |
| 5.4 | Sets up cron job: "Every Monday at 9am, summarize my week" | Cron expression `0 9 * * MON` created. Runs in isolated session. | Scheduler (P1.9) |
| 5.5 | Receives Monday morning summary | Agent ran in isolated session, produced summary, sent notification | Session Routing (P2.16) |

**Desktop Advantage**: Unlike server-based assistants (OpenClaw), TauriClaw uses native OS notifications, system tray, and runs without a browser. The agent feels like a native desktop companion.

---

## Stage 6: Daily Power Usage

**User Goal**: TauriClaw is an indispensable part of the daily workflow.

```
Morning Boot                     Active Work                    Background
┌──────────────┐    ┌──────────────────────────┐    ┌────────────────────┐
│ App launches  │    │ Multi-turn agent sessions │    │ Heartbeat checks   │
│ Boot sequence │───▶│ Tool execution            │───▶│ Cron jobs          │
│ Daily memory  │    │ Memory accumulation       │    │ Desktop notifs     │
│ Provider warm │    │ Identity-aware responses  │    │ Memory archival    │
└──────────────┘    └──────────────────────────┘    └────────────────────┘
```

### Typical Daily Flow

| Time | Activity | Modules Involved |
|------|----------|-----------------|
| **9:00 AM** | App launches. Boot sequence loads identity, warms providers, starts scheduler. Agent greets user with morning briefing from heartbeat results. | Boot, Identity, Scheduler, Notifications |
| **9:15 AM** | User asks agent to analyze a new codebase. Agent runs 10+ tool calls autonomously, reads files, builds understanding. | Agent Loop, Tools, Security, Memory |
| **10:00 AM** | User has follow-up questions. Agent recalls earlier analysis from memory without re-reading files. | Memory (recall), Session |
| **12:00 PM** | Background: heartbeat runs, checks email/calendar/GitHub. No user interaction needed. | Scheduler, Heartbeat, Event Bus |
| **12:01 PM** | Desktop notification: "PR #42 has merge conflicts" | Notifications |
| **2:00 PM** | User asks agent to help fix the merge conflict. Agent has context from morning analysis + notification. | Agent Loop, Memory, Tools |
| **5:00 PM** | End of day. Agent auto-summarizes the day's work into `memory/2026-02-16.md`. | Daily Memory |
| **5:01 PM** | App minimized to tray. Heartbeat continues on schedule. | Scheduler, System Tray |

---

## User Personas

### Persona 1: Solo Developer ("Alex")
- **Goal**: AI coding assistant that knows their codebase
- **Journey**: Setup → Chat → Agent Loop (code analysis) → Memory (codebase knowledge)
- **Key features**: Agent loop, tool execution, memory persistence
- **Autonomy level**: Supervised → Full within 2 weeks

### Persona 2: Team Lead ("Jordan")
- **Goal**: Proactive monitoring and summarization
- **Journey**: Setup → Heartbeat config → Cron jobs → Daily briefings
- **Key features**: Scheduler, notifications, daily memory
- **Autonomy level**: Supervised (prefers to approve important actions)

### Persona 3: AI Enthusiast ("Sam")
- **Goal**: Experiment with different LLMs and agent behaviors
- **Journey**: Setup multiple providers → Identity customization → Tool creation
- **Key features**: Multi-provider routing, identity files, extensibility
- **Autonomy level**: Full (comfortable with autonomous agents)

---

## Journey Dependencies (Module Map)

```
Stage 1 (Onboarding)
  ├── Existing: Provider UI, Credential Store, Frontend
  └── New: Boot Sequence, Identity Files (defaults)

Stage 2 (First Chat)
  ├── Existing: Streaming Chat, AI Elements, Session State
  └── New: ReliableProvider (retry on failure)

Stage 3 (Agent Loop)
  ├── New: Agent Loop, Tool Trait + Registry, Security Policy
  ├── New: Event Bus (connects subsystems)
  └── New: Agent Loop UI (tool status, approval overlay)

Stage 4 (Personalization)
  ├── New: Identity Files (SOUL.md, USER.md, etc.)
  ├── New: Memory System (vector + keyword search)
  ├── New: Daily Memory Files
  └── New: Identity Editor UI, Memory Search UI

Stage 5 (Proactive)
  ├── New: Scheduler (heartbeat + cron)
  ├── New: Desktop Notifications
  ├── New: Session Routing (isolated sessions for jobs)
  └── New: System Tray integration

Stage 6 (Daily Power Use)
  └── All of the above working together

Stage 7 (Channels)
  ├── New: TelegramChannel (long-polling + bot API)
  ├── New: WhatsAppChannel (WhatsApp Web protocol, future)
  ├── Depends on: Channel Trait (P1.7), Agent Loop, Security, Session Routing
  └── Desktop remains approval authority for all channels

Stage 8 (Mobile & Tablet)
  ├── Responsive frontend (built from Phase 0.5)
  ├── Tauri Mobile compilation (iOS/Android)
  ├── Touch gestures and haptic feedback
  ├── Mobile push notifications via APNs/FCM (Phase 7.4)
  └── Mobile lifecycle management (background tasks, foreground service)

Stage 9 (Contributing)
  ├── New: Issue templates (bug report, feature request)
  ├── New: PR template with validation/security/rollback
  ├── New: CONTRIBUTING.md (setup, conventions, tracks)
  ├── New: SECURITY.md, CODEOWNERS, CODE_OF_CONDUCT.md
  ├── New: GitHub Actions CI/CD (lint, test, build, release)
  └── New: Auto-labeling, dependabot, stale management

Stage 10 (Sidecar Modules)
  ├── New: Module system core (SidecarModule trait, ModuleRegistry, manifest parser)
  ├── New: SidecarTool (stdin/stdout JSON protocol, process spawning)
  ├── New: SidecarService (long-lived HTTP services)
  ├── New: McpServer (MCP JSON-RPC client, tool discovery)
  ├── New: ContainerRuntime (Docker/Podman abstraction, auto-detection)
  ├── Depends on: Tool Registry (Phase 2.2), SecurityPolicy, EventBus
  └── Enables: Composio integration, custom Python/Node.js tools, containerized execution
```

---

## Stage 7: Messaging Channel Integration

**User Goal**: Interact with the agent from messaging apps — not just the desktop UI.

### 7a: Telegram Setup

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 7a.1 | Opens Settings → Channels → Telegram | Sees setup instructions: "Create a Telegram bot via @BotFather" | Channel Config UI |
| 7a.2 | Creates bot via BotFather, copies token | Pastes bot token into TauriClaw. Token stored in OS keyring | Credential Store |
| 7a.3 | Clicks "Connect" | App starts Telegram long-polling listener in background Tokio task. Status indicator shows "Connected" | TelegramChannel (P7.1) |
| 7a.4 | Sends message to bot in Telegram | Message received → routed to agent loop via EventBus → agent responds → response sent back to Telegram | Channel Manager, Agent Loop |
| 7a.5 | Receives agent response in Telegram | Markdown-formatted response in Telegram. Code blocks, links preserved. | TelegramChannel.send() |
| 7a.6 | Agent triggers tool execution | If Supervised mode: approval notification appears on desktop. User approves from desktop app. Agent completes in Telegram. | Security Policy, Notifications |

### 7b: WhatsApp Setup (Future)

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 7b.1 | Opens Settings → Channels → WhatsApp | Sees QR code for WhatsApp Web pairing | WhatsAppChannel |
| 7b.2 | Scans QR code with phone | Connection established via WhatsApp Web protocol | Channel Manager |
| 7b.3 | Sends message to agent in WhatsApp | Same agent loop flow as Telegram — message → agent → tools → response | Agent Loop |

### Channel Interaction Model

```
                    ┌──────────────┐
                    │  Agent Loop  │
                    │  (single)    │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
       ┌──────▼──────┐ ┌──▼──────┐ ┌──▼────────────┐
       │ Desktop UI  │ │Telegram │ │ WhatsApp       │
       │ (Tauri IPC) │ │ (bot)   │ │ (future)       │
       └─────────────┘ └─────────┘ └────────────────┘
```

**Key Design Decision**: All channels feed into the **same agent loop and memory system**. The agent has one brain, multiple mouths. A conversation started in Telegram can reference memory from a desktop session. Approvals always route to the desktop app (single point of trust).

### Channel-Specific Considerations

| Aspect | Desktop | Telegram | WhatsApp |
|--------|---------|----------|----------|
| Rich formatting | Full HTML/CSS | Markdown subset | Basic formatting |
| Code blocks | Syntax highlighted | Monospace blocks | Plain text |
| File sharing | Native file dialog | Telegram file API | WhatsApp media API |
| Approval UI | Overlay dialog | Desktop notification | Desktop notification |
| Session scope | `main:dm:tauri:user` | `main:dm:telegram:{chat_id}` | `main:dm:whatsapp:{phone}` |
| Rate limiting | App-level | Bot API limits (30 msg/sec) | WhatsApp API limits |
| Startup | Always available | Requires bot token | Requires QR pairing |

---

## Stage 8: Mobile & Tablet Usage (Tauri Mobile)

**User Goal**: Access the same agent from phone or tablet when away from desktop. Same codebase, same UI, no code adjustments.

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 8.1 | Installs TauriClaw from TestFlight (iOS) or Play Console (Android) | Same React frontend compiled via Tauri Mobile. No code changes from desktop — responsive layout adapts automatically | Tauri Mobile + Responsive Layout |
| 8.2 | Opens app on phone | Single-column layout activates (< 640px). Bottom navigation bar visible. Touch targets already 44x44px. | Responsive Breakpoints (Phase 0.5) |
| 8.3 | Opens app on tablet (iPad / Android tablet) | 2-column layout activates (768-1024px). Collapsible sidebar. Works in split-view / multitasking | Responsive Breakpoints |
| 8.4 | Chats with agent | Same streaming responses. Virtual keyboard handling adjusts chat area. Safe area insets for notch/home indicator | Mobile Polish (Phase 7.3) |
| 8.5 | Swipes right to open sidebar | Touch gestures for navigation. Haptic feedback on interactions | Gesture System |
| 8.6 | Receives push notification | iOS: APNs. Android: FCM. Same heartbeat/cron/approval events as desktop notifications | Mobile Push (Phase 7.4) |
| 8.7 | App goes to background | iOS: 30s background task to save state. Android: foreground service maintains daemon connection | Mobile Lifecycle (Phase 7.4) |
| 8.8 | Rotates tablet to landscape | Layout adapts: sidebar becomes persistent. 2-column or 3-column depending on width | Responsive Layout |

**Key principle**: Zero code adjustments for mobile. The responsive layout foundation (Phase 0.5) + mobile polish (Phase 7.3) ensure the same React frontend works across all form factors. Tauri Mobile compiles the same WebView code to native iOS/Android containers.

**Responsive Design — Single Codebase**:
- **Desktop** (>1024px): Full sidebar + chat + detail panel (3-column)
- **Tablet landscape** (>1024px): Same as desktop
- **Tablet portrait** (768-1024px): Collapsible sidebar + chat (2-column)
- **Phone** (<768px): Single-column, bottom nav, full-screen chat
- **Phone small** (<640px): Compact single-column, reduced spacing

**Supported devices**:
- iPhone (all modern, arm64)
- iPad (all sizes, split-view / Stage Manager supported)
- Android phones (arm64-v8a primary, armeabi-v7a legacy)
- Android tablets (Samsung Galaxy Tab, Pixel Tablet, etc.)

---

## Stage 9: Contributing to TauriClaw

**User Goal**: Contribute improvements, report bugs, or request features.

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 9.1 | Finds a bug | Opens issue using bug report template (severity, component, repro steps) | `.github/ISSUE_TEMPLATE/bug_report.yml` |
| 9.2 | Has a feature idea | Opens issue using feature request template (problem statement, proposed solution, acceptance criteria) | `.github/ISSUE_TEMPLATE/feature_request.yml` |
| 9.3 | Wants to contribute code | Reads CONTRIBUTING.md: setup, conventions, risk-based PR tracks | `CONTRIBUTING.md` |
| 9.4 | Opens a PR | PR template auto-fills. Sections: summary, validation evidence, security impact, rollback plan | `.github/pull_request_template.md` |
| 9.5 | CI runs automatically | Lint (clippy + ultracite) + tests (cargo test + vitest) + format check on all platforms | GitHub Actions CI |
| 9.6 | Auto-labeling applied | PR labeled by changed files: `core`, `frontend`, `security`, `ai`, `docs`, etc. | `.github/labeler.yml` |
| 9.7 | Review assigned | Risk-based: A (docs/tests, 1 reviewer), B (providers/channels, 1+ test), C (security/runtime, 2-pass) | `CODEOWNERS` |
| 9.8 | PR merged | Squash merge with conventional commit title. Release pipeline picks up changes. | GitHub Actions Release |

## Stage 10: Extending with Sidecar Modules

**User Goal**: Extend the agent's capabilities with custom Python/Node.js scripts, container-based tools, or MCP-compatible tool servers.

### 10a: Adding a Python Sidecar Tool

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 10a.1 | `tauriclaw module create python-analyst --type tool --runtime python` | Scaffolds `~/.tauriclaw/modules/python-analyst/` with `manifest.toml` template and `main.py` stub | Module CLI |
| 10a.2 | Edits `main.py` with custom analysis logic | Python script reads stdin JSON, processes, writes stdout JSON | User code |
| 10a.3 | `tauriclaw module reload` | Module system re-scans modules directory, discovers new module, registers tool in ToolRegistry | ModuleRegistry |
| 10a.4 | `tauriclaw "analyze sales.csv using python-analyst"` | Agent calls `python-analyst` tool → spawns Python process → sends request → receives result → presents to user | SidecarTool + Agent Loop |

### 10b: Adding a Container-Based Tool

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 10b.1 | Edits `manifest.toml`: sets `runtime.type = "docker"`, `runtime.image = "python:3.12-slim"` | Module will run inside a container with volume mounts, memory limits, network isolation | Manifest |
| 10b.2 | `tauriclaw module reload` | Container runtime auto-detected (Podman or Docker). Image pulled if needed | ContainerRuntime |
| 10b.3 | Agent calls the tool | Container spawned → request sent → response received → container stopped. Isolated execution with no host filesystem access beyond declared volumes | ContainerRuntime + SidecarTool |

### 10c: Adding Composio MCP Tools

| Step | User Action | System Response | Key Module |
|------|-------------|-----------------|------------|
| 10c.1 | `npm install -g composio-core` | Composio CLI installed globally | User setup |
| 10c.2 | Creates `~/.tauriclaw/modules/composio/manifest.toml` with `type = "mcp"` and `command = "npx composio-core mcp-server"` | Manifest defines MCP server module | Manifest |
| 10c.3 | `tauriclaw module reload` | MCP client starts Composio server → sends `tools/list` → discovers 500+ tools → registers selected tools in ToolRegistry | McpServer + MCP Client |
| 10c.4 | `tauriclaw "send an email via Gmail about today's summary"` | Agent calls `mcp:composio:gmail_send` → MCP client sends `tools/call` → Composio handles OAuth + API call → email sent | MCP Client + Agent Loop |

### Module Extension Model

```
                    ┌──────────────┐
                    │  Agent Loop  │
                    └──────┬───────┘
                           │ calls tools by name
              ┌────────────┼────────────────────────┐
              │            │                         │
       ┌──────▼──────┐ ┌──▼──────────┐ ┌───────────▼───────┐
       │ Built-in    │ │ Sidecar     │ │ MCP Server        │
       │ Tools       │ │ Modules     │ │ Modules           │
       │ (Rust)      │ │ (Python,    │ │ (Composio, custom │
       │             │ │  Node.js)   │ │  MCP servers)     │
       └─────────────┘ └─────────────┘ └───────────────────┘
```

**Key principle**: The agent doesn't know or care whether a tool is built-in Rust, a Python sidecar, a containerized script, or an MCP server. The ToolRegistry abstracts all module types behind the same `Tool` trait interface.

---

*Document created: February 2026*
*References: docs/claw-ecosystem-analysis.md, docs/tauriclaw-gap-analysis.md*
