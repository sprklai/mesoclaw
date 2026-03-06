# CLAUDE.md -- MesoClaw

## Project Overview

MesoClaw is a Rust workspace producing 5 binaries from a shared core:
- **Desktop** (Tauri 2 + Svelte 5), **Mobile** (Tauri 2 iOS/Android), **CLI** (clap), **TUI** (ratatui), **Daemon** (headless axum)

All clients communicate via HTTP+WebSocket gateway (axum at 127.0.0.1:18981).

## References

- **V1 Implementation**: `/home/rakesh/RD/NSRTech/Tauri/tauriclaw` — the original v1 codebase, useful for understanding existing patterns and porting logic
- **Migration Plan**: `no_commit/migrate_v1_2_v2_plan.md` — detailed plan for migrating from v1 to v2 architecture

## Tech Stack

Rust 2024 | Tokio | rig-core (AI) | rusqlite + sqlite-vec (DB) | axum (gateway) | Svelte 5 + Tauri 2 (frontend/desktop/mobile) | openclaw-channels (messaging) | comrak + Tera (content pipeline)

## Commands

```bash
cargo check --workspace                    # Compile check
cargo test --workspace                     # Run all tests
cargo clippy --workspace                   # Lint
cargo run -p mesoclaw-daemon               # Start daemon
cargo run -p mesoclaw-cli -- chat          # CLI chat
cd web && bun run dev                      # Frontend dev
cd crates/mesoclaw-desktop && cargo tauri dev  # Desktop app
./scripts/build.sh --target native --release  # Build binaries
```

## Workspace Structure

```
crates/mesoclaw-core/       # Shared library (ALL business logic, NO Tauri dep)
  src/error.rs              # MesoError enum (thiserror)
  src/config/               # TOML config (schema + load/save + OS paths)
  src/db/                   # rusqlite pool + WAL + migrations + spawn_blocking
  src/event_bus/            # EventBus trait + TokioBroadcastBus
  src/memory/               # Memory trait + SqliteMemoryStore (FTS5 + vectors)
  src/credential/           # CredentialStore trait + KeyringStore + InMemoryStore
  src/security/             # SecurityPolicy, AutonomyLevel
  src/tools/                # Agent tools (websearch, sysinfo, shell, file ops)
  src/ai/                   # Rig-based agent + providers
  src/gateway/              # axum HTTP+WS server (~40 routes)
  src/identity/             # Soul/Persona (markdown+YAML, comrak+Tera, hot-reload)
  src/skills/               # Prompt templates (SkillRegistry, parameter substitution)
  src/user/                 # User profile + progressive learning
  src/channels/             # openclaw-channels integration (feature-gated)
  src/scheduler/            # Cron jobs (feature-gated)
  src/boot.rs               # init_services() -> Services bundle
crates/mesoclaw-desktop/    # Tauri 2 shell (macOS, Windows, Linux)
crates/mesoclaw-mobile/     # Tauri 2 shell (iOS, Android)
crates/mesoclaw-cli/        # clap CLI (thin wrapper)
crates/mesoclaw-tui/        # ratatui TUI (thin wrapper)
crates/mesoclaw-daemon/     # Headless daemon (thin wrapper)
web/                        # Svelte 5 frontend (shared by desktop + mobile)
docs/                       # Architecture diagrams, phase details, process flows
plans/                      # Detailed per-phase implementation plans
tests/                      # Per-phase test plans and results
scripts/build.sh            # Cross-platform build script
```

## Strict Rules

1. **No std::sync::Mutex in async paths** -- use tokio::sync::Mutex or DashMap
2. **No block_on()** -- use tokio::spawn or .await
3. **No Result<T, String>** -- use MesoError enum (thiserror)
4. **All SQLite ops via spawn_blocking** -- rusqlite is sync
5. **Zero business logic in binary crates** -- everything in mesoclaw-core
6. **No code duplication** -- if used twice, extract to mesoclaw-core
7. **TDD: plan -> user approves -> write tests -> user approves -> implement -> cargo test -> user validates**
8. **No phase proceeds without user confirmation at all 3 gates (plan, tests, completion)**
9. **All public functions must have unit tests**
10. **Feature flags for optional modules** -- keep default binary lean
11. **Research before adding dependencies** -- search internet for crates, compare alternatives, document rationale in `plans/`
12. **Binary size matters** -- prefer lightweight crates, check dependency trees, avoid bloat

## Conventions

- Error handling: MesoError enum with thiserror, no `.map_err(|e| e.to_string())`
- Async: tokio::sync primitives only, never std::sync in async code
- Concurrency: DashMap for concurrent HashMaps, tokio::sync::Mutex for async locks
- Testing: `#[cfg(test)]` in same file, integration tests in `tests/`
- Naming: snake_case (Rust), camelCase (TypeScript/Svelte)
- Imports: std -> external crates -> internal modules (blank line separated)
- Logging: `tracing` macros only (info!, warn!, error!, debug!), never println!
- Frontend: max 1 `$effect` per Svelte component, WS for real-time, no polling
- Paths: absolute in code, relative when referencing to user
- SQL: parameterized queries only, WAL mode, migrations in transactions
- Security: never log credentials, use zeroize for sensitive data, keyring for storage
- Structs: derive `Debug, Clone, Serialize, Deserialize` on all public structs
- Enums: `#[non_exhaustive]` on public enums that may grow
- Async locks: never hold across `.await` points
- Testing: test success + failure paths, use `tempfile` for FS tests, mock external APIs

## Plan Mode Requirement

**Always start in Plan Mode** when:
- Implementing a new feature
- Starting a new phase implementation
- Making architectural changes
- Any task involving multiple files or modules

Enter plan mode (`EnterPlanMode`) first to think through the approach, identify affected files, and outline the steps before writing any code. Exit plan mode only after the plan is clear, then execute.

This prevents wasted work from wrong assumptions and ensures alignment with the Phase Gate Workflow below.

## Phase Gate Workflow

Every phase has **3 user gates** -- no skipping allowed:

1. **Gate 1 -- Plan**: Create detailed plan in `plans/phaseN_*.md`. This includes:
   - Scope, API signatures, data models, dependencies
   - **Dependency research**: Search the internet for candidate crates/libraries. Compare alternatives on: binary size impact, compile time, maintenance activity, dependency tree depth, feature completeness. Prefer lightweight crates that keep the binary lean.
   - **Tech selection rationale**: For every dependency chosen (or rejected), document *why* in the plan. Include alternatives considered, trade-offs, and size/performance implications.
   - **Assumptions log**: List all assumptions with rationale. Flag any that need user confirmation.
   - Present to user. **User must approve before any code.**
2. **Gate 2 -- Tests (TDD)**: Write unit tests first based on the approved plan. Present tests to user. **User must approve test design before implementation.**
3. **Gate 3 -- Completion**: Implement -> `cargo test` -> `cargo clippy` -> present summary with diagrams. **User confirms before next phase.**

Between gates, **ask user for inputs** on design decisions, preferences, and constraints. Never assume -- wrong assumptions cost more than a question.

See `docs/phases.md` for full phase details and checklist.

## Best Practices

- **Read before write**: Always read existing code before modifying. Understand context first.
- **Minimal changes**: Only change what's needed. Don't refactor, add docs, or "improve" adjacent code.
- **Don't touch working code**: Never refactor or restructure code that already works unless the user explicitly asks for it or it's strictly necessary for the current task.
- **Validate before claiming done**: Run `cargo test --workspace && cargo clippy --workspace` -- never skip.
- **No guessing**: If unclear, ask. Wrong assumptions cost more than a question.
- **Atomic commits**: One logical change per commit. Don't bundle unrelated changes.
- **Check compilation early**: Run `cargo check` after structural changes, don't wait until the end.
- **Prefer existing patterns**: Match the style and patterns already in the codebase.
- **No dead code**: Don't leave commented-out code, unused imports, or placeholder stubs.
- **Latest packages**: Always use the latest stable versions of all dependencies (Rust crates, npm/bun packages, Tauri plugins). When adding new dependencies, check for the current latest version first. Periodically verify existing deps are up to date via `cargo upgrade --dry-run` and equivalent frontend tooling.
- **Learn from errors**: When a build/test/runtime error occurs, diagnose the root cause and save the pattern + fix to memory (`~/.claude/projects/*/memory/`) so the same mistake is never repeated.

## Documentation Requirement

After each phase, update `docs/` and `README.md` with Mermaid diagrams showing what changed and how it fits the architecture. See `docs/architecture.md` and `docs/processes.md`.

## Markdown Compatibility Rules

- **Mermaid line breaks**: Use `<br>` not `<br/>` — Mermaid 11.x Langium parser rejects self-closing `<br/>` with "Syntax error in text"
- **Mermaid parentheses**: Use `#40;` and `#41;` for `(` and `)` inside node labels — bare parentheses trigger "Unsupported markdown: list" in Mermaid 11.x. Does NOT apply to subgraph titles or sequence diagram participants — use plain text or dashes there instead.
- **Mermaid numbered lists**: Never start node label text with `1.`, `2.`, etc. — Mermaid interprets these as Markdown ordered list items
- **Directory trees**: Use Unicode box-drawing characters (`├──`, `└──`, `│`) not ASCII `+--` and `|` — the `+` is a valid Markdown list marker and triggers "unsupported list" warnings in renderers

## Feature Flags

```bash
cargo build -p mesoclaw-daemon                          # Core only
cargo build -p mesoclaw-daemon --features channels      # + messaging
cargo build -p mesoclaw-daemon --features scheduler     # + cron
cargo build -p mesoclaw-daemon --features web-dashboard # + web UI
cargo build -p mesoclaw-daemon --all-features           # Everything
```
