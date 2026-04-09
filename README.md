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
- 15 base tools, with channels, scheduler, and workflows tools available behind feature flags
- 114 total API routes: 86 base routes and 28 feature-gated routes
- MIT license

## Install

Use the install script on Linux or macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | bash
zenii-daemon &
```

Or download platform binaries and desktop packages from
[GitHub Releases](https://github.com/sprklai/zenii/releases/latest).

Full platform notes, package names, and source builds:

- [Installation & Usage](https://docs.zenii.sprklai.com/installation-and-usage)
- [Deployment Guide](https://docs.zenii.sprklai.com/deployment)

## Interfaces

| Surface | Best for |
|---|---|
| `zenii-daemon` | Local API server for scripts, automations, and services |
| `zenii` | Quick prompts, shell pipelines, and terminal workflows |
| `zenii-tui` | Terminal-native interactive use |
| `zenii-desktop` | Native desktop UI on top of the same backend |
| `zenii-mcp-server` | Exposing Zenii tools to external coding agents |

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
