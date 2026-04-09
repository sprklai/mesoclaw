# Contributing to Zenii

Zenii is open to small fixes and larger feature work, but they do not need the same process.

## Fast Path Contributions

These can go straight to a pull request:

- Documentation fixes
- Typo and copy edits
- Test improvements
- Small bug fixes with a clear reproduction
- Narrow UI polish that does not change architecture

If the change is small, you do not need a planning ceremony first. Open the PR, explain the problem, and keep the scope tight.

## Feature Work

For non-trivial features, architectural refactors, or behavior changes that affect multiple subsystems:

1. Open an issue or discussion first if the direction is not already obvious
2. Write a short plan before implementation
3. Add or update tests before landing behavior changes
4. Keep the implementation scoped to the approved plan

The goal is not bureaucracy. It is to avoid half-designed feature work in a repo that already spans backend, desktop, CLI, TUI, and docs.

## Development Setup

Prerequisites:

- Rust 1.85+
- Bun
- SQLite development libraries

Clone and verify the repo:

```bash
git clone https://github.com/<your-username>/zenii.git
cd zenii

cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check

cd web && bun run test
```

More detail lives in [docs/development.md](docs/development.md).

## Pull Request Expectations

- One logical change per PR
- Tests for changed behavior when practical
- Documentation updates when user-facing behavior changes
- A clear description of the problem, change, and risk
- No unrelated cleanup mixed into the same PR

## Code Style

Zenii follows the conventions documented in [CLAUDE.md](CLAUDE.md). The practical rules that matter most:

- Use `ZeniiError`-based error handling, not ad hoc strings
- Use `tracing` macros, not `println!`
- Keep async paths on `tokio` primitives
- Avoid dead code, placeholder stubs, and commented-out logic
- Keep SQL parameterized and migrations transactional

## Good Ways to Help

- Tighten docs and examples so first-time users can get value faster
- Add focused tests around existing behavior
- Fix papercuts in install, config, and cross-surface flows
- Improve issue reports with exact steps, logs, and environment details

## Reporting Issues

- Bugs: use the bug report template and include exact reproduction steps
- Features: focus on the user problem first, then propose a shape
- Security issues: follow [SECURITY.md](SECURITY.md)

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).
