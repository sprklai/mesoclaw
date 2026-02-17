# Mesoclaw Comprehensive Test Plan

> Unit tests, task tests, phase/wave integration tests, and manual verification procedures.
> Organized by implementation phase matching `docs/implementation-plan.md`.

---

## Test Strategy Overview

### Test Pyramid

```
                    ┌─────────┐
                    │ Manual  │  ~40 scenarios
                    │  Tests  │  Human verification
                ┌───┴─────────┴───┐
                │  Phase / Wave   │  ~30 integration suites
                │  Integration    │  Cross-module, end-to-end
            ┌───┴─────────────────┴───┐
            │    Task Tests           │  ~50 integration tests
            │    (per implementation  │  Module-level integration
            │     task)               │
        ┌───┴─────────────────────────┴───┐
        │       Unit Tests                │  ~450+ tests
        │       (per module/function)     │  Isolated, mocked deps
        └─────────────────────────────────┘
```

### Test Infrastructure

| Layer           | Backend (Rust)                        | Frontend (TypeScript)            |
| --------------- | ------------------------------------- | -------------------------------- |
| **Framework**   | `cargo test` (built-in)               | Vitest                           |
| **Mocking**     | `mockall` crate for trait mocks       | `vitest` mocks / `msw` for HTTP  |
| **Async**       | `tokio::test` macro                   | `async/await` in test functions  |
| **Fixtures**    | `tempfile` crate for temp dirs/files  | In-memory stores                 |
| **Coverage**    | `cargo-tarpaulin` or `cargo-llvm-cov` | `vitest --coverage` (v8)         |
| **Integration** | `axum::test` + `reqwest` for gateway  | Playwright for E2E               |
| **CI**          | `cargo test --lib` in GitHub Actions  | `bun run test` in GitHub Actions |

### Test Naming Convention

```rust
// Backend: module::submodule::tests::test_<scenario>
#[test]
fn test_reliable_provider_retries_on_transient_failure() { ... }

#[test]
fn test_security_policy_blocks_path_traversal_with_double_dots() { ... }

#[tokio::test]
async fn test_agent_loop_stops_at_max_iterations() { ... }
```

```typescript
// Frontend: describe("<Component>") > it("<behavior>")
describe("PromptInput", () => {
  it("submits on Enter key press", () => { ... });
  it("expands to multiline on Shift+Enter", () => { ... });
});
```

### Coverage Targets

| Module              | Target   | Rationale                                         |
| ------------------- | -------- | ------------------------------------------------- |
| `security/`         | **90%+** | Security-critical code needs highest coverage     |
| `agent/`            | **85%+** | Core agent loop is business-critical              |
| `gateway/`          | **80%+** | API contract must be reliable                     |
| `providers/`        | **75%+** | External API wrappers, hard to test without mocks |
| `memory/`           | **80%+** | Data integrity matters                            |
| `tools/`            | **80%+** | Tools execute real operations                     |
| `scheduler/`        | **75%+** | Timing-dependent code                             |
| `identity/`         | **70%+** | File I/O focused                                  |
| Frontend components | **60%+** | UI tests have diminishing returns                 |
| Frontend stores     | **80%+** | State logic should be well-tested                 |

---

## Phase 0: Slim Down + Responsive + CLI Restructure

### Task 0.1: Strict Clippy Lints

#### Unit Tests

| Test ID | Test Name                              | Description                                                             | Type    | Location                   |
| ------- | -------------------------------------- | ----------------------------------------------------------------------- | ------- | -------------------------- |
| U-0.1.1 | `test_no_unsafe_code_in_codebase`      | `cargo clippy -- -D unsafe_code` passes                                 | Lint    | CI only                    |
| U-0.1.2 | `test_clippy_deny_level_lints_pass`    | `cargo clippy -- -D warnings` (deny-level only) passes                  | Lint    | CI only                    |
| U-0.1.3 | `test_feature_flag_sidecars_enabled`   | `sidecars` feature flag enabled → module code compiles                  | Feature | `modules/mod.rs`           |
| U-0.1.4 | `test_feature_flag_sidecars_disabled`  | `sidecars` feature disabled → module code excluded                      | Feature | CI only                    |
| U-0.1.5 | `test_feature_flag_containers_enabled` | `containers` feature enabled → container runtime available              | Feature | `modules/container/mod.rs` |
| U-0.1.6 | `test_feature_flag_default_set`        | Default features include `core`, `cli`, `desktop` (containers optional) | Feature | `Cargo.toml`               |

#### Task Tests

| Test ID | Test Name          | Description                                       | Pass Criteria                                                                 |
| ------- | ------------------ | ------------------------------------------------- | ----------------------------------------------------------------------------- |
| T-0.1.1 | Clippy clean build | Run `cargo clippy -- -D warnings` on entire crate | Zero errors. Warnings allowed for `unwrap_used`/`expect_used` (at warn level) |

---

### Task 0.2: Consolidate AI Providers (async-openai)

#### Unit Tests

| Test ID  | Test Name                                    | Description                                                                  | Location               |
| -------- | -------------------------------------------- | ---------------------------------------------------------------------------- | ---------------------- |
| U-0.2.1  | `test_generic_provider_config_openai`        | GenericProvider constructs correct base_url and auth for OpenAI              | `providers/generic.rs` |
| U-0.2.2  | `test_generic_provider_config_openrouter`    | GenericProvider constructs correct headers (HTTP-Referer) for OpenRouter     | `providers/generic.rs` |
| U-0.2.3  | `test_generic_provider_config_vercel`        | GenericProvider constructs correct base_url for Vercel AI Gateway            | `providers/generic.rs` |
| U-0.2.4  | `test_generic_provider_config_ollama`        | GenericProvider uses localhost:11434 with no auth for Ollama                 | `providers/generic.rs` |
| U-0.2.5  | `test_complete_builds_correct_request`       | CompletionRequest maps correctly to async-openai CreateChatCompletionRequest | `providers/generic.rs` |
| U-0.2.6  | `test_complete_handles_api_error`            | API 401/429/500 responses mapped to descriptive error messages               | `providers/generic.rs` |
| U-0.2.7  | `test_stream_yields_chunks`                  | Mock SSE stream yields CompletionChunk items in correct order                | `providers/generic.rs` |
| U-0.2.8  | `test_stream_handles_connection_drop`        | Streaming gracefully handles mid-stream disconnection                        | `providers/generic.rs` |
| U-0.2.9  | `test_provider_factory_creates_generic`      | Factory function returns GenericProvider for openai/openrouter/vercel config | `providers/mod.rs`     |
| U-0.2.10 | `test_provider_name_returns_configured_name` | `provider_name()` returns the configured provider identifier                 | `providers/generic.rs` |

#### Task Tests

| Test ID | Test Name                           | Description                                                                | Pass Criteria                                                       |
| ------- | ----------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| T-0.2.1 | Provider factory round-trip         | Create provider via factory → call `provider_name()` → verify correct name | All 4 provider types return correct name                            |
| T-0.2.2 | Streaming with Ollama (integration) | Start Ollama locally → create GenericProvider → stream response            | Tokens arrive, response completes. Skip in CI if Ollama unavailable |

---

### Task 0.3: Replace Skills with Prompt Templates

#### Unit Tests

| Test ID | Test Name                                         | Description                                                                      | Location            |
| ------- | ------------------------------------------------- | -------------------------------------------------------------------------------- | ------------------- |
| U-0.3.1 | `test_template_loads_from_embedded`               | Embedded templates (include_str!) load correctly                                 | `prompts/loader.rs` |
| U-0.3.2 | `test_template_loads_from_filesystem`             | Templates in `~/.mesoclaw/prompts/` are discovered and loaded                    | `prompts/loader.rs` |
| U-0.3.3 | `test_filesystem_overrides_embedded`              | Same-named filesystem template takes priority over embedded                      | `prompts/loader.rs` |
| U-0.3.4 | `test_render_substitutes_variables`               | `{{table_name}}` replaced with provided value                                    | `prompts/mod.rs`    |
| U-0.3.5 | `test_render_errors_on_missing_required_variable` | Missing required variable returns clear error, not Tera panic                    | `prompts/mod.rs`    |
| U-0.3.6 | `test_render_ignores_extra_variables`             | Extra variables not in template are silently ignored                             | `prompts/mod.rs`    |
| U-0.3.7 | `test_template_list_returns_all`                  | `list_templates()` returns combined embedded + filesystem templates              | `prompts/loader.rs` |
| U-0.3.8 | `test_template_metadata_parsed`                   | Template name, description, and parameter definitions extracted from frontmatter | `prompts/loader.rs` |

#### Task Tests

| Test ID | Test Name           | Description                                                                               | Pass Criteria                                   |
| ------- | ------------------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------- |
| T-0.3.1 | Template round-trip | Load template → render with variables → verify output contains substituted values         | Rendered string contains all substituted values |
| T-0.3.2 | IPC compatibility   | Call `execute_skill_command` via mock IPC → verify it uses new template system internally | Same function signature, template-based backend |

---

### Task 0.4: Diesel to rusqlite (IF EXECUTED)

#### Unit Tests

| Test ID | Test Name                         | Description                                                      | Location                         |
| ------- | --------------------------------- | ---------------------------------------------------------------- | -------------------------------- |
| U-0.4.1 | `test_database_initialization`    | Database creates all tables on first run                         | `database/mod.rs`                |
| U-0.4.2 | `test_migration_applies_in_order` | SQL migration scripts execute sequentially                       | `database/mod.rs`                |
| U-0.4.3 | `test_settings_crud`              | Create, read, update, delete settings                            | `database/models/settings.rs`    |
| U-0.4.4 | `test_provider_config_crud`       | Create, read, update, delete provider configs                    | `database/models/ai_provider.rs` |
| U-0.4.5 | `test_concurrent_read_access`     | Multiple reads don't deadlock with Arc<Mutex<Connection>>        | `database/mod.rs`                |
| U-0.4.6 | `test_data_migration_from_diesel` | Existing Diesel DB data is correctly migrated to rusqlite schema | `database/migration.rs`          |

#### Task Tests

| Test ID | Test Name         | Description                                                              | Pass Criteria                                               |
| ------- | ----------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------- |
| T-0.4.1 | Data preservation | Copy real app.db → run migration → verify all rows present in new schema | Zero data loss. All settings, providers, sessions preserved |

---

### Task 0.5: Responsive Layout Foundation

#### Unit Tests (Frontend)

| Test ID | Test Name                                      | Description                                              | Location                                    |
| ------- | ---------------------------------------------- | -------------------------------------------------------- | ------------------------------------------- |
| U-0.5.1 | `test_useBreakpoint_returns_xl`                | Hook returns "xl" when window width > 1280px             | `src/hooks/useBreakpoint.test.ts`           |
| U-0.5.2 | `test_useBreakpoint_returns_xs`                | Hook returns "xs" when window width < 640px              | `src/hooks/useBreakpoint.test.ts`           |
| U-0.5.3 | `test_useBreakpoint_updates_on_resize`         | Hook value changes when window resizes across breakpoint | `src/hooks/useBreakpoint.test.ts`           |
| U-0.5.4 | `test_MobileNav_hidden_on_desktop`             | MobileNav component not rendered when breakpoint >= md   | `src/components/MobileNav.test.tsx`         |
| U-0.5.5 | `test_MobileNav_visible_on_mobile`             | MobileNav renders with 4 navigation items on xs/sm       | `src/components/MobileNav.test.tsx`         |
| U-0.5.6 | `test_ResponsiveSidebar_persistent_on_desktop` | Sidebar always visible when breakpoint >= md             | `src/components/ResponsiveSidebar.test.tsx` |
| U-0.5.7 | `test_ResponsiveSidebar_drawer_on_mobile`      | Sidebar renders as overlay drawer on xs/sm               | `src/components/ResponsiveSidebar.test.tsx` |
| U-0.5.8 | `test_touch_targets_minimum_size`              | All interactive elements have min-height/min-width 44px  | Snapshot / style test                       |

#### Task Tests

| Test ID | Test Name                 | Description                                            | Pass Criteria                                                |
| ------- | ------------------------- | ------------------------------------------------------ | ------------------------------------------------------------ |
| T-0.5.1 | Layout at all breakpoints | Render app at 480, 640, 768, 1024, 1280, 1440px widths | No overflow, no broken layouts, correct column count at each |

---

### Task 0.6: Restructure to lib + Two Binaries

#### Task Tests

| Test ID | Test Name               | Description                                   | Pass Criteria          |
| ------- | ----------------------- | --------------------------------------------- | ---------------------- |
| T-0.6.1 | CLI binary compiles     | `cargo build --bin mesoclaw`                  | Exits 0, binary exists |
| T-0.6.2 | Desktop binary compiles | `cargo build --bin mesoclaw-desktop`          | Exits 0, binary exists |
| T-0.6.3 | Tauri dev mode works    | `bun run tauri dev` launches GUI              | Window opens, no crash |
| T-0.6.4 | Shared lib imports      | Both binaries successfully import from lib.rs | No linker errors       |

---

### Task 0.7: Basic CLI Shell

#### Unit Tests

| Test ID | Test Name                             | Description                                                                               | Location     |
| ------- | ------------------------------------- | ----------------------------------------------------------------------------------------- | ------------ |
| U-0.7.1 | `test_cli_parses_agent_status`        | `mesoclaw agent status` parsed as AgentCommand::Status                                    | `bin/cli.rs` |
| U-0.7.2 | `test_cli_parses_one_shot`            | `mesoclaw "analyze this"` parsed as default run command                                   | `bin/cli.rs` |
| U-0.7.3 | `test_cli_parses_raw_flag`            | `--raw` flag sets output mode to Raw                                                      | `bin/cli.rs` |
| U-0.7.4 | `test_cli_parses_json_flag`           | `--json` flag sets output mode to Json                                                    | `bin/cli.rs` |
| U-0.7.5 | `test_cli_help_lists_all_subcommands` | `--help` output contains: daemon, agent, memory, identity, config, schedule, channel, gui | `bin/cli.rs` |

#### Task Tests

| Test ID | Test Name          | Description                             | Pass Criteria                  |
| ------- | ------------------ | --------------------------------------- | ------------------------------ |
| T-0.7.1 | Help output        | `cargo run --bin mesoclaw -- --help`    | Lists all subcommands, exits 0 |
| T-0.7.2 | Version output     | `cargo run --bin mesoclaw -- --version` | Shows version string, exits 0  |
| T-0.7.3 | Unknown subcommand | `cargo run --bin mesoclaw -- invalid`   | Error message, exits non-zero  |

---

### Phase 0 Wave Test (Integration)

> Run after ALL Phase 0 tasks complete. Tests cross-cutting concerns.

| Wave Test ID | Test Name                         | Description                                                        | Pass Criteria                                     |
| ------------ | --------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------- |
| W-0.1        | **Full build from clean**         | `cargo clean && cargo build --release`                             | Both binaries compile, no warnings at deny-level  |
| W-0.2        | **All existing tests pass**       | `cargo test --lib`                                                 | >= 120 tests pass (existing count), zero failures |
| W-0.3        | **Frontend tests pass**           | `bun run test`                                                     | All existing frontend tests pass                  |
| W-0.4        | **LOC reduction verified**        | `tokei src-tauri/src/`                                             | < 5,500 lines (down from 9,152)                   |
| W-0.5        | **CLI + GUI both work**           | Start CLI → verify help. Start GUI → verify window opens.          | Both entry points functional                      |
| W-0.6        | **No regression: chat flow**      | GUI: select provider → enter key → send message → receive response | Streaming response displays correctly             |
| W-0.7        | **No regression: skills/prompts** | GUI: select prompt → fill params → execute → receive response      | Template-based execution works                    |
| W-0.8        | **Responsive layout**             | Resize browser: 480px → 768px → 1024px → 1440px                    | Layout adapts correctly at each breakpoint        |

### Phase 0 Manual Tests

| Manual Test ID | Test Name                     | Steps                                                                                                     | Expected Result                                               |
| -------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| M-0.1          | **Fresh install experience**  | 1. Delete `~/.mesoclaw/`. 2. Run `mesoclaw`. 3. Observe first-run behavior.                               | CLI prompts for provider + API key. Creates config directory. |
| M-0.2          | **Prompt template execution** | 1. Open GUI Settings → Prompts. 2. Select "Schema Explanation". 3. Enter table name "users". 4. Execute.  | LLM response about users table schema                         |
| M-0.3          | **Responsive sidebar**        | 1. Open GUI. 2. Resize window to < 768px. 3. Click hamburger menu. 4. Select nav item. 5. Sidebar closes. | Sidebar opens as drawer, closes on selection                  |
| M-0.4          | **Mobile bottom nav**         | 1. Resize to < 640px. 2. Verify bottom nav visible. 3. Tap each item.                                     | Bottom nav shows 4 icons. Each navigates correctly.           |
| M-0.5          | **CLI help is comprehensive** | 1. Run `mesoclaw --help`. 2. Run `mesoclaw agent --help`. 3. Run `mesoclaw memory --help`.                | Each shows clear subcommands and descriptions                 |

---

## Phase 1: Foundation

### Task 1.1: Release Profile Optimization

#### Task Tests

| Test ID | Test Name             | Description                              | Pass Criteria                                     |
| ------- | --------------------- | ---------------------------------------- | ------------------------------------------------- |
| T-1.1.1 | Binary size reduction | Build release → measure binary size      | < 15 MB (ideally < 10 MB). Document before/after. |
| T-1.1.2 | Release binary runs   | Build release → launch → basic operation | No crashes, startup < 2s                          |

---

### Task 1.2: ReliableProvider Wrapper

#### Unit Tests

| Test ID  | Test Name                                  | Description                                                       | Location                |
| -------- | ------------------------------------------ | ----------------------------------------------------------------- | ----------------------- |
| U-1.2.1  | `test_reliable_succeeds_first_try`         | Primary provider succeeds → returns result, no retry              | `providers/reliable.rs` |
| U-1.2.2  | `test_reliable_retries_on_transient_error` | Primary fails once → retry succeeds → returns result              | `providers/reliable.rs` |
| U-1.2.3  | `test_reliable_retries_max_times`          | Primary fails 3x → all retries exhausted → tries fallback         | `providers/reliable.rs` |
| U-1.2.4  | `test_reliable_fallback_chain`             | Primary fails → fallback 1 fails → fallback 2 succeeds            | `providers/reliable.rs` |
| U-1.2.5  | `test_reliable_all_providers_fail`         | All providers fail → returns clear error with all failure reasons | `providers/reliable.rs` |
| U-1.2.6  | `test_reliable_exponential_backoff_timing` | Verify delays: 1s → 2s → 4s between retries                       | `providers/reliable.rs` |
| U-1.2.7  | `test_reliable_timeout_enforcement`        | Individual attempt exceeds timeout → treated as failure → retry   | `providers/reliable.rs` |
| U-1.2.8  | `test_reliable_stream_retries_from_start`  | Stream fails mid-way → full stream restarted on retry             | `providers/reliable.rs` |
| U-1.2.9  | `test_warmup_success`                      | warmup() pre-establishes connection, subsequent calls faster      | `providers/reliable.rs` |
| U-1.2.10 | `test_warmup_failure_logs_warning`         | warmup() fails → warning logged, no panic, provider still usable  | `providers/reliable.rs` |

#### Task Tests

| Test ID | Test Name                 | Description                                                                                  | Pass Criteria                   |
| ------- | ------------------------- | -------------------------------------------------------------------------------------------- | ------------------------------- |
| T-1.2.1 | Retry with mock provider  | Create MockLLMProvider (fails 2x then succeeds) → wrap in ReliableProvider → call complete() | Returns success after 2 retries |
| T-1.2.2 | Fallback chain with mocks | Primary always fails, fallback succeeds → wrap both                                          | Returns fallback's result       |

---

### Phase 1 Wave Test

| Wave Test ID | Test Name                   | Description                                                 | Pass Criteria                                     |
| ------------ | --------------------------- | ----------------------------------------------------------- | ------------------------------------------------- |
| W-1.1        | **Release binary size**     | `cargo build --release` → `ls -la target/release/mesoclaw`  | < 15 MB                                           |
| W-1.2        | **Retry behavior observed** | Configure wrong API key → send message → observe retry logs | 3 retries visible in logs, then fallback or error |
| W-1.3        | **All tests pass**          | `cargo test --lib`                                          | All new + existing tests pass                     |

### Phase 1 Manual Tests

| Manual Test ID | Steps                                                                                 | Expected Result                                                     |
| -------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| M-1.1          | 1. Set invalid OpenAI API key. 2. Configure Ollama as fallback. 3. Send chat message. | OpenAI fails → retries 3x → falls back to Ollama → response arrives |
| M-1.2          | 1. Build release. 2. Compare binary size to debug build.                              | Release < 15 MB vs debug ~50+ MB                                    |

---

## Phase 2: Core Infrastructure + Gateway

### Task 2.1: Event Bus

#### Unit Tests

| Test ID | Test Name                               | Description                                                                      | Location                    |
| ------- | --------------------------------------- | -------------------------------------------------------------------------------- | --------------------------- |
| U-2.1.1 | `test_publish_subscribe_receives_event` | Publish event → subscriber receives it                                           | `event_bus/tokio_bus.rs`    |
| U-2.1.2 | `test_multiple_subscribers_all_receive` | 3 subscribers → publish 1 event → all 3 receive                                  | `event_bus/tokio_bus.rs`    |
| U-2.1.3 | `test_filtered_subscription_type`       | Subscribe to AgentToolStart only → publish mixed events → only matching received | `event_bus/tokio_bus.rs`    |
| U-2.1.4 | `test_filtered_subscription_session`    | Subscribe filtered by session_id → only that session's events received           | `event_bus/tokio_bus.rs`    |
| U-2.1.5 | `test_channel_capacity_overflow`        | Publish more events than channel capacity → oldest dropped, no hang              | `event_bus/tokio_bus.rs`    |
| U-2.1.6 | `test_subscriber_dropped_no_crash`      | Drop a subscriber → publish events → remaining subscribers unaffected            | `event_bus/tokio_bus.rs`    |
| U-2.1.7 | `test_event_serialization`              | AppEvent serializes to JSON and back                                             | `event_bus/traits.rs`       |
| U-2.1.8 | `test_tauri_bridge_forwards_events`     | Bridge receives event → emits to mock app_handle                                 | `event_bus/tauri_bridge.rs` |

---

### Task 2.2: Tool Trait + Registry

#### Unit Tests

| Test ID  | Test Name                                | Description                                                                   | Location            |
| -------- | ---------------------------------------- | ----------------------------------------------------------------------------- | ------------------- |
| U-2.2.1  | `test_registry_register_and_get`         | Register tool → get by name → returns same tool                               | `tools/registry.rs` |
| U-2.2.2  | `test_registry_get_unknown_returns_none` | Get non-existent tool → returns None                                          | `tools/registry.rs` |
| U-2.2.3  | `test_registry_list_all_tools`           | Register 3 tools → list → returns 3 ToolInfo entries with correct schemas     | `tools/registry.rs` |
| U-2.2.4  | `test_shell_tool_executes_ls`            | ShellTool.execute({"command": "ls /tmp"}) → returns directory listing         | `tools/shell.rs`    |
| U-2.2.5  | `test_shell_tool_returns_exit_code`      | ShellTool with failing command → result.success == false, exit code in output | `tools/shell.rs`    |
| U-2.2.6  | `test_shell_tool_respects_working_dir`   | ShellTool with working_dir → command runs in that directory                   | `tools/shell.rs`    |
| U-2.2.7  | `test_file_read_tool_reads_file`         | Create temp file → FileReadTool.execute → returns contents                    | `tools/file_ops.rs` |
| U-2.2.8  | `test_file_read_tool_nonexistent`        | FileReadTool on missing file → clear error message                            | `tools/file_ops.rs` |
| U-2.2.9  | `test_file_write_tool_creates_file`      | FileWriteTool → file exists with correct content                              | `tools/file_ops.rs` |
| U-2.2.10 | `test_file_list_tool_lists_directory`    | FileListTool on temp dir with 3 files → returns 3 entries                     | `tools/file_ops.rs` |
| U-2.2.11 | `test_file_list_tool_recursive`          | FileListTool with recursive=true → includes subdirectory contents             | `tools/file_ops.rs` |
| U-2.2.12 | `test_tool_schema_valid_json_schema`     | Each built-in tool's parameters_schema() is valid JSON Schema                 | `tools/` all        |

---

### Task 2.3: Security Policy

#### Unit Tests (Minimum 40 — security needs highest coverage)

| Test ID                       | Test Name                               | Description                                                | Location             |
| ----------------------------- | --------------------------------------- | ---------------------------------------------------------- | -------------------- |
| **Command Classification**    |                                         |                                                            |                      |
| U-2.3.1                       | `test_classify_ls_as_low`               | "ls" → Low risk                                            | `security/policy.rs` |
| U-2.3.2                       | `test_classify_cat_as_low`              | "cat foo.txt" → Low risk                                   | `security/policy.rs` |
| U-2.3.3                       | `test_classify_grep_as_low`             | "grep pattern file" → Low risk                             | `security/policy.rs` |
| U-2.3.4                       | `test_classify_git_status_as_low`       | "git status" → Low risk                                    | `security/policy.rs` |
| U-2.3.5                       | `test_classify_git_commit_as_medium`    | "git commit -m msg" → Medium risk                          | `security/policy.rs` |
| U-2.3.6                       | `test_classify_npm_install_as_medium`   | "npm install express" → Medium risk                        | `security/policy.rs` |
| U-2.3.7                       | `test_classify_mkdir_as_medium`         | "mkdir new_dir" → Medium risk                              | `security/policy.rs` |
| U-2.3.8                       | `test_classify_rm_as_high`              | "rm file.txt" → High risk                                  | `security/policy.rs` |
| U-2.3.9                       | `test_classify_rm_rf_as_high`           | "rm -rf dir/" → High risk                                  | `security/policy.rs` |
| U-2.3.10                      | `test_classify_sudo_as_high`            | "sudo anything" → High risk                                | `security/policy.rs` |
| U-2.3.11                      | `test_classify_curl_as_high`            | "curl http://example.com" → High risk                      | `security/policy.rs` |
| U-2.3.12                      | `test_classify_chmod_as_high`           | "chmod 777 file" → High risk                               | `security/policy.rs` |
| **Autonomy Levels**           |                                         |                                                            |                      |
| U-2.3.13                      | `test_readonly_allows_low_risk`         | ReadOnly + Low → Allowed                                   | `security/policy.rs` |
| U-2.3.14                      | `test_readonly_denies_medium_risk`      | ReadOnly + Medium → Denied                                 | `security/policy.rs` |
| U-2.3.15                      | `test_readonly_denies_high_risk`        | ReadOnly + High → Denied                                   | `security/policy.rs` |
| U-2.3.16                      | `test_supervised_allows_low_risk`       | Supervised + Low → Allowed                                 | `security/policy.rs` |
| U-2.3.17                      | `test_supervised_needs_approval_medium` | Supervised + Medium → NeedsApproval                        | `security/policy.rs` |
| U-2.3.18                      | `test_supervised_needs_approval_high`   | Supervised + High → NeedsApproval                          | `security/policy.rs` |
| U-2.3.19                      | `test_full_allows_all_risks`            | Full + any risk → Allowed (within rate limit)              | `security/policy.rs` |
| **Injection Protection**      |                                         |                                                            |                      |
| U-2.3.20                      | `test_blocks_backtick_injection`        | "ls \`rm -rf /\`" → Denied (injection)                     | `security/policy.rs` |
| U-2.3.21                      | `test_blocks_dollar_paren_injection`    | "echo $(cat /etc/passwd)" → Denied                         | `security/policy.rs` |
| U-2.3.22                      | `test_blocks_dollar_brace_injection`    | "echo ${HOME}" → Denied                                    | `security/policy.rs` |
| U-2.3.23                      | `test_blocks_redirect_overwrite`        | "echo data > /etc/hosts" → Denied                          | `security/policy.rs` |
| U-2.3.24                      | `test_blocks_redirect_append`           | "echo data >> ~/.bashrc" → Denied                          | `security/policy.rs` |
| U-2.3.25                      | `test_blocks_pipe_to_dangerous`         | "cat file \| sh" → Denied                                  | `security/policy.rs` |
| U-2.3.26                      | `test_blocks_semicolon_chaining`        | "ls; rm -rf /" → Denied                                    | `security/policy.rs` |
| U-2.3.27                      | `test_blocks_and_chaining`              | "ls && rm -rf /" → Denied (high risk in chain)             | `security/policy.rs` |
| **Path Traversal Prevention** |                                         |                                                            |                      |
| U-2.3.28                      | `test_blocks_double_dot_traversal`      | "../../../etc/passwd" → Denied                             | `security/policy.rs` |
| U-2.3.29                      | `test_blocks_null_byte`                 | "file.txt\0.sh" → Denied                                   | `security/policy.rs` |
| U-2.3.30                      | `test_blocks_url_encoded_traversal`     | "%2e%2e%2f" → Denied                                       | `security/policy.rs` |
| U-2.3.31                      | `test_blocks_symlink_escape`            | Symlink pointing outside workspace → Denied                | `security/policy.rs` |
| U-2.3.32                      | `test_blocks_etc_access`                | "/etc/passwd" → Denied                                     | `security/policy.rs` |
| U-2.3.33                      | `test_blocks_ssh_dir_access`            | "~/.ssh/id_rsa" → Denied                                   | `security/policy.rs` |
| U-2.3.34                      | `test_blocks_aws_dir_access`            | "~/.aws/credentials" → Denied                              | `security/policy.rs` |
| U-2.3.35                      | `test_blocks_gnupg_access`              | "~/.gnupg/private-keys" → Denied                           | `security/policy.rs` |
| U-2.3.36                      | `test_allows_workspace_path`            | File within workspace root → Allowed                       | `security/policy.rs` |
| U-2.3.37                      | `test_allows_absolute_workspace_path`   | Absolute path within workspace → Allowed                   | `security/policy.rs` |
| **Rate Limiting**             |                                         |                                                            |                      |
| U-2.3.38                      | `test_rate_limit_allows_within_window`  | 19 actions in window of 20 → All allowed                   | `security/policy.rs` |
| U-2.3.39                      | `test_rate_limit_denies_over_window`    | 21 actions in window of 20 → 21st denied                   | `security/policy.rs` |
| U-2.3.40                      | `test_rate_limit_resets_after_window`   | 20 actions → wait for window expiry → next action allowed  | `security/policy.rs` |
| **Audit Trail**               |                                         |                                                            |                      |
| U-2.3.41                      | `test_audit_log_records_allowed`        | Allowed action → AuditEntry created with decision=Allowed  | `security/policy.rs` |
| U-2.3.42                      | `test_audit_log_records_denied`         | Denied action → AuditEntry with decision=Denied and reason | `security/policy.rs` |

---

### Task 2.4: Identity System

#### Unit Tests

| Test ID | Test Name                              | Description                                                               | Location             |
| ------- | -------------------------------------- | ------------------------------------------------------------------------- | -------------------- |
| U-2.4.1 | `test_loads_identity_from_filesystem`  | Create temp identity dir with SOUL.md → loads content                     | `identity/loader.rs` |
| U-2.4.2 | `test_falls_back_to_defaults`          | Empty identity dir → loads embedded defaults                              | `identity/loader.rs` |
| U-2.4.3 | `test_creates_defaults_on_first_run`   | No identity dir exists → creates dir + copies defaults                    | `identity/loader.rs` |
| U-2.4.4 | `test_system_prompt_assembly_order`    | Build prompt → verify sections appear in order: SOUL, AGENTS, USER, TOOLS | `identity/mod.rs`    |
| U-2.4.5 | `test_hot_reload_detects_file_change`  | Modify file → hot reload fires → new content loaded                       | `identity/loader.rs` |
| U-2.4.6 | `test_missing_optional_file_not_fatal` | HEARTBEAT.md doesn't exist → loaded as empty string, no error             | `identity/loader.rs` |

---

### Task 2.5: Gateway

#### Unit Tests

| Test ID  | Test Name                         | Description                                                    | Location            |
| -------- | --------------------------------- | -------------------------------------------------------------- | ------------------- |
| U-2.5.1  | `test_health_endpoint_returns_ok` | GET /api/v1/health → 200 OK                                    | `gateway/routes.rs` |
| U-2.5.2  | `test_auth_rejects_missing_token` | GET /api/v1/status without Authorization → 401                 | `gateway/auth.rs`   |
| U-2.5.3  | `test_auth_rejects_wrong_token`   | GET with wrong Bearer token → 401                              | `gateway/auth.rs`   |
| U-2.5.4  | `test_auth_accepts_correct_token` | GET with correct Bearer token → 200                            | `gateway/auth.rs`   |
| U-2.5.5  | `test_agent_session_create`       | POST /agent/sessions → 201 with session_id                     | `gateway/routes.rs` |
| U-2.5.6  | `test_agent_session_list`         | GET /agent/sessions → array of sessions                        | `gateway/routes.rs` |
| U-2.5.7  | `test_agent_session_delete`       | DELETE /agent/sessions/{id} → 200                              | `gateway/routes.rs` |
| U-2.5.8  | `test_memory_search_endpoint`     | GET /memory/search?q=test → results array                      | `gateway/routes.rs` |
| U-2.5.9  | `test_identity_get_endpoint`      | GET /identity/soul → SOUL.md content                           | `gateway/routes.rs` |
| U-2.5.10 | `test_identity_put_endpoint`      | PUT /identity/soul with body → updates file                    | `gateway/routes.rs` |
| U-2.5.11 | `test_websocket_connection`       | Connect to /api/v1/ws → handshake succeeds                     | `gateway/ws.rs`     |
| U-2.5.12 | `test_websocket_subscribe_events` | Subscribe to "agent.\*" → receive matching events              | `gateway/ws.rs`     |
| U-2.5.13 | `test_pid_file_written`           | Start daemon → ~/.mesoclaw/daemon.pid exists with correct JSON | `gateway/daemon.rs` |
| U-2.5.14 | `test_token_file_permissions`     | daemon.token created with mode 0600                            | `gateway/daemon.rs` |
| U-2.5.15 | `test_port_autoincrement`         | Port 18790 occupied → daemon binds to 18791                    | `gateway/daemon.rs` |

---

### Task 2.6: Connect CLI to Gateway

#### Unit Tests

| Test ID | Test Name                       | Description                                               | Location     |
| ------- | ------------------------------- | --------------------------------------------------------- | ------------ |
| U-2.6.1 | `test_cli_reads_pid_file`       | PID file exists → CLI extracts port                       | `bin/cli.rs` |
| U-2.6.2 | `test_cli_reads_token_file`     | Token file exists → CLI extracts token                    | `bin/cli.rs` |
| U-2.6.3 | `test_cli_detects_stdin_pipe`   | Stdin is not TTY → pipe mode activated                    | `bin/cli.rs` |
| U-2.6.4 | `test_raw_output_strips_chrome` | --raw mode → output contains only agent text, no spinners | `bin/cli.rs` |

---

### Task 2.7: Migrate Frontend to Gateway

#### Unit Tests (Frontend)

| Test ID | Test Name                                   | Description                                        | Location                           |
| ------- | ------------------------------------------- | -------------------------------------------------- | ---------------------------------- |
| U-2.7.1 | `test_gateway_client_sends_auth_header`     | GatewayClient includes Bearer token in requests    | `src/lib/gateway-client.test.ts`   |
| U-2.7.2 | `test_gateway_client_handles_401`           | 401 response → clear error "unauthorized"          | `src/lib/gateway-client.test.ts`   |
| U-2.7.3 | `test_gateway_client_handles_network_error` | Fetch fails → "daemon not running" error           | `src/lib/gateway-client.test.ts`   |
| U-2.7.4 | `test_websocket_reconnects`                 | WebSocket drops → auto-reconnect with backoff      | `src/lib/gateway-client.test.ts`   |
| U-2.7.5 | `test_provider_store_uses_gateway`          | providerStore.listProviders() calls GET /providers | `src/stores/providerStore.test.ts` |

---

### Task 2.8: Sidecar Module System Core

#### Unit Tests

| Test ID  | Test Name                                | Description                                                | Location                         |
| -------- | ---------------------------------------- | ---------------------------------------------------------- | -------------------------------- |
| U-2.8.1  | `test_manifest_parses_tool_type`         | Valid tool manifest → ModuleManifest struct                | `modules/manifest.rs`            |
| U-2.8.2  | `test_manifest_parses_service_type`      | Valid service manifest → ModuleManifest                    | `modules/manifest.rs`            |
| U-2.8.3  | `test_manifest_parses_mcp_type`          | Valid MCP manifest → ModuleManifest                        | `modules/manifest.rs`            |
| U-2.8.4  | `test_manifest_rejects_invalid_type`     | `type = "invalid"` → clear error                           | `modules/manifest.rs`            |
| U-2.8.5  | `test_manifest_validates_security`       | Missing security section → defaults applied                | `modules/manifest.rs`            |
| U-2.8.6  | `test_module_discovery_finds_manifests`  | Create temp modules dir with 2 manifests → discovers both  | `modules/mod.rs`                 |
| U-2.8.7  | `test_module_discovery_skips_invalid`    | Dir without manifest.toml → skipped, no error              | `modules/mod.rs`                 |
| U-2.8.8  | `test_sidecar_tool_spawns_process`       | SidecarTool with echo script → spawn → execute → result    | `modules/sidecar_tool.rs`        |
| U-2.8.9  | `test_sidecar_tool_handles_timeout`      | Slow script exceeds timeout → killed, error returned       | `modules/sidecar_tool.rs`        |
| U-2.8.10 | `test_sidecar_tool_handles_crash`        | Script exits non-zero → error with stderr                  | `modules/sidecar_tool.rs`        |
| U-2.8.11 | `test_stdio_json_request_framing`        | Request serialized correctly with id, method, params       | `modules/protocol/stdio_json.rs` |
| U-2.8.12 | `test_stdio_json_response_parsing`       | Valid JSON response parsed → result extracted              | `modules/protocol/stdio_json.rs` |
| U-2.8.13 | `test_stdio_json_error_parsing`          | Error JSON response → error extracted                      | `modules/protocol/stdio_json.rs` |
| U-2.8.14 | `test_module_registers_in_tool_registry` | Discovered module → registered as tool → agent can call    | `modules/mod.rs`                 |
| U-2.8.15 | `test_module_events_emitted`             | Tool execution → ModuleToolStart + ModuleToolResult events | `modules/mod.rs`                 |

---

### Task 2.9: Container Runtime

#### Unit Tests

| Test ID  | Test Name                                | Description                                                 | Location                      |
| -------- | ---------------------------------------- | ----------------------------------------------------------- | ----------------------------- |
| U-2.9.1  | `test_docker_runtime_available`          | Docker installed → is_available() returns true              | `modules/container/docker.rs` |
| U-2.9.2  | `test_podman_runtime_available`          | Podman installed → is_available() returns true              | `modules/container/podman.rs` |
| U-2.9.3  | `test_auto_detect_prefers_podman`        | Both available → selects Podman                             | `modules/container/mod.rs`    |
| U-2.9.4  | `test_auto_detect_falls_back_to_docker`  | Only Docker → selects Docker                                | `modules/container/mod.rs`    |
| U-2.9.5  | `test_auto_detect_falls_back_to_native`  | Neither available → native with warning                     | `modules/container/mod.rs`    |
| U-2.9.6  | `test_config_override_runtime`           | Config says "docker" → uses Docker even if Podman available | `modules/container/mod.rs`    |
| U-2.9.7  | `test_container_config_volumes`          | Manifest volumes → correct mount flags                      | `modules/container/docker.rs` |
| U-2.9.8  | `test_container_config_network_disabled` | `network = false` → `--network=none` flag                   | `modules/container/docker.rs` |
| U-2.9.9  | `test_container_config_memory_limit`     | `max_memory_mb = 512` → `--memory=512m` flag                | `modules/container/docker.rs` |
| U-2.9.10 | `test_container_timeout_kills`           | Container exceeds timeout → forcefully stopped              | `modules/container/mod.rs`    |

---

### Task 2.10: MCP Protocol Client

#### Unit Tests

| Test ID  | Test Name                          | Description                                                            | Location                |
| -------- | ---------------------------------- | ---------------------------------------------------------------------- | ----------------------- |
| U-2.10.1 | `test_mcp_initialize_handshake`    | Send initialize → receive capabilities                                 | `modules/mcp_client.rs` |
| U-2.10.2 | `test_mcp_tools_list_discovery`    | Send tools/list → parse tool definitions                               | `modules/mcp_client.rs` |
| U-2.10.3 | `test_mcp_tools_call_execution`    | Send tools/call → receive result                                       | `modules/mcp_client.rs` |
| U-2.10.4 | `test_mcp_tools_call_error`        | tools/call returns error → propagated correctly                        | `modules/mcp_client.rs` |
| U-2.10.5 | `test_mcp_tool_naming_convention`  | Discovered tool "gmail_send" → registered as "mcp:composio:gmail_send" | `modules/mcp_client.rs` |
| U-2.10.6 | `test_mcp_server_health_check`     | Server responds to ping → healthy                                      | `modules/mcp_client.rs` |
| U-2.10.7 | `test_mcp_server_restart_on_crash` | Server process exits → auto-restart → rediscover tools                 | `modules/mcp_client.rs` |
| U-2.10.8 | `test_mcp_json_rpc_framing`        | Request/response follow JSON-RPC 2.0 spec                              | `modules/mcp_client.rs` |

---

### Phase 2 Wave Test

| Wave Test ID | Test Name                              | Description                                                                                                           | Pass Criteria                                                    |
| ------------ | -------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| W-2.1        | **Gateway starts and responds**        | Start daemon → `curl -H "Authorization: Bearer $(cat ~/.mesoclaw/daemon.token)" http://127.0.0.1:18790/api/v1/health` | Returns `{"status": "ok"}`                                       |
| W-2.2        | **CLI connects to gateway**            | Start daemon → `mesoclaw agent status`                                                                                | Returns session list (may be empty)                              |
| W-2.3        | **CLI REPL streams response**          | Start daemon → `mesoclaw` → type message → observe streaming                                                          | Tokens stream character by character                             |
| W-2.4        | **GUI connects to gateway**            | Start desktop app → send chat message                                                                                 | Response streams via WebSocket (not Tauri IPC)                   |
| W-2.5        | **Tool execution with security**       | CLI: ask agent to read a file → security policy validates → tool executes                                             | File content returned. Approval prompt shown in Supervised mode. |
| W-2.6        | **Event bus connects subsystems**      | Agent executes tool → EventBus publishes AgentToolStart → WebSocket receives event → CLI displays tool status         | Tool indicator visible in CLI output                             |
| W-2.7        | **Identity loads into prompt**         | Edit SOUL.md → start new session → verify agent behavior reflects personality                                         | Agent's tone matches SOUL.md content                             |
| W-2.8        | **All tests pass**                     | `cargo test --lib && bun run test`                                                                                    | Zero failures                                                    |
| W-2.9        | **Module system discovers modules**    | Create module in `~/.mesoclaw/modules/` → restart → verify tool registered                                            | Tool appears in `mesoclaw module list`                           |
| W-2.10       | **SidecarTool executes Python script** | Create Python sidecar → agent calls it → result returned                                                              | Python script output in agent response                           |
| W-2.11       | **MCP server tools discovered**        | Start MCP server module → tools/list → tools appear in registry                                                       | MCP tools callable by agent                                      |
| W-2.12       | **Container runtime executes tool**    | Docker/Podman available → container tool → isolated execution                                                         | Result returned, container cleaned up                            |

### Phase 2 Manual Tests

| Manual Test ID | Steps                                                                                                                                                                          | Expected Result                                                           |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------- |
| M-2.1          | 1. Start daemon. 2. `curl http://127.0.0.1:18790/api/v1/health`. 3. `curl` without auth token.                                                                                 | Health returns OK. No-auth returns 401.                                   |
| M-2.2          | 1. `mesoclaw`. 2. Type "list files in current directory". 3. Observe tool execution. 4. Type "y" to approve.                                                                   | Agent calls `ls`, shows results. Approval prompt appears (if Supervised). |
| M-2.3          | 1. Edit `~/.mesoclaw/identity/SOUL.md` to say "Always respond in haiku format". 2. Start new CLI session. 3. Ask a question.                                                   | Agent responds in haiku format.                                           |
| M-2.4          | 1. Start daemon. 2. Open GUI via `mesoclaw gui`. 3. Send message in GUI. 4. Send message in CLI simultaneously.                                                                | Both work. Both use same gateway. Separate sessions.                      |
| M-2.5          | 1. Ask agent to read `/etc/passwd`. 2. Ask agent to read `~/.ssh/id_rsa`.                                                                                                      | Both denied by SecurityPolicy. Clear error message.                       |
| M-2.6          | 1. `mesoclaw module create test-tool --type tool --runtime python`. 2. Edit main.py to echo input. 3. `mesoclaw module reload`. 4. `mesoclaw "use test-tool to process data"`. | Agent calls test-tool, Python script executes, result shown               |
| M-2.7          | 1. `mesoclaw module list`. 2. Verify all registered modules shown with status.                                                                                                 | Table shows name, type, runtime, status for each module                   |

---

## Phase 3: Agent Intelligence

### Task 3.1: Agent Loop

#### Unit Tests

| Test ID         | Test Name                                   | Description                                                                    | Location               |
| --------------- | ------------------------------------------- | ------------------------------------------------------------------------------ | ---------------------- |
| U-3.1.1         | `test_single_turn_no_tools`                 | Message → LLM responds with text only → loop exits with response               | `agent/loop_.rs`       |
| U-3.1.2         | `test_multi_turn_one_tool_call`             | LLM responds with tool call → tool executes → LLM gets result → final response | `agent/loop_.rs`       |
| U-3.1.3         | `test_multi_turn_three_tool_calls`          | LLM makes 3 sequential tool calls → all execute → final response               | `agent/loop_.rs`       |
| U-3.1.4         | `test_max_iterations_enforced`              | LLM keeps calling tools → stops at max_iterations with warning                 | `agent/loop_.rs`       |
| U-3.1.5         | `test_history_trimming_at_50`               | Add 60 messages → trimming keeps system prompt + first + last 48               | `agent/loop_.rs`       |
| U-3.1.6         | `test_approval_flow_approved`               | Tool needs approval → approval granted → tool executes                         | `agent/loop_.rs`       |
| U-3.1.7         | `test_approval_flow_denied`                 | Tool needs approval → denied → LLM informed "tool denied"                      | `agent/loop_.rs`       |
| U-3.1.8         | `test_approval_flow_timeout`                | Tool needs approval → no response in 30s → treated as denied                   | `agent/loop_.rs`       |
| U-3.1.9         | `test_tool_error_fed_back_to_llm`           | Tool execution fails → error message sent back to LLM as tool result           | `agent/loop_.rs`       |
| U-3.1.10        | `test_cancellation_stops_loop`              | Cancel signal sent → loop exits cleanly, partial results returned              | `agent/loop_.rs`       |
| U-3.1.11        | `test_events_emitted_for_tool_calls`        | Tool call → AgentToolStart and AgentToolResult events emitted                  | `agent/loop_.rs`       |
| U-3.1.12        | `test_identity_injected_into_system_prompt` | Agent loop uses Identity.build_system_prompt() as system message               | `agent/loop_.rs`       |
| **Tool Parser** |                                             |                                                                                |                        |
| U-3.1.13        | `test_parse_openai_tool_call_format`        | JSON `tool_calls` array → Vec<ParsedToolCall>                                  | `agent/tool_parser.rs` |
| U-3.1.14        | `test_parse_xml_tool_call_format`           | `<tool_call>{"name":"...","arguments":{...}}</tool_call>` parsed               | `agent/tool_parser.rs` |
| U-3.1.15        | `test_parse_multiple_tool_calls`            | Response with 2 tool calls → both parsed                                       | `agent/tool_parser.rs` |
| U-3.1.16        | `test_parse_no_tool_calls`                  | Plain text response → empty Vec                                                | `agent/tool_parser.rs` |
| U-3.1.17        | `test_parse_malformed_json_graceful`        | Broken JSON in tool call → error, not panic                                    | `agent/tool_parser.rs` |
| U-3.1.18        | `test_parse_nested_xml_handled`             | XML with nested content → correctly extracted                                  | `agent/tool_parser.rs` |

---

### Task 3.2: Memory System

#### Unit Tests

| Test ID  | Test Name                               | Description                                                                                     | Location               |
| -------- | --------------------------------------- | ----------------------------------------------------------------------------------------------- | ---------------------- |
| U-3.2.1  | `test_store_and_recall_by_keyword`      | Store "PostgreSQL uses MVCC" → recall("MVCC") → found                                           | `memory/sqlite.rs`     |
| U-3.2.2  | `test_store_and_recall_by_semantic`     | Store "the database uses optimistic concurrency" → recall("MVCC") → found via vector similarity | `memory/sqlite.rs`     |
| U-3.2.3  | `test_hybrid_search_scoring`            | Verify final*score = 0.7 * vector + 0.3 \_ bm25                                                 | `memory/sqlite.rs`     |
| U-3.2.4  | `test_recall_respects_limit`            | Store 20 entries → recall with limit=5 → returns 5                                              | `memory/sqlite.rs`     |
| U-3.2.5  | `test_recall_empty_returns_empty`       | Recall from empty memory → empty vec, no error                                                  | `memory/sqlite.rs`     |
| U-3.2.6  | `test_forget_removes_entry`             | Store → forget → recall → not found                                                             | `memory/sqlite.rs`     |
| U-3.2.7  | `test_forget_nonexistent_returns_false` | Forget key that doesn't exist → returns false                                                   | `memory/sqlite.rs`     |
| U-3.2.8  | `test_category_filtering`               | Store in Core and Daily → recall with category=Core → only Core returned                        | `memory/sqlite.rs`     |
| U-3.2.9  | `test_embedding_cache_hit`              | Same text embedded twice → second call uses cache                                               | `memory/embeddings.rs` |
| U-3.2.10 | `test_embedding_cache_lru_eviction`     | Fill cache to max → add one more → oldest evicted                                               | `memory/embeddings.rs` |
| U-3.2.11 | `test_chunker_splits_long_text`         | 2000 token text → chunker produces ~4 chunks of ~512 tokens                                     | `memory/chunker.rs`    |
| U-3.2.12 | `test_chunker_preserves_overlap`        | Adjacent chunks share ~50 tokens of overlap                                                     | `memory/chunker.rs`    |
| U-3.2.13 | `test_chunker_short_text_no_split`      | 100 token text → returns single chunk                                                           | `memory/chunker.rs`    |
| U-3.2.14 | `test_fts5_bm25_scoring`                | Store 3 entries → FTS5 query → results ranked by BM25 relevance                                 | `memory/sqlite.rs`     |
| U-3.2.15 | `test_memory_events_emitted`            | Store → MemoryStored event emitted. Recall → MemoryRecalled event emitted                       | `memory/sqlite.rs`     |

---

### Task 3.3: Daily Memory

#### Unit Tests

| Test ID | Test Name                                | Description                                                         | Location          |
| ------- | ---------------------------------------- | ------------------------------------------------------------------- | ----------------- |
| U-3.3.1 | `test_store_daily_creates_file`          | store_daily("learned X") → file YYYY-MM-DD.md created               | `memory/daily.rs` |
| U-3.3.2 | `test_store_daily_appends`               | Store twice → file has both entries with timestamps                 | `memory/daily.rs` |
| U-3.3.3 | `test_recall_daily_today`                | Store today → recall_daily(today) → returns content                 | `memory/daily.rs` |
| U-3.3.4 | `test_recall_daily_missing_date`         | recall_daily(nonexistent date) → returns None                       | `memory/daily.rs` |
| U-3.3.5 | `test_get_recent_daily`                  | Create today + yesterday files → get_recent_daily() → both returned | `memory/daily.rs` |
| U-3.3.6 | `test_daily_injected_into_system_prompt` | build_system_prompt() includes today + yesterday daily memory       | `identity/mod.rs` |

---

### Phase 3 Wave Test

| Wave Test ID | Test Name                             | Description                                                                                               | Pass Criteria                                                 |
| ------------ | ------------------------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| W-3.1        | **Agent executes multi-step task**    | CLI: "list all .rs files and count lines in each" → agent calls shell tool multiple times → returns table | Correct output with file listing                              |
| W-3.2        | **Agent stores to memory**            | Ask agent to "remember that this project uses Tauri 2" → verify in memory search                          | `mesoclaw memory search "Tauri"` returns the entry            |
| W-3.3        | **Agent recalls from memory**         | Store fact → new session → ask question requiring that fact                                               | Agent uses remembered fact in response                        |
| W-3.4        | **Daily memory persists**             | End session → check `~/.mesoclaw/memory/` for today's file                                                | File exists with session highlights                           |
| W-3.5        | **Security blocks dangerous command** | Ask agent to "delete all files in /tmp"                                                                   | SecurityPolicy blocks `rm` command. Agent informed of denial. |
| W-3.6        | **All tests pass**                    | `cargo test --lib && bun run test`                                                                        | Zero failures, test count > 200                               |

### Phase 3 Manual Tests

| Manual Test ID | Steps                                                                                                                                    | Expected Result                                               |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| M-3.1          | 1. `mesoclaw`. 2. "Read and explain the Cargo.toml in this directory". 3. Approve file read. 4. Observe multi-turn.                      | Agent reads file → explains dependencies → done in 2-3 turns  |
| M-3.2          | 1. `mesoclaw`. 2. "Remember that I prefer functional programming style". 3. `/quit`. 4. `mesoclaw`. 5. "Help me refactor this function". | Agent references the functional programming preference        |
| M-3.3          | 1. `mesoclaw --auto "find all TODO comments in src/"`. 2. Observe.                                                                       | Agent runs grep/find autonomously (no approval), returns list |
| M-3.4          | 1. `echo "fn main() { println!(\"hello\"); }" \| mesoclaw "review this code"`                                                            | Agent receives piped code, provides review                    |
| M-3.5          | 1. Check `~/.mesoclaw/memory/$(date +%Y-%m-%d).md` after a session.                                                                      | File contains timestamped entries from the session            |

---

## Phase 4: Proactive Behavior

### Task 4.1: Scheduler

#### Unit Tests

| Test ID  | Test Name                          | Description                                                 | Location                       |
| -------- | ---------------------------------- | ----------------------------------------------------------- | ------------------------------ |
| U-4.1.1  | `test_cron_parse_every_30_min`     | "_/30 _ \* \* \*" parses correctly                          | `scheduler/cron_parser.rs`     |
| U-4.1.2  | `test_cron_parse_weekday_morning`  | "0 9 \* \* MON-FRI" → next occurrence is correct            | `scheduler/cron_parser.rs`     |
| U-4.1.3  | `test_cron_parse_invalid`          | "invalid" → clear error                                     | `scheduler/cron_parser.rs`     |
| U-4.1.4  | `test_scheduler_add_job`           | Add job → list_jobs includes it                             | `scheduler/tokio_scheduler.rs` |
| U-4.1.5  | `test_scheduler_remove_job`        | Add then remove → list_jobs excludes it                     | `scheduler/tokio_scheduler.rs` |
| U-4.1.6  | `test_scheduler_fires_job`         | Add job with 1s interval → wait 2s → job execution recorded | `scheduler/tokio_scheduler.rs` |
| U-4.1.7  | `test_scheduler_respects_disabled` | Add disabled job → wait → no execution                      | `scheduler/tokio_scheduler.rs` |
| U-4.1.8  | `test_heartbeat_reads_checklist`   | HEARTBEAT.md exists → heartbeat loads checklist items       | `scheduler/tokio_scheduler.rs` |
| U-4.1.9  | `test_stuck_detection`             | Job runs > 120s → flagged as stuck                          | `scheduler/tokio_scheduler.rs` |
| U-4.1.10 | `test_error_backoff`               | Job fails → next run delayed by backoff (30s, 1m, 5m...)    | `scheduler/tokio_scheduler.rs` |
| U-4.1.11 | `test_events_published`            | Job fires → HeartbeatTick or CronFired event on EventBus    | `scheduler/tokio_scheduler.rs` |
| U-4.1.12 | `test_jobs_persist_in_sqlite`      | Add job → restart scheduler → job still listed              | `scheduler/tokio_scheduler.rs` |

---

### Task 4.4: Sidecar Service Module

#### Unit Tests

| Test ID | Test Name                               | Description                                       | Location                     |
| ------- | --------------------------------------- | ------------------------------------------------- | ---------------------------- |
| U-4.4.1 | `test_service_starts_and_health_checks` | Start service → health endpoint returns 200       | `modules/sidecar_service.rs` |
| U-4.4.2 | `test_service_execute_via_http`         | POST to execute endpoint → result returned        | `modules/sidecar_service.rs` |
| U-4.4.3 | `test_service_auto_restart_on_crash`    | Service process dies → auto-restart detected      | `modules/sidecar_service.rs` |
| U-4.4.4 | `test_service_startup_timeout`          | Service doesn't respond within timeout → error    | `modules/sidecar_service.rs` |
| U-4.4.5 | `test_service_graceful_shutdown`        | Stop service → process terminated cleanly         | `modules/sidecar_service.rs` |
| U-4.4.6 | `test_service_registered_after_healthy` | Service healthy → tool registered in ToolRegistry | `modules/sidecar_service.rs` |

---

### Phase 4 Wave Test

| Wave Test ID | Test Name                        | Description                                                                | Pass Criteria                           |
| ------------ | -------------------------------- | -------------------------------------------------------------------------- | --------------------------------------- |
| W-4.1        | **Heartbeat runs on schedule**   | Configure 1-minute heartbeat → wait 2 minutes → verify 2 executions logged | Execution history shows 2 runs          |
| W-4.2        | **Cron job fires**               | `mesoclaw schedule add --cron "* * * * *" --prompt "say hello"` → wait 60s | Job executes, result visible in history |
| W-4.3        | **Desktop notification appears** | Heartbeat completes → notification on desktop                              | OS notification visible                 |
| W-4.4        | **Isolated sessions**            | Heartbeat runs in isolated session → main chat history unaffected          | Main session has no heartbeat messages  |

### Phase 4 Manual Tests

| Manual Test ID | Steps                                                                                                                       | Expected Result                                            |
| -------------- | --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| M-4.1          | 1. `mesoclaw schedule add --cron "*/2 * * * *" --prompt "check system health"`. 2. Wait 4 min. 3. `mesoclaw schedule list`. | Shows 2 executions in history                              |
| M-4.2          | 1. Edit `~/.mesoclaw/identity/HEARTBEAT.md`: add "Check if any test files were modified today". 2. Wait for heartbeat.      | Heartbeat checks for modified test files                   |
| M-4.3          | 1. Minimize GUI to tray. 2. Wait for heartbeat. 3. Observe desktop notification. 4. Click notification.                     | Notification appears. Click opens app to relevant session. |

---

## Phase 5: Config & DX

### Phase 5 Wave Test

| Wave Test ID | Test Name                  | Description                                                                                 | Pass Criteria                                |
| ------------ | -------------------------- | ------------------------------------------------------------------------------------------- | -------------------------------------------- |
| W-5.1        | **TOML config round-trip** | `mesoclaw config set provider.default anthropic` → `mesoclaw config show` → shows anthropic | Value persisted and retrieved                |
| W-5.2        | **Env override**           | `MESOCLAW_PROVIDER=ollama mesoclaw config show` → shows ollama as active                    | Env var takes priority over file             |
| W-5.3        | **Provider routing**       | Configure router: code tasks → claude, general → gpt-4o → ask code question                 | Response comes from Claude (visible in logs) |

---

## Phase 6: Extensions & UI

### Phase 6 Wave Test

| Wave Test ID | Test Name                           | Description                                                            | Pass Criteria                                               |
| ------------ | ----------------------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------- |
| W-6.1        | **Boot sequence completes**         | Fresh start → boot sequence runs all steps → SystemReady event emitted | All identity loaded, providers warmed, scheduler started    |
| W-6.2        | **Agent loop UI shows tool status** | GUI: trigger tool call → UI shows spinner + tool name + result         | Tool execution visible in real-time                         |
| W-6.3        | **Approval overlay works**          | Supervised mode + tool call → overlay appears → approve → continues    | Dialog shows command + risk level. Approve/deny functional. |
| W-6.4        | **Memory search UI**                | GUI: navigate to memory → search "Tauri" → results displayed           | Results with scores, click to expand                        |
| W-6.5        | **Identity editor saves**           | GUI: Settings → Identity → edit SOUL.md → save → start new session     | New personality reflected immediately                       |
| W-6.6        | **Scheduler UI manages jobs**       | GUI: Settings → Scheduler → create job → toggle → delete               | All CRUD operations work                                    |

### Phase 6 Manual Tests

| Manual Test ID | Steps                                                                                           | Expected Result                                                        |
| -------------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| M-6.1          | 1. Start fresh (delete ~/.mesoclaw). 2. Launch GUI. 3. Observe boot.                            | Setup wizard appears. Default identity created. Provider prompt shown. |
| M-6.2          | 1. GUI: send complex request. 2. Watch tool execution indicators. 3. Expand execution log.      | Each tool shows: name, args, spinner, duration, result (expandable)    |
| M-6.3          | 1. GUI: Settings → Identity → Templates. 2. Select "Professional Assistant". 3. Apply. 4. Chat. | Agent adopts professional tone from template                           |

---

## Phase 7: Channels & Mobile

### Task 7.1: Telegram Channel

#### Unit Tests

| Test ID  | Test Name                                   | Description                                                                | Location               |
| -------- | ------------------------------------------- | -------------------------------------------------------------------------- | ---------------------- |
| U-7.1.1  | `test_telegram_message_normalization`       | Telegram Update → ChannelMessage with correct fields                       | `channels/telegram.rs` |
| U-7.1.2  | `test_telegram_allowed_chat_ids`            | Message from allowed chat → accepted                                       | `channels/telegram.rs` |
| U-7.1.3  | `test_telegram_blocks_unknown_chat`         | Message from unknown chat_id → silently rejected                           | `channels/telegram.rs` |
| U-7.1.4  | `test_telegram_markdown_v2_formatting`      | Internal markdown → MarkdownV2 (special chars escaped)                     | `channels/telegram.rs` |
| U-7.1.5  | `test_telegram_long_message_split`          | 5000 char message → split into 2 messages at paragraph boundary            | `channels/telegram.rs` |
| U-7.1.6  | `test_telegram_split_preserves_code_blocks` | Split doesn't break mid-code-block                                         | `channels/telegram.rs` |
| U-7.1.7  | `test_telegram_bot_command_start`           | "/start" message → greeting response                                       | `channels/telegram.rs` |
| U-7.1.8  | `test_telegram_bot_command_status`          | "/status" → returns agent/memory/scheduler status                          | `channels/telegram.rs` |
| U-7.1.9  | `test_telegram_reconnection_backoff`        | Simulate connection failure → backoff delays: 1s, 2s, 4s, 8s... max 60s    | `channels/telegram.rs` |
| U-7.1.10 | `test_telegram_approval_routes_to_desktop`  | Tool needs approval → NOT sent to Telegram, routed to desktop notification | `channels/telegram.rs` |

---

### Phase 7 Wave Test

| Wave Test ID | Test Name                            | Description                                                                               | Pass Criteria                          |
| ------------ | ------------------------------------ | ----------------------------------------------------------------------------------------- | -------------------------------------- |
| W-7.1        | **Telegram end-to-end**              | Configure bot → send message from Telegram → agent responds in Telegram                   | Response appears in Telegram chat      |
| W-7.2        | **Telegram + Desktop memory shared** | Chat in Telegram: "remember X" → Desktop CLI: `mesoclaw memory search "X"`                | Memory entry found from CLI            |
| W-7.3        | **Telegram security**                | Telegram: ask to run command → approval appears on desktop → approve → result in Telegram | Approval never shown in Telegram       |
| W-7.4        | **Mobile responsive**                | Open GUI on iOS simulator → resize to mobile → test layout                                | Single column, bottom nav, no overflow |

### Phase 7 Manual Tests

| Manual Test ID | Steps                                                                                                                      | Expected Result                                                   |
| -------------- | -------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| M-7.1          | 1. Create Telegram bot via @BotFather. 2. `mesoclaw channel connect telegram`. 3. Enter bot token. 4. Send message to bot. | Bot responds with agent's answer                                  |
| M-7.2          | 1. Send 10,000 char message to Telegram bot.                                                                               | Response split into 2-3 Telegram messages. No truncation.         |
| M-7.3          | 1. Send message from unknown Telegram account.                                                                             | No response. No error message. Silent rejection.                  |
| M-7.4          | 1. Open GUI on actual iPhone/Android. 2. Navigate all routes. 3. Send message. 4. Check gestures.                          | All features work. Swipe gestures. Keyboard handling. Safe areas. |

---

## Phase 8: CI/CD, Distribution & Community Tests

### Unit Tests — CI Pipeline

| ID    | Test Name                      | Description                                     | File                            | Pass Criteria                              |
| ----- | ------------------------------ | ----------------------------------------------- | ------------------------------- | ------------------------------------------ |
| U-8.1 | test_ci_yaml_valid_syntax      | Validate CI workflow YAML parses correctly      | `.github/workflows/ci.yml`      | Valid YAML, no syntax errors               |
| U-8.2 | test_release_yaml_valid_syntax | Validate release workflow YAML parses correctly | `.github/workflows/release.yml` | Valid YAML, no syntax errors               |
| U-8.3 | test_dependabot_config_valid   | Validate dependabot.yml structure               | `.github/dependabot.yml`        | Valid configuration                        |
| U-8.4 | test_labeler_config_maps_paths | Verify labeler.yml maps file paths to labels    | `.github/labeler.yml`           | All source directories have label mappings |

### Task Tests — Build Matrix

| ID    | Test Name                              | Description                                                    | Pass Criteria                             |
| ----- | -------------------------------------- | -------------------------------------------------------------- | ----------------------------------------- |
| T-8.1 | test_build_matrix_covers_all_platforms | Verify build matrix includes all 8 desktop configs             | All platforms represented in matrix       |
| T-8.2 | test_release_creates_draft             | Verify release workflow creates draft release with version tag | Draft release exists with correct version |
| T-8.3 | test_mobile_build_ios_succeeds         | iOS build produces IPA artifact                                | IPA file generated                        |
| T-8.4 | test_mobile_build_android_succeeds     | Android build produces AAB artifact                            | AAB file generated for arm64-v8a          |

### Wave Test — W8: CI/CD Pipeline End-to-End

| ID    | Test Name                           | Description                                               | Pass Criteria                   |
| ----- | ----------------------------------- | --------------------------------------------------------- | ------------------------------- |
| W-8.1 | test_pr_triggers_ci                 | Opening a PR triggers CI on all 3 platforms               | CI status checks appear on PR   |
| W-8.2 | test_ci_fails_on_lint_error         | Clippy warning causes CI failure                          | PR blocked from merge           |
| W-8.3 | test_release_produces_all_artifacts | Release pipeline produces all desktop binaries            | 8 platform artifacts in release |
| W-8.4 | test_auto_labeling_works            | PR touching src-tauri/src/security/ gets `security` label | Label applied automatically     |
| W-8.5 | test_dependabot_creates_prs         | Dependabot creates grouped PRs for minor/patch updates    | PRs created with correct labels |

### Manual Tests — Community Infrastructure

| ID    | Test Name                        | Steps                                                                                                                | Expected Result                                                                        |
| ----- | -------------------------------- | -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| M-8.1 | Issue template — Bug Report      | 1. Click "New Issue" on GitHub. 2. Select "Bug Report". 3. Fill all required fields. 4. Submit.                      | Form submits with severity, component, version fields. Labels `bug` applied.           |
| M-8.2 | Issue template — Feature Request | 1. Click "New Issue". 2. Select "Feature Request". 3. Fill all required fields. 4. Submit.                           | Form submits with problem statement, acceptance criteria. Label `enhancement` applied. |
| M-8.3 | PR template auto-fills           | 1. Create branch. 2. Push change. 3. Open PR.                                                                        | Template sections appear: Summary, Change type, Validation, Security impact, Rollback. |
| M-8.4 | CODEOWNERS assigns reviewers     | 1. Open PR modifying `src-tauri/src/security/`. 2. Check reviewer assignment.                                        | Maintainer automatically requested as reviewer.                                        |
| M-8.5 | Blank issues disabled            | 1. Click "New Issue". 2. Try to open blank issue.                                                                    | No blank issue option. Only templates + contact links shown.                           |
| M-8.6 | Mobile build — iOS Simulator     | 1. Run `bun run tauri ios dev`. 2. App opens in iOS Simulator. 3. Verify responsive layout. 4. Test chat.            | Single-column layout, bottom nav visible, chat works.                                  |
| M-8.7 | Mobile build — Android Emulator  | 1. Run `bun run tauri android dev`. 2. App opens in Android emulator. 3. Verify responsive layout. 4. Test chat.     | Same as iOS. Responsive layout adapts.                                                 |
| M-8.8 | Tablet layout — iPad             | 1. Run on iPad simulator (landscape). 2. Verify 2-column layout. 3. Rotate to portrait. 4. Verify sidebar collapses. | Layout transitions correctly between orientations.                                     |

---

## Task Coverage Addendum (Reconciliation)

> Adds task-level coverage targets for implementation tasks that were previously only covered by phase wave tests.

| Task                                  | Added Coverage Target                                                                                                                                                             |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.2 Desktop Notifications             | Unit: event-to-notification mapping, DND filtering, click-to-open route payload. Integration: heartbeat event triggers desktop notification.                                      |
| 4.3 Session Management                | Unit: session key parser/formatter, router resolution, isolated scope enforcement, compaction boundary conditions. Integration: cron/heartbeat sessions do not pollute main chat. |
| 0.8 Cargo Feature Flags               | Compile matrix tests for `core,cli,gateway`, default, and `--all-features` (including sidecars/containers/mcp-client).                                                            |
| 5.1 TOML Configuration                | Unit: load/save/atomic write/env override precedence. Integration: runtime reload path from updated config.                                                                       |
| 5.2 Provider Router                   | Unit: rule selection, alias resolution, fallback routing on provider health failure. Integration: task-type route observed in provider logs.                                      |
| 5.3 Prelude Module                    | Compile tests: prelude exports resolve in downstream modules and no circular import regressions.                                                                                  |
| 5.4 Dual Tool-Call Parser Hardening   | Fuzz/property tests for malformed JSON/XML, partial payloads, and multi-call responses.                                                                                           |
| 5.5 Module CLI Commands               | Unit: clap parsing for all `module` subcommands. Integration: gateway `/api/v1/modules/*` command round-trips.                                                                    |
| 5.6 Tauri Plugin Baseline Hardening   | Integration: plugin init smoke tests (`shell`, `single-instance`, `updater`, `deep-link`), capability permission regression checks, single-instance lock behavior tests.          |
| 6.1 Channel Trait                     | Unit: trait contract compliance for Tauri/Telegram/Webhook adapters. Integration: ChannelManager lifecycle start/stop/reconnect.                                                  |
| 6.2 Boot Sequence                     | Unit: startup ordering invariants. Integration: cold start emits `SystemReady` only after required subsystems initialized.                                                        |
| 6.3 Agent Loop UI                     | Component tests: tool status rendering, approval queue UX, cancel action dispatch.                                                                                                |
| 6.4 Memory Search UI                  | Component/store tests: debounce behavior, score sorting, entry expansion.                                                                                                         |
| 6.5 Identity & Scheduler UIs          | Component/store tests: markdown save flow, cron builder output validity, job toggle/delete actions.                                                                               |
| 6.6 Module Management UI              | Component/store tests: module list/status polling, start/stop actions, detail panel rendering.                                                                                    |
| 6.7 Memory Hygiene                    | Unit: archive and purge thresholds, idempotent maintenance runs, disabled mode no-op.                                                                                             |
| 6.8 WASM Extension Spike              | Feature-gated compile tests and adapter contract tests for `WasmToolAdapter`.                                                                                                     |
| 7.2 Channel Management UI             | Component tests: connect/disconnect controls, validation for Telegram settings fields, status badges.                                                                             |
| 7.3 Mobile Frontend Polish            | E2E mobile tests: gesture thresholds, keyboard resize behavior, offline queue flush on reconnect.                                                                                 |
| 7.4 Tauri Mobile Build & Distribution | CI smoke tests: iOS/Android build artifacts generated with expected target ABIs.                                                                                                  |
| 8.1 CI Pipeline                       | Workflow validation and dry-run checks for lint/test matrix and cache correctness.                                                                                                |
| 8.2 Build & Release Pipeline          | Workflow integration tests for matrix fan-out, artifact naming, release attachment verification.                                                                                  |
| 8.3 Mobile CI Workflow                | Workflow tests for signed IPA/AAB production paths and upload step gating.                                                                                                        |
| 8.4 PR Hygiene Automation             | Workflow tests for labeler, stale rules, template enforcement checks.                                                                                                             |
| 8.5 Contribution Infrastructure       | Repository policy tests: issue templates enforced, CODEOWNERS routing, blank issue disabled.                                                                                      |

---

## End-to-End Acceptance Tests

> Run after all phases complete. These test the full product experience.

| E2E Test ID | Test Name                        | Description                                                                                             | Pass Criteria                                                               |
| ----------- | -------------------------------- | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| E2E-1       | **Developer quick start**        | Fresh install → `mesoclaw` → configure provider → "analyze this project" → agent reads files → responds | Working in < 3 minutes. Agent response references actual project files.     |
| E2E-2       | **GUI non-developer onboarding** | Fresh install → `mesoclaw gui` → setup wizard → first chat → response                                   | Working in < 3 minutes. No terminal knowledge needed.                       |
| E2E-3       | **CLI pipe workflow**            | `git diff \| mesoclaw "review this diff" --raw > review.md`                                             | review.md contains meaningful code review                                   |
| E2E-4       | **Multi-channel continuity**     | Desktop: "I'm working on auth module" → Telegram: "What was I working on?"                              | Telegram agent recalls "auth module" from shared memory                     |
| E2E-5       | **Proactive heartbeat**          | Configure heartbeat with "check for failing tests" → break a test → wait for heartbeat                  | Desktop notification: "test X is failing"                                   |
| E2E-6       | **Session resume**               | `mesoclaw` → multi-turn conversation → `/quit` → `mesoclaw --resume <id>` → continue                    | Full conversation context restored                                          |
| E2E-7       | **Autonomous task**              | `mesoclaw "find and fix all clippy warnings in src/" --auto`                                            | Agent runs clippy → edits files → runs clippy again → reports clean         |
| E2E-8       | **Security boundary**            | Ask agent to "read /etc/shadow" in Full autonomy mode                                                   | Still denied (blocked system directory). SecurityPolicy overrides autonomy. |
| E2E-9       | **Offline resilience**           | Disable network → send message → observe retry → enable network → observe recovery                      | ReliableProvider retries → eventually succeeds when network returns         |
| E2E-10      | **Daily memory cycle**           | Use throughout day → check YYYY-MM-DD.md → start next day → agent references yesterday                  | Daily summary generated. Next-day session includes yesterday's context.     |

---

## Test Execution Commands

### Running Tests

```bash
# ─── Backend ───
cargo test --lib                            # All unit tests
cargo test --lib security                   # Security module only
cargo test --lib -- --nocapture             # With stdout output
cargo test --lib -- test_reliable           # Tests matching pattern

# ─── Frontend ───
bun run test                                # All frontend tests
bun run test:watch                          # Watch mode
bun run test:coverage                       # With coverage report
bun run test -- --grep "PromptInput"        # Specific component

# ─── Integration (Gateway) ───
cargo test --test gateway_integration       # Gateway integration tests
cargo test --test cli_integration           # CLI integration tests

# ─── Coverage ───
cargo tarpaulin --out html --lib            # Backend coverage → tarpaulin-report.html
bun run test:coverage                       # Frontend coverage → coverage/index.html

# ─── Full CI suite ───
cargo clippy -- -D warnings && cargo test --lib && bun run test
```

### CI Pipeline

```yaml
# .github/workflows/test.yml (pseudocode)
jobs:
  backend-tests:
    - cargo clippy -- -D warnings
    - cargo test --lib
    - cargo tarpaulin --out xml (upload to codecov)

  frontend-tests:
    - bun install
    - bun run test
    - bun run test:coverage (upload to codecov)

  integration-tests:
    - cargo build --bin mesoclaw
    - cargo test --test gateway_integration
    - cargo test --test cli_integration

  build-check:
    - cargo build --release --bin mesoclaw
    - cargo build --release --bin mesoclaw-desktop
    - bun run tauri build (matrix: linux, macos, windows)
```

---

## Test Count Summary

| Phase     | Unit Tests | Task Tests | Wave Tests | Manual Tests | Total   |
| --------- | ---------- | ---------- | ---------- | ------------ | ------- |
| Phase 0   | 31         | 12         | 8          | 5            | 56      |
| Phase 1   | 12         | 4          | 3          | 2            | 21      |
| Phase 2   | 105        | 10         | 12         | 7            | 134     |
| Phase 3   | 39         | 4          | 6          | 5            | 54      |
| Phase 4   | 18         | 0          | 4          | 3            | 25      |
| Phase 5   | 0          | 0          | 3          | 0            | 3       |
| Phase 6   | 0          | 0          | 6          | 3            | 9       |
| Phase 7   | 10         | 0          | 4          | 4            | 18      |
| Phase 8   | 4          | 4          | 5          | 8            | 21      |
| E2E       | 0          | 0          | 0          | 10           | 10      |
| **Total** | **219**    | **34**     | **51**     | **47**       | **351** |

Combined with existing 128 tests → target **~500+ tests** at project completion.

---

_Document created: February 2026_
_References: docs/implementation-plan.md, docs/product-requirements.md_
