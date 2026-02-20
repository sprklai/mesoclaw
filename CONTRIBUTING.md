# Contributing to MesoClaw

Thank you for your interest in contributing to MesoClaw! This guide covers everything you need to get started.

## Welcome

MesoClaw is an AI-powered desktop application built with Tauri 2 (Rust + React). We welcome contributions of all kinds: bug fixes, new features, documentation improvements, and more.

Before you start, please read our [Code of Conduct](CODE_OF_CONDUCT.md) and [Security Policy](SECURITY.md).

## Quick Start

```bash
# Prerequisites
# - Node.js 20+ and bun
# - Rust 1.75+ (stable)
# - Tauri CLI: cargo install tauri-cli

# Clone and set up
git clone https://github.com/mesoclaw/mesoclaw.git
cd mesoclaw
bun install

# Run in development mode
bun run tauri dev
```

## Risk-Based Contribution Tracks

We use a risk-based review system to balance velocity and safety.

### Track A — Documentation & Configuration (Low Risk)

**What:** README, docs/, CLAUDE.md, .github/ files, tauri.conf.json, package.json dependency bumps

**Process:** Submit PR → 1 maintainer review → merge

**No special requirements.**

### Track B — Frontend & Tests (Medium Risk)

**What:** `src/` TypeScript/React, `src-tauri/src/commands/`, test files

**Process:** Submit PR with validation evidence → 1 frontend/backend reviewer → merge

**Required:**
- `bunx ultracite check` must pass
- New frontend features need a Vitest test
- New Tauri commands need a `cargo test` test

### Track C — Backend Rust & Security (High Risk)

**What:** `src-tauri/src/security/`, `src-tauri/src/ai/`, cryptographic code, IPC surface changes, database migrations

**Process:** Submit PR with detailed validation → 2 reviewers (including 1 security reviewer) → merge

**Required:**
- All 420+ unit tests must pass (`cargo test --lib`)
- New security checks must have unit tests with adversarial inputs
- Database migrations must be backward-compatible or include migration guide
- Discuss approach in an issue before starting large changes

## Development Setup

### System Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| bun | latest | `curl -fsSL https://bun.sh/install \| bash` |
| Tauri CLI | 2.x | `cargo install tauri-cli` |
| Diesel CLI | latest | `cargo install diesel_cli --no-default-features --features sqlite` |

### Linux Additional Dependencies

```bash
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

### Running Tests

```bash
# Backend (Rust)
cd src-tauri && cargo test --lib

# Frontend (TypeScript)
bun run test

# Code quality
bunx ultracite check   # Lint
bunx ultracite fix     # Auto-fix
cargo clippy           # Rust lints
```

## Naming Conventions

### Rust (Backend)

- **Tauri commands:** `*_command` suffix — `get_llm_provider_config_command`
- **Error handling:** All commands return `Result<T, String>`
- **Tests:** In same file with `#[cfg(test)]`, descriptive names

### TypeScript (Frontend)

- **Components:** PascalCase — `UserProfile.tsx`
- **Hooks:** camelCase with `use` prefix — `useAuth.ts`
- **Stores:** camelCase with `Store` suffix — `agentStore.ts`
- **Constants:** UPPER_SNAKE_CASE — `MAX_RETRIES`

## Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:** `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `security`

**Examples:**
```
feat(scheduler): add cron expression validation
fix(memory): prevent data loss on app restart
security(policy): add path traversal protection
docs(contributing): add risk-based tracks
```

## Database Migrations

When adding or changing database schema:

```bash
# Create migration
cd src-tauri
diesel migration generate my_migration_name

# Apply
diesel migration run

# Verify schema.rs updated
git diff src/database/schema.rs
```

Migration files must be backward-compatible where possible. Include a rollback migration.

## Submitting a Pull Request

1. Fork the repo and create your branch from `main`
2. Make your changes following the conventions above
3. Run all relevant tests and paste output in the PR
4. Fill out all sections of the PR template
5. Request review from the appropriate team (see CODEOWNERS)

## Getting Help

- **Discussions:** GitHub Discussions for questions
- **Issues:** Bug reports and feature requests via issue templates
- **Security:** See [SECURITY.md](SECURITY.md) for vulnerability reporting

We aim to review PRs within 48 hours on business days.
