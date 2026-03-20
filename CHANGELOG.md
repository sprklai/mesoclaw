# Changelog

All notable changes to Zenii will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.33] - 2026-03-20

### Added
- Delegation mode for CLI chat with workflows enabled by default
- Real-time workflow execution visibility via WebSocket streaming
- WorkflowTool exposing workflow system to AI agent with context plugin
- Update/modify capabilities for workflows and scheduler
- Edit and re-execute for user messages in chat UI

### Fixed
- Windows boot detection using IPC with diagnostic logging
- macOS release timeout increased from 30m to 45m

### Changed
- Reduced vertical padding across UI components

### Docs
- Delegation system flow sequence diagram

## [0.0.32] - 2026-03-20

### Added
- Workflow Engine: multi-step automation pipelines defined in TOML with DAG-based execution (petgraph), 5 step types (tool, LLM, condition, parallel, delay), minijinja template resolution between steps, retry/timeout/failure policies, DB-persisted run history, scheduler integration, and 7 API endpoints (feature-gated: `--features workflows`)
- Agent Delegation: parallel sub-agent execution for complex tasks with LLM-powered task decomposition, isolated sessions per sub-agent, tool filtering via allowlist, dependency-based wave execution, token budget and timeout controls, real-time event bus updates, cancel support, and 2 API endpoints
- Multi-agent state display with real-time delegation tracking in frontend
- Copy and retry action buttons on user messages in chat UI

### Fixed
- Windows startup failure + cross-platform compatibility audit

### Style
- Applied cargo fmt across workspace

## [0.0.31] - 2026-03-20

### Added
- Human schedule type for scheduler with cron timezone fix and auto-cleanup of one-time jobs
- Unified diagnostic file logging across all binaries (desktop, daemon, CLI, TUI)
- Session list refresh button with auto-reload on focus
- Documentation link in sidebar

### Fixed
- Whole-app workflow audit: security, agent safety, lifecycle, and cleanup improvements
- Channel broadcast for scheduler/notifications, replaced datetime-local input
- Scheduler execution audit: publish agent responses, parallel jobs, error backoff
- Shell command permissions in chat: OS-aware security, expanded risk lists, surface-based tool filtering

### Docs
- Documented unified diagnostic logging across all binaries

## [0.0.30] - 2026-03-19

### Added
- FileCredentialStore: AES-256-GCM encrypted file-based credential storage as persistent fallback when OS keyring is unavailable
- Credential fallback chain: KeyringStore → FileCredentialStore → InMemoryCredentialStore
- `credential_file_path` config option to override default credential file location

### Fixed
- API keys no longer lost on macOS after binary recompilation or app update (keyring signature revocation now falls back to encrypted file instead of volatile RAM)

### Changed
- Updated docs-site with FileCredentialStore fallback chain documentation
- Updated README with FileCredentialStore fallback chain details

### CI
- Added GitHub Actions workflow to auto-deploy docs-site to Vercel
- Fixed Vercel deployment pipeline to use pull/build/deploy --prebuilt pattern

## [0.0.29] - 2026-03-19

### Fixed
- Onboarding no longer re-triggers when API keys are missing — only `user_name` and `user_location` gate setup
- Auto-migration for pre-v0.0.28 configs: infers completed onboarding from filled profile fields

### Changed
- Updated competitive comparison table in README with NemoClaw

## [0.0.28] - 2026-03-18

### Fixed
- Prevent onboarding re-trigger on restart with persistent `onboarding_completed` flag in config.toml
- Works across all platforms: macOS Keychain revocation, Linux in-memory fallback, Windows
- Chat view shows actionable banner when API key is missing instead of re-running full wizard

## [0.0.27] - 2026-03-18

### Fixed
- Onboarding re-trigger bug: log credential access errors instead of silently swallowing them
- Onboarding wizard now auto-skips to first incomplete step when re-triggered
- Error handling added to model selection step in onboarding wizard
- Improved keyring fallback warning with persistence and macOS code-signature hints
- Added setup status diagnostic logging for debugging onboarding flow

### Changed
- Updated AI provider models: added GPT-5.4 family, Gemini 3.x, expanded OpenRouter and Vercel AI Gateway
- Updated Anthropic model context limits (Opus/Sonnet 4.6 to 1M tokens)
- Added provider documentation links in AI Providers settings page
- LICENSE updated

## [0.0.26] - 2026-03-18

### Fixed
- Correct boot.rs test assertions for CI feature unification

### Changed
- Updated README
- Updated Tauri ACL manifests schema for latest plugin versions
- Added provider setup description in Settings AI Providers page (matching onboarding step 1)

## [0.0.25] - 2026-03-17

### Fixed
- Removed redundant "No API key" warning from onboarding step 1 (still shown in Settings)
- Replaced confusing blue "missing key" warning in step 2 with green model confirmation message

### Changed
- Added custom provider guidance note in onboarding step 1 pointing to "+ Add Provider" button
- Updated contact email in CODE_OF_CONDUCT.md and SECURITY.md

## [0.0.24] - 2026-03-17

### Added
- Top navigation arrows (chevron left/right) in onboarding wizard step indicator
- Clickable completed-step circles to jump back to previous steps
- Auto-select first available model when entering step 2

### Changed
- Replaced hardcoded Anthropic API key warning with provider-agnostic guidance message that reads the actual default provider from config

## [0.0.23] - 2026-03-16

### Changed
- README Quick Start now leads with GitHub Release installer download table (Linux, macOS, Windows, ARM)
- "Getting Started" section renamed to "Building from Source"
- Website and docs header links displayed on separate lines

## [0.0.22] - 2026-03-16

### Added
- Disclaimer to onboarding, about dialog, and README
- Docusaurus documentation site with frontmatter for all docs
- Install script and updated README with quick start guide
- Architecture diagram images (system architecture, 6 layers of defense)
- Documentation badge in README
- Comprehensive docs site landing page with feature tables and quick links

### Fixed
- Incorrect API fields, org names, and URLs across all documentation
- Use `| bash` instead of `| sh` in install instructions

### Changed
- README hero text revised for broader audience ("Powered by Rust" instead of "Just Rust")
- Replaced mermaid architecture diagram in README with PNG image
- Dependency updates:
  - crossterm 0.28.1 → 0.29.0
  - rusqlite 0.38.0 → 0.39.0
  - dialoguer 0.11.0 → 0.12.0
  - ratatui 0.29.0 → 0.30.0
  - pulldown-cmark 0.12.2 → 0.13.1
  - vite 7.3.1 → 8.0.0
  - jsdom 28.1.0 → 29.0.0
  - @lucide/svelte 0.561.0 → 0.577.0
  - @sveltejs/vite-plugin-svelte → 7.0.0

## [0.0.21] - 2026-03-15

### Added
- Token usage logging with date-rotated JSONL files for tracking AI API consumption
- Browse official plugins UI and plugin integration tests

## [0.0.20] - 2026-03-14

### Added
- Channels setup step in onboarding wizard across Desktop, CLI, and TUI (Telegram, Slack, Discord)

### Changed
- `/ship` command now includes automatic documentation update step before committing

## [0.0.19] - 2026-03-14

### Added
- Plugin UI enabled across Desktop/Web (removed experimental placeholder) and TUI (new PluginList mode with table view and keybindings)

### Changed
- Updated architecture and process documentation for plugin system across all interfaces

## [0.0.18] - 2026-03-14

### Added
- Onboarding wizard across Desktop, CLI, and TUI with provider selection and API key setup
- LLM-based automatic fact extraction from conversations

### Changed
- UI polish — sidebar, settings, and consistency improvements
- README comparison table trimmed to OpenClaw and ZeroClaw only

### Fixed
- Ignore fastembed tests that require ONNX model download in CI

## [0.0.17] - 2026-03-13

### Added
- Top-level `zenii` crate with workspace metadata for crates.io publishing
- Workspace-level repository, homepage, keywords, categories fields across all crates

### Changed
- macOS release builds now produce separate arm64 and x86_64 binaries instead of universal (lipo) binaries
- README rewritten with comparison table, pain-point framing, and "What Zenii is NOT" section

### Fixed
- Window close and tray quit now both fully exit the app (no more background orphan processes)

## [0.0.16] - 2026-03-13

### Changed
- Full rebrand from MesoClaw to Zenii across entire codebase
- Renamed all crate directories (`mesoclaw-*` → `zenii-*`) and package names
- Updated binary names, config paths, keyring service IDs, and desktop app metadata
- Updated CI/CD workflows, documentation, scripts, and frontend references
- Replaced obsolete `logo.png` with new Zenii branding assets

## [0.0.15] - 2026-03-13

### Added
- Per-tool call limits in ToolCallCache with canonical argument hashing for deduplication
- Contacts module for channel message routing with contact resolution
- ChannelSendTool expanded with contact lookup and `list_contacts` action
- Shimmer loading animation component for active tool execution in chat UI
- Tool permission system with tool dedup cache

### Changed
- Telegram formatter improvements for paragraph spacing
- Context injection and prompt refinements for better agent behavior
- Desktop branding updates (app name, desktop entry)
- Sanitized example names in docs and tests

## [0.0.14] - 2026-03-12

### Added
- Plugin-based prompt strategy system with ~65% token reduction (PromptStrategy + PromptPlugin traits, CompactStrategy, LegacyStrategy, PromptStrategyRegistry)
- 6 built-in prompt plugins: Memory, UserObservations, Skills, LearnedRules, Channel, Scheduler
- Token budget overflow protection with priority-based trimming
- Config fields: `prompt_compact_identity`, `prompt_max_preamble_tokens`
- Frontend "Compact Prompts" toggle and "Max Preamble Tokens" input in Agent Features settings

### Fixed
- Telegram long-polling timeout mismatch causing persistent failures
- sqlite_vec transmute for aarch64 compatibility (c_char/c_int types)
- rustfmt cross-version compatibility via `#[rustfmt::skip]`

### Changed
- Chat, WebSocket, and channel router handlers refactored to use single `prompt_strategy.assemble()` call
- Docker image publishing temporarily disabled in release workflow
- Documentation updated: installation, deployment, architecture, configuration, processes

## [0.0.13] - 2026-03-12

### Added
- Notification router with desktop OS notifications and channel routing
- Per-event-type notification routing rules (desktop, channel targets)
- Desktop notification support via Tauri IPC (`showNotification` wrapper)
- GeneralSettings UI toggle for desktop notifications
- Gateway config handler for notification route CRUD
- Frontend notification store wired to WebSocket events
- TUI client with async event loop and WebSocket streaming
- Channel router with supervision, exponential backoff, and auto-reconnect
- Docker multi-arch image publishing to GitHub Container Registry
- TUI binary included in multi-platform release pipeline

## [0.0.12] - 2026-03-11

### Fixed
- Read user location/timezone from live config instead of frozen BootContext, so onboarding location changes take effect immediately without restart

## [0.0.11] - 2026-03-11

### Added
- Context-driven auto-discovery with keyword-based domain detection (Channels/Scheduler/Skills/Tools) for filtered context injection
- AgentSelfTool (`agent_notes`): agent-writable behavioral rules by category, stored in DB, auto-injected into context
- OpenAPI 3.1 interactive documentation via utoipa + Scalar UI at `/api-docs` (feature-gated `api-docs`)
- First-run onboarding flow with browser timezone auto-detection and user location input (`GET /setup/status` + SetupDialog)
- User location and timezone awareness injected into agent context for location-sensitive queries
- Model capability validation (`supports_tools` pre-check) to prevent tool-calling errors with incompatible models
- Agent reasoning protocol refinements in context composition
- Embedding availability tracking in provider registry
- `GET /config/file` endpoint for raw config file content
- `GET /setup/status` endpoint for first-run setup detection

### Changed
- Tool count: 14 base + 2 feature-gated = 16 total (added `agent_self`)
- Route count: 75 base + 17 feature-gated = 92 total

### Fixed
- rustfmt formatting issues

### Documentation
- Updated README, architecture, phases, and processes docs for all post-v0.0.10 features
- Updated test badge: 1046 Rust + 37 JS
- Added 5 new architecture sections with Mermaid diagrams
- Added 3 new process flows (onboarding, auto-discovery, agent self-learning)

## [0.0.10] - 2026-03-10

### Fixed
- Fix release asset name collisions: standalone binaries now use platform-specific names (zenii-linux, zenii-macos) to prevent duplicate upload failures

## [0.0.9] - 2026-03-10

### Fixed
- Release pipeline: exclude keyring/dbus from embedded cross-builds, fix asset name collisions, disable musl target pending OpenSSL cross-compilation fix
- Remove musl references from release checksums and artifact globs to prevent missing file warnings
- Fix GitHub Actions injection pattern in version extraction step

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

- Core foundation: error handling (ZeniiError with thiserror), TOML config, SQLite database with WAL mode and migrations
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
