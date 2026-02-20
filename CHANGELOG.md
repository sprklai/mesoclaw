# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2026-02-20

### Added

- **Multi-provider LLM chat** - Support for OpenAI, Anthropic, Google AI, Groq, Ollama, Vercel AI Gateway, and OpenRouter with streaming responses via Server-Sent Events.
- **Prompt template skill system** - Filesystem-based markdown skill templates with Tera rendering, stored in `~/.mesoclaw/prompts/` and hot-reloaded on file change.
- **Gateway REST API and WebSocket event stream** - Local HTTP control plane at `127.0.0.1:18790` with bearer token authentication; any HTTP client can interact with the running daemon.
- **Channel integrations** - Telegram bot (long-polling, MarkdownV2, allowed-chat allowlist), Discord, Slack, and HTTP webhook listener for external integrations.
- **Scheduler** - Heartbeat and cron job support with configurable intervals and stuck-task detection.
- **Sidecar module system** - Extend the agent with native processes (`SidecarTool`), long-lived HTTP services (`SidecarService`), and MCP-compatible tool servers (`McpServer`), with optional Docker/Podman container isolation.
- **Identity system** - Editable markdown files (`SOUL.md`, `USER.md`, `AGENTS.md`, `HEARTBEAT.md`, `BOOT.md`, `TOOLS.md`) define agent personality; hot-reloaded without restart.
- **Memory system** - SQLite-backed vector and FTS5/BM25 hybrid search for long-term agent memory with daily summary files.
- **Multi-turn agent loop** - Tool system with dual JSON/XML tool-call parser, configurable max iterations, and conversation history trimming.
- **Log viewer** - In-app log viewer page with level filtering, text search, live tail mode with auto-refresh, scroll-to-bottom button, and ascending sort.
- **Onboarding flow** - First-launch setup sequence to guide users through provider configuration and initial identity setup.
- **CLI binary** - Headless `mesoclaw` CLI binary (via Clap) that connects to the same gateway as the desktop app for terminal-based interaction.
- **Dark/light theme** - Respects `prefers-color-scheme` with a manual override toggle.
- **System tray and native notifications** - Tauri tray icon integration and OS notification support via `tauri-plugin-notification`.
- **Autostart** - Optional launch-on-login via `tauri-plugin-autostart` (macOS, Windows, Linux).
- **TOML configuration** - Single `~/.mesoclaw/config.toml` with environment variable overrides and atomic save (write-to-temp, fsync, backup, rename).
- **Branding sync** - Centralized `branding.config.json` with a `bun run branding:sync` script to propagate product name, slug, and identifiers across all project files.

### Security

- **OS keyring credential storage** - API keys stored exclusively in the OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service) via the `keyring` crate; never written to disk and zeroized from memory after use (`zeroize` crate).
- **Three-level autonomy policy** - `readOnly`, `supervised` (default), and `full` modes controlling which shell commands the agent may execute; blocked commands (`rm`, `sudo`, `dd`, `mkfs`, `shutdown`) are denied in all modes.
- **Shell injection prevention** - Backticks, `$()`, `${}`, `>`, `>>`, pipe splitting, and other injection patterns in LLM-provided command strings are always blocked.
- **Filesystem sandboxing** - File access restricted to the workspace; paths under `/etc`, `/root`, `~/.ssh`, `~/.aws`, and `~/.gnupg` are blocked with path traversal prevention.
- **Diesel ORM parameterized queries** - All database queries use Diesel's type-safe query DSL; no raw string concatenation in SQL.
- **Rate limiting** - Sliding-window rate limit (20 tool actions/hour default) applied to `full` autonomy mode; configurable per deployment.
- **Tool execution audit trail** - All tool calls logged with timestamp, arguments, and result to `~/.mesoclaw/logs/audit.jsonl`.
- **Content Security Policy** - Tauri CSP configured to restrict WebView resource loading to local origins.

[Unreleased]: https://github.com/sprklai/mesoclaw/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/sprklai/mesoclaw/releases/tag/v0.0.1
