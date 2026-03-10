# Changelog

All notable changes to MesoClaw will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.4] - 2026-03-10

- v0.0.4

## [0.0.3] - 2026-03-09

- v0.0.3

## [0.0.2] - 2026-03-09

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
