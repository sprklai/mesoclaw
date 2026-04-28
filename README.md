# Zenii *(zen-ee-eye)*

<p align="center">
  <img src="assets/zenii-master.gif" alt="Zenii demo" width="720" />
</p>

<h1 align="center">One local AI backend for your scripts, tools, and agents.</h1>

<p align="center">
  Zenii runs a daemon on <code>http://localhost:18981</code> so your desktop app, CLI, TUI,
  scripts, and MCP clients all use the same memory, tools, model providers, and permissions.
</p>

<p align="center">
  <a href="https://github.com/sprklai/zenii/releases/latest">
    <img src="https://img.shields.io/github/v/release/sprklai/zenii?style=flat-square" alt="Latest release" />
  </a>
  <a href="https://github.com/sprklai/zenii/actions/workflows/ci.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/sprklai/zenii/ci.yml?style=flat-square&label=CI" alt="CI" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT license" />
  </a>
  <a href="https://github.com/sprklai/zenii/pulls">
    <img src="https://img.shields.io/badge/PRs-welcome-brightgreen?style=flat-square" alt="PRs welcome" />
  </a>
  <a href="https://github.com/sprklai/zenii/actions/workflows/ci.yml">
    <img src="https://img.shields.io/badge/tests-1696-blue?style=flat-square" alt="1696 tests" />
  </a>
</p>

Zenii is for developers who want AI to behave like infrastructure instead of a browser tab.
Run one local service. Call it from `curl`, scripts, cron jobs, or an MCP client. Use the same
backend from the native desktop app, CLI, or TUI.

## See It Work

```bash
curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | bash
zenii-daemon &

# Store something once
curl -s -X POST http://localhost:18981/memory \
  -H "Content-Type: application/json" \
  -d '{"key":"deploy","content":"Production database is on port 5434"}' >/dev/null

# Ask through chat later
curl -s -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"session_id":"ops","prompt":"What port is the production database on?"}' | jq -r '.response'
```

That is the core value: write state once, use it from anywhere that talks to Zenii.

## What Zenii Is

- A local daemon with a REST and WebSocket API at `localhost:18981`
- A shared AI backend for the desktop app, CLI, TUI, scripts, and MCP clients
- Persistent memory, provider routing, and tool execution in one local service
- A native Rust/Tauri stack instead of an Electron wrapper

## Architecture

<p align="center">
  <img src="docs/assets/zenii_architecture.png" alt="Zenii system architecture" width="720" />
</p>

One Rust library crate (`zenii-core`) contains all business logic. Thin binary crates (daemon, CLI, TUI, desktop) are shell wrappers. All share the same axum gateway, SQLite database, agent loop, and tool registry.

## Good Fit

- Local automations that need shared memory across scripts, bots, and tools
- Developer tooling that wants one AI backend behind HTTP or MCP
- Self-hosted workflows where privacy and local control matter
- Projects that want a desktop UI and a scriptable backend without maintaining both separately

## Current Product Boundaries

- Zenii is not a hosted SaaS product
- Zenii is not a drop-in OpenAI-compatible server today
- Mobile is planned, but not shipped in this repository

## What Ships Today

- `zenii-daemon`: local API server
- `zenii`: CLI client
- `zenii-tui`: terminal UI
- `zenii-desktop`: Tauri desktop app
- `zenii-mcp-server`: MCP server for Claude Code, Cursor, and similar clients
- 19 tools total (16 base + 3 feature-gated: channels, scheduler, workflows)
- 133 total API routes: 105 base routes and 28 feature-gated routes
- 6+ AI providers (OpenAI, Anthropic, Gemini, OpenRouter, Vercel AI Gateway, Ollama) — any OpenAI-compatible API endpoint can be added as a custom provider
- LLM wiki with binary document ingestion (PDF, DOCX, PPTX, XLSX, images via MarkItDown)
- Memory intelligence: BM25 field weighting, temporal decay scoring, semantic deduplication
- MCP Server (`zenii-mcp-server`): expose all 19 tools to Claude Code, Cursor, VS Code, and other MCP clients
- MCP Client (`mcp-client` feature): consume tools from external MCP servers (GitHub, Postgres, Filesystem, etc.) via stdio — HTTP transport planned
- MIT license

## Install

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | sh
```

Installs `zenii` (CLI) and `zenii-daemon` to `~/.local/bin`.

### Windows

Download and run the desktop installer (`.msi` or `.exe`) from
[GitHub Releases](https://github.com/sprklai/zenii/releases/latest) —
it includes the daemon and full GUI.

For headless / CLI-only, grab `zenii.exe` and `zenii-daemon.exe` from the same Releases page.

### Any platform (Rust / Cargo)

```sh
cargo install --git https://github.com/sprklai/zenii zenii zenii-daemon
```

Full platform notes and source builds: [Installation & Usage](https://docs.zenii.sprklai.com/installation-and-usage)

## Build from Source

Prerequisites: Rust 1.85+, Bun, SQLite development libraries.

```bash
git clone https://github.com/sprklai/zenii.git
cd zenii
cargo build --release -p zenii-daemon       # headless server
cargo build --release -p zenii-cli          # CLI client
cd crates/zenii-desktop && cargo tauri build # desktop app
```

Full setup and cross-compilation instructions: [docs/development.md](docs/development.md)

## Interfaces

| Surface | Best for |
|---|---|
| `zenii-daemon` | Local API server for scripts, automations, and services |
| `zenii` | Quick prompts, shell pipelines, and terminal workflows |
| `zenii-tui` | Terminal-native interactive use |
| `zenii-desktop` | Native desktop UI on top of the same backend |
| `zenii-mcp-server` | Exposing Zenii tools to external coding agents |
| `mcp-client` feature | Consuming tools from external MCP servers inside the agent |

## MCP Example

Add Zenii to `.mcp.json`:

```json
{
  "mcpServers": {
    "zenii": {
      "command": "zenii-mcp-server",
      "args": ["--transport", "stdio"]
    }
  }
}
```

More integration detail lives in [AGENT.md](AGENT.md).

## Docs

- [Documentation site](https://docs.zenii.sprklai.com)
- [Installation & Usage](https://docs.zenii.sprklai.com/installation-and-usage)
- [API Reference](https://docs.zenii.sprklai.com/api-reference)
- [CLI Reference](https://docs.zenii.sprklai.com/cli-reference)
- [Configuration](https://docs.zenii.sprklai.com/configuration)
- [LLM Wiki](https://docs.zenii.sprklai.com/wiki)
- [Architecture](https://docs.zenii.sprklai.com/architecture)
- [Development](https://docs.zenii.sprklai.com/development)
- [CHANGELOG.md](CHANGELOG.md)
- [ROADMAP.md](ROADMAP.md)

## Contributing

Small documentation fixes, typo fixes, tests, and focused bug fixes can go straight to a PR.
Larger feature work should start with [CONTRIBUTING.md](CONTRIBUTING.md).

If Zenii is useful to you, star the repo:
<https://github.com/sprklai/zenii>

## License

MIT
