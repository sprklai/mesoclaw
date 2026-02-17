# Moltis & MicroClaw Analysis: Relevance to Mesoclaw

> Critical assessment of two additional "claw" ecosystem repositories.
> Conducted: February 2026

---

## Moltis (moltis-org/moltis)

**What it is**: A self-contained AI gateway written in Rust. 27-crate workspace, ~150k lines, 981 stars, version 0.8.15. Single 60MB binary with multi-provider LLM support, hybrid memory, agent loop, sandbox execution, hooks, channels, cron, identity files, and MCP support.

**Architecture**: Modular crate-based design — Gateway (Axum HTTP/WS) → Agents → Tools → Channels → Sessions → Memory → MCP → Skills → Hooks → Sandbox.

**Tech Stack**: Rust 2024, Tokio, Axum, SQLite (sqlx), async-openai, genai, llama-cpp-2, teloxide (Telegram), webauthn-rs.

### What's Real and Excellent

1. **Memory system** (`crates/memory/`) — Production-quality hybrid vector + full-text search with markdown chunking by heading, LRU caching for embeddings, graceful fallback when no embeddings provider
2. **Agent loop** (`crates/agents/src/runner.rs`) — Clean iteration limiting (25 max), tool result sanitization (base64 stripping, size limits), concurrent tool execution, stream-based responses
3. **Hook system** (`crates/plugins/`) — BeforeToolCall/AfterToolCall lifecycle events, shell script hooks with metadata (HOOK.md), circuit breaker for failing hooks, dry-run mode
4. **Sandbox security** (`crates/tools/`) — Docker/Apple Container backends, per-session isolation, package allowlisting, secret redaction in output
5. **Identity files** — IDENTITY.md, SOUL.md, USER.md with YAML frontmatter, workspace-scoped (.moltis/ directory)
6. **Strict linting** — `deny(unsafe_code)`, `deny(unwrap_used)`, `deny(expect_used)`

### What's Overhyped

1. **"Desktop app"** — Tauri integration is PLANNED but not shipped. It's a web server, not a desktop app.
2. **"Production-ready"** — Version 0.8.x with multiple breaking changes per week
3. **Test coverage** — Codecov badge exists but percentage hidden. No public coverage report.
4. **"One binary, no runtime"** — Requires Docker/Podman for core sandbox features

### Relevance to Mesoclaw

| Mesoclaw Gap   | Moltis Status                                                | Verdict               |
| -------------- | ------------------------------------------------------------ | --------------------- |
| Event Bus      | Missing — uses WebSocket broadcast                           | Need to build our own |
| Heartbeat/Cron | Has it — `crates/cron/` with heartbeat prompts               | Study pattern         |
| Identity Files | Has it — YAML frontmatter in .md files                       | Copy pattern          |
| Memory         | Excellent — hybrid vector + FTS, chunking, SQLite            | Strong inspiration    |
| Agent Loop     | Has it — 25-iteration limit, tool calling, streaming         | Study implementation  |
| Tools          | Robust — 17 built-in, MCP bridge, hooks, sandbox             | Learn from this       |
| Security       | Strong — sandboxing, auth, SSRF protection, secret redaction | Adopt patterns        |
| Channels       | Extensible — Telegram done, trait-based abstraction          | Copy architecture     |

**Bottom line**: Goldmine for patterns, not a library to import. Can't `cargo add moltis` — it's an application, not a crate. Study the memory system, agent loop, and security patterns. Avoid the 27-crate complexity and web-first architecture.

---

## MicroClaw (microclaw/microclaw)

**What it is**: A 9-day-old experimental Rust chat bot daemon. 168 stars, 3 contributors. Multi-platform (Telegram, Discord, Slack, Feishu, Web). Created Feb 7, 2026.

**Architecture**: Channel-agnostic core with platform adapters. 25 public modules, ~32 production dependencies.

### Honest Assessment

**Creator's own words**: _"This is still a toy / experimental project. There's no sandboxing, permission model, or security hardening — the agent has full access to bash and the filesystem."_

| Category             | Score | Notes                            |
| -------------------- | ----- | -------------------------------- |
| Code quality         | 6/10  | Compiles, has CI, but zero tests |
| Documentation        | 5/10  | README exists, no API docs       |
| Maturity             | 2/10  | 9 days old                       |
| Security             | 1/10  | Creator admits no sandboxing     |
| Relevance to Tauri   | 0/10  | Not Tauri-related at all         |
| Production readiness | 2/10  | Experimental                     |

### What's Interesting

- Multi-platform channel adapter pattern in Rust
- Anthropic-compatible skills format
- MCP server integration
- Context compaction for long sessions

### What's Not Ready

- Zero tests (CI runs `cargo test` on empty test suite)
- No sandboxing, no permission model
- Not published on crates.io
- No documented public API
- Weekend hack pace (entire repo is 9 days old)

**Bottom line**: Not relevant for Mesoclaw. Too immature, no Tauri integration, no tests, no security. Glance at channel adapter patterns if curious, nothing more.

---

## Decision: Neither Is Importable as a Dependency

Both Moltis and MicroClaw are **applications**, not libraries. You cannot `cargo add` either one.

**What we CAN do**:

- Study Moltis patterns for memory, agent loop, hooks, and identity files
- Use Moltis's strict linting config (`deny(unsafe_code, unwrap_used, expect_used)`)
- Adopt Moltis's agent loop architecture (iteration limits, tool result sanitization, concurrent execution)
- Ignore MicroClaw entirely for now

---

## Codebase Audit: Mesoclaw Needs Slimming

Analysis of the existing Mesoclaw backend revealed significant over-engineering:

### By the Numbers

| Module        | Lines     | % of Backend | Issue                                                                                   |
| ------------- | --------- | ------------ | --------------------------------------------------------------------------------------- |
| skills/       | 2,631     | 29%          | Over-engineered — custom parser, composer, selector, executor for prompt templates      |
| ai/providers/ | 1,798     | 20%          | 3 near-identical implementations (OpenAI-compatible: 678, OpenRouter: 566, Vercel: 554) |
| commands/     | 1,826     | 20%          | Reasonable — mostly IPC boilerplate                                                     |
| database/     | 955       | 10%          | Diesel ORM overkill for single-user SQLite desktop app                                  |
| services/     | 624       | 7%           | Reasonable                                                                              |
| adapters/     | 236       | 3%           | Reasonable                                                                              |
| **Total**     | **9,152** | **100%**     | **~62% reduction possible**                                                             |

### Top Savings Opportunities

| Change                                          | Lines Saved | Impact                         |
| ----------------------------------------------- | ----------- | ------------------------------ |
| Replace skills/ with simple prompt templates    | ~2,600      | Remove 29% of codebase         |
| Consolidate 3 AI providers using `async-openai` | ~1,500      | Remove 20% of codebase         |
| Replace Diesel with `rusqlite`                  | ~400        | Smaller binary, faster compile |
| Remove duplicate logging (log + tracing)        | ~50         | Cleaner dependency tree        |
| **Total**                                       | **~4,550**  | **50% reduction**              |

### Projected After Slimming

| Metric              | Before          | After          |
| ------------------- | --------------- | -------------- |
| Lines of Rust       | 9,152           | ~4,600         |
| Direct dependencies | 48              | ~25            |
| Binary size         | 10-20 MB (est.) | 5-10 MB (est.) |
| Compile time        | Full            | ~40-60% faster |

### Recommendation

**Slim down BEFORE adding new features.** Adding 25 gap analysis items (~54 new files) on top of a bloated 9k-line codebase creates unmaintainable tech debt. Cut first, grow second.
