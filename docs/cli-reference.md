# Zenii CLI Reference

Complete reference for the `zenii` command-line interface.

> **Note**: This document was generated with AI assistance and may contain inaccuracies. If you find errors, please [report an issue](https://github.com/sprklai/zenii/issues).

## Installation

### From GitHub Releases

Download the latest binary for your platform from [GitHub Releases](https://github.com/NSRTech/zenii/releases):

```bash
# macOS (Apple Silicon)
curl -LO https://github.com/NSRTech/zenii/releases/latest/download/zenii-aarch64-apple-darwin.tar.gz
tar xzf zenii-aarch64-apple-darwin.tar.gz
sudo mv zenii /usr/local/bin/

# Linux (x86_64)
curl -LO https://github.com/NSRTech/zenii/releases/latest/download/zenii-x86_64-unknown-linux-gnu.tar.gz
tar xzf zenii-x86_64-unknown-linux-gnu.tar.gz
sudo mv zenii /usr/local/bin/
```

### From Source

```bash
cargo install --path crates/zenii-cli
```

## Quick Start

```bash
# 1. Start the daemon
zenii daemon start

# 2. Set your API key (stored in OS keyring)
zenii key set openai sk-your-key-here

# 3. Set the default model
zenii provider default openai gpt-4o

# 4. Start chatting
zenii chat

# 5. Search your memory
zenii memory search "project notes"
```

## Global Options

These options apply to all commands and can appear before or after the subcommand.

| Option | Default | Description |
|--------|---------|-------------|
| `--host <HOST>` | `127.0.0.1` | Daemon host address |
| `--port <PORT>` | `18981` | Daemon port |
| `--token <TOKEN>` | _(none)_ | Auth token (or set `ZENII_TOKEN` env var) |

Examples:

```bash
# Connect to a remote daemon
zenii --host 192.168.1.100 --port 9000 daemon status

# Use a token from the environment
export ZENII_TOKEN=my-secret-token
zenii chat
```

## Command Reference

---

### `daemon` -- Manage the daemon process

#### `daemon start`

Start the Zenii daemon process.

```bash
zenii daemon start
```

#### `daemon stop`

Stop the running daemon process.

```bash
zenii daemon stop
```

#### `daemon status`

Check whether the daemon is running and healthy.

```bash
zenii daemon status
```

---

### `setup` -- First-run onboarding wizard

Run the interactive onboarding wizard to configure your AI provider, messaging channels, and user profile. This is automatically triggered on first launch if setup is incomplete.

```bash
zenii setup
```

The wizard walks through these steps:

1. **Provider selection** -- choose from available AI providers (OpenAI, Anthropic, Google, Ollama, etc.)
2. **API key** -- enter your API key (stored securely in OS keyring). Skipped for providers that don't require a key (e.g., Ollama)
3. **Model selection** -- choose a default model from the selected provider
4. **Channels** (optional) -- configure messaging channel credentials (Telegram, Slack, or Discord). Defaults to skip.
5. **Profile** -- enter your name, location, and timezone (timezone is auto-detected)

Examples:

```bash
# Run the onboarding wizard
zenii setup

# Re-run setup to change provider or profile
zenii setup
```

---

### `chat` -- Interactive chat (WebSocket)

Open an interactive streaming chat session with the AI agent. Messages are streamed token-by-token over WebSocket.

```
zenii chat [--session <ID>] [--model <MODEL>]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `--session <ID>` | No | Session ID to continue a previous conversation |
| `--model <MODEL>` | No | Model override (e.g. `gpt-4o`, `claude-sonnet-4-20250514`) |

Examples:

```bash
# Start a new chat session
zenii chat

# Continue an existing session
zenii chat --session 550e8400-e29b-41d4-a716-446655440000

# Use a specific model
zenii chat --model claude-sonnet-4-20250514
```

---

### `run` -- Single prompt

Send a single prompt and print the response. Useful for scripting and pipelines.

```
zenii run <PROMPT> [--session <ID>] [--model <MODEL>]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROMPT>` | Yes | The prompt text to send |
| `--session <ID>` | No | Session ID to use for context |
| `--model <MODEL>` | No | Model override |

Examples:

```bash
# Simple one-shot prompt
zenii run "Summarize the Rust ownership model in 3 sentences"

# Use in a pipeline
echo "Translate to French: Hello world" | xargs zenii run

# Continue a session with a one-off question
zenii run "What did we discuss earlier?" --session abc123
```

---

### `memory` -- Manage memory entries

#### `memory search`

Search stored memories using full-text and vector search.

```
zenii memory search <QUERY> [--limit <N>] [--offset <N>]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<QUERY>` | Yes | Search query text |
| `--limit <N>` | No | Maximum number of results |
| `--offset <N>` | No | Offset for pagination |

Examples:

```bash
zenii memory search "rust async patterns"
zenii memory search "meeting notes" --limit 10
zenii memory search "project ideas" --limit 5 --offset 10
```

#### `memory add`

Add a new memory entry.

```
zenii memory add <KEY> <CONTENT>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<KEY>` | Yes | Unique memory key |
| `<CONTENT>` | Yes | Memory content text |

Examples:

```bash
zenii memory add "rust-tip-1" "Use Arc<Mutex<T>> for shared mutable state across threads"
zenii memory add "project-deadline" "v2 launch scheduled for Q2 2026"
```

#### `memory remove`

Remove a memory entry by key.

```
zenii memory remove <KEY>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<KEY>` | Yes | Memory key to remove |

Examples:

```bash
zenii memory remove "rust-tip-1"
zenii memory remove "outdated-note"
```

---

### `config` -- View or update configuration

#### `config show`

Display the current configuration as JSON.

```bash
zenii config show
```

#### `config set`

Set a configuration value.

```
zenii config set <KEY> <VALUE>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<KEY>` | Yes | Configuration key |
| `<VALUE>` | Yes | New value |

Examples:

```bash
zenii config set log_level debug
zenii config set autonomy_level supervised
zenii config set gateway_port 9090
```

---

### `key` -- Manage API keys and credentials

All credentials are stored in the OS keyring (or in-memory fallback).

#### `key set`

Set an API key for a provider or service.

```
zenii key set <PROVIDER> <KEY>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROVIDER>` | Yes | Provider/service name (e.g. `openai`, `anthropic`, `tavily`, `brave`) |
| `<KEY>` | Yes | API key value |

The key is stored as `api_key:<provider>` in the credential store.

Examples:

```bash
zenii key set openai sk-proj-abc123
zenii key set tavily tvly-xyz789
zenii key set brave BSA-key-here
```

#### `key remove`

Remove an API key for a provider.

```
zenii key remove <PROVIDER>
```

Examples:

```bash
zenii key remove tavily
```

#### `key set-channel`

Set a credential field for a messaging channel.

```
zenii key set-channel <CHANNEL> <FIELD> <VALUE>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<CHANNEL>` | Yes | Channel name (e.g. `telegram`, `slack`, `discord`, `matrix`) |
| `<FIELD>` | Yes | Credential field (e.g. `token`, `bot_token`, `access_token`) |
| `<VALUE>` | Yes | Credential value |

The key is stored as `channel:<channel>:<field>` in the credential store.

Examples:

```bash
zenii key set-channel telegram token "bot123456:ABCdefGHIjklMNO"
zenii key set-channel slack bot_token "xoxb-your-token"
zenii key set-channel discord token "MTIz.abc.xyz"
```

#### `key remove-channel`

Remove a channel credential field.

```
zenii key remove-channel <CHANNEL> <FIELD>
```

Examples:

```bash
zenii key remove-channel slack bot_token
zenii key remove-channel telegram token
```

#### `key set-raw`

Set a raw credential key (advanced). Use the full colon-separated key directly.

```
zenii key set-raw <KEY> <VALUE>
```

Examples:

```bash
zenii key set-raw "channel:telegram:token" "bot123:abc"
zenii key set-raw "custom:my-service:secret" "s3cr3t"
```

#### `key remove-raw`

Remove a raw credential key.

```
zenii key remove-raw <KEY>
```

Examples:

```bash
zenii key remove-raw "channel:telegram:token"
```

#### `key list`

List all stored credential keys, grouped by type. Values are not displayed.

```bash
zenii key list
```

---

### `provider` -- Manage AI providers

#### `provider list`

List all registered providers with their API key status.

```bash
zenii provider list
```

#### `provider test`

Test connectivity to a provider by making a lightweight API call.

```
zenii provider test <PROVIDER_ID>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROVIDER_ID>` | Yes | Provider ID (e.g. `openai`, `anthropic`) |

Examples:

```bash
zenii provider test openai
zenii provider test anthropic
```

#### `provider add`

Add a custom OpenAI-compatible provider.

```
zenii provider add <ID> --base-url <URL> [--name <NAME>]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<ID>` | Yes | Provider ID (alphanumeric + hyphens) |
| `--base-url <URL>` | Yes | Base URL for the API |
| `--name <NAME>` | No | Display name (defaults to ID) |

Examples:

```bash
zenii provider add ollama --base-url http://localhost:11434/v1
zenii provider add my-proxy --base-url https://proxy.example.com/v1 --name "My Proxy"
```

#### `provider remove`

Remove a user-defined provider. Built-in providers cannot be removed.

```
zenii provider remove <PROVIDER_ID>
```

Examples:

```bash
zenii provider remove my-proxy
```

#### `provider default`

Set the default model used for chat and run commands.

```
zenii provider default <PROVIDER_ID> <MODEL_ID>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROVIDER_ID>` | Yes | Provider ID |
| `<MODEL_ID>` | Yes | Model ID |

Examples:

```bash
zenii provider default openai gpt-4o
zenii provider default anthropic claude-sonnet-4-20250514
zenii provider default ollama llama3
```

---

### `schedule` -- Manage scheduled jobs

#### `schedule list`

List all scheduled jobs.

```bash
zenii schedule list
```

#### `schedule create`

Create a new scheduled job.

```
zenii schedule create <NAME> [OPTIONS]
```

| Argument | Required | Default | Description |
|----------|----------|---------|-------------|
| `<NAME>` | Yes | -- | Job name |
| `--schedule-type <TYPE>` | No | `interval` | Schedule type: `interval` or `cron` |
| `--interval-secs <N>` | No | -- | Interval in seconds (for interval type) |
| `--cron-expr <EXPR>` | No | -- | Cron expression (for cron type) |
| `--payload <TYPE>` | No | `heartbeat` | Payload type: `heartbeat`, `notify`, or `agent_turn` |
| `--message <TEXT>` | No | -- | Message for `notify` payload |
| `--prompt <TEXT>` | No | -- | Prompt for `agent_turn` payload |
| `--one-shot` | No | `false` | Delete after first execution |

Examples:

```bash
# Health check every 5 minutes
zenii schedule create health-check --interval-secs 300

# Daily summary via cron
zenii schedule create daily-summary \
  --schedule-type cron \
  --cron-expr "0 9 * * *" \
  --payload agent_turn \
  --prompt "Summarize my tasks for today"

# One-shot reminder
zenii schedule create reminder \
  --interval-secs 3600 \
  --payload notify \
  --message "Time for a break!" \
  --one-shot
```

#### `schedule toggle`

Toggle a job between enabled and disabled.

```
zenii schedule toggle <ID>
```

Examples:

```bash
zenii schedule toggle job-123
```

#### `schedule delete`

Delete a scheduled job permanently.

```
zenii schedule delete <ID>
```

Examples:

```bash
zenii schedule delete job-456
```

#### `schedule history`

Show execution history for a job.

```
zenii schedule history <ID>
```

Examples:

```bash
zenii schedule history job-789
```

#### `schedule status`

Show overall scheduler status (running jobs, next execution times).

```bash
zenii schedule status
```

---

### `embedding` -- Manage semantic memory embeddings

#### `embedding activate`

Activate an embedding provider.

```
zenii embedding activate <PROVIDER>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROVIDER>` | Yes | Provider name: `openai` or `local` |

Examples:

```bash
# Activate local embeddings (no API key required)
zenii embedding activate local

# Activate OpenAI embeddings (requires api_key:openai)
zenii embedding activate openai
```

#### `embedding deactivate`

Deactivate the current embedding provider (fall back to FTS5 only).

```bash
zenii embedding deactivate
```

#### `embedding status`

Show the current embedding provider status.

```bash
zenii embedding status
```

#### `embedding test`

Test embedding generation with the current provider.

```bash
zenii embedding test
```

#### `embedding reindex`

Re-embed all stored memories with the current provider.

```bash
zenii embedding reindex
```

---

### `plugin` -- Manage external plugins

#### `plugin list`

List all installed plugins.

```bash
zenii plugin list
```

#### `plugin install`

Install a plugin from a git URL or local path.

```
zenii plugin install <SOURCE> [--local] [--all]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<SOURCE>` | Yes | Git URL or local path to the plugin |
| `--local` | No | Treat source as a local directory path |
| `--all` | No | Install all plugins found in a local directory (requires `--local`) |

Examples:

```bash
# Install a single-repo plugin from git
zenii plugin install https://github.com/sprklai/word-count

# Install a specific plugin from a monorepo subdirectory (use #subdir fragment)
zenii plugin install https://github.com/sprklai/zenii-plugins#plugins/json-formatter

# Install from local directory
zenii plugin install ./my-plugin --local

# Install all plugins from a local directory
zenii plugin install ./plugins-dir --local --all
```

#### `plugin remove`

Remove an installed plugin.

```
zenii plugin remove <NAME>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<NAME>` | Yes | Plugin name to remove |

Examples:

```bash
zenii plugin remove weather
```

#### `plugin update`

Update a git-sourced plugin to the latest version.

```
zenii plugin update <NAME>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<NAME>` | Yes | Plugin name to update |

Examples:

```bash
zenii plugin update weather
```

#### `plugin enable`

Enable a disabled plugin.

```
zenii plugin enable <NAME>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<NAME>` | Yes | Plugin name to enable |

Examples:

```bash
zenii plugin enable weather
```

#### `plugin disable`

Disable a plugin.

```
zenii plugin disable <NAME>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<NAME>` | Yes | Plugin name to disable |

Examples:

```bash
zenii plugin disable weather
```

#### `plugin info`

Show plugin details.

```
zenii plugin info <NAME>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<NAME>` | Yes | Plugin name to inspect |

Examples:

```bash
zenii plugin info weather
```

---

### `completions` -- Generate shell completions (hidden)

Generate shell completion scripts. This command is hidden from `--help` output.

```
zenii completions <SHELL>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<SHELL>` | Yes | Target shell: `bash`, `zsh`, `fish`, `powershell`, `elvish` |

See [Shell Completions](#shell-completions) for installation instructions.

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ZENII_TOKEN` | Auth token for the daemon. Equivalent to `--token`. |
| `ZENII_GATEWAY_URL` | Override the gateway URL (used by the desktop app for external daemon mode). |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error (API error, invalid input, command failure) |
| `2` | Connection error (daemon not running, network unreachable) |

## Shell Completions

Generate and install shell completions for tab-completion of commands and options.

### Bash

```bash
zenii completions bash > ~/.local/share/bash-completion/completions/zenii
# Or system-wide:
zenii completions bash | sudo tee /etc/bash_completion.d/zenii > /dev/null
```

### Zsh

```bash
zenii completions zsh > ~/.zfunc/_zenii
# Ensure ~/.zfunc is in your fpath (add to ~/.zshrc):
#   fpath=(~/.zfunc $fpath)
#   autoload -Uz compinit && compinit
```

### Fish

```bash
zenii completions fish > ~/.config/fish/completions/zenii.fish
```

### PowerShell

```powershell
zenii completions powershell > $HOME\Documents\PowerShell\Completions\zenii.ps1
# Add to your $PROFILE:
#   . $HOME\Documents\PowerShell\Completions\zenii.ps1
```

## Recipes

### Setup from scratch

```bash
# Start the daemon
zenii daemon start

# Store your OpenAI API key
zenii key set openai sk-proj-your-key

# Set the default model
zenii provider default openai gpt-4o

# Verify the provider is working
zenii provider test openai

# Start chatting
zenii chat
```

### Switch AI provider

```bash
# Add your Anthropic key
zenii key set anthropic sk-ant-your-key

# Set Anthropic as the default
zenii provider default anthropic claude-sonnet-4-20250514

# Verify it works
zenii provider test anthropic
```

### Add a local Ollama provider

```bash
# Register Ollama as a custom provider
zenii provider add ollama --base-url http://localhost:11434/v1 --name "Ollama Local"

# No API key needed for local Ollama
# Set as default
zenii provider default ollama llama3
```

### Schedule a daily report

```bash
# Create a cron job that runs at 9 AM every day
zenii schedule create morning-briefing \
  --schedule-type cron \
  --cron-expr "0 9 * * *" \
  --payload agent_turn \
  --prompt "Give me a summary of my recent conversations and any pending tasks"

# Check it was created
zenii schedule list

# View execution history later
zenii schedule history <job-id>
```

### Connect Telegram bot

```bash
# Store the Telegram bot token
zenii key set-channel telegram token "bot123456789:ABCdefGHIjklMNOpqrsTUVwxyz"

# Verify the key is stored
zenii key list

# Test the channel (if channels feature is enabled)
# The daemon will pick up the credentials automatically
```

### Backup and restore memory

```bash
# Export all memories matching a query
zenii memory search "" --limit 1000 > memories-backup.json

# Add memories back after a fresh install
zenii memory add "key-1" "content of memory 1"
zenii memory add "key-2" "content of memory 2"
```

### Use in scripts

```bash
#!/bin/bash
# Automated daily log processor
SUMMARY=$(zenii run "Summarize today's system logs: $(journalctl --since today --no-pager | tail -50)")
echo "$SUMMARY" >> ~/daily-summaries.log

# Check daemon health in monitoring
if ! zenii daemon status > /dev/null 2>&1; then
  echo "Zenii daemon is down!" | mail -s "Alert" admin@example.com
fi
```
