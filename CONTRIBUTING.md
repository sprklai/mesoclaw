# Contributing to MesoClaw

Thank you for your interest in contributing to MesoClaw! This guide will help you get started.

## Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/mesoclaw.git
   cd mesoclaw
   ```
3. **Install prerequisites**:
   - Rust 1.85+ (2024 edition)
   - [Bun](https://bun.sh/) (for frontend development)
   - SQLite3 development libraries (see README for platform-specific instructions)

## Branch Naming

Use descriptive branch names with one of these prefixes:

- `feature/` -- new features (e.g., `feature/tui-binary`)
- `fix/` -- bug fixes (e.g., `fix/memory-pagination`)
- `docs/` -- documentation changes (e.g., `docs/api-reference`)

## Phase Gate Protocol

MesoClaw follows a strict phase gate workflow for all non-trivial changes. See [docs/phases.md](docs/phases.md) for the full protocol. In short:

1. **Gate 1 -- Plan**: Write a plan document in `plans/`. Get approval before writing code.
2. **Gate 2 -- Tests**: Write tests first (TDD). Get approval before implementing.
3. **Gate 3 -- Completion**: Implement, pass all checks, present summary for review.

For small bug fixes or documentation updates, a standard PR workflow is sufficient.

## Code Style

MesoClaw follows the conventions documented in [CLAUDE.md](CLAUDE.md). Key points:

- **Error handling**: Use `MesoError` enum (thiserror). Never `Result<T, String>`.
- **Async**: tokio::sync primitives only. Never `std::sync::Mutex` in async paths.
- **Logging**: `tracing` macros only (`info!`, `warn!`, `error!`, `debug!`). Never `println!`.
- **Naming**: `snake_case` (Rust), `camelCase` (TypeScript/Svelte).
- **Imports**: std, then external crates, then internal modules (blank-line separated).
- **SQL**: Parameterized queries only. WAL mode. Migrations in transactions.
- **Testing**: `#[cfg(test)]` in same file. Integration tests in `tests/`.
- **No dead code**: No commented-out code, unused imports, or placeholder stubs.

## Testing Requirements

All PRs must pass these checks locally before submission:

```bash
# Rust checks
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check

# Frontend checks
cd web && bun run test
```

## Commit Messages

- Use **imperative mood** (e.g., "Add memory pagination" not "Added memory pagination")
- Keep the subject line under 72 characters
- One logical change per commit -- do not bundle unrelated changes
- Reference related issues when applicable (e.g., "Fix #42: correct pagination offset")

Examples:
```
Add FTS5 full-text search to memory module
Fix WebSocket reconnection on auth token expiry
Update Tauri to 2.10.3 for tray icon fix
```

## Pull Request Process

### PR Checklist

Before submitting your PR, verify:

- [ ] Tests added or updated for all changed behavior
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `cd web && bun run test` passes (if frontend changes)
- [ ] Documentation updated (if applicable)
- [ ] No breaking changes (or clearly documented in PR description)
- [ ] No secrets or credentials committed

### Review Process

1. Submit your PR with a clear description of the changes and motivation.
2. A maintainer will review your PR, typically within a few business days.
3. Address any requested changes by pushing additional commits (do not force-push during review).
4. Once approved, a maintainer will merge your PR.

### What to Expect

- PRs that add new features should include tests and documentation.
- PRs that fix bugs should include a test that reproduces the bug.
- Large architectural changes should go through the Phase Gate protocol.
- Maintainers may suggest alternative approaches or request changes.

## Reporting Issues

- **Bugs**: Use the [Bug Report](https://github.com/nsrtech/mesoclaw/issues/new?template=bug_report.md) template.
- **Features**: Use the [Feature Request](https://github.com/nsrtech/mesoclaw/issues/new?template=feature_request.md) template.
- **Security**: See [SECURITY.md](SECURITY.md) for responsible disclosure.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold this code.
