# Zenii *(zen-ee-eye)*

<p align="center">
  <img src="assets/zenii-master.gif" alt="Zenii demo" width="720" />
</p>

<h2 align="center">One local AI backend. Every interface.</h2>

<p align="center">
  Run a daemon at <code>localhost:18981</code>. Your desktop app, CLI, TUI, scripts,
  and MCP clients share the same memory, tools, providers, and permissions — no sync, no duplication.
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
  <a href="https://github.com/sprklai/zenii/actions/workflows/ci.yml">
    <img src="https://img.shields.io/badge/tests-1720-blue?style=flat-square" alt="1720 tests" />
  </a>
  <a href="https://github.com/sprklai/zenii/pulls">
    <img src="https://img.shields.io/badge/PRs-welcome-brightgreen?style=flat-square" alt="PRs welcome" />
  </a>
</p>

---

## Start in 60 seconds

```bash
curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | bash
zenii-daemon &

# Store a fact once
curl -s -X POST http://localhost:18981/memory \
  -H "Content-Type: application/json" \
  -d '{"key":"deploy","content":"Production database is on port 5434"}' >/dev/null

# Ask about it later — from a script, cron job, or another machine
curl -s -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"session_id":"ops","prompt":"What port is the production database on?"}' | jq -r '.response'
```

That is the core contract: write state once, read it from anywhere that speaks HTTP.

---

## Why Zenii

Most AI tools are per-session and per-interface. You get memory in the chat UI but not in your shell script. You wire a tool to one agent and have to re-wire it to the next.

Zenii solves this with a **shared local backend**:

| Without Zenii | With Zenii |
|---|---|
| Each script manages its own AI context | One daemon holds memory for all of them |
| Tools re-implemented per project | 19 tools registered once, available everywhere |
| Provider API keys scattered across configs | One credential store, one place to rotate |
| Desktop UI and scripts drift apart | Both call the same gateway |
| MCP tools only available inside the IDE | Expose the same tools to any MCP client |

---

## Architecture

<p align="center">
  <img src="docs/assets/zenii_architecture.png" alt="Zenii system architecture" width="720" />
</p>

One Rust library crate (`zenii-core`) holds all business logic. Five thin binary crates (daemon, CLI, TUI, desktop, MCP server) are shell wrappers around the same axum gateway, SQLite database, agent loop, and tool registry.

---

## What Ships Today

**Interfaces**

| Binary | Use it for |
|---|---|
| `zenii-daemon` | Local HTTP + WebSocket API server — the core of everything |
| `zenii` | Quick prompts, shell pipelines, terminal workflows |
| `zenii-tui` | Interactive terminal UI |
| `zenii-desktop` | Native Tauri desktop app |
| `zenii-mcp-server` | Expose all 19 Zenii tools to Claude Code, Cursor, VS Code |

**Capabilities**

- **19 tools** (16 base + 3 feature-gated: channels, scheduler, workflows)
- **133 API routes** (105 base + 28 feature-gated)
- **6+ AI providers**: OpenAI, Anthropic, Gemini, OpenRouter, Vercel AI Gateway, Ollama — or any OpenAI-compatible endpoint
- **MCP server**: expose tools to external agents
- **MCP client**: consume tools from external MCP servers (GitHub, Postgres, Filesystem, etc.)
- **Persistent memory**: BM25 field weighting, temporal decay scoring, semantic deduplication
- **LLM wiki**: ingest PDFs, DOCX, PPTX, XLSX, and images via MarkItDown
- **Channels** (feature-gated): Telegram, Slack, Discord

---

## Install

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | sh
```

Installs `zenii` (CLI) and `zenii-daemon` to `~/.local/bin`.

### Windows

Download and run the desktop installer (`.msi` or `.exe`) from
[GitHub Releases](https://github.com/sprklai/zenii/releases/latest).

For headless / CLI-only, grab `zenii.exe` and `zenii-daemon.exe` from the same page.

### Cargo

```sh
cargo install --git https://github.com/sprklai/zenii zenii zenii-daemon
```

Full platform notes: [Installation & Usage](https://docs.zenii.sprklai.com/installation-and-usage)

---

## Build from Source

Prerequisites: Rust 1.85+, Bun, SQLite development libraries.

```bash
git clone https://github.com/sprklai/zenii.git
cd zenii
cargo build --release -p zenii-daemon       # headless server
cargo build --release -p zenii-cli          # CLI client
cd crates/zenii-desktop && cargo tauri build # desktop app
```

Full setup guide: [docs/development.md](docs/development.md)

---

## Use with Claude Code / Cursor (MCP)

Add to `.mcp.json`:

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

Full integration guide: [AGENT.md](AGENT.md)

---

## Good Fit

- Local automations that need shared memory across scripts, bots, and scheduled jobs
- Developer tooling that wants a single AI backend reachable via HTTP or MCP
- Self-hosted workflows where privacy and local control matter
- Projects that want a desktop UI and a scriptable backend without maintaining two stacks

## Not a Good Fit

- Hosted SaaS or multi-user deployments (single-user local daemon only)
- Drop-in OpenAI-compatible server (Zenii has its own API surface)
- Mobile apps (planned, not yet shipped)

---

## Docs

- [Website](https://zenii.sprklai.com)
- [Documentation](https://docs.zenii.sprklai.com)
- [Installation & Usage](https://docs.zenii.sprklai.com/installation-and-usage)
- [API Reference](https://docs.zenii.sprklai.com/api-reference)
- [CLI Reference](https://docs.zenii.sprklai.com/cli-reference)
- [Configuration](https://docs.zenii.sprklai.com/configuration)
- [LLM Wiki](https://docs.zenii.sprklai.com/wiki)
- [Architecture](https://docs.zenii.sprklai.com/architecture)
- [CHANGELOG.md](CHANGELOG.md)
- [ROADMAP.md](ROADMAP.md)

---

## Contributing

Typo fixes, tests, and focused bug fixes can go straight to a PR.
Larger feature work should start with [CONTRIBUTING.md](CONTRIBUTING.md).

If Zenii is useful to you — [star the repo](https://github.com/sprklai/zenii) and tell a developer friend.

## License

MIT
