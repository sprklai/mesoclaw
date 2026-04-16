---
slug: /
sidebar_position: 0
title: Welcome
---

# Zenii Documentation

Zenii is a local AI backend for developers who want one service behind their scripts, desktop UI, terminal tools, and MCP clients.

## Start Here

- **[Installation & Usage](./installation-and-usage)** — Get up and running in minutes
- **[API Reference](./api-reference)** — HTTP and WebSocket endpoints
- **[CLI Reference](./cli-reference)** — Command-line usage and recipes
- **[Configuration](./configuration)** — Runtime settings and defaults
- **[Architecture](./architecture)** — System design and internals
- **[Deployment](./deployment)** — Native, Docker, and server deployment
- **[Development](./development)** — Build, test, and contribute

## What Ships Today

- **Daemon API** — Local REST and WebSocket service at `http://localhost:18981`
- **Shipped interfaces** — Desktop app, CLI, TUI, and MCP server
- **Persistent memory** — Shared local state across interfaces and automations
- **Built-in tools** — 16 base tools, with channels, scheduler, and workflows behind feature flags
- **Local-first deployment** — Self-hosted, MIT licensed, zero telemetry in the product surface

## Good Fit

- Developers building local automations, coding workflows, or agent tooling
- Teams that want one internal AI backend instead of separate per-tool integrations
- People who want a native desktop UI and a scriptable local API backed by the same service

## Current Boundaries

- Zenii is not a hosted SaaS
- Zenii is not a drop-in OpenAI-compatible server today
- Mobile is planned, but not shipped in this repository

## Interactive API Explorer

When running the Zenii daemon, visit [localhost:18981/api-docs](http://localhost:18981/api-docs) for the interactive Scalar API explorer with try-it-out functionality.
