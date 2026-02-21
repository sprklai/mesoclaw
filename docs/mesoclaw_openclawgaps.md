# MesoClaw vs OpenClaw: Gap Analysis & Implementation Plan

**Document Version:** 1.3
**Last Updated:** 2026-02-21
**Status:** Gap Analysis Phases A, B, C Complete; Web Tools Implemented

---

## Executive Summary

This document provides a comprehensive gap analysis between MesoClaw (Tauri-based AI desktop app) and OpenClaw (Node.js AI gateway framework). Based on research from OpenClaw's documentation and MesoClaw's current implementation status (Phases 0-8 complete), we identify remaining feature gaps and provide actionable implementation tasks.

### Architecture Comparison

| Aspect | OpenClaw | MesoClaw | Gap Level |
|--------|----------|----------|-----------|
| **Runtime** | Node.js + TypeScript | Rust 2024 + Tauri 2 | Different stacks (OK) |
| **Communication** | WebSocket (port 18789) | HTTP REST + WS (port 18790) | Similar ✅ |
| **Database** | SQLite + sqlite-vec | SQLite + vector BLOBs | Similar ✅ |
| **Config Format** | JSON5 with Zod schemas | TOML + env overrides | Minor difference |
| **Agent Runtime** | runEmbeddedPiAgent | AgentLoop (orchestrator.rs) | Similar ✅ |
| **Tool Isolation** | Docker sandbox (modes/scopes) | ContainerRuntime trait | Partial ✅ |
| **Memory** | Hybrid vector+BM25 | Hybrid search with FTS5 | Similar ✅ |

### Current Implementation Status

Based on `docs/implementation-plan.md`, MesoClaw has completed:

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 0 | Slim Down + Responsive | ✅ Complete |
| Phase 1 | Foundation (Release Profile, ReliableProvider) | ✅ Complete |
| Phase 2 | Core Infrastructure (EventBus, Tools, Security, Identity, Gateway, Modules, Containers, MCP) | ✅ Complete |
| Phase 3 | Agent Intelligence (Agent Loop, Memory, Daily Memory) | ✅ Complete |
| Phase 4 | Proactive Behavior (Scheduler, Notifications, Sessions, Sidecar Services) | ✅ Complete |
| Phase 5 | Config & DX (TOML Config, Model Router, Prelude, Parser Hardening, Module CLI, Plugin Baseline) | ✅ Complete |
| Phase 6 | Extensions & UI (Channels, Boot Sequence, Frontend UIs, Memory Hygiene, WASM Spike) | ✅ Complete |
| Phase 7 | Channels & Mobile (Telegram, Channel UI, Mobile Polish, Mobile Builds) | ✅ Complete |
| Phase 8 | CI/CD & Community (GitHub Actions, Release Pipeline, Mobile Pipeline, Workflows, Contribution) | ✅ Complete |

### Gap Priority Matrix (Remaining Items)

| Priority | Gap Area | Effort | Impact | Status |
|----------|----------|--------|--------|--------|
| P1 High | Tool Sandbox Isolation | Medium | Security | ✅ **COMPLETE** - SandboxManager wired to AgentLoop |
| P1 High | Semantic Memory Enhancements | Medium | Core Feature | ✅ **COMPLETE** - sqlite-vec evaluated, deferred |
| P1 High | Additional Tools (apply_patch, etc.) | Low | Flexibility | ✅ **COMPLETE** - PatchTool, ProcessTool, CronTool, SessionSpawnTool |
| P1 High | Tool Profile Abstraction | Low | Flexibility | ✅ **COMPLETE** - ToolProfile/ToolGroup enums |
| P2 Medium | Hot-Reload Config (JSON5) | Low | Developer UX | TOML hot-reload exists |
| P2 Medium | Additional Channels (WhatsApp direct) | Medium | Platform Coverage | Matrix bridge available |
| P2 Medium | Voice Interaction (STT/TTS) | High | Accessibility | Not implemented |
| P3 Low | Mobile App Distribution | High | Platform Reach | Tauri Mobile targets exist |
| P3 Low | Extension Marketplace | High | Ecosystem | npm plugin pattern different |

---

## 1. What MesoClaw Already Has (vs OpenClaw)

### 1.1 Core Architecture — Parity Achieved ✅

| Feature | OpenClaw | MesoClaw | Notes |
|---------|----------|----------|-------|
| **Gateway/Control Plane** | WebSocket server (18789) | HTTP REST + WebSocket (18790) | MesoClaw has both |
| **Event System** | Event emitter | EventBus (tokio broadcast) | Similar patterns |
| **Configuration** | JSON5 + Zod | TOML + env overrides | Both support hot-reload |
| **Agent Loop** | runEmbeddedPiAgent | AgentLoop (loop_.rs) | Multi-turn with tools |
| **Tool System** | 25+ tools with profiles | ToolRegistry + built-in tools | Extensible pattern |
| **Memory System** | Hybrid vector+BM25 | SQLite + FTS5 + embeddings | Similar hybrid search |
| **Channel System** | 8+ platforms | 4 platforms (Telegram, Discord, Slack, Matrix) | Matrix bridges others |
| **Scheduler** | Cron + heartbeat | TokioScheduler + cron/interval | Both support heartbeat |
| **Security** | Autonomy levels | SecurityPolicy (6 layers) | Similar patterns |

### 1.2 Unique MesoClaw Advantages

| Feature | Advantage |
|---------|-----------|
| **CLI Binary** | Separate `mesoclaw` CLI with REPL (OpenClaw is gateway-only) |
| **Desktop Native** | Tauri provides native window, tray, notifications |
| **Mobile Ready** | Same codebase compiles to iOS/Android |
| **Rust Performance** | Native performance vs Node.js runtime |
| **OS Keyring** | Secure credential storage via keyring crate |
| **Responsive UI** | Mobile-first design from Phase 0 |

---

## 2. Identified Gaps

### 2.1 Sandbox System (P1 — ✅ COMPLETE)

**OpenClaw Features:**
- Sandbox modes: `off`, `non-main`, `all`
- Sandbox scopes: `session`, `agent`, `shared`
- Automatic container lifecycle
- Volume mounting, network isolation, resource limits

**MesoClaw Status:**
- ✅ `ContainerRuntime` trait implemented (Phase 2.9)
- ✅ `DockerRuntime` and `PodmanRuntime` implementations
- ✅ Auto-detection (Podman → Docker → native fallback)
- ✅ `SandboxMode` enum: `Off`, `NonMain` (default), `All`
- ✅ `SandboxConfig` struct with memory limits, network isolation, timeouts
- ✅ `SandboxManager` wraps ContainerRuntime for tool execution
- ✅ AgentLoop integration with feature-gated sandbox field
- ✅ Volume mounting and resource limits
- ✅ Conditional execution: sandboxed vs direct based on mode

**Implementation Files:**
- `src-tauri/src/config/schema.rs` — SandboxMode, SandboxConfig
- `src-tauri/src/modules/container/sandbox.rs` — SandboxManager
- `src-tauri/src/agent/loop_.rs` — AgentLoop integration

### 2.2 Semantic Memory Enhancements (P1 — ✅ EVALUATED)

**OpenClaw Features:**
- sqlite-vec extension for vector operations
- 400-token chunks with 80-token overlap
- BM25 via FTS5
- Hybrid ranking (0.7 vector + 0.3 BM25)
- Multiple memory sources (MEMORY.md, memory/*.md, session transcripts)

**MesoClaw Status:**
- ✅ SQLite backend with FTS5 virtual table
- ✅ Embedding generation (OpenAI/Ollama)
- ✅ Document chunker (`chunker.rs`)
- ✅ Hybrid search implementation
- ✅ Daily memory files (`daily.rs`)
- ✅ Memory hygiene (archive/purge)
- ✅ sqlite-vec evaluated (crate v0.1.7-alpha.10 available)
- ✅ Decision: Defer until memory scale >5000 entries (current BLOB approach adequate)

**sqlite-vec Research Summary:**
- Compatible with rusqlite 0.32 + bundled-full
- Integration via `sqlite3_auto_extension` (requires unsafe block)
- Expected 10-100x performance improvement for >5000 entries
- Prototype plan documented for future implementation

### 2.3 Tool System (P1 — ✅ COMPLETE)

**OpenClaw Tools:**
- `apply_patch` — Diff-based file editing
- `browser` — Web automation
- `canvas` — Visual workspace
- `nodes` — Graph operations
- `cron` — Agent-initiated scheduling
- `sessions_spawn` — Sub-agent creation
- `process` — Process management
- Tool profiles: minimal, coding, messaging, full
- Tool groups: runtime, fs, sessions, memory, web, ui

**MesoClaw Status:**
- ✅ Tool trait and ToolRegistry
- ✅ ShellTool, FileReadTool, FileWriteTool, FileListTool
- ✅ Memory tools (store, recall, forget)
- ✅ SecurityPolicy integration
- ✅ **PatchTool** — Diff-based file editing with `diffy` crate
- ✅ **ProcessTool** — Process listing (list) and management (kill)
- ✅ **CronTool** — Agent-initiated job scheduling with interval/cron support
- ✅ **SessionSpawnTool** — Sub-agent session spawning with depth tracking
- ✅ **WebFetchTool** — HTTP GET requests to fetch web content
- ✅ **WebRequestTool** — Generic HTTP requests (GET, POST, PUT, DELETE, etc.)
- ✅ **WebSearchTool** — Web search using DuckDuckGo (no API key required)
- ✅ **ToolProfile enum** — Minimal, Coding, Messaging, Full
- ✅ **ToolGroup enum** — Runtime, Fs, Sessions, Memory, Web, Ui
- ✅ **Profile resolution** — `is_tool_allowed()` for access control
- ✅ **ToolProfileEditor** — Frontend component for profile selection
- ❌ Missing: browser, canvas, nodes (specialized tools)

**Implementation Files:**
- `src-tauri/src/tools/patch.rs` — PatchTool
- `src-tauri/src/tools/process.rs` — ProcessTool
- `src-tauri/src/tools/cron.rs` — CronTool
- `src-tauri/src/tools/session_spawn.rs` — SessionSpawnTool
- `src-tauri/src/tools/web.rs` — WebFetchTool, WebRequestTool
- `src-tauri/src/tools/profiles.rs` — ToolProfile, ToolGroup
- `src-tauri/src/tools/registry.rs` — `list_filtered()` method
- `src/components/settings/ToolProfileEditor.tsx` — Frontend UI

### 2.4 Additional Channels (P2 — Minor Gap)

**OpenClaw Channels:**
- Built-in: WhatsApp, Telegram, Discord, Slack, Signal, iMessage, Email, Web
- Extensions: Matrix, Zalo, MS Teams

**MesoClaw Status:**
- ✅ Telegram (teloxide)
- ✅ Discord (serenity)
- ✅ Slack (slack-morphism)
- ✅ Matrix (matrix-sdk) — bridges WhatsApp, Signal, IRC, etc.
- ❌ No direct WhatsApp (use Matrix bridge)
- ❌ No Signal direct (use Matrix bridge)
- ❌ No iMessage, Email, Web widget

**Gap:**
Matrix bridge covers most platforms. Direct integrations are nice-to-have.

### 2.5 Voice Interaction (P2 — Not Implemented)

**OpenClaw Features:**
- "Hey Claw" wake word detection
- Push-to-talk mode
- Speech-to-text (Whisper)
- Text-to-speech responses
- Voice activity detection

**MesoClaw Status:**
- ❌ No voice input
- ❌ No speech synthesis
- ❌ No wake word detection

**Gap:**
Voice is not implemented but could be added with Whisper integration.

### 2.6 Mobile Distribution (P3 — Infrastructure Exists)

**OpenClaw Features:**
- iOS companion app
- Android companion app
- Push notifications (APNs/FCM)

**MesoClaw Status:**
- ✅ Tauri Mobile targets configured
- ✅ iOS and Android build pipelines
- ✅ Responsive mobile UI
- ✅ Mobile-specific features (gestures, keyboard handling)
- ❌ No TestFlight/Play Store distribution yet

**Gap:**
Technical capability exists, distribution is a deployment/operational task.

### 2.7 Extension System (P3 — Different Model)

**OpenClaw Features:**
- npm packages as extensions
- Plugin manifest in package.json
- npm registry discovery
- Automatic dependency resolution

**MesoClaw Status:**
- ✅ Module system with TOML manifests
- ✅ Sidecar tools and services
- ✅ MCP server support
- ✅ Container runtime for isolation
- ❌ No npm-based plugin discovery
- ❌ No marketplace ecosystem

**Gap:**
Different but equivalent extensibility model. npm ecosystem is not applicable to Rust.

---

## 3. Implementation Plan

### Phase A: Tool Sandbox Integration ✅ COMPLETE

**Goal:** Wire ContainerRuntime to tool execution for isolation.

**Completed Tasks:**

| # | Task | Status |
|---|------|--------|
| A.1 | Add sandbox_mode to AgentConfig | ✅ SandboxMode enum (Off, NonMain, All) |
| A.2 | Add sandbox_scope to SessionConfig | ✅ SandboxConfig struct |
| A.3 | Implement SandboxManager | ✅ Wraps ContainerRuntime, manages lifecycle |
| A.4 | Integrate with AgentLoop | ✅ Feature-gated sandbox field |
| A.5 | Add volume mounting | ✅ Supported in ContainerConfig |
| A.6 | Add resource limits | ✅ Memory, network, timeout |
| A.7 | Wire to SecurityPolicy | ✅ Conditional execution based on mode |
| A.8 | Write tests | ✅ 21 container tests, 11 agent loop tests |

**Commit:** `675dcb1` — feat(tools): add sandbox integration, tool profiles, and additional tools

### Phase B: Tool System Enhancements ✅ COMPLETE

**Goal:** Add missing tools and profile abstraction.

**Completed Tasks:**

| # | Task | Status |
|---|------|--------|
| B.1 | Implement apply_patch tool | ✅ PatchTool with diffy crate |
| B.2 | Implement sessions_spawn tool | ✅ SessionSpawnTool with depth tracking |
| B.3 | Implement process tool | ✅ ProcessTool (list/kill) |
| B.4 | Implement cron tool | ✅ CronTool with scheduler integration |
| B.5 | Add ToolProfile enum | ✅ Minimal, Coding, Messaging, Full |
| B.6 | Add ToolGroup enum | ✅ Runtime, Fs, Sessions, Memory, Web, Ui |
| B.7 | Implement profile resolution | ✅ is_tool_allowed() method |
| B.8 | Add profile config UI | ✅ ToolProfileEditor component |
| B.9 | Write tests | ✅ 58 tool-specific tests |

**Commit:** `675dcb1` — feat(tools): add sandbox integration, tool profiles, and additional tools

### Phase C: Memory Enhancements ✅ EVALUATED

**Goal:** Evaluate sqlite-vec integration for performance.

**Completed Tasks:**

| # | Task | Status |
|---|------|--------|
| C.1 | Research sqlite-vec for Rust | ✅ Evaluated crate v0.1.7-alpha.10 |
| C.2 | Benchmark current BLOB approach | ✅ Current approach adequate for <5000 entries |
| C.3 | Prototype sqlite-vec integration | ⏸️ Deferred until scale justifies |
| C.4 | Optimize embedding cache | ✅ LruEmbeddingCache exists |
| C.5 | Add memory source tracking | ⏸️ Future enhancement |
| C.6 | Write tests | ✅ Existing memory tests pass |

**Recommendation:** Defer sqlite-vec integration until memory scale exceeds 5000 entries. Current BLOB approach is functionally equivalent and performs adequately.

### Phase D: Voice Interaction (1 week)

**Goal:** Add speech-to-text and text-to-speech capabilities.

**Tasks:**

| # | Task | Details |
|---|------|---------|
| D.1 | Add Whisper integration | Use `whisper-rs` or OpenAI Whisper API |
| D.2 | Implement audio recording | Microphone access via Tauri |
| D.3 | Create voice input component | Push-to-talk button in chat UI |
| D.4 | Add TTS integration | Use `tts` crate or cloud TTS API |
| D.5 | Implement speech queue | Queue for multiple TTS outputs |
| D.6 | Add voice settings | Voice selection, speed, language |
| D.7 | Write tests | STT accuracy, TTS output |

**Files to create:**
- `src-tauri/src/voice/stt.rs` — speech-to-text
- `src-tauri/src/voice/tts.rs` — text-to-speech
- `src-tauri/src/voice/mod.rs` — voice module
- `src/components/chat/VoiceInput.tsx` — voice button component
- `src/components/settings/VoiceSettings.tsx` — voice configuration

### Phase E: Additional Channels (Optional, 3 days each)

**Goal:** Add direct channel integrations beyond Matrix bridge.

**Tasks for Email Channel:**

| # | Task | Details |
|---|------|---------|
| E.1 | Add SMTP/IMAP crates | `lettre` for SMTP, `imap` for receiving |
| E.2 | Implement EmailChannel | Implement Channel trait |
| E.3 | Add threading support | Group emails by thread |
| E.4 | Create config UI | SMTP/IMAP credentials form |
| E.5 | Write tests | Send, receive, threading |

**Tasks for Web Widget:**

| # | Task | Details |
|---|------|---------|
| E.6 | Design embeddable widget | JavaScript bundle for websites |
| E.7 | Add WebSocket endpoint | For widget communication |
| E.8 | Implement widget auth | Token-based authentication |
| E.9 | Create installation docs | Embed code, configuration |
| E.10 | Write tests | Widget connection, auth |

---

## 4. Recommended Implementation Order

Based on impact and effort:

### ~~Sprint 1: Security Enhancement (Week 1)~~ ✅ COMPLETE
1. **Tool Sandbox Integration** (Phase A)
   - ✅ Highest security impact — DONE
   - ✅ Infrastructure exists, needs wiring — DONE
   - Completed: 2026-02-21

### ~~Sprint 2: Developer Experience (Week 2)~~ ✅ COMPLETE
2. **Tool System Enhancements** (Phase B)
   - ✅ apply_patch is frequently requested — DONE
   - ✅ Profile system improves usability — DONE
   - Completed: 2026-02-21

3. **Memory Enhancements** (Phase C)
   - ✅ Performance optimization — EVALUATED
   - ✅ Optional sqlite-vec evaluation — DONE (deferred)
   - Completed: 2026-02-21

### Sprint 3: Accessibility (Next Priority)
4. **Voice Interaction** (Phase D)
   - Accessibility improvement
   - Competitive feature parity
   - Estimated: 5 days

### Sprint 4+: Optional Channels (As Needed)
5. **Email Channel** — If users request direct email
6. **Web Widget** — If embedding use case emerges

---

## 5. Success Metrics

### Technical Metrics

| Metric | Before | After | Status |
|--------|--------|-------|--------|
| Tool isolation | None | Container-wrapped | ✅ SandboxManager |
| Tool count | ~15 | ~20 | ✅ +5 new tools |
| Tool profiles | None | 4 profiles | ✅ ToolProfile enum |
| Memory search | BLOB | BLOB + eval | ✅ sqlite-vec deferred |
| Voice STT latency | N/A | N/A | ⏸️ Not implemented |
| Channel count | 4 | 4 | ✅ Matrix bridges others |

### Feature Parity

| Feature | OpenClaw | MesoClaw | Gap |
|---------|----------|----------|-----|
| Gateway | ✅ | ✅ | **None** |
| Agent Loop | ✅ | ✅ | **None** |
| Tool System | ✅ 25+ | ✅ 20+ | **None** (browser/canvas/nodes specialized) |
| Tool Profiles | ✅ | ✅ | **None** |
| Memory | ✅ | ✅ | **None** (sqlite-vec deferred, not needed) |
| Channels | ✅ 8+ | ✅ 4 | **Minor** (Matrix bridges) |
| Scheduler | ✅ | ✅ | **None** |
| Security | ✅ | ✅ | **None** (sandbox wired) |
| Sandbox | ✅ | ✅ | **None** |
| Voice | ✅ | ❌ | **Major** |
| Mobile | ✅ | ✅ | **Minor** (distribution) |
| Extensions | ✅ npm | ✅ TOML | **Different model** |

---

## 6. References

- OpenClaw Documentation: https://deepwiki.com/openclaw/openclaw
- MesoClaw Implementation Plan: docs/implementation-plan.md
- MesoClaw Architecture: docs/architecture-diagram.md
- MesoClaw User Guide: docs/app_usage.md
- MesoClaw Autonomous Plan: docs/mesoclaw_autonomous.md
- MesoClaw UI Gap Analysis: docs/mesoclaw_gapopenclawUI.md

---

## Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2026-02-20 | 1.0 | Initial gap analysis document |
| 2026-02-20 | 1.1 | Updated with actual implementation status from implementation-plan.md; corrected gap assessments based on completed Phases 0-8; added accurate current state comparison |
| 2026-02-21 | 1.2 | **Phases A, B, C Complete**: Added SandboxManager integration (Phase A); Added PatchTool, ProcessTool, CronTool, SessionSpawnTool (Phase B); Added ToolProfile/ToolGroup abstraction (Phase B); Evaluated sqlite-vec, deferred until scale justifies (Phase C); Updated gap matrix and success metrics |
| 2026-02-21 | 1.3 | **Web Tools Added**: Implemented WebFetchTool, WebRequestTool, and WebSearchTool for HTTP requests and web search; Agents can now fetch web content, interact with REST APIs, and perform web searches |
