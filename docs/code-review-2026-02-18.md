# Code Review Report (2026-02-18)

## Scope
Reviewed all commits from `1653997` through `09f02f1` on `master`:
- `1653997` fix(backend): stdout flush lint fix
- `370026e` feat(backend): skills -> prompt templates refactor
- `7f60317` merge
- `53a269f` fix(backend): clippy lint cleanup in prompts
- `e71e528` docs
- `a1beb2e` feat(build): release profile size optimizations
- `7a4cd5a` feat(ai): `ReliableProvider` retries/fallback
- `020c318` merge
- `09f02f1` docs

## Findings (Ordered by Severity)

### 1) High: Skill configuration commands now acknowledge writes but never persist them
- Location: `src-tauri/src/commands/skills.rs:61`, `src-tauri/src/commands/skills.rs:71`, `src-tauri/src/commands/skills.rs:82`, `src-tauri/src/commands/skills.rs:104`
- Location: `src-tauri/src/commands/skills.rs:39`
- What happens:
  - `set_skill_enabled_command`, `update_skill_config_command`, `initialize_skill_defaults_command`, and `set_skill_auto_select_command` always return `Ok(())` and do nothing.
  - `get_skill_settings_command` always reconstructs synthetic defaults (`enabled: true`, `auto_select: false`) regardless of prior user actions.
- Impact:
  - Frontend/API callers receive successful responses for writes that are discarded.
  - UI state can drift from persisted backend state expectations and reset on reload.
- Recommendation:
  - Either implement persistence (db/file) or return explicit `Err("not supported")` so callers can gate UX and avoid false success.

### 2) Medium: Skill suggestion endpoint is effectively disabled
- Location: `src-tauri/src/commands/skills.rs:116`
- What happens:
  - `suggest_skills_command` always returns an empty list.
- Impact:
  - Any feature relying on server-side suggestions silently degrades.
- Recommendation:
  - Implement minimal keyword/category matching over prompt metadata, or return explicit not-supported errors until a replacement strategy exists.

### 3) Medium: Release profile optimization commit is currently ineffective in workspace layout
- Location: `src-tauri/Cargo.toml:134`
- Supporting evidence: `cargo check --manifest-path src-tauri/Cargo.toml --lib` emits:
  - `profiles for the non root package will be ignored, specify profiles at the workspace root`
- What happens:
  - `[profile.release]` changes added to `src-tauri/Cargo.toml` are ignored because this crate is a workspace member and profile settings must be defined at workspace root (`Cargo.toml`).
- Impact:
  - Claimed binary-size optimization may not be applied in real release builds.
- Recommendation:
  - Move release profile configuration to workspace root `Cargo.toml`.

### 4) Medium: `ReliableProvider` capability reporting can misrepresent fallback behavior
- Location: `src-tauri/src/ai/providers/reliable.rs:97`, `src-tauri/src/ai/providers/reliable.rs:101`
- What happens:
  - `context_limit()` and `supports_tools()` always proxy the primary provider.
  - If execution falls back to a provider with stricter limits or different feature support, pre-checks based on wrapper metadata may be invalid.
- Impact:
  - Requests accepted under primary limits may fail after fallback switch.
- Recommendation:
  - Expose conservative aggregate capabilities (e.g., minimum context limit across chain; tool support based on fallback policy), or document that fallback providers must be capability-compatible.

## Commit-by-Commit Notes
- `1653997`: Safe lint cleanup; no functional issue found.
- `370026e`: Major simplification; introduces behavioral regressions in skill settings/suggestions (findings #1, #2).
- `7f60317`: Merge-only.
- `53a269f`: Clippy-focused cleanup; no new defect identified.
- `e71e528`: Docs-only.
- `a1beb2e`: Good intent, but ineffective placement in member crate (finding #3).
- `7a4cd5a`: Retry/fallback implementation is functional; metadata consistency gap remains (finding #4).
- `020c318`: Merge-only.
- `09f02f1`: Docs-only.

## Validation Performed
- `cargo check --manifest-path src-tauri/Cargo.toml --lib` (passes)
- `cargo test --manifest-path src-tauri/Cargo.toml --lib reliable::tests -- --nocapture` (4/4 pass)
- `cargo test --manifest-path src-tauri/Cargo.toml reliable -- --nocapture` (fails due to unrelated existing integration test import issues under `src-tauri/tests/`)

## Residual Risk / Test Gaps
- No integration tests validating new prompt-command behavior (`set_*` + `get_*` round trips, suggestion behavior).
- No tests covering fallback capability mismatch scenarios in `ReliableProvider`.
