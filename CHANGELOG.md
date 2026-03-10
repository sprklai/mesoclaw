# Changelog

All notable changes to MesoClaw will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.9] - 2026-03-10

### Fixed
- Release pipeline: exclude keyring/dbus from embedded cross-builds, fix asset name collisions, disable musl target pending OpenSSL cross-compilation fix

## [0.0.8] - 2026-03-10

### Added
- Plugin system (Phase 9): manifest parser, registry with JSON persistence, JSON-RPC process manager, tool adapter, installer with git integration, and 8 gateway API endpoints

### Fixed
- Clippy warnings breaking CI: duplicate cfg gate on local_embeddings, missing transmute annotations in boot.rs

## [0.0.7] - 2026-03-10

### Fixed
- Security hardening: CORS origins, credential exposure, path traversal, shell injection, error sanitization
- Concurrency hazards: DashMap guard held across await, scheduler double-start race, mutex poisoning (parking_lot)
- Channel reliability: UTF-8 safe message splitting, atomic registry operations, session caching
- First-run UX: AuthGate exponential backoff, chat no-model CTA, WebSocket reconnection
- Config state integrity: ArcSwap for hot-reload, session ordering, frontend config key alignment
- Settings truthfulness: hide phantom features (Perplexity, SerpAPI, Matrix), verify channel status
- Svelte 5 compiler warnings eliminated (19 → 0)
- FTS5 query escaping for special characters
- Skills registry duplicate key rejection
- Release pipeline: macOS code signing bypass, Windows timeout, embedded cross-compilation

### Added
- All-features CI testing (cargo test + clippy with full feature set)
- CODEOWNERS for automated review assignment
- Branch protection ruleset for main

### Changed
- Desktop devtools disabled by default (opt-in via feature flag)
- CSP tightened with script-src restriction
- Embedded builds exclude keyring feature (environment-based credentials)

## [0.0.6] - 2026-03-10

- Fix embedded cross-compilation by adding Cross.toml with OpenSSL pre-build

## [0.0.5] - 2026-03-10

- Enable macOS and embedded builds in release workflow

## [0.0.4] - 2026-03-10

- Initial multi-platform release pipeline (Linux, macOS, Windows, embedded ARM)

## [0.0.3] - 2026-03-09

- Frontend build fixes and Tauri desktop integration

## [0.0.2] - 2026-03-09

- Core library stabilization and test infrastructure

## [0.0.1] - 2026-03-08

### Added

- Core foundation: error handling (MesoError with thiserror), TOML config, SQLite database with WAL mode and migrations
- Memory system with FTS5 full-text search and sqlite-vec vector embeddings
- Security policy enforcement with autonomy levels (supervised/autonomous/strict) and rate limiting
- 11 built-in agent tools: websearch, sysinfo, shell, file read/write/list/search, patch, process, learn, skill_proposal
- AI agent with rig-core supporting 18 providers (OpenAI, Anthropic, Google, Ollama, and more)
- HTTP + WebSocket gateway with axum (74 routes)
- Soul/Persona system with 3 identity files (SOUL/IDENTITY/USER.md) and dynamic prompt composition
- Skills system with bundled + user markdown skills loaded into agent context
- Progressive user learning with SQLite-backed observations, category filtering, and privacy controls
- CLI with 8 command groups (daemon, chat, run, memory, config, key, provider, schedule)
- Shell completions for bash, zsh, fish, and PowerShell via clap_complete
- Svelte 5 SPA frontend with shadcn-svelte, Tailwind CSS v4, and paraglide-js i18n
- Tauri 2 desktop app with system tray, close-to-tray, and window state persistence
- Secure credentials via OS keyring with zeroize memory protection
- Messaging channels: Telegram, Slack, Discord with lifecycle hooks (feature-gated)
- Cron scheduler with automated recurring tasks and notification push (feature-gated)
- Context-aware agent with 3-tier adaptive context injection (Full/Minimal/Summary)
- Self-evolving framework with skill proposals and human-in-the-loop approval
- Cross-platform build system with Docker-based cross-compilation
- CI/CD pipeline with GitHub Actions (PR checks, multi-platform releases)
- Quality gates: workspace lints, cargo audit, banned pattern detection
- Comprehensive documentation: CLI reference, API reference, configuration guide, deployment guide, development guide
