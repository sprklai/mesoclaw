# CLAUDE.md -- Zenii

## Project Overview

Zenii is a Rust workspace producing 5 binaries (Desktop, Mobile, CLI, TUI, Daemon) from a
single shared core. All clients communicate via HTTP+WebSocket gateway (axum at 127.0.0.1:18981).

## v2 Philosophy

1. **Use proven crates, don't hand-roll** -- `sysinfo`, `websearch`, `rig-core`, `ignore` over custom implementations.
2. **Port patterns, not code** -- port v1 design (trait-based tools, security policy, memory abstraction) adapted to v2 conventions.
3. **Lean by default** -- feature-gate optional modules. Default binary includes only core operation.
4. **Single shared core** -- ALL business logic in `zenii-core`. Binary crates are thin shells (<100 lines).

## Tech Stack

Rust 2024 | Tokio | rig-core | rusqlite + sqlite-vec | axum | Svelte 5 + Tauri 2 | openclaw-channels | comrak + Tera

## Commands

```bash
cargo check --workspace                      # Compile check
cargo test --workspace                       # Run all tests
cargo clippy --workspace                     # Lint
cargo run -p zenii-daemon                    # Start daemon
cargo run -p zenii-cli -- chat               # CLI chat
cd web && bun run dev                        # Frontend dev
cd crates/zenii-desktop && cargo tauri dev   # Desktop app
./scripts/build.sh --target native --release # Build binaries
```

## Workspace Structure

→ Full module detail in `docs/architecture.md`

```
crates/zenii-core/     # Shared library — ALL business logic, NO Tauri dep
crates/zenii-desktop/  # Tauri 2 shell (macOS, Windows, Linux)
crates/zenii-mobile/   # Tauri 2 shell (iOS, Android)
crates/zenii-cli/      # clap CLI (thin wrapper)
crates/zenii-tui/      # ratatui TUI (thin wrapper)
crates/zenii-daemon/   # Headless daemon (thin wrapper)
web/                   # Svelte 5 frontend (shared by desktop + mobile)
docs/                  # Architecture diagrams, phase details, process flows
plans/                 # Detailed per-phase implementation plans
tests/                 # Per-phase test plans and results
```

## Strict Rules

1. **No std::sync::Mutex in async paths** -- use tokio::sync::Mutex or DashMap
2. **No block_on()** -- use tokio::spawn or .await
3. **No Result<T, String>** -- use ZeniiError enum (thiserror)
4. **All SQLite ops via spawn_blocking** -- rusqlite is sync
5. **Zero business logic in binary crates** -- everything in zenii-core
6. **No code duplication** -- if used twice, extract to zenii-core
7. **TDD gate order**: plan → user approves → write tests → user approves → implement → cargo test → user validates
8. **No phase proceeds without user confirmation at all 3 gates**
9. **All public functions must have unit tests**
10. **Feature flags for optional modules** -- keep default binary lean
11. **Research before adding dependencies** -- compare alternatives, document rationale in `plans/`
12. **Binary size matters** -- prefer lightweight crates, check dependency trees
13. **Never skip the workflow** -- plan file in `plans/` + test plan in `tests/` MUST exist on disk before any `.rs` file is created or modified. No exceptions.

## Conventions

→ Verbose details for credential naming, dark-mode select, and no-magic-numbers in `docs/conventions.md`

- Error handling: `ZeniiError` enum with thiserror, no `.map_err(|e| e.to_string())`
- Async: tokio::sync primitives only, never std::sync in async code
- Concurrency: DashMap for concurrent HashMaps, tokio::sync::Mutex for async locks
- Testing: `#[cfg(test)]` in same file, integration tests in `tests/`; test success + failure paths
- Naming: snake_case (Rust), camelCase (TypeScript/Svelte)
- Imports: std → external crates → internal modules (blank line separated)
- Logging: `tracing` macros only (info!, warn!, error!, debug!), never println!
- Paths: absolute in code, relative when referencing to user
- SQL: parameterized queries only, WAL mode, migrations in transactions
- Security: never log credentials, use zeroize for sensitive data, keyring for storage
- Credential keys: colon-separated namespacing — `api_key:{provider_id}`, `channel:{id}:{field}`
- Structs: derive `Debug, Clone, Serialize, Deserialize` on all public structs
- Enums: `#[non_exhaustive]` on public enums that may grow
- Async locks: never hold across `.await` points
- No magic numbers: all tunables defined as `AppConfig` fields in `schema.rs`
- Native `<select>` dark mode: use `bg-background text-foreground`, never `bg-transparent`

## Agent Usage

- **Explore agents**: broad codebase research, deep traversal, or unfamiliar modules (>3 queries).
- **Parallel task agents**: independent workstreams (e.g., updating unrelated modules simultaneously).
- **Research agents**: dependency research, v1 analysis, documentation lookups.
- **Skip agents**: single Glob/Grep suffices, sequential steps, trivial 1-2 file edits.

Do not duplicate work an agent is already doing. Delegate, then use the results.

## Plan Mode Requirement

Always start in Plan Mode for: new features, new phases, architectural changes, or tasks spanning
multiple files. Enter `EnterPlanMode` first, outline steps, exit only when the plan is clear.

## Phase Gate Workflow

Every phase has 3 user gates — no skipping. → Full checklist in `docs/phases.md`

1. **Gate 1 — Plan**: Write `plans/phaseN_*.md`. Include scope, API signatures, dependency research
   (compare alternatives on size/maintenance), v1 analysis, assumptions. **User approves before any code.**
2. **Gate 2 — Tests**: Write unit tests based on approved plan. **User approves before implementation.**
3. **Gate 3 — Completion**: Implement → `cargo test` → `cargo clippy` → present summary. **User confirms.**
4. **Post-Gate — Docs**: Update `docs/architecture.md`, `docs/phases.md`, `docs/processes.md`,
   `README.md`, `no_commit/todo_tracker.md`. Mandatory before next phase.

Ask user for design decisions between gates. Never assume.

## Best Practices

- **Read before write**: understand existing code before modifying.
- **Think first**: state assumptions explicitly before implementing. If multiple interpretations exist, present them — don't pick silently. Push back when a simpler approach exists.
- **Ask when unclear**: stop and name what's confusing. Don't assume and run with it.
- **Minimal changes**: only change what's needed; don't refactor adjacent code. Every changed line must trace directly to the user's request.
- **Surgical edits**: modify only the relevant function/block. Match existing style, even if you'd do it differently. Don't improve adjacent comments or formatting.
- **Dead code rule**: remove imports/variables/functions YOUR changes made unused. Don't delete pre-existing dead code unless asked — mention it instead.
- **No verbose output**: responses must be concise. Use bullet lists or tables — not paragraphs. Omit preamble ("I'll now..."), narration, and trailing summaries.
- **Validate before done**: run `cargo test --workspace && cargo clippy --workspace`.
- **Verifiable goals**: transform tasks into success criteria — "fix bug" → "write test that reproduces it, then make it pass". Multi-step tasks need a brief plan with a verify step per step.
- **Atomic commits**: one logical change per commit.
- **No dead code**: no commented-out code, unused imports, or placeholder stubs.
- **Latest packages**: always use latest stable versions; check with `cargo upgrade --dry-run`.
- **Learn from errors**: save root cause + fix to memory so the same mistake is never repeated.

## Documentation Requirement

After Gate 3 approval: update `docs/architecture.md`, `docs/phases.md`, `docs/processes.md`,
`README.md`, and `no_commit/todo_tracker.md` before proceeding. Not optional.

## Markdown / Mermaid Rules

→ Full rules in `docs/style-guide.md`. Key points: use `<br>` not `<br/>`, escape `(` as `#40;`,
avoid subgraph/node ID collisions, no `1.` numbered lists in node labels.

## TODO / MOCK / FIX Tracking

Tag incomplete work in code and in `no_commit/todo_tracker.md`:

```rust
// TODO: <description> — <phase>
// MOCK: <description> — <what it replaces, when to remove>
// FIX: <description> — <what's wrong, severity>
// STUB: <description> — <what it should become>
```

Tracker table format and status values are defined in `no_commit/todo_tracker.md`.

## Feature Flags

```bash
cargo build -p zenii-daemon                          # Core only
cargo build -p zenii-daemon --features channels      # + messaging
cargo build -p zenii-daemon --features scheduler     # + cron
cargo build -p zenii-daemon --features web-dashboard # + web UI
cargo build -p zenii-daemon --all-features           # Everything
```

## Wiki

LLM wiki in `wiki/`. Schema: `wiki/SCHEMA.md` (read first). Sources: `wiki/sources/`. Ops: ingest, query, lint — see `docs/wiki.md`.

<!-- rtk-instructions v2 -->
# RTK (Rust Token Killer) - Token-Optimized Commands

## Golden Rule

**Always prefix commands with `rtk`**. If RTK has a dedicated filter, it uses it. If not, it passes through unchanged. This means RTK is always safe to use.

**Important**: Even in command chains with `&&`, use `rtk`:
```bash
# ❌ Wrong
git add . && git commit -m "msg" && git push

# ✅ Correct
rtk git add . && rtk git commit -m "msg" && rtk git push
```

## RTK Commands by Workflow

### Build & Compile (80-90% savings)
```bash
rtk cargo build         # Cargo build output
rtk cargo check         # Cargo check output
rtk cargo clippy        # Clippy warnings grouped by file (80%)
rtk tsc                 # TypeScript errors grouped by file/code (83%)
rtk lint                # ESLint/Biome violations grouped (84%)
rtk prettier --check    # Files needing format only (70%)
rtk next build          # Next.js build with route metrics (87%)
```

### Test (90-99% savings)
```bash
rtk cargo test          # Cargo test failures only (90%)
rtk vitest run          # Vitest failures only (99.5%)
rtk playwright test     # Playwright failures only (94%)
rtk test <cmd>          # Generic test wrapper - failures only
```

### Git (59-80% savings)
```bash
rtk git status          # Compact status
rtk git log             # Compact log (works with all git flags)
rtk git diff            # Compact diff (80%)
rtk git show            # Compact show (80%)
rtk git add             # Ultra-compact confirmations (59%)
rtk git commit          # Ultra-compact confirmations (59%)
rtk git push            # Ultra-compact confirmations
rtk git pull            # Ultra-compact confirmations
rtk git branch          # Compact branch list
rtk git fetch           # Compact fetch
rtk git stash           # Compact stash
rtk git worktree        # Compact worktree
```

Note: Git passthrough works for ALL subcommands, even those not explicitly listed.

### GitHub (26-87% savings)
```bash
rtk gh pr view <num>    # Compact PR view (87%)
rtk gh pr checks        # Compact PR checks (79%)
rtk gh run list         # Compact workflow runs (82%)
rtk gh issue list       # Compact issue list (80%)
rtk gh api              # Compact API responses (26%)
```

### JavaScript/TypeScript Tooling (70-90% savings)
```bash
rtk pnpm list           # Compact dependency tree (70%)
rtk pnpm outdated       # Compact outdated packages (80%)
rtk pnpm install        # Compact install output (90%)
rtk npm run <script>    # Compact npm script output
rtk npx <cmd>           # Compact npx command output
rtk prisma              # Prisma without ASCII art (88%)
```

### Files & Search (60-75% savings)
```bash
rtk ls <path>           # Tree format, compact (65%)
rtk read <file>         # Code reading with filtering (60%)
rtk grep <pattern>      # Search grouped by file (75%)
rtk find <pattern>      # Find grouped by directory (70%)
```

### Analysis & Debug (70-90% savings)
```bash
rtk err <cmd>           # Filter errors only from any command
rtk log <file>          # Deduplicated logs with counts
rtk json <file>         # JSON structure without values
rtk deps                # Dependency overview
rtk env                 # Environment variables compact
rtk summary <cmd>       # Smart summary of command output
rtk diff                # Ultra-compact diffs
```

### Infrastructure (85% savings)
```bash
rtk docker ps           # Compact container list
rtk docker images       # Compact image list
rtk docker logs <c>     # Deduplicated logs
rtk kubectl get         # Compact resource list
rtk kubectl logs        # Deduplicated pod logs
```

### Network (65-70% savings)
```bash
rtk curl <url>          # Compact HTTP responses (70%)
rtk wget <url>          # Compact download output (65%)
```

### Meta Commands
```bash
rtk gain                # View token savings statistics
rtk gain --history      # View command history with savings
rtk discover            # Analyze Claude Code sessions for missed RTK usage
rtk proxy <cmd>         # Run command without filtering (for debugging)
rtk init                # Add RTK instructions to CLAUDE.md
rtk init --global       # Add RTK to ~/.claude/CLAUDE.md
```

## Token Savings Overview

| Category | Commands | Typical Savings |
|----------|----------|-----------------|
| Tests | vitest, playwright, cargo test | 90-99% |
| Build | next, tsc, lint, prettier | 70-87% |
| Git | status, log, diff, add, commit | 59-80% |
| GitHub | gh pr, gh run, gh issue | 26-87% |
| Package Managers | pnpm, npm, npx | 70-90% |
| Files | ls, read, grep, find | 60-75% |
| Infrastructure | docker, kubectl | 85% |
| Network | curl, wget | 65-70% |

Overall average: **60-90% token reduction** on common development operations.
<!-- /rtk-instructions -->
