# Mesoclaw Implementation Plan & Task List

> Step-by-step execution plan for transforming Mesoclaw from a chat application into a full desktop AI agent.
> Reference: `docs/claw-ecosystem-analysis.md`, `docs/mesoclaw-gap-analysis.md`

---

## Plan Overview

```
Phase 0          Phase 1         Phase 2            Phase 3           Phase 4          Phase 5         Phase 6          Phase 7         Phase 8
Slim Down    →  Foundation  →  Core Infra     →  Agent Intel    →  Proactive     →  Config & DX  →  Extensions   →  Channels &   → CI/CD &
+ Responsive    (2 items)      (10 items)        (3 items)         (4 items)        (6 items)       & UI (8)       Mobile (4)     Community (5)
(8 items)       2 files        ~37 new files     ~10 new files     ~11 new files    ~8 new files    ~21 files      ~17 files      ~19 files
```

**Total**: 49 items across 8 phases. ~141 new files, ~15 deleted.

**Critical Path**: Slim Down + Responsive → Release Profile → Event Bus → Tools → Security → Identity → Module System → MCP Client → Agent Loop → Memory → Scheduler → Telegram → Notifications → Mobile Builds → CI/CD → Release Pipeline

---

## Phase 0: Slim Down (Technical Debt Reduction)

**Goal**: Reduce backend from 9,152 → ~5,200 lines. Create clean foundation for new features.

**Estimated savings**: ~3,900 lines removed, net 0 crate changes.

### Task 0.1: Add Strict Clippy Lints (S4)

**Risk**: None | **Dependencies**: None | **Files**: 1

| #     | Task                                                                                  | Details                                                                                                                                |
| ----- | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| 0.1.1 | Add lint configuration to `src-tauri/Cargo.toml`                                      | Add `[lints.rust]` section: `unsafe_code = "deny"`. Add `[lints.clippy]` section: `unwrap_used = "warn"`, `expect_used = "warn"`       |
| 0.1.2 | Run `cargo clippy` and catalog all new warnings                                       | Record count of `unwrap_used` and `expect_used` occurrences                                                                            |
| 0.1.3 | Fix critical unwrap/expect calls in error paths                                       | Replace with `?` operator or proper error handling. Don't fix all at once — just the ones in code paths that could panic in production |
| 0.1.4 | Verify: `cargo clippy -- -D warnings` passes (with `unwrap_used` still at warn level) | All deny-level lints pass. Warn-level lints documented for future cleanup                                                              |

---

### Task 0.2: Consolidate AI Providers with async-openai (S2)

**Risk**: Low | **Dependencies**: 0.1 | **Files**: 4 deleted, 1 created, 2 modified | **Saves**: ~1,200 lines

| #      | Task                                                  | Details                                                                                                                                                                                                                                     |
| ------ | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.2.1  | Add `async-openai` to `src-tauri/Cargo.toml`          | `async-openai = "0.26"` (check latest version)                                                                                                                                                                                              |
| 0.2.2  | Read all three existing provider files thoroughly     | `openai_compatible.rs` (678 lines), `openrouter.rs` (566 lines), `vercel_gateway.rs` (554 lines). Map shared patterns vs. differences                                                                                                       |
| 0.2.3  | Create `src-tauri/src/ai/providers/generic.rs`        | Single `GenericProvider` struct. Parameters: `base_url: String`, `api_key: Option<String>`, `headers: HashMap<String, String>`, `model_prefix: Option<String>`. Implement `LLMProvider` trait by delegating to `async-openai`               |
| 0.2.4  | Implement `complete()` method                         | Build `CreateChatCompletionRequest` from `CompletionRequest`. Map model name. Call `async-openai` client                                                                                                                                    |
| 0.2.5  | Implement `stream()` method                           | Use `async-openai` streaming API. Yield `CompletionChunk` items matching existing format                                                                                                                                                    |
| 0.2.6  | Implement provider-specific configuration             | OpenAI: `base_url = "https://api.openai.com/v1"`, auth via Bearer token. OpenRouter: `base_url = "https://openrouter.ai/api/v1"`, auth via Bearer + `HTTP-Referer` header. Vercel: `base_url = "https://sdk.vercel.ai/v1"`, auth via Bearer |
| 0.2.7  | Update factory in `src-tauri/src/ai/providers/mod.rs` | Replace match arms for OpenAI/OpenRouter/Vercel with `GenericProvider::new(config)`                                                                                                                                                         |
| 0.2.8  | Write unit tests for `GenericProvider`                | Test config construction for each provider type. Mock HTTP responses for complete/stream                                                                                                                                                    |
| 0.2.9  | Integration test: verify streaming works end-to-end   | Test with at least one real provider (use Ollama for free local testing)                                                                                                                                                                    |
| 0.2.10 | Delete old files                                      | Remove `openai_compatible.rs`, `openrouter.rs`, `vercel_gateway.rs`                                                                                                                                                                         |
| 0.2.11 | Verify: all existing frontend flows still work        | Manual test: select provider → enter key → send message → receive streaming response                                                                                                                                                        |

---

### Task 0.3: Replace Skills System with Prompt Templates (S1)

**Risk**: Medium | **Dependencies**: 0.1 | **Files**: 10 deleted, 2 created, 3 modified | **Saves**: ~2,300 lines

| #      | Task                                                 | Details                                                                                                                                                  |
| ------ | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.3.1  | Add `tera` to `src-tauri/Cargo.toml`                 | `tera = "1"` (template engine)                                                                                                                           |
| 0.3.2  | Inventory all embedded skills                        | Read `src-tauri/src/skills/` directory. List each skill's name, parameters, and prompt content. There are 8 embedded skills                              |
| 0.3.3  | Convert each embedded skill to a `.md` template      | Create `src-tauri/src/prompts/templates/` directory. Each skill becomes a markdown file with `{{variable}}` placeholders (Tera syntax)                   |
| 0.3.4  | Create `src-tauri/src/prompts/mod.rs`                | Define `PromptTemplate` struct: `name: String`, `description: String`, `template: String`, `parameters: Vec<ParameterDef>`. Define `PromptLoader` struct |
| 0.3.5  | Create `src-tauri/src/prompts/loader.rs`             | Load templates from: (1) embedded via `include_str!()`, (2) filesystem at `~/.mesoclaw/prompts/`. ~100-150 lines total                                   |
| 0.3.6  | Implement `render()` method                          | Use Tera to substitute `{{variable}}` placeholders with provided values. Return rendered prompt string                                                   |
| 0.3.7  | Update `execute_skill_command` IPC handler           | Keep same function signature for frontend compatibility. Internally: load template → render → send to LLM provider                                       |
| 0.3.8  | Update `list_skills_command` and `get_skill_command` | Return template metadata instead of skill objects. Same response shape if possible                                                                       |
| 0.3.9  | Update frontend skill selection UI                   | If response shape changed, update `skillsStore.ts` and related components. Should be minimal if IPC signatures preserved                                 |
| 0.3.10 | Write unit tests for template loading and rendering  | Test: embedded templates load, filesystem templates override embedded, variable substitution works, missing variables error gracefully                   |
| 0.3.11 | Delete entire `src-tauri/src/skills/` directory      | Remove: loader.rs, registry.rs, selector.rs, composer.rs, executor.rs, types.rs, error.rs, settings.rs, state.rs, mod.rs                                 |
| 0.3.12 | Update `src-tauri/src/lib.rs`                        | Replace `pub mod skills;` with `pub mod prompts;`                                                                                                        |
| 0.3.13 | Verify: skill execution still works from frontend    | Manual test: go to skills UI → select a skill → fill parameters → execute → verify LLM response                                                          |

---

### Task 0.4: Diesel → rusqlite Migration (S3) — DEFER IF NEEDED

**Risk**: High | **Dependencies**: 0.1, 0.2, 0.3 | **Files**: ~7 modified, 1 deleted | **Saves**: ~400 lines, 2 crates removed

> This is the most disruptive change. If Diesel is working well and time is limited, skip this and proceed to Phase 1. The skills and provider consolidation have better ROI.

| #     | Task                                        | Details                                                                                                                   |
| ----- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| 0.4.1 | Add `rusqlite` to `src-tauri/Cargo.toml`    | `rusqlite = { version = "0.38", features = ["bundled"] }`                                                                 |
| 0.4.2 | Inventory all Diesel queries                | Read all files in `src-tauri/src/database/`. List every query, model, and migration                                       |
| 0.4.3 | Create data migration script                | Write a Rust script that reads existing Diesel-managed SQLite DB and writes to new rusqlite-managed DB. Preserve all data |
| 0.4.4 | Rewrite `database/mod.rs`                   | Replace Diesel pool with `Arc<Mutex<rusqlite::Connection>>`. Apply migrations as `.sql` scripts on startup                |
| 0.4.5 | Rewrite each model file                     | Replace Diesel `Queryable`/`Insertable` derives with manual `rusqlite::Row` mapping                                       |
| 0.4.6 | Delete `schema.rs`                          | Remove Diesel-generated schema file                                                                                       |
| 0.4.7 | Remove Diesel from `Cargo.toml`             | Remove `diesel`, `diesel_migrations`, `libsqlite3-sys`                                                                    |
| 0.4.8 | Run all 120 existing tests                  | Ensure zero regressions                                                                                                   |
| 0.4.9 | Test data migration with real user database | Use a copy of the existing app database under the OS app data directory (for example `~/.local/share/<bundle-id>/app.db` on Linux)                                                               |

---

### Task 0.5: Responsive Layout Foundation

**Risk**: Low | **Dependencies**: None | **Files**: ~6 modified, 2 created

> This must happen in Phase 0 — retrofitting responsive design later is 10x harder. The same React frontend will run on Tauri Mobile (iOS/Android), so mobile-ready layouts are not optional.

| #      | Task                                            | Details                                                                                                                                                                         |
| ------ | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.5.1  | Define breakpoint system                        | Establish 5 breakpoints in Tailwind config: `xs` (<640px), `sm` (640-768px), `md` (768-1024px), `lg` (1024-1280px), `xl` (>1280px). Document in a shared constants file         |
| 0.5.2  | Create responsive root layout in `__root.tsx`   | 3-column grid on xl (`grid-cols-[256px_1fr_320px]`), 2-column on md (`grid-cols-[256px_1fr]`), single column on xs/sm. Use Tailwind responsive prefixes                         |
| 0.5.3  | Create `MobileNav` bottom navigation component  | Fixed bottom bar visible only on `< md`. Icons: Chat, Channels, Memory, Settings. Uses `cn()` helper for conditional classes. Minimum 44x44px touch targets                     |
| 0.5.4  | Create `ResponsiveSidebar` component            | Desktop (`>= md`): persistent sidebar, 256px wide. Mobile (`< md`): slide-in drawer from left, triggered by hamburger button or swipe-right gesture. Overlay backdrop on mobile |
| 0.5.5  | Update `PromptInput` for mobile                 | Sticky bottom positioning. Add `pb-safe` class for safe area padding (home indicator on iOS). Handle virtual keyboard resize via `visualViewport` API                           |
| 0.5.6  | Update modal/dialog components for mobile       | Desktop: centered modal with backdrop. Mobile (`< md`): bottom sheet pattern (slides up from bottom, swipe down to dismiss). Apply to existing Dialog components                |
| 0.5.7  | Add viewport meta tag and safe area CSS         | Ensure `<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">` in index.html. Add `env(safe-area-inset-*)` utilities to Tailwind             |
| 0.5.8  | Audit all existing components for touch targets | Verify every button, link, and interactive element is minimum 44x44px. Fix any that are smaller. Audit spacing between tappable elements (minimum 8px gap)                      |
| 0.5.9  | Test at all breakpoints                         | Resize browser to each breakpoint. Verify: sidebar behavior, navigation, input positioning, modal behavior, no horizontal overflow, no text truncation breaking layout          |
| 0.5.10 | Create `useBreakpoint()` hook                   | Custom hook returning current breakpoint name. Use for conditional rendering where CSS alone isn't sufficient (e.g., different component trees on mobile vs desktop)            |

---

### Task 0.6: Restructure to Library + Two Binaries

**Risk**: Medium | **Dependencies**: None | **Files**: 3 new, 2 modified

| #     | Task                                               | Details                                                                                                                                                      |
| ----- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 0.6.1 | Create `src-tauri/src/bin/` directory              |                                                                                                                                                              |
| 0.6.2 | Move current `main.rs` content to `bin/desktop.rs` | Tauri GUI entry point. Starts daemon in-process, launches Tauri window                                                                                       |
| 0.6.3 | Create `bin/cli.rs`                                | Minimal CLI entry point with clap. Subcommands: daemon, agent, memory, identity, config, schedule, channel, gui. Interactive REPL as default (no subcommand) |
| 0.6.4 | Refactor `lib.rs`                                  | Extract all module declarations and app setup into lib.rs. Both binaries import from lib. lib.rs becomes the core crate API                                  |
| 0.6.5 | Update `Cargo.toml`                                | Add `[[bin]]` sections for both cli and desktop. Add clap dependency: `clap = { version = "4", features = ["derive"] }`                                      |
| 0.6.6 | Verify both binaries compile                       | `cargo build --bin mesoclaw` (CLI) and `cargo build --bin mesoclaw-desktop` (GUI)                                                                            |
| 0.6.7 | Verify Tauri build still works                     | `bun run tauri dev` should use desktop binary                                                                                                                |

---

### Task 0.7: Basic CLI Shell (Minimal -- Gateway Comes Later)

**Risk**: Low | **Dependencies**: 0.6 | **Files**: 2 new, 1 modified

| #     | Task                                                            | Details                                                                                                       |
| ----- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 0.7.1 | Add CLI dependencies to `Cargo.toml`                            | `rustyline = "14"`, `termimad = "0.30"`                                                                       |
| 0.7.2 | Implement clap command structure in `bin/cli.rs`                | Define all subcommands as clap derive structs. Most commands print "not yet implemented" until gateway exists |
| 0.7.3 | Implement basic REPL loop                                       | `rustyline` editor with history. Read line, print "gateway not connected" until Phase 2                       |
| 0.7.4 | Add `--raw` and `--json` output mode flags                      | Global flags parsed but not functional until gateway exists                                                   |
| 0.7.5 | Verify: `cargo run --bin mesoclaw -- --help` shows all commands |                                                                                                               |

---

### Task 0.8: Cargo Feature Flags (Modularity Foundation)

**Risk**: Low | **Dependencies**: 0.6 | **Files**: 1 modified

| #     | Task                                            | Details                                                                                                                                                                                                                                                                     |
| ----- | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0.8.1 | Define feature flags in `Cargo.toml`            | `[features]` section: `default = ["core", "cli", "desktop"]`, `core = []`, `cli = [...]`, `desktop = [...]`, `gateway = [...]`, `sidecars = []`, `containers = ["sidecars"]`, `mcp-client = ["sidecars"]`, `channels-telegram = []`, `scheduler = []`, `memory-vector = []` |
| 0.8.2 | Gate existing modules behind features           | Wrap module declarations with `#[cfg(feature = "...")]`. Core modules always compiled. CLI/Desktop/Gateway behind respective features                                                                                                                                       |
| 0.8.3 | Verify all build profiles compile               | `cargo build --features core,cli,gateway` (minimal), `cargo build` (default), `cargo build --all-features` (full)                                                                                                                                                           |
| 0.8.4 | Update CI to test multiple feature combinations | Add matrix entries for minimal, default, and full feature sets                                                                                                                                                                                                              |

---

### Phase 0 Checkpoint — COMPLETE (2026-02-18)

Before proceeding to Phase 1, verify:

- [x] `cargo clippy -- -D warnings` passes (deny-level lints) ✅
- [x] `cargo test --lib` — 56 backend + 24 frontend tests pass ✅
- [ ] Frontend: provider selection, chat, and skill execution all work (manual test pending)
- [x] LOC count reduced by ~3,355 lines (−4,090 insertions / +735 deletions across 34 files) ✅
- [x] Net crate change: added `tera` only (per plan) ✅
- [x] Responsive layout foundation (Stream B) — breakpoints, MobileNav, ResponsiveSidebar ✅
- [ ] Touch targets >= 44x44px on all interactive elements (visual audit pending)
- [x] Two binaries compile: `mesoclaw` (CLI) and `mesoclaw-desktop` (Tauri GUI) ✅
- [x] `mesoclaw --help` shows full command structure ✅
- [ ] `bun run tauri dev` still works with desktop binary (build test pending)
- [x] Feature flags defined: `core`, `cli`, `desktop`, `gateway`, `sidecars`, `containers`, `mcp-client`, `channels-telegram`, `scheduler`, `memory-vector` ✅

---

## Phase 1: Foundation

**Goal**: Build the reliability wrapper and optimize binary size. Quick wins with immediate impact.

### Task 1.1: Release Profile Optimization (P0.2)

**Risk**: None | **Dependencies**: None | **Files**: 1

| #     | Task                                          | Details                                                                                                      |
| ----- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 1.1.1 | Add release profile to `src-tauri/Cargo.toml` | `[profile.release]`: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"` |
| 1.1.2 | Build release binary                          | `cd src-tauri && cargo build --release`                                                                      |
| 1.1.3 | Measure binary size before and after          | Record: binary size before (default profile) vs after (optimized). Expect 3-5x reduction                     |
| 1.1.4 | Verify release binary runs correctly          | `bun run tauri build` → install → launch → test basic chat flow                                              |

---

### Task 1.2: ReliableProvider Wrapper (P0.1)

**Risk**: Low | **Dependencies**: 0.2 (consolidated providers) | **Files**: 3

| #     | Task                                                 | Details                                                                                                                                                                                |
| ----- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1.2.1 | Add `warmup()` default method to `LLMProvider` trait | In `src-tauri/src/ai/provider.rs`: `async fn warmup(&self) -> Result<(), String> { Ok(()) }`. Default no-op, providers can override                                                    |
| 1.2.2 | Create `src-tauri/src/ai/providers/reliable.rs`      | `ReliableProvider` struct wrapping `Arc<dyn LLMProvider>`. Fields: `primary: Arc<dyn LLMProvider>`, `fallbacks: Vec<Arc<dyn LLMProvider>>`, `max_retries: u32`, `base_delay: Duration` |
| 1.2.3 | Implement retry logic for `complete()`               | On failure: retry up to `max_retries` times with exponential backoff (base_delay \* 2^attempt). After all retries exhausted, try fallback providers in order                           |
| 1.2.4 | Implement retry logic for `stream()`                 | Same retry pattern. Note: streaming retries restart the entire stream (no partial resume)                                                                                              |
| 1.2.5 | Implement `warmup()`                                 | Pre-establish TLS connection to provider API. On error, log warning but don't fail (provider may come online later)                                                                    |
| 1.2.6 | Wire up in provider factory                          | In `src-tauri/src/ai/providers/mod.rs`: wrap created providers with `ReliableProvider` when fallback chain is configured                                                               |
| 1.2.7 | Write unit tests                                     | Test: retry on transient failure, fallback chain activation, warmup success/failure, max retries respected                                                                             |

---

### Phase 1 Checkpoint ✅ COMPLETE (2026-02-17)

- [x] Release profile optimized: `opt-level = "z"`, `lto = true`, `panic = "abort"`, `strip = true`, `codegen-units = 1`
- [x] `ReliableProvider` passes all 4 new tests (60 total, 0 failed)
- [x] Retry-with-exponential-backoff + fallback chain implemented and tested
- [ ] Binary size verification pending (requires full `cargo build --release`; profile settings are in place)

---

## Phase 2: Core Infrastructure

**Goal**: Build the four subsystems that the Agent Loop depends on: Event Bus, Tools, Security, Identity.

### Task 2.1: Event Bus (P1.8)

**Risk**: Low | **Dependencies**: None | **Files**: 4 new

| #     | Task                                            | Details                                                                                                                                                                                                                                                                               |
| ----- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2.1.1 | Create `src-tauri/src/event_bus/` directory     |                                                                                                                                                                                                                                                                                       |
| 2.1.2 | Define `AppEvent` enum in `traits.rs`           | Variants: `AgentToolStart`, `AgentToolResult`, `AgentComplete`, `ApprovalNeeded`, `ApprovalResponse`, `HeartbeatTick`, `CronFired`, `ChannelMessage`, `MemoryStored`, `MemoryRecalled`, `SystemReady`, `SystemError`, `ProviderHealthChange`. All variants carry relevant data fields |
| 2.1.3 | Define `EventBus` trait in `traits.rs`          | Methods: `async publish(event: AppEvent) -> Result<()>`, `subscribe(event_type: EventType) -> broadcast::Receiver<AppEvent>`, `subscribe_filtered(filter: EventFilter) -> broadcast::Receiver<AppEvent>`                                                                              |
| 2.1.4 | Implement `TokioBroadcastBus` in `tokio_bus.rs` | Use `tokio::sync::broadcast::channel` with configurable capacity (default 1024). Implement all trait methods                                                                                                                                                                          |
| 2.1.5 | Implement `TauriBridge` in `tauri_bridge.rs`    | Subscribes to EventBus. Filters UI-relevant events. Forwards them to frontend via `app_handle.emit("app-event", event)`                                                                                                                                                               |
| 2.1.6 | Create `mod.rs` with public API                 | Re-export traits, default implementation, and bridge                                                                                                                                                                                                                                  |
| 2.1.7 | Register EventBus in Tauri app state            | In `lib.rs` setup: create `TokioBroadcastBus`, wrap in `Arc<dyn EventBus>`, manage via Tauri state                                                                                                                                                                                    |
| 2.1.8 | Write unit tests                                | Test: publish/subscribe, filtered subscription, multiple subscribers, channel capacity                                                                                                                                                                                                |

---

### Task 2.2: Tool Trait & Registry (P1.4)

**Risk**: Low | **Dependencies**: None | **Files**: 5 new

| #     | Task                                            | Details                                                                                                                                                                                                                                                                                                  |
| ----- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2.2.1 | Create `src-tauri/src/tools/` directory         |                                                                                                                                                                                                                                                                                                          |
| 2.2.2 | Define `Tool` trait in `traits.rs`              | Methods: `fn name(&self) -> &str`, `fn description(&self) -> &str`, `fn parameters_schema(&self) -> serde_json::Value` (JSON Schema), `async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>`. Define `ToolResult` struct: `output: String`, `success: bool`, `metadata: Option<Value>` |
| 2.2.3 | Implement `ToolRegistry` in `registry.rs`       | `HashMap<String, Arc<dyn Tool>>`. Methods: `register(tool)`, `get(name) -> Option<Arc<dyn Tool>>`, `list() -> Vec<ToolInfo>` (name + description + schema for LLM tool_use)                                                                                                                              |
| 2.2.4 | Implement `ShellTool` in `shell.rs`             | Executes shell commands. Input: `command: String`, `working_dir: Option<String>`. Output: stdout/stderr + exit code. **Must validate via SecurityPolicy before execution**                                                                                                                               |
| 2.2.5 | Implement `FileReadTool` in `file_ops.rs`       | Read file contents. Input: `path: String`, `max_lines: Option<usize>`. Output: file contents as string. **Must validate path via SecurityPolicy**                                                                                                                                                        |
| 2.2.6 | Implement `FileWriteTool` in `file_ops.rs`      | Write content to file. Input: `path: String`, `content: String`. Output: success message. **Must validate path via SecurityPolicy**                                                                                                                                                                      |
| 2.2.7 | Implement `FileListTool` in `file_ops.rs`       | List directory contents. Input: `path: String`, `recursive: Option<bool>`. Output: list of entries                                                                                                                                                                                                       |
| 2.2.8 | Create `mod.rs` with built-in tool registration | Function `register_builtin_tools(registry: &mut ToolRegistry, policy: Arc<SecurityPolicy>)`                                                                                                                                                                                                              |
| 2.2.9 | Write unit tests for each tool                  | Test with temp directories. Test registry lookup by name. Test schema generation                                                                                                                                                                                                                         |

---

### Task 2.3: Security Policy (P1.6)

**Risk**: Medium | **Dependencies**: None | **Files**: 2 new

| #      | Task                                          | Details                                                                                                                                                                                                                                         |
| ------ | --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2.3.1  | Create `src-tauri/src/security/` directory    |                                                                                                                                                                                                                                                 |
| 2.3.2  | Define `SecurityPolicy` struct in `policy.rs` | Fields: `autonomy_level: AutonomyLevel` (ReadOnly/Supervised/Full), `workspace_root: PathBuf`, `blocked_dirs: Vec<PathBuf>`, `rate_limiter: SlidingWindow`, `action_log: Vec<AuditEntry>`                                                       |
| 2.3.3  | Define `AutonomyLevel` enum                   | `ReadOnly` — only read operations. `Supervised` — approve medium/high risk. `Full` — all ops with rate limiting                                                                                                                                 |
| 2.3.4  | Implement `classify_command_risk()`           | Input: command string. Output: `RiskLevel` (Low/Medium/High). Low: ls, cat, grep, git status, echo, pwd, which, file. Medium: git commit, npm install, touch, mkdir, cp, mv. High: rm, sudo, curl, wget, chmod, chown, kill, shutdown, dd, mkfs |
| 2.3.5  | Implement `validate_command()`                | Check: (1) autonomy level permits risk level, (2) no injection patterns (backticks, `$()`, `${}`, `>`, `>>`), (3) no blocked executables. Return `ValidationResult`: Allowed / NeedsApproval / Denied(reason)                                   |
| 2.3.6  | Implement `validate_path()`                   | Check: (1) resolve symlinks, (2) no `..` traversal, (3) no null bytes, (4) not in blocked dirs list (/etc, /root, ~/.ssh, ~/.aws, ~/.gnupg, /proc, /sys, /dev), (5) within workspace_root (if configured). Return `ValidationResult`            |
| 2.3.7  | Implement `SlidingWindow` rate limiter        | Track action timestamps. Configurable window (default 1 hour) and max actions (default 20). Reject when limit exceeded                                                                                                                          |
| 2.3.8  | Implement `AuditEntry` logging                | Struct: `timestamp`, `tool_name`, `args`, `risk_level`, `decision` (allowed/denied/approved), `result` (success/failure)                                                                                                                        |
| 2.3.9  | Write security-focused tests (minimum 30)     | Test: every risk classification. Every injection pattern blocked. Every path traversal attempt blocked. Rate limiting enforcement. Symlink escape detection. Each blocked directory                                                             |
| 2.3.10 | Add IPC command for approval responses        | `approve_action_command(action_id: String, approved: bool)` — called from frontend approval overlay                                                                                                                                             |

---

### Task 2.4: Identity System (P1.10)

**Risk**: Low | **Dependencies**: None | **Files**: 6 new + default templates

| #     | Task                                                                     | Details                                                                                                                                                                                                                                                                                                                                                                            |
| ----- | ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2.4.1 | Create `src-tauri/src/identity/` directory with `defaults/` subdirectory |                                                                                                                                                                                                                                                                                                                                                                                    |
| 2.4.2 | Create default template files                                            | `defaults/SOUL.md`: Default personality (helpful, concise, technical). `defaults/USER.md`: Placeholder for user info. `defaults/AGENTS.md`: Default operating instructions. `defaults/IDENTITY.md`: Default agent name and description. `defaults/TOOLS.md`: Tool usage guidance. `defaults/HEARTBEAT.md`: Empty checklist template. `defaults/BOOT.md`: Default startup checklist |
| 2.4.3 | Define `Identity` struct in `types.rs`                                   | Fields for each file's content: `soul: String`, `user: String`, `agents: String`, `identity: IdentityMeta`, `tools: String`, `heartbeat: String`, `boot: String`                                                                                                                                                                                                                   |
| 2.4.4 | Implement `IdentityLoader` in `loader.rs`                                | Load from `~/.mesoclaw/identity/`. Fall back to embedded defaults if file doesn't exist. Create directory and copy defaults on first run                                                                                                                                                                                                                                           |
| 2.4.5 | Implement `build_system_prompt()`                                        | Assembly order: SOUL → AGENTS → USER → TOOLS → (MEMORY placeholder) → (daily memory placeholder). Return complete system prompt string                                                                                                                                                                                                                                             |
| 2.4.6 | Add file watcher for hot-reload                                          | Use `notify` crate (or Tauri file watcher). Watch identity directory. On change, reload affected file and emit event                                                                                                                                                                                                                                                               |
| 2.4.7 | Create IPC commands in `identity_commands.rs`                            | `get_identity_file_command(file_name)` — return file content. `update_identity_file_command(file_name, content)` — save file. `list_identity_files_command()` — list all identity files with metadata                                                                                                                                                                              |
| 2.4.8 | Write unit tests                                                         | Test: loading from filesystem, fallback to defaults, system prompt assembly order, hot-reload detection                                                                                                                                                                                                                                                                            |

---

### Task 2.5: Gateway (HTTP REST + WebSocket)

**Risk**: Medium | **Dependencies**: 2.1 (EventBus) | **Files**: 5 new

| #     | Task                                                | Details                                                                                                                    |
| ----- | --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| 2.5.1 | Add gateway dependencies to `Cargo.toml`            | `axum = "0.8"`, `axum-extra = "0.10"`, `tower-http = "0.6"`, `utoipa = "5"`                                                |
| 2.5.2 | Create `src-tauri/src/gateway/` directory           |                                                                                                                            |
| 2.5.3 | Implement bearer token auth middleware in `auth.rs` | Generate random token on startup, write to `~/.mesoclaw/daemon.token` (mode 0600). Validate `Authorization: Bearer` header |
| 2.5.4 | Implement REST routes in `routes.rs`                | Agent CRUD, provider management, memory search/store, identity CRUD, scheduler CRUD, channel management, health/status     |
| 2.5.5 | Implement WebSocket handler in `ws.rs`              | Accept connections at `/api/v1/ws`. Subscribe to EventBus. Forward events to clients. Accept commands                      |
| 2.5.6 | Implement daemon startup in `daemon.rs`             | Bind to 127.0.0.1:18790. Auto-increment port on conflict. Write PID file. Graceful shutdown on SIGTERM                     |
| 2.5.7 | Create `mod.rs` with `start_gateway()` function     | Called from both bin/cli.rs and bin/desktop.rs                                                                             |
| 2.5.8 | Write integration tests                             | Test: health endpoint, auth rejection, agent session CRUD, WebSocket streaming                                             |

---

### Task 2.6: Connect CLI to Gateway

**Risk**: Low | **Dependencies**: 2.5 (Gateway), 0.7 (CLI shell) | **Files**: 3 modified

| #     | Task                                        | Details                                                                                                        |
| ----- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| 2.6.1 | Implement gateway client in CLI             | HTTP client (reqwest) that reads daemon.pid for port, daemon.token for auth. Auto-starts daemon if not running |
| 2.6.2 | Implement streaming REPL                    | Connect to WebSocket. Post message, subscribe to events, render tokens, show tool status, prompt for approval  |
| 2.6.3 | Implement all CLI subcommands               | Wire each clap subcommand to REST endpoints                                                                    |
| 2.6.4 | Implement `--raw` and `--json` output modes | Functional output formatting                                                                                   |
| 2.6.5 | Implement stdin pipe detection              | If stdin is not TTY: read all, prepend as context, one-shot mode                                               |

---

### Task 2.7: Migrate Frontend from Tauri IPC to Gateway

**Risk**: Medium | **Dependencies**: 2.5 (Gateway) | **Files**: ~10 modified

| #     | Task                                       | Details                                                                                                               |
| ----- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| 2.7.1 | Create `src/lib/gateway-client.ts`         | HTTP client wrapping fetch() to gateway REST API. WebSocket client for streaming                                      |
| 2.7.2 | Replace invoke() calls with gateway client | Agent, provider, memory, identity, scheduler use HTTP/WebSocket. Tauri IPC remains for window/tray/notifications only |
| 2.7.3 | Update Zustand stores                      | Stores fetch from gateway client instead of Tauri commands                                                            |
| 2.7.4 | Test end-to-end                            | Verify chat streaming, provider config, memory search all work via gateway                                            |

---

### Task 2.8: Sidecar Module System Core

**Risk**: Medium | **Dependencies**: 2.2 (Tool Registry) | **Files**: 6 new

> Tauri sidecar packaging/security details should follow official guidance:
> https://v2.tauri.app/develop/sidecar/

| #      | Task                                                             | Details                                                                                                                                           |
| ------ | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2.8.1  | Create `src-tauri/src/modules/` directory                        |                                                                                                                                                   |
| 2.8.2  | Define `SidecarModule` trait in `mod.rs`                         | Extends `Tool` trait. Methods: `module_type()`, `runtime()`, `health_check()`, `start()`, `stop()`                                                |
| 2.8.3  | Implement TOML manifest parser in `manifest.rs`                  | Parse `manifest.toml` files. Validate schema, security settings, runtime config. Support tool/service/mcp types                                   |
| 2.8.4  | Implement `SidecarTool` in `sidecar_tool.rs`                     | On-demand process spawning. Stdin/Stdout JSON protocol. Spawn → send request → read response → kill                                               |
| 2.8.5  | Implement `ModuleRegistry` in `mod.rs`                           | Scan `~/.mesoclaw/modules/` for manifests. Register discovered modules in ToolRegistry. Lifecycle management (start services on boot, lazy tools) |
| 2.8.6  | Implement stdin/stdout JSON protocol in `protocol/stdio_json.rs` | Request/response framing: `{"id", "method", "params"}` → `{"id", "result"}` or `{"id", "error"}`. Progress support optional                       |
| 2.8.7  | Wire modules into EventBus                                       | Emit `ModuleToolStart`, `ModuleToolResult` events. Integrate with audit logging                                                                   |
| 2.8.8  | Add bundled sidecar config in `tauri.conf.json`                  | Configure trusted shipped sidecars in `bundle.externalBin` (for built-in helper binaries)                                                         |
| 2.8.9  | Add shell plugin + capabilities for bundled sidecars             | Add `tauri-plugin-shell`, initialize plugin, and scope capability permissions to allowed sidecar commands only                                    |
| 2.8.10 | Keep user-installed modules on backend process path              | Dynamic modules under `~/.mesoclaw/modules/` continue to run via backend process/container manager (not bundled sidecar list)                     |
| 2.8.11 | Write unit tests (minimum 15)                                    | Test: manifest parsing, module discovery, SidecarTool spawn/execute/kill, protocol framing, registry integration, health checks                   |

---

### Task 2.9: Container Runtime Abstraction

**Risk**: Medium | **Dependencies**: 2.8 (Module core) | **Files**: 4 new

| #     | Task                                                          | Details                                                                                                               |
| ----- | ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| 2.9.1 | Define `ContainerRuntime` trait in `modules/container/mod.rs` | Methods: `is_available()`, `pull_image()`, `run()`, `stop()`, `exec()`                                                |
| 2.9.2 | Implement `DockerRuntime` in `container/docker.rs`            | Use `bollard` crate for Docker API. Support volume mounts, memory limits, network policy, timeout                     |
| 2.9.3 | Implement `PodmanRuntime` in `container/podman.rs`            | Podman uses Docker-compatible API — delegate to bollard with socket path adjustment                                   |
| 2.9.4 | Implement auto-detection in `container/mod.rs`                | Priority: Podman → Docker → native fallback. Configurable override in `config.toml`                                   |
| 2.9.5 | Integrate with SidecarTool                                    | When manifest specifies `runtime.type = "docker"` or `"podman"`, use ContainerRuntime instead of direct process spawn |
| 2.9.6 | Add `bollard` dependency (feature-gated)                      | `bollard = { version = "0.18", optional = true }` behind `containers` feature flag                                    |
| 2.9.7 | Write unit tests (minimum 10)                                 | Test: auto-detection, Docker config construction, Podman socket path, container lifecycle, volume mount validation    |

---

### Task 2.10: MCP Protocol Client

**Risk**: Medium | **Dependencies**: 2.8 (Module core) | **Files**: 2 new

| #      | Task                                            | Details                                                                                                                              |
| ------ | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 2.10.1 | Implement MCP client in `modules/mcp_client.rs` | JSON-RPC over stdin/stdout. Support `initialize`, `tools/list`, `tools/call` methods                                                 |
| 2.10.2 | Implement tool discovery                        | On module start: send `initialize` → send `tools/list` → register discovered tools in ToolRegistry with `mcp:{module}:{tool}` naming |
| 2.10.3 | Implement tool execution                        | Agent calls MCP tool → translate to `tools/call` JSON-RPC → send to MCP server → return result                                       |
| 2.10.4 | Implement MCP server lifecycle                  | Start MCP server process on module start. Health check via periodic `ping`. Restart on failure                                       |
| 2.10.5 | Write unit tests (minimum 8)                    | Test: JSON-RPC framing, tool discovery, tool execution, error handling, process lifecycle                                            |

---

### Phase 2 Checkpoint

- [ ] EventBus: publish → subscribe → receive verified
- [ ] ToolRegistry: all 4 built-in tools registered and executing
- [ ] SecurityPolicy: all 30+ security tests pass
- [ ] Identity: files loaded, system prompt assembled, hot-reload works
- [ ] All tests pass: `cargo test --lib`
- [ ] Gateway: `curl http://127.0.0.1:18790/api/v1/health` returns OK
- [ ] Gateway: unauthorized request returns 401
- [ ] CLI: `mesoclaw agent status` returns session list
- [ ] CLI: interactive REPL streams agent responses
- [ ] Frontend: chat works via gateway (not Tauri IPC)
- [ ] Module System: test module manifest parses, SidecarTool executes stdin/stdout protocol
- [ ] Container Runtime: Docker or Podman detected, test container module runs
- [ ] MCP Client: test MCP server discovered, tools registered, execution works

---

## Phase 3: Agent Intelligence

**Goal**: Build the agent loop, memory system, and daily memory — the core intelligence layer.

### Task 3.1: Agent Loop (P0.3)

**Risk**: High | **Dependencies**: 2.1 (EventBus), 2.2 (Tools), 2.3 (Security), 2.4 (Identity) | **Files**: 4 new

| #     | Task                                                | Details                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ----- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 3.1.1 | Create `src-tauri/src/agent/` directory             |                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 3.1.2 | Implement dual tool-call parser in `tool_parser.rs` | Parse OpenAI format: `tool_calls` array in response JSON. Parse XML format: `<tool_call>{"name": "...", "arguments": {...}}</tool_call>`. Output: `Vec<ParsedToolCall>` with `name: String`, `arguments: Value`                                                                                                                                                                                                                                                                             |
| 3.1.3 | Implement `AgentLoop` struct in `loop_.rs`          | Fields: `provider: Arc<dyn LLMProvider>`, `tool_registry: Arc<ToolRegistry>`, `security_policy: Arc<SecurityPolicy>`, `event_bus: Arc<dyn EventBus>`, `identity: Arc<Identity>`, `max_iterations: usize` (default 20), `max_history: usize` (default 50)                                                                                                                                                                                                                                    |
| 3.1.4 | Implement `run()` method — the main loop            | Algorithm: (1) Build context: identity system prompt + conversation history. (2) Call LLM via ReliableProvider. (3) Parse response for tool calls. (4) If no tool calls → return final response. (5) For each tool call: validate via SecurityPolicy → if NeedsApproval, emit event and wait → execute tool → emit result event → append to history. (6) Increment iteration counter. (7) If iteration < max → go to step 2. (8) If iteration >= max → return partial response with warning |
| 3.1.5 | Implement history trimming                          | When messages.len() > max_history: keep system prompt + first user message + last (max_history - 2) messages. This preserves context while preventing overflow                                                                                                                                                                                                                                                                                                                              |
| 3.1.6 | Implement approval flow                             | When SecurityPolicy returns `NeedsApproval`: emit `ApprovalNeeded` event → subscribe to `ApprovalResponse` → wait with timeout (30 seconds) → if approved, execute → if denied or timeout, skip and inform LLM                                                                                                                                                                                                                                                                              |
| 3.1.7 | Create `agent_commands.rs`                          | `start_agent_session_command(message: String)` — start agent loop for a message. `cancel_agent_session_command(session_id: String)` — abort running loop. Uses Tauri events to stream intermediate results to frontend                                                                                                                                                                                                                                                                      |
| 3.1.8 | Create `mod.rs` with public API                     | Re-export `AgentLoop`, `ParsedToolCall`, `AgentConfig`                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 3.1.9 | Write unit tests (minimum 15)                       | Test: single-turn (no tool calls), multi-turn (1 tool call), max iterations reached, history trimming, approval flow (approved), approval flow (denied), both parser formats, cancellation                                                                                                                                                                                                                                                                                                  |

---

### Task 3.2: Memory System (P1.5)

**Risk**: Medium | **Dependencies**: 2.1 (EventBus) | **Files**: 6 new

| #      | Task                                              | Details                                                                                                                                                                                                                                                                                                     |
| ------ | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 3.2.1  | Create `src-tauri/src/memory/` directory          |                                                                                                                                                                                                                                                                                                             |
| 3.2.2  | Define `Memory` trait in `traits.rs`              | Methods: `async store(key, content, category) -> Result<()>`, `async recall(query, limit) -> Result<Vec<MemoryEntry>>`, `async forget(key) -> Result<bool>`, `async store_daily(content) -> Result<()>`, `async recall_daily(date) -> Result<Option<String>>`                                               |
| 3.2.3  | Define `MemoryEntry` struct                       | Fields: `id: String`, `key: String`, `content: String`, `category: MemoryCategory`, `score: f32`, `created_at: DateTime`, `updated_at: DateTime`                                                                                                                                                            |
| 3.2.4  | Define `MemoryCategory` enum                      | Variants: `Core`, `Daily`, `Conversation`, `Custom`                                                                                                                                                                                                                                                         |
| 3.2.5  | Implement `SqliteMemory` in `sqlite.rs`           | Create tables: `memory_entries` (id, key, content, category, embedding BLOB, created_at, updated_at). Create FTS5 virtual table: `memory_fts` (content)                                                                                                                                                     |
| 3.2.6  | Implement `store()`                               | Insert into `memory_entries`. Insert into `memory_fts`. Generate embedding via embedding provider (OpenAI/Ollama). Store embedding as BLOB. Emit `MemoryStored` event                                                                                                                                       |
| 3.2.7  | Implement `recall()` — hybrid search              | Step 1: Vector search — compute query embedding, cosine similarity against all stored embeddings. Step 2: Keyword search — FTS5 query with BM25 scoring. Step 3: Merge — `final_score = 0.7 * vector_similarity + 0.3 * bm25_score`. Step 4: Sort by final_score, return top N. Emit `MemoryRecalled` event |
| 3.2.8  | Implement embedding generation in `embeddings.rs` | Support OpenAI embeddings API (`text-embedding-3-small`) and Ollama embeddings. LRU cache (10,000 entries) using existing `lru` crate                                                                                                                                                                       |
| 3.2.9  | Implement document chunker in `chunker.rs`        | Split long text into chunks of ~512 tokens. Overlap: 50 tokens between chunks. Store each chunk as separate memory entry with reference to parent                                                                                                                                                           |
| 3.2.10 | Create memory IPC commands                        | `store_memory_command(key, content, category)`, `search_memory_command(query, limit)`, `forget_memory_command(key)`, `get_daily_memory_command(date)`                                                                                                                                                       |
| 3.2.11 | Register memory tools in ToolRegistry             | `memory_store` tool: agent can store facts. `memory_recall` tool: agent can search memory. `memory_forget` tool: agent can remove entries                                                                                                                                                                   |
| 3.2.12 | Write unit tests (minimum 20)                     | Test: store/recall round-trip, hybrid search scoring, chunking, FTS5 queries, embedding cache hits, category filtering                                                                                                                                                                                      |

---

### Task 3.3: Daily Memory Files (P2.17)

**Risk**: Low | **Dependencies**: 3.2 (Memory) | **Files**: 2 new, 2 modified

| #     | Task                                           | Details                                                                                                                                                |
| ----- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 3.3.1 | Create `src-tauri/src/memory/daily.rs`         | `DailyMemory` struct. Manages `~/.mesoclaw/memory/` directory                                                                                          |
| 3.3.2 | Implement `store_daily()`                      | Append entry to today's file (`YYYY-MM-DD.md`). Format: `## HH:MM\n{content}\n\n`                                                                      |
| 3.3.3 | Implement `recall_daily(date)`                 | Read file for given date. Return None if doesn't exist                                                                                                 |
| 3.3.4 | Implement `get_recent_daily()`                 | Load today + yesterday files. Used by identity loader for system prompt injection                                                                      |
| 3.3.5 | Update `IdentityLoader.build_system_prompt()`  | After identity files, append: today's daily memory + yesterday's daily memory                                                                          |
| 3.3.6 | Implement auto-summary (optional, LLM-powered) | At configurable time (default midnight): summarize today's interactions into a concise daily memory entry. Uses LLM provider                           |
| 3.3.7 | Create `MEMORY.md` support                     | Curated long-term memory file at `~/.mesoclaw/memory/MEMORY.md`. Loaded into system prompt. User can edit manually or agent can update via memory tool |
| 3.3.8 | Write unit tests                               | Test: daily file creation, appending, reading, recent daily retrieval                                                                                  |

---

### Phase 3 Checkpoint

- [ ] Agent loop: send complex request → LLM calls tool → tool executes → LLM receives result → final answer
- [ ] Security: tool execution blocked in ReadOnly mode, approval shown in Supervised mode
- [ ] Memory: store fact → recall by keyword → recall by semantic similarity
- [ ] Daily memory: today's file created, loaded into system prompt
- [ ] All tests pass: `cargo test --lib`

---

## Phase 4: Proactive Behavior

**Goal**: Make the agent proactive — monitoring, scheduling, and notifying without user prompting.

### Task 4.1: Scheduler / Heartbeat (P1.9)

**Risk**: Medium | **Dependencies**: 2.1 (EventBus), 2.4 (Identity for HEARTBEAT.md) | **Files**: 4 new

| #      | Task                                               | Details                                                                                                                                                                                                                      |
| ------ | -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1.1  | Create `src-tauri/src/scheduler/` directory        |                                                                                                                                                                                                                              |
| 4.1.2  | Add `cron` crate to `Cargo.toml`                   | `cron = "0.12"` or `croner = "2"` (lighter)                                                                                                                                                                                  |
| 4.1.3  | Define `Scheduler` trait in `traits.rs`            | Methods: `async start()`, `async stop()`, `add_job(ScheduledJob) -> JobId`, `remove_job(JobId)`, `list_jobs() -> Vec<ScheduledJob>`, `job_history(JobId) -> Vec<JobExecution>`                                               |
| 4.1.4  | Define `ScheduledJob` struct                       | Fields: `id: JobId`, `name: String`, `schedule: Schedule` (cron expr or interval), `session_target: SessionTarget` (Main/Isolated), `payload: JobPayload`, `enabled: bool`, `error_count: u32`, `next_run: Option<DateTime>` |
| 4.1.5  | Implement `TokioScheduler` in `tokio_scheduler.rs` | Spawn a background Tokio task. Maintain a sorted queue of next-run times. On tick: check if any jobs are due → execute → record result → reschedule                                                                          |
| 4.1.6  | Implement heartbeat mode                           | Load `HEARTBEAT.md` from identity files. Parse checklist items. Run as a single agent turn (batch all checks). Default interval: 30 minutes. Error backoff: 30s → 1m → 5m → 15m → 60m                                        |
| 4.1.7  | Implement cron mode                                | Parse 5-field cron expressions. Calculate next run time. Execute payload at scheduled time. Support one-shot jobs (run once then remove)                                                                                     |
| 4.1.8  | Implement stuck detection                          | If a job runs > 120 seconds, flag as stuck. Emit `SystemError` event. Don't start duplicate instances                                                                                                                        |
| 4.1.9  | Publish events to EventBus                         | `HeartbeatTick` with check results. `CronFired` with job payload. `SystemError` on failures                                                                                                                                  |
| 4.1.10 | Create IPC commands in `scheduler_commands.rs`     | `list_jobs_command`, `create_job_command`, `toggle_job_command`, `delete_job_command`, `job_history_command`                                                                                                                 |
| 4.1.11 | Persist jobs in SQLite                             | New table: `scheduled_jobs`. New table: `job_executions` (history)                                                                                                                                                           |
| 4.1.12 | Write unit tests                                   | Test: cron parsing, interval scheduling, heartbeat execution, stuck detection, error backoff, job persistence                                                                                                                |

---

### Task 4.2: Desktop Notifications (P2.15)

**Risk**: Low | **Dependencies**: 2.1 (EventBus), 4.1 (Scheduler) | **Files**: 2 new, 2 modified

| #     | Task                                                                  | Details                                                                                                                                                                                                                    |
| ----- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.2.1 | Add `tauri-plugin-notification` to `Cargo.toml` and `tauri.conf.json` | Follow Tauri plugin installation. Add notification permissions                                                                                                                                                             |
| 4.2.2 | Create `src-tauri/src/services/notification_service.rs`               | `NotificationService` struct. Subscribes to EventBus. Filters notification-worthy events                                                                                                                                   |
| 4.2.3 | Implement notification routing                                        | Map event types to notification categories: `HeartbeatTick` → heartbeat alert (if actionable results). `CronFired` → cron reminder. `AgentComplete` → task completion. `ApprovalNeeded` → approval request (high priority) |
| 4.2.4 | Implement click-to-open                                               | Notification payload includes session_id. Click handler opens app and navigates to relevant session                                                                                                                        |
| 4.2.5 | Add notification preferences                                          | Per-category enable/disable stored in config. Do Not Disturb mode                                                                                                                                                          |
| 4.2.6 | Write tests                                                           | Test: event → notification mapping, preference filtering, DND mode                                                                                                                                                         |

---

### Task 4.3: Session Management (P2.16)

**Risk**: Low | **Dependencies**: 3.1 (Agent Loop), 4.1 (Scheduler) | **Files**: 2 new, 1 migration

| #     | Task                           | Details                                                                                                                                                  |
| ----- | ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.3.1 | Define structured session keys | Format: `{agent}:{scope}:{channel}:{peer}`. Examples: `main:dm:tauri:user`, `main:cron:daily-report`, `main:heartbeat:check`, `isolated:task:analyze-db` |
| 4.3.2 | Create `session_router.rs`     | `SessionRouter` struct. Methods: `resolve(channel, context) -> SessionKey`, `create_session(key) -> Session`, `get_session(key) -> Option<Session>`      |
| 4.3.3 | Add database migration         | Add columns to `chat_sessions`: `session_key TEXT`, `scope TEXT`, `channel TEXT`. Migrate existing sessions to `main:dm:tauri:user`                      |
| 4.3.4 | Implement session isolation    | Cron jobs and heartbeat create isolated sessions. Isolated sessions don't pollute main chat history. Each session has independent message history        |
| 4.3.5 | Implement session compaction   | For long-running sessions: summarize old messages (keep last N + summary of earlier). Prevents unbounded growth                                          |
| 4.3.6 | Write tests                    | Test: key resolution, session creation, isolation, compaction                                                                                            |

---

### Task 4.4: Sidecar Service Module

**Risk**: Low | **Dependencies**: 2.8 (Module core), 2.9 (Container runtime) | **Files**: 2 new, 1 modified

| #     | Task                                                       | Details                                                                                                        |
| ----- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| 4.4.1 | Implement `SidecarService` in `modules/sidecar_service.rs` | Long-lived process/container management. Start on boot, health check endpoint polling, auto-restart on failure |
| 4.4.2 | Implement HTTP client for service communication            | POST to service `execute_endpoint`, GET to `health_endpoint`. Parse JSON responses                             |
| 4.4.3 | Integrate with boot sequence                               | Services start during boot after module discovery. Wait for health check before registering tools              |
| 4.4.4 | Write unit tests (minimum 6)                               | Test: service start/stop, health check polling, HTTP execution, auto-restart, timeout handling                 |

---

### Phase 4 Checkpoint

- [ ] Scheduler: heartbeat fires every 30 min (or configured interval), cron jobs execute on schedule
- [ ] Notifications: heartbeat results appear as desktop notifications
- [ ] Sessions: cron/heartbeat run in isolated sessions, main chat unaffected
- [ ] Approval notifications: clicking opens app to approval overlay
- [ ] Sidecar Services: long-lived service module starts on boot, health check passes, HTTP execution works

---

## Phase 5: Configuration & Developer Experience

**Goal**: Improve configuration, model routing, code organization, and parser robustness.

### Task 5.1: TOML Configuration (P2.11)

| #     | Task                                                      | Details                                                                                                                                                 |
| ----- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 5.1.1 | Add `toml` crate to `Cargo.toml`                          | `toml = "1"`                                                                                                                                            |
| 5.1.2 | Define config schema in `src-tauri/src/config/schema.rs`  | Serde structs with `#[serde(default)]` for all fields. Sections: `[provider]`, `[security]`, `[scheduler]`, `[memory]`, `[identity]`, `[notifications]` |
| 5.1.3 | Implement config loading in `src-tauri/src/config/mod.rs` | Load from `~/.mesoclaw/config.toml`. Override with env vars (`MESOCLAW_*`). Fall back to defaults if file doesn't exist                                 |
| 5.1.4 | Implement atomic config saves                             | Write to temp file → fsync → backup old config → atomic rename                                                                                          |
| 5.1.5 | Migrate runtime settings to TOML                          | User-facing config (provider, model, autonomy level) moves to TOML. Runtime state (sessions, cache) stays in SQLite                                     |
| 5.1.6 | Write tests                                               | Test: loading, env overrides, atomic save, missing file fallback                                                                                        |

---

### Task 5.2: Provider Router (P2.12)

| #     | Task                                          | Details                                                                                                                              |
| ----- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 5.2.1 | Create `src-tauri/src/ai/providers/router.rs` | `ModelRouter` struct. Maps task types to providers. Reads routing config from TOML                                                   |
| 5.2.2 | Implement routing logic                       | Rules: model alias resolution, task-type mapping (code → claude, general → gpt-4o), cost-tier selection, availability-based fallback |
| 5.2.3 | Wire into provider factory                    | Router sits between commands layer and providers. Transparent to consumers                                                           |
| 5.2.4 | Write tests                                   | Test: alias resolution, task routing, fallback on unavailable provider                                                               |

---

### Task 5.3: Prelude Module (P2.13)

| #     | Task                                     | Details                                                                                                                                                                                                                                                                                         |
| ----- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 5.3.1 | Add prelude to `src-tauri/src/lib.rs`    | `pub mod prelude { pub use crate::providers::traits::LLMProvider; pub use crate::tools::traits::{Tool, ToolResult}; pub use crate::memory::traits::{Memory, MemoryEntry}; pub use crate::security::policy::SecurityPolicy; pub use crate::event_bus::traits::{EventBus, AppEvent}; /* etc */ }` |
| 5.3.2 | Update internal consumers to use prelude | Replace verbose import paths with `use crate::prelude::*;` where appropriate                                                                                                                                                                                                                    |

---

### Task 5.4: Dual Tool-Call Parser Hardening (P2.14)

| #     | Task                                       | Details                                                                                                               |
| ----- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| 5.4.1 | Add edge case handling to `tool_parser.rs` | Handle: malformed JSON in tool calls, nested XML, partial tool calls (streaming), multiple tool calls in one response |
| 5.4.2 | Add fuzzing tests                          | Test with malformed inputs, extremely long strings, Unicode edge cases                                                |

---

### Task 5.5: Module CLI Commands

**Risk**: Low | **Dependencies**: 2.8 (Module core), 2.6 (CLI) | **Files**: 2 modified

| #     | Task                                  | Details                                                                                                  |
| ----- | ------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| 5.5.1 | Add `module` subcommand group to CLI  | `mesoclaw module list/install/remove/start/stop/health/reload/create`                                    |
| 5.5.2 | Implement `module list`               | Show registered modules, type (tool/service/mcp), runtime, status (running/stopped/error)                |
| 5.5.3 | Implement `module create` scaffolding | `mesoclaw module create <name> --type tool --runtime python` — generates manifest.toml + template script |
| 5.5.4 | Add module endpoints to gateway       | `GET/POST /api/v1/modules/*` — list, start, stop, restart, health, reload                                |
| 5.5.5 | Write tests                           | Test: CLI command parsing, gateway endpoint responses                                                    |

---

### Task 5.6: Tauri Plugin Baseline Hardening

**Risk**: Medium | **Dependencies**: 2.5 (Gateway), 2.8 (Modules), 4.2 (Notifications) | **Files**: 3 modified, 1-2 new

> Plugin references:
>
> - Sidecar: https://v2.tauri.app/develop/sidecar/
> - Plugin index: https://v2.tauri.app/plugin/

| #     | Task                                              | Details                                                                                                    |
| ----- | ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 5.6.1 | Add `tauri-plugin-shell` (if not already enabled) | Required for bundled sidecar command integration and permission-scoped execution of trusted binaries       |
| 5.6.2 | Add `tauri-plugin-single-instance`                | Ensure one desktop instance controls the daemon, prevent duplicate background services and port contention |
| 5.6.3 | Add `tauri-plugin-updater` runtime integration    | Wire signed update manifests and runtime update checks to match FR-16.6/NFR-5.11                           |
| 5.6.4 | Add `tauri-plugin-deep-link` (OAuth readiness)    | Handle callback URLs for provider/channel OAuth flows (Composio, GitHub, Google) in desktop app context    |
| 5.6.5 | Expand capabilities/permissions per plugin        | Add least-privilege permissions in `src-tauri/capabilities/default.json` and avoid broad default grants    |
| 5.6.6 | Document plugin policy                            | Add `docs/architecture/tauri-plugin-baseline.md` with mandatory vs optional plugin list and rationale      |
| 5.6.7 | Write tests                                       | Startup/plugin init tests, single-instance lock behavior, updater/deep-link handler smoke tests            |

---

## Phase 6: Extensions & UI Polish

**Goal**: Add channels, boot sequence, and frontend UIs for new backend capabilities.

### Task 6.1: Channel Trait (P1.7)

| #     | Task                                                         | Details                                                                                                                                                                                |
| ----- | ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6.1.1 | Define `Channel` trait in `src-tauri/src/channels/traits.rs` | Methods: `fn name(&self) -> &str`, `async send(message, recipient) -> Result<()>`, `async listen(tx: mpsc::Sender<ChannelMessage>) -> Result<()>`, `async health_check(&self) -> bool` |
| 6.1.2 | Implement `TauriIpcChannel`                                  | Wraps existing Tauri IPC as a channel. Default implementation                                                                                                                          |
| 6.1.3 | Implement `WebhookChannel` (optional)                        | HTTP listener using axum. Receives POST requests → converts to ChannelMessage → publishes to EventBus                                                                                  |
| 6.1.4 | Implement `ChannelManager`                                   | Lifecycle management: start/stop channels, health monitoring, reconnection                                                                                                             |

---

### Task 6.2: Boot Sequence (P3.23)

| #     | Task                                    | Details                                                                                                                                                                                                                                                        |
| ----- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 6.2.1 | Create `src-tauri/src/services/boot.rs` | `BootSequence` service. Runs on Tauri setup hook                                                                                                                                                                                                               |
| 6.2.2 | Implement startup sequence              | Order: (1) Create `~/.mesoclaw/` directory structure. (2) Load config. (3) Load identity files. (4) Load daily memory. (5) Warm up providers. (6) Start scheduler. (7) Start channels. (8) Execute BOOT.md checklist (if exists). (9) Emit `SystemReady` event |
| 6.2.3 | Wire into Tauri setup                   | In `lib.rs`: call `BootSequence::run()` in the `setup` closure                                                                                                                                                                                                 |

---

### Task 6.3: Frontend — Agent Loop UI (P3.22)

| #     | Task                                   | Details                                                                                                |
| ----- | -------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| 6.3.1 | Create `agentStore.ts`                 | State: current session, tool execution status, approval queue, iteration count                         |
| 6.3.2 | Create `ToolExecutionStatus` component | Shows: tool name, arguments (truncated), spinner while running, result (expandable)                    |
| 6.3.3 | Create `ApprovalOverlay` component     | Dialog: "Agent wants to run `{command}` (risk: {level})" with Allow Once / Always Allow / Deny buttons |
| 6.3.4 | Create `AgentProgress` component       | Shows iteration count, cancel button, expandable execution log                                         |
| 6.3.5 | Listen for Tauri events                | Subscribe to `agent-tool-start`, `agent-tool-result`, `agent-approval-needed`, `agent-complete`        |

---

### Task 6.4: Frontend — Memory Search UI (P3.21)

| #     | Task                             | Details                                                                                                       |
| ----- | -------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 6.4.1 | Create `memoryStore.ts`          | State: search query, results, loading, selected entry                                                         |
| 6.4.2 | Create `MemorySearch` component  | Search input with debounce. Results list showing: key, category, score, preview. Click to expand full content |
| 6.4.3 | Create `DailyTimeline` component | Calendar-style view of daily memory files. Click date → show that day's memory                                |
| 6.4.4 | Add memory route or panel        | Either new route (`/memory`) or sidebar panel in main chat view                                               |

---

### Task 6.5: Frontend — Identity & Scheduler UIs (P3.24, P3.25)

| #     | Task                              | Details                                                                                                     |
| ----- | --------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| 6.5.1 | Create `identityStore.ts`         | State: loaded identity files, editing state                                                                 |
| 6.5.2 | Create `IdentityEditor` component | Markdown editor for each identity file. Preview pane. Save button calls `update_identity_file_command`      |
| 6.5.3 | Create `schedulerStore.ts`        | State: job list, history, creation form                                                                     |
| 6.5.4 | Create `JobList` component        | Table: job name, schedule, next run, status (active/paused), toggle, delete                                 |
| 6.5.5 | Create `CronBuilder` component    | Visual cron expression builder (select minutes, hours, days, etc.). Preview: "Runs every Monday at 9:00 AM" |
| 6.5.6 | Add to Settings route             | New tabs: Identity, Scheduler (alongside existing Provider settings)                                        |

---

### Task 6.6: Module Management Frontend UI

**Risk**: Low | **Dependencies**: 5.5 (Module API), 6.3 (Agent UI patterns) | **Files**: 4 new

| #     | Task                            | Details                                                                               |
| ----- | ------------------------------- | ------------------------------------------------------------------------------------- |
| 6.6.1 | Create `moduleStore.ts`         | State: module list, status, health, selected module details                           |
| 6.6.2 | Create `ModuleList` component   | Card per module: name, type badge, runtime badge, status indicator, start/stop toggle |
| 6.6.3 | Create `ModuleDetail` component | Expand: manifest details, health history, recent executions, logs                     |
| 6.6.4 | Add "Modules" tab to Settings   | Alongside Provider, Identity, Scheduler tabs                                          |
| 6.6.5 | Create `ModuleScaffold` dialog  | UI for `module create`: name, type dropdown, runtime dropdown, generates manifest     |

---

### Task 6.7: Memory Hygiene / Auto-Archiving (P3.18)

**Risk**: Low | **Dependencies**: 3.2 (Memory), 4.1 (Scheduler) | **Files**: 2 new, 1 modified

| #     | Task                                     | Details                                                                              |
| ----- | ---------------------------------------- | ------------------------------------------------------------------------------------ |
| 6.7.1 | Create `src-tauri/src/memory/hygiene.rs` | `MemoryHygiene` service with archive/purge policies                                  |
| 6.7.2 | Implement archive policy                 | Move memory/session artifacts older than 7 days to archive storage                   |
| 6.7.3 | Implement purge policy                   | Delete archived artifacts older than 30 days                                         |
| 6.7.4 | Add scheduler integration                | Run hygiene task daily as a low-priority maintenance job                             |
| 6.7.5 | Add config knobs                         | `memory.hygiene.archive_days`, `memory.hygiene.purge_days`, `memory.hygiene.enabled` |
| 6.7.6 | Write tests                              | Test archive threshold, purge threshold, disabled mode, idempotency                  |

---

### Task 6.8: WASM Extension System Spike (P3.20)

**Risk**: Medium | **Dependencies**: 2.2 (Tools), 2.3 (Security), 2.8 (Modules) | **Files**: 3 new (feature-gated)

| #     | Task                               | Details                                                                   |
| ----- | ---------------------------------- | ------------------------------------------------------------------------- |
| 6.8.1 | Add feature flag `wasm-ext`        | Compile-time optional extension path (off by default)                     |
| 6.8.2 | Create `src-tauri/src/extensions/` | `mod.rs`, `runtime.rs`, `adapter.rs` skeletons for WASM tool hosting      |
| 6.8.3 | Define `WasmToolAdapter` interface | Bridge WASM-exposed tools into `ToolRegistry` without changing agent loop |
| 6.8.4 | Build proof-of-concept tool        | One sandboxed demo tool executed through WASM runtime                     |
| 6.8.5 | Document go/no-go criteria         | Binary size impact, startup cost, sandbox guarantees, DX complexity       |
| 6.8.6 | Write tests                        | Feature-gated compile tests + adapter contract tests                      |

---

### Phase 6 Checkpoint

- [ ] Boot sequence runs on app launch without errors
- [ ] Agent loop UI shows tool execution status and approval overlay
- [ ] Memory search returns relevant results with scores
- [ ] Identity editor saves and hot-reloads
- [ ] Scheduler UI can create/edit/delete jobs
- [ ] Module Management UI: list modules, show status, start/stop, view health history
- [ ] Memory hygiene runs daily and archives/purges according to config
- [ ] WASM extension spike is documented with a clear go/no-go decision
- [ ] Full end-to-end: launch app → agent greets → ask complex question → tools execute → memory stores → heartbeat fires → notification appears

---

## Phase 7: Channels & Mobile

**Goal**: Connect the agent to messaging platforms (Telegram first) and prepare for Tauri Mobile deployment.

### Task 7.1: Telegram Channel Integration (P7.1)

**Risk**: Medium | **Dependencies**: 2.1 (EventBus), 2.3 (Security), 3.1 (Agent Loop), 4.3 (Session Routing) | **Files**: 4 new, 3 modified

| #      | Task                                                 | Details                                                                                                                                                                                                                                                                                                                                                                                                            |
| ------ | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 7.1.1  | Add `teloxide` crate to `Cargo.toml`                 | `teloxide = { version = "0.13", features = ["macros"] }` — mature Rust Telegram bot framework. Alternative: raw `reqwest` calls to Bot API (lighter but more work)                                                                                                                                                                                                                                                 |
| 7.1.2  | Create `src-tauri/src/channels/telegram.rs`          | `TelegramChannel` struct implementing `Channel` trait. Fields: `bot: Bot` (teloxide), `allowed_chat_ids: Vec<i64>`, `polling_timeout: u32`                                                                                                                                                                                                                                                                         |
| 7.1.3  | Implement `listen()` — long-polling message receiver | Spawn Tokio task running `teloxide::dispatching::Dispatcher`. On new message: (1) Check `allowed_chat_ids` — reject unknown senders, (2) Normalize to `ChannelMessage { channel: "telegram", peer: chat_id.to_string(), content: text }`, (3) Publish to EventBus                                                                                                                                                  |
| 7.1.4  | Implement `send()` — message sender                  | Accept response text. Format for Telegram: (1) Convert internal markdown to MarkdownV2 (escape special chars: `.`, `!`, `(`, `)`, etc.), (2) Split long messages at 4096 char boundary (split at paragraph or sentence, not mid-word), (3) Send via `bot.send_message(chat_id, text).parse_mode(MarkdownV2)`                                                                                                       |
| 7.1.5  | Implement Telegram → Agent Loop routing              | EventBus subscriber: on `ChannelMessage` where channel == "telegram": (1) Resolve session via SessionRouter (`main:dm:telegram:{chat_id}`), (2) Create session if doesn't exist, (3) Feed message to Agent Loop, (4) Stream agent response back via `send()`                                                                                                                                                       |
| 7.1.6  | Implement bot token management                       | Store bot token in OS keyring (same as LLM API keys). Frontend UI: text input + "Test Connection" button. Test: call `getMe` API endpoint to verify token                                                                                                                                                                                                                                                          |
| 7.1.7  | Implement `allowed_chat_ids` security                | Config: list of allowed Telegram chat IDs. On unknown sender: ignore message silently (don't reveal bot exists). Admin command: `/allow {chat_id}` from already-allowed chat to add new IDs. Store in config.toml                                                                                                                                                                                                  |
| 7.1.8  | Implement bot commands                               | `/start` — greeting message with agent identity. `/status` — agent status, active jobs, memory stats. `/cancel` — cancel current agent loop for this session. `/help` — list available commands                                                                                                                                                                                                                    |
| 7.1.9  | Handle media messages (photos, documents, voice)     | Photos: download via Bot API, save to temp dir, pass path to agent as context. Documents: download, extract text if possible (PDF, txt), pass to agent. Voice: download audio, note in context (transcription is P3)                                                                                                                                                                                               |
| 7.1.10 | Implement approval routing                           | When agent needs approval (Supervised mode) during a Telegram conversation: (1) Send desktop notification: "Telegram agent wants to run `{command}`", (2) Show approval overlay in desktop app, (3) **Never** send approval buttons to Telegram (security: approvals only via trusted desktop UI), (4) Send "Waiting for approval on desktop..." message to Telegram, (5) On approval/denial, continue in Telegram |
| 7.1.11 | Implement reconnection logic                         | On connection loss: exponential backoff (1s → 2s → 4s → 8s → max 60s). On persistent failure: emit `SystemError` event + desktop notification. Health check: periodic `getMe` call                                                                                                                                                                                                                                 |
| 7.1.12 | Wire into Channel Manager                            | Register TelegramChannel in channel manager. Start on app boot if configured. Stop/restart from settings UI                                                                                                                                                                                                                                                                                                        |
| 7.1.13 | Write tests (minimum 15)                             | Test: message normalization, markdown formatting for Telegram, long message splitting, allowed_chat_ids filtering, reconnection backoff, bot command parsing, approval routing                                                                                                                                                                                                                                     |

---

### Task 7.2: Channel Management Frontend UI

**Risk**: Low | **Dependencies**: 7.1 (Telegram backend) | **Files**: 5 new

| #     | Task                              | Details                                                                                                                                                                  |
| ----- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 7.2.1 | Create `channelStore.ts`          | State: channels list (name, status, connected/disconnected), connection config per channel, message counts                                                               |
| 7.2.2 | Create `ChannelList` component    | Card per channel: icon + name + status indicator (green/red/yellow). "Connect" / "Disconnect" toggle. Click to expand config                                             |
| 7.2.3 | Create `TelegramConfig` component | Fields: bot token (password input), allowed chat IDs (tag input), polling timeout. "Test Connection" button. Instructions: step-by-step BotFather guide with screenshots |
| 7.2.4 | Create `ChannelMessages` view     | Optional: separate view showing messages per channel (Telegram, webhook, etc.). Timeline of inbound/outbound messages. Useful for debugging                              |
| 7.2.5 | Add "Channels" tab to Settings    | Alongside existing Provider, Identity, Scheduler tabs. Shows ChannelList + per-channel config                                                                            |
| 7.2.6 | Add channel status to sidebar     | Small status indicators next to channel names. Green = connected, red = disconnected, yellow = reconnecting                                                              |

---

### Task 7.3: Mobile-Specific Frontend Polish

**Risk**: Low | **Dependencies**: 0.5 (Responsive foundation) | **Files**: ~8 modified

> This task polishes the responsive foundation from Phase 0 for actual mobile deployment.

| #     | Task                                     | Details                                                                                                                                                                                                |
| ----- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 7.3.1 | Implement swipe gestures                 | Swipe right from left edge → open sidebar drawer. Swipe left on sidebar → close. Use `@use-gesture/react` or native touch events. Threshold: 50px horizontal, <30px vertical                           |
| 7.3.2 | Implement pull-to-refresh                | Pull down on message list → load older messages. Visual indicator: spinner at top of list. Disable when already at oldest message                                                                      |
| 7.3.3 | Implement virtual keyboard handling      | Listen to `window.visualViewport.resize` event. When keyboard opens: scroll chat to bottom, resize input area. When keyboard closes: restore layout. Test on iOS Safari + Android Chrome WebView       |
| 7.3.4 | Implement haptic feedback (Tauri Mobile) | Button press: light haptic. Approval action: medium haptic. Error: heavy haptic. Use Tauri's haptics plugin (if available) or platform-specific APIs                                                   |
| 7.3.5 | Implement offline message queuing        | When network unavailable: queue outbound messages in IndexedDB/localStorage. Show "pending" indicator on queued messages. Auto-send when back online. Conflict resolution: server wins for agent state |
| 7.3.6 | Test on actual mobile devices            | iOS: Safari WebView via Tauri iOS. Android: WebView via Tauri Android. Test: gestures, keyboard, safe areas, notifications, channel connections                                                        |
| 7.3.7 | Optimize for mobile performance          | Virtualize long message lists (react-window or @tanstack/virtual). Lazy-load heavy components (code blocks, memory search). Reduce re-renders with `React.memo` on message components                  |

---

### Task 7.4: Tauri Mobile Build & Distribution

**Risk**: Medium | **Dependencies**: 0.5 (Responsive), 7.3 (Mobile polish) | **Files**: Config files + CI workflows

| #     | Task                                       | Details                                                                                                                                                                                                                                             |
| ----- | ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 7.4.1 | Initialize Tauri Mobile targets            | `bun run tauri ios init` + `bun run tauri android init`. Creates Xcode project and Android Gradle project                                                                                                                                           |
| 7.4.2 | Configure iOS build                        | Set bundle identifier (`com.nsrtech.mesoclaw`), app icons (all required sizes), Info.plist (camera, microphone, file access permissions). Configure code signing for development (Apple Developer account). Test on iOS Simulator (iPhone 15, iPad) |
| 7.4.3 | Configure Android build                    | Set package name (`com.nsrtech.mesoclaw`), app icons, AndroidManifest.xml (internet, file access, notification permissions). Configure signing for debug build. Test on Android emulator (Pixel 7, Galaxy Tab)                                      |
| 7.4.4 | Implement mobile push notifications        | iOS: APNs via `tauri-plugin-notification` mobile support. Android: FCM via same plugin. Map heartbeat/cron/approval events to push notifications                                                                                                    |
| 7.4.5 | Handle mobile-specific lifecycle           | iOS: background execution limits (max 30s background task via `beginBackgroundTask`). Android: foreground service for persistent daemon connection. Handle app suspend/resume without losing agent state. Handle low-memory warnings gracefully     |
| 7.4.6 | Tablet-specific layout validation          | Test iPad (landscape + portrait) and Android tablets. Verify 2-column layout activates correctly. Ensure split-view / multitasking works on iPad                                                                                                    |
| 7.4.7 | Mobile-specific Settings                   | Haptic feedback toggle. Push notification preferences. Battery optimization warning (Android). Background refresh settings (iOS)                                                                                                                    |
| 7.4.8 | TestFlight & Internal Testing distribution | iOS: Upload to TestFlight for beta distribution. Android: Internal testing track on Google Play Console. Both require signing certificates                                                                                                          |

---

### Phase 7 Checkpoint

- [ ] Telegram: send message to bot → agent responds → tool calls work → memory stores
- [ ] Telegram: unknown chat_id is silently rejected
- [ ] Telegram: approval needed → desktop notification → approve from desktop → Telegram continues
- [ ] Telegram: long message (>4096 chars) splits correctly
- [ ] Channel UI: connect/disconnect Telegram, see status, configure allowlist
- [ ] Mobile: app runs on iOS simulator and Android emulator
- [ ] Mobile: responsive layout works, gestures work, keyboard handling works
- [ ] Cross-channel: start conversation on desktop → continue in Telegram (memory persists)

---

## Phase 8: CI/CD, Distribution & Community

**Goal**: Automate builds for all platforms, set up release pipelines, and create contribution infrastructure.

### Task 8.1: GitHub Actions — CI Pipeline (Test & Lint)

**Risk**: Low | **Dependencies**: None | **Files**: 3 new in `.github/workflows/`

| #     | Task                               | Details                                                                                                                                                                                                                                                                               |
| ----- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 8.1.1 | Create `.github/workflows/ci.yml`  | Triggers on push to main and all PRs. Matrix: `[ubuntu-latest, macos-latest, windows-latest]`. Steps: checkout → setup Rust (stable) → setup Bun → `cargo fmt --check` → `cargo clippy -- -D warnings` → `cargo test --lib` → `bun install` → `bun run test` → `bunx ultracite check` |
| 8.1.2 | Add Rust caching                   | `swatinem/rust-cache@v2` with workspace `./src-tauri -> target`. Cache key includes platform + Cargo.lock hash                                                                                                                                                                        |
| 8.1.3 | Add Bun caching                    | Cache `node_modules/` and `~/.bun/install/cache` based on `bun.lockb` hash                                                                                                                                                                                                            |
| 8.1.4 | Configure fail-fast: false         | Matrix jobs run independently; one platform failure doesn't stop others                                                                                                                                                                                                               |
| 8.1.5 | Add Linux system dependencies step | Install `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libasound2-dev` on Ubuntu runners                                                                                                                                                                           |

---

### Task 8.2: GitHub Actions — Build & Release Pipeline

**Risk**: Medium | **Dependencies**: 8.1 | **Files**: 2 new in `.github/workflows/`

| #     | Task                                            | Details                                                                                                                                                                                                                   |
| ----- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 8.2.1 | Create `.github/workflows/build.yml` (reusable) | Reusable workflow called by both test-build and release. Inputs: `platform`, `target`, `build-args`, `release-id`, `sign-binaries`. Handles platform-specific dependencies, Rust target installation, and artifact upload |
| 8.2.2 | Define build matrix (8 configurations)          | See platform matrix below. Each config: runner OS, Rust target triple, bundle types                                                                                                                                       |
| 8.2.3 | Create `.github/workflows/release.yml`          | Triggered manually (workflow_dispatch). Reads version from `src-tauri/tauri.conf.json`. Creates draft GitHub Release. Triggers parallel builds via `build.yml` reusable workflow. All artifacts uploaded to release       |
| 8.2.4 | Implement macOS code signing                    | Import Apple certificate from GitHub secrets (`APPLE_CERTIFICATE` base64). Notarize via `APPLE_ID` + `APPLE_TEAM_ID`. Create universal binary combining x86_64 + aarch64                                                  |
| 8.2.5 | Implement Windows code signing                  | Azure Trusted Signing via `trusted-signing-cli` or self-signed certificate. Configure in `tauri.conf.json` signCommand                                                                                                    |
| 8.2.6 | Implement Tauri updater signing                 | Generate keypair: `TAURI_SIGNING_PRIVATE_KEY` + public key in `tauri.conf.json`. Updater endpoints point to GitHub Releases `latest.json`                                                                                 |
| 8.2.7 | Create `.github/workflows/build-test.yml`       | Manual trigger for test builds. Same matrix as release but without signing. Artifacts uploaded with 30-day retention                                                                                                      |
| 8.2.8 | AppImage optimization                           | Remove `libwayland-client.so` from AppImage for broader compatibility. Repackage with `appimagetool --no-appstream`                                                                                                       |

#### Platform Build Matrix

| Platform                   | Runner                 | Target                      | Bundle Types  | Signing               |
| -------------------------- | ---------------------- | --------------------------- | ------------- | --------------------- |
| macOS (Apple Silicon)      | `macos-latest`         | `aarch64-apple-darwin`      | DMG, APP      | Apple notarization    |
| macOS (Intel)              | `macos-latest`         | `x86_64-apple-darwin`       | DMG, APP      | Apple notarization    |
| macOS (Universal)          | `macos-latest`         | Universal binary (lipo)     | DMG, APP      | Apple notarization    |
| Windows (x64)              | `windows-latest`       | `x86_64-pc-windows-msvc`    | MSI, NSIS     | Azure Trusted Signing |
| Windows (ARM64)            | `windows-11-arm`       | `aarch64-pc-windows-msvc`   | MSI, NSIS     | Azure Trusted Signing |
| Linux (x64 — Ubuntu 22.04) | `ubuntu-22.04`         | `x86_64-unknown-linux-gnu`  | DEB           | None                  |
| Linux (x64 — Ubuntu 24.04) | `ubuntu-24.04`         | `x86_64-unknown-linux-gnu`  | AppImage, RPM | None                  |
| Linux (ARM64)              | `ubuntu-24.04` + cross | `aarch64-unknown-linux-gnu` | DEB, AppImage | None                  |

#### Required GitHub Secrets

| Secret                               | Purpose                                   |
| ------------------------------------ | ----------------------------------------- |
| `APPLE_CERTIFICATE`                  | Base64-encoded .p12 certificate           |
| `APPLE_CERTIFICATE_PASSWORD`         | Certificate password                      |
| `APPLE_ID`                           | Apple Developer account email             |
| `APPLE_ID_PASSWORD`                  | App-specific password                     |
| `APPLE_TEAM_ID`                      | Developer team identifier                 |
| `AZURE_CLIENT_ID`                    | Azure service principal (Windows signing) |
| `AZURE_CLIENT_SECRET`                | Azure secret                              |
| `AZURE_TENANT_ID`                    | Azure tenant                              |
| `TAURI_SIGNING_PRIVATE_KEY`          | Updater signing key                       |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Key passphrase                            |

---

### Task 8.3: GitHub Actions — Mobile Builds

**Risk**: Medium | **Dependencies**: 7.4 (Mobile setup) | **Files**: 1 new in `.github/workflows/`

| #     | Task                                  | Details                                                                                                                                                                  |
| ----- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 8.3.1 | Create `.github/workflows/mobile.yml` | Triggered manually. Two jobs: iOS and Android                                                                                                                            |
| 8.3.2 | iOS build job                         | Runner: `macos-latest`. Steps: setup Xcode → `bun run tauri ios build`. Sign with distribution certificate. Upload IPA to TestFlight via `xcrun altool`                  |
| 8.3.3 | Android build job                     | Runner: `ubuntu-latest`. Steps: setup JDK 17 → setup Android SDK → `bun run tauri android build`. Sign APK/AAB with keystore. Upload AAB to Google Play Internal Testing |
| 8.3.4 | Android target architectures          | Build for: `arm64-v8a` (primary), `armeabi-v7a` (legacy 32-bit), `x86_64` (emulators). Use Gradle `splits` or `abiFilters`                                               |
| 8.3.5 | iOS target architectures              | Build for: `arm64` (all modern devices). Simulator: `arm64` (Apple Silicon Macs) + `x86_64` (Intel Macs)                                                                 |

---

### Task 8.4: Automated Workflows (PR Hygiene, Stale, Dependabot)

**Risk**: Low | **Dependencies**: None | **Files**: 5 new in `.github/`

| #     | Task                                      | Details                                                                                                                                                                                                                                                                                                                                                                                          |
| ----- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 8.4.1 | Create `.github/dependabot.yml`           | Weekly updates for Cargo dependencies (limit 5 open PRs, group minor+patch). Weekly updates for GitHub Actions (limit 3 open PRs)                                                                                                                                                                                                                                                                |
| 8.4.2 | Create `.github/labeler.yml`              | Auto-label PRs by changed files. Labels: `core` (src-tauri/src/\*.rs), `frontend` (src/**), `ai` (src-tauri/src/ai/**), `security` (src-tauri/src/security/**), `agent` (src-tauri/src/agent/**), `memory` (src-tauri/src/memory/**), `tools` (src-tauri/src/tools/**), `channels` (src-tauri/src/channels/**), `ci` (.github/**), `docs` (docs/\*\*), `dependencies` (Cargo.toml, package.json) |
| 8.4.3 | Create `.github/workflows/labeler.yml`    | Uses `actions/labeler@v5` to auto-apply labels on PR creation                                                                                                                                                                                                                                                                                                                                    |
| 8.4.4 | Create `.github/workflows/stale.yml`      | Mark issues stale after 60 days of inactivity. Close stale issues after 14 more days. Exempt: `pinned`, `security`, `bug` labels                                                                                                                                                                                                                                                                 |
| 8.4.5 | Create `.github/workflows/pr-hygiene.yml` | Check: PR template filled, linked issue exists, size label applied. Nudge every 12 hours for stale/failing PRs                                                                                                                                                                                                                                                                                   |

---

### Task 8.5: Contribution Infrastructure

**Risk**: Low | **Dependencies**: None | **Files**: 8 new in `.github/` and root

| #     | Task                                                | Details                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ----- | --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 8.5.1 | Create `.github/ISSUE_TEMPLATE/bug_report.yml`      | YAML form: summary, affected component (dropdown: agent/provider/channel/memory/security/tools/ui/cli/gateway), severity (S0-S3), current behavior, expected behavior, steps to reproduce, version, OS, regression check                                                                                                                                                                                                                                                                                                                      |
| 8.5.2 | Create `.github/ISSUE_TEMPLATE/feature_request.yml` | YAML form: summary, problem statement, proposed solution, alternatives considered, acceptance criteria, architecture impact, breaking change (yes/no)                                                                                                                                                                                                                                                                                                                                                                                         |
| 8.5.3 | Create `.github/ISSUE_TEMPLATE/config.yml`          | Disable blank issues. Add contact links: security vulnerability → SECURITY.md, contribution guide → CONTRIBUTING.md                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 8.5.4 | Create `.github/pull_request_template.md`           | Sections: Summary (2-5 bullets), Change type (bug/feature/refactor/docs/security/chore), Linked issue, Validation evidence (cargo fmt, clippy, test commands run), Security impact (4 yes/no questions), Compatibility/migration, Rollback plan                                                                                                                                                                                                                                                                                               |
| 8.5.5 | Create `CONTRIBUTING.md`                            | Setup instructions (clone, install deps, build, test). Collaboration tracks: A (low risk: docs/tests/chore, 1 reviewer), B (medium risk: providers/channels/memory, 1 subsystem reviewer), C (high risk: security/runtime/gateway, 2-pass review). PR Definition of Ready and Done. Naming conventions (snake_case modules, PascalCase types). Commit convention (Conventional Commits). Trait-based extensibility examples (Provider, Channel, Tool). Pre-push hook setup (`cargo fmt --check && cargo clippy -- -D warnings && cargo test`) |
| 8.5.6 | Create `SECURITY.md`                                | Reporting: GitHub Security Advisories (not public issues). Response SLA: acknowledge 48h, assess 1 week, fix 2 weeks critical. Security architecture summary (autonomy levels, sandboxing, injection protection)                                                                                                                                                                                                                                                                                                                              |
| 8.5.7 | Create `.github/CODEOWNERS`                         | Default: project maintainer. High-risk: `src-tauri/src/security/**`, `src-tauri/src/agent/**`, `.github/**` → maintainer. CI/Docs: separate reviewer if applicable                                                                                                                                                                                                                                                                                                                                                                            |
| 8.5.8 | Create `CODE_OF_CONDUCT.md`                         | Contributor Covenant v2.1 (standard)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |

---

### Phase 8 Checkpoint

- [ ] CI pipeline: push to main triggers test suite on all 3 platforms
- [ ] PR pipeline: lint + test + format checks pass before merge
- [ ] Release: manual trigger produces signed binaries for all 8 desktop configurations
- [ ] Mobile: iOS IPA uploads to TestFlight, Android AAB uploads to Play Console
- [ ] Dependabot: weekly PRs for dependency updates
- [ ] Auto-labeling: PRs labeled correctly by changed files
- [ ] Issue templates: bug report and feature request forms work
- [ ] PR template: all sections render correctly
- [ ] CONTRIBUTING.md: complete with setup, conventions, and extensibility examples

---

## Summary: Complete Task Count

| Phase                               | Tasks  | Subtasks | New Files                 | Risk   |
| ----------------------------------- | ------ | -------- | ------------------------- | ------ |
| **Phase 0**: Slim Down + Responsive | 8      | 63       | 10 created, 10+ deleted   | Medium |
| **Phase 1**: Foundation             | 2      | 11       | 2 created                 | Low    |
| **Phase 2**: Core Infrastructure    | 10     | 79       | 39 created                | Medium |
| **Phase 3**: Agent Intelligence     | 3      | 29       | 12 created                | High   |
| **Phase 4**: Proactive Behavior     | 4      | 28       | 11 created                | Medium |
| **Phase 5**: Config & DX            | 6      | 26       | 8 created                 | Medium |
| **Phase 6**: Extensions & UI        | 8      | 42       | 25 created                | Medium |
| **Phase 7**: Channels & Mobile      | 4      | 46       | 17 created                | Medium |
| **Phase 8**: CI/CD & Community      | 5      | 31       | 19 created                | Low    |
| **Total**                           | **49** | **349**  | **~141 new, ~15 deleted** |        |

---

## Cross-Cutting Track: i18n (Frontend Localization) ✅ COMPLETE

**Completed:** 2026-02-17 | All 12 tasks done | 24/24 tests passing

This track runs alongside the phase roadmap and is defined in:
- `docs/plans/2026-02-16-i18n-design.md`
- `docs/plans/2026-02-16-i18n-implementation.md`

Implementation requirements:
- [x] Initialize i18n before React mount.
- [x] Add namespace-based locale files under `src/locales/`.
- [x] Provide Settings-based language override with persistence.
- [x] Ensure route-level UI strings are translation-backed (no hardcoded UI copy in user-facing surfaces).

Acceptance criteria:
- [x] i18n plan tasks are completed and verified.
- [x] Existing UI behavior remains unchanged when locale is `en`.
- [x] Non-English locale switch applies without app restart.
- [x] i18n tests in `docs/test-plan.md` pass.

---

_Document created: February 2026_
_References: docs/claw-ecosystem-analysis.md, docs/mesoclaw-gap-analysis.md_
