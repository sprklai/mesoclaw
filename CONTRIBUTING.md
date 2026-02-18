# Contributing to MesoClaw

Thank you for your interest in contributing! This guide covers setup, conventions, and the review process.

## Table of Contents

1. [Development Setup](#development-setup)
2. [Running Tests](#running-tests)
3. [Code Conventions](#code-conventions)
4. [Contribution Tracks](#contribution-tracks)
5. [Commit Convention](#commit-convention)
6. [Pull Request Checklist](#pull-request-checklist)
7. [Extending MesoClaw](#extending-mesoclaw)
8. [Pre-push Hook](#pre-push-hook)

---

## Development Setup

**Prerequisites**

| Tool | Version | Install |
|------|---------|---------|
| Rust | stable | `rustup install stable` |
| Bun | latest | https://bun.sh |
| Tauri CLI | v2 | `cargo install tauri-cli` |

```bash
# 1. Clone
git clone https://github.com/rakeshdhote/tauriclaw.git
cd tauriclaw

# 2. Install frontend dependencies
bun install

# 3. Start development (hot reload for both frontend and Rust backend)
bun run tauri dev

# 4. Frontend only (no Rust rebuild)
bun run dev
```

---

## Running Tests

```bash
# Backend — Rust unit tests (367 tests)
cd src-tauri
cargo test --lib

# Backend — specific module
cargo test --lib channels::telegram

# Frontend — Vitest
bun run test
bun run test:watch     # watch mode
bun run test:coverage  # coverage report

# Type check
bun run check

# Format & lint
cargo fmt
bunx ultracite fix
```

---

## Code Conventions

### Rust

- All exported Tauri commands use the `*_command` suffix
- Commands return `Result<T, String>` for consistent frontend error propagation
- Use `tokio::sync` types for async synchronisation; never `std::sync::Mutex` in async code
- Use `async_runtime::spawn` (not `tokio::spawn`) inside Tauri-managed code
- Sensitive data must be zeroized (`zeroize` crate)
- No `unwrap()` in production paths — use `?` or proper error handling

### TypeScript / React

- Function components only (no classes)
- Zustand stores live in `src/stores/`; keep them focused
- Custom hooks in `src/hooks/` with `use` prefix
- Use `cn()` from `@/lib/utils` for conditional Tailwind classes
- No hardcoded UI strings — use i18n keys from `src/locales/`

### Naming

| Scope | Convention | Example |
|-------|-----------|---------|
| Rust module | `snake_case` | `channel_manager` |
| Rust type | `PascalCase` | `TelegramChannel` |
| TS component | `PascalCase` | `ChannelList.tsx` |
| TS hook | `camelCase` + `use` | `useMobileSwipe` |
| TS store | `camelCase` | `channelStore.ts` |
| Constant | `UPPER_SNAKE_CASE` | `MAX_RETRIES` |

---

## Contribution Tracks

### Track A — Low Risk (docs, tests, chore, typo fixes)

- Requires: 1 reviewer (maintainer auto-assigned via CODEOWNERS)
- Process: open PR → CI must pass → merge

### Track B — Medium Risk (providers, channels, memory, frontend features)

- Requires: 1 subsystem reviewer familiar with the area
- Process: open PR → CI → reviewer approval → merge
- Labels: `ai`, `channels`, `memory`, `frontend`

### Track C — High Risk (security, runtime, gateway, agent loop)

- Requires: 2-pass review (maintainer + one additional)
- Process: open draft PR → design discussion → implementation → 2 approvals → merge
- Labels: `security`, `agent`, `core`

---

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short description>

<optional body>

<optional footer>
```

| Type | When |
|------|------|
| `feat` | New feature |
| `fix` | Bug fix |
| `refactor` | Code restructure (no behaviour change) |
| `docs` | Documentation only |
| `test` | Adding/fixing tests |
| `chore` | Dependency bumps, config changes |
| `security` | Security-related fix |
| `perf` | Performance improvement |

**Examples**

```
feat(channels): add Telegram MarkdownV2 escaping

fix(agent): prevent feedback loop on AgentComplete event

chore(deps): bump teloxide to 0.14
```

---

## Pull Request Checklist

Before opening a PR, verify:

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test --lib` passes (all tests green)
- [ ] `bun run check` passes (no TypeScript errors)
- [ ] `bun run test` passes (all frontend tests green)
- [ ] PR description follows the template
- [ ] New behaviour is covered by at least one test
- [ ] No secrets, keys, or credentials in the diff

---

## Extending MesoClaw

### Adding an AI Provider

1. Create `src-tauri/src/ai/providers/<name>.rs`
2. Implement the `LLMProvider` trait:
   ```rust
   #[async_trait]
   impl LLMProvider for MyProvider {
       async fn send(&self, messages: &[Message]) -> Result<Stream, String> { ... }
       fn name(&self) -> &str { "my-provider" }
   }
   ```
3. Register in `src-tauri/src/ai/llm_provider.rs`
4. Add frontend config in `src/stores/llm.ts`

### Adding a Channel

1. Create `src-tauri/src/channels/<name>.rs`
2. Implement the `Channel` async-trait
3. Register in `src-tauri/src/channels/mod.rs` (consider feature-gating)
4. Add UI in `src/components/settings/`

### Adding a Tool

1. Create `src-tauri/src/tools/<name>.rs`
2. Implement the `Tool` trait with `name()`, `description()`, `execute()`
3. Register in the tool registry

---

## Pre-push Hook

Install a pre-push hook to catch issues before CI:

```bash
cat > .git/hooks/pre-push << 'EOF'
#!/bin/sh
set -e
cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cargo test --lib
cd ..
bun run test
EOF
chmod +x .git/hooks/pre-push
```
