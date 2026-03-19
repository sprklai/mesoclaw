---
slug: /
sidebar_position: 0
title: Welcome
---

# Zenii Documentation

**20 megabytes. AI everywhere.**

Install one binary. Now your scripts have **AI memory**. Your cron jobs **reason**. Your Telegram bot **thinks**. A private AI backend for everything on your machine — native desktop app, plugins in any language, and an API your `curl` can call.

> *"ChatGPT is a tab you open. Zenii is a capability your machine gains."*

---

## What is Zenii?

Zenii is a local AI backend that gives every tool on your machine access to AI — through a single binary, a 96-route REST/WebSocket API, and a native desktop app. No cloud account, no SDK, no framework to learn. Just HTTP.

- **For developers** — `curl` the API from scripts, cron jobs, CI pipelines, or any language
- **For power users** — desktop app, CLI, and TUI for interactive AI chat with persistent memory
- **For teams** — self-hosted, MIT licensed, zero telemetry, encrypted credential storage
- **For tinkerers** — plugin system in any language (Python, Go, JS) via JSON-RPC over stdio

### Key capabilities

| Capability | What it means |
|-----------|--------------|
| **96 API routes** | Full REST + WebSocket gateway at `localhost:18981` |
| **18 AI providers** | OpenAI, Anthropic, Google, Ollama, and more — switch with one config change |
| **Semantic memory** | SQLite FTS5 + vector search — your AI remembers across sessions and restarts |
| **16 built-in tools** | Web search, file ops, shell, scheduling, channels, and more |
| **Plugin system** | Any language, JSON-RPC 2.0 over stdio — a plugin is ~15 lines |
| **Self-evolution** | AI proposes skill changes, you approve — like a PR from your assistant |
| **Cron scheduler** | Automated recurring AI tasks without external orchestration |
| **6-layer security** | OS keyring with encrypted file fallback, autonomy levels, FS sandbox, injection detection, rate limits, audit trail |
| **Cross-platform** | macOS, Linux, Windows, ARM — desktop, CLI, TUI, or headless daemon |
| **Under 20 MB** | Native Tauri 2 + Svelte 5 desktop app — not Electron |

### 5 ways to use it

| Interface | Best for |
|-----------|---------|
| **Desktop** | Interactive chat with GUI (Tauri 2 + Svelte 5) |
| **CLI** | Quick prompts, scripting, piping output |
| **TUI** | Terminal-native interactive UI |
| **Daemon** | Headless API server for automation and integrations |
| **Mobile** | iOS + Android (coming soon) |

All interfaces talk to the same backend — your data, memory, and configuration are shared.

---

## Quick Links

- **[Installation & Usage](./installation-and-usage.md)** — Download a binary or build from source
- **[Configuration](./configuration.md)** — All 70+ config fields with types and defaults
- **[CLI Reference](./cli-reference.md)** — Commands, options, shell completions, recipes
- **[API Reference](./api-reference.md)** — All 96 REST & WebSocket routes
- **[Architecture](./architecture.md)** — System design, crate dependencies, diagrams
- **[Deployment](./deployment.md)** — Native, Docker, systemd, Raspberry Pi, reverse proxy
- **[Development](./development.md)** — Prerequisites, building, testing, contributing
- **[Process Flows](./processes.md)** — Chat request, startup, error handling, WebSocket flows
- **[Scheduling](./scheduling.md)** — Cron jobs and one-shot task automation

## Architecture Overview

<p align="center">
  <img src="/img/system-architecture.png" alt="Zenii System Architecture" width="720" />
</p>

## Security

Zenii implements 6 layers of defense to keep your data safe and your system under control:

<p align="center">
  <img src="/img/6-layers-of-defense.png" alt="Zenii 6 Layers of Defense" width="720" />
</p>

## Interactive API Explorer

When running the Zenii daemon, visit [localhost:18981/api-docs](http://localhost:18981/api-docs) for the interactive Scalar API explorer with try-it-out functionality.
