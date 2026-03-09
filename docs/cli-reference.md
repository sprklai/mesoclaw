# MesoClaw CLI Reference

Complete reference for the `mesoclaw` command-line interface.

## Installation

### From GitHub Releases

Download the latest binary for your platform from [GitHub Releases](https://github.com/NSRTech/mesoclaw/releases):

```bash
# macOS (Apple Silicon)
curl -LO https://github.com/NSRTech/mesoclaw/releases/latest/download/mesoclaw-aarch64-apple-darwin.tar.gz
tar xzf mesoclaw-aarch64-apple-darwin.tar.gz
sudo mv mesoclaw /usr/local/bin/

# Linux (x86_64)
curl -LO https://github.com/NSRTech/mesoclaw/releases/latest/download/mesoclaw-x86_64-unknown-linux-gnu.tar.gz
tar xzf mesoclaw-x86_64-unknown-linux-gnu.tar.gz
sudo mv mesoclaw /usr/local/bin/
```

### From Source

```bash
cargo install --path crates/mesoclaw-cli
```

## Quick Start

```bash
# 1. Start the daemon
mesoclaw daemon start

# 2. Set your API key (stored in OS keyring)
mesoclaw key set openai sk-your-key-here

# 3. Set the default model
mesoclaw provider default openai gpt-4o

# 4. Start chatting
mesoclaw chat

# 5. Search your memory
mesoclaw memory search "project notes"
```

## Global Options

These options apply to all commands and can appear before or after the subcommand.

| Option | Default | Description |
|--------|---------|-------------|
| `--host <HOST>` | `127.0.0.1` | Daemon host address |
| `--port <PORT>` | `18981` | Daemon port |
| `--token <TOKEN>` | _(none)_ | Auth token (or set `MESOCLAW_TOKEN` env var) |

Examples:

```bash
# Connect to a remote daemon
mesoclaw --host 192.168.1.100 --port 9000 daemon status

# Use a token from the environment
export MESOCLAW_TOKEN=my-secret-token
mesoclaw chat
```

## Command Reference

---

### `daemon` -- Manage the daemon process

#### `daemon start`

Start the MesoClaw daemon process.

```bash
mesoclaw daemon start
```

#### `daemon stop`

Stop the running daemon process.

```bash
mesoclaw daemon stop
```

#### `daemon status`

Check whether the daemon is running and healthy.

```bash
mesoclaw daemon status
```

---

### `chat` -- Interactive chat (WebSocket)

Open an interactive streaming chat session with the AI agent. Messages are streamed token-by-token over WebSocket.

```
mesoclaw chat [--session <ID>] [--model <MODEL>]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `--session <ID>` | No | Session ID to continue a previous conversation |
| `--model <MODEL>` | No | Model override (e.g. `gpt-4o`, `claude-sonnet-4-20250514`) |

Examples:

```bash
# Start a new chat session
mesoclaw chat

# Continue an existing session
mesoclaw chat --session 550e8400-e29b-41d4-a716-446655440000

# Use a specific model
mesoclaw chat --model claude-sonnet-4-20250514
```

---

### `run` -- Single prompt

Send a single prompt and print the response. Useful for scripting and pipelines.

```
mesoclaw run <PROMPT> [--session <ID>] [--model <MODEL>]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROMPT>` | Yes | The prompt text to send |
| `--session <ID>` | No | Session ID to use for context |
| `--model <MODEL>` | No | Model override |

Examples:

```bash
# Simple one-shot prompt
mesoclaw run "Summarize the Rust ownership model in 3 sentences"

# Use in a pipeline
echo "Translate to French: Hello world" | xargs mesoclaw run

# Continue a session with a one-off question
mesoclaw run "What did we discuss earlier?" --session abc123
```

---

### `memory` -- Manage memory entries

#### `memory search`

Search stored memories using full-text and vector search.

```
mesoclaw memory search <QUERY> [--limit <N>] [--offset <N>]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<QUERY>` | Yes | Search query text |
| `--limit <N>` | No | Maximum number of results |
| `--offset <N>` | No | Offset for pagination |

Examples:

```bash
mesoclaw memory search "rust async patterns"
mesoclaw memory search "meeting notes" --limit 10
mesoclaw memory search "project ideas" --limit 5 --offset 10
```

#### `memory add`

Add a new memory entry.

```
mesoclaw memory add <KEY> <CONTENT>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<KEY>` | Yes | Unique memory key |
| `<CONTENT>` | Yes | Memory content text |

Examples:

```bash
mesoclaw memory add "rust-tip-1" "Use Arc<Mutex<T>> for shared mutable state across threads"
mesoclaw memory add "project-deadline" "v2 launch scheduled for Q2 2026"
```

#### `memory remove`

Remove a memory entry by key.

```
mesoclaw memory remove <KEY>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<KEY>` | Yes | Memory key to remove |

Examples:

```bash
mesoclaw memory remove "rust-tip-1"
mesoclaw memory remove "outdated-note"
```

---

### `config` -- View or update configuration

#### `config show`

Display the current configuration as JSON.

```bash
mesoclaw config show
```

#### `config set`

Set a configuration value.

```
mesoclaw config set <KEY> <VALUE>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<KEY>` | Yes | Configuration key |
| `<VALUE>` | Yes | New value |

Examples:

```bash
mesoclaw config set log_level debug
mesoclaw config set autonomy_level supervised
mesoclaw config set gateway_port 9090
```

---

### `key` -- Manage API keys and credentials

All credentials are stored in the OS keyring (or in-memory fallback).

#### `key set`

Set an API key for a provider or service.

```
mesoclaw key set <PROVIDER> <KEY>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROVIDER>` | Yes | Provider/service name (e.g. `openai`, `anthropic`, `tavily`, `brave`) |
| `<KEY>` | Yes | API key value |

The key is stored as `api_key:<provider>` in the credential store.

Examples:

```bash
mesoclaw key set openai sk-proj-abc123
mesoclaw key set tavily tvly-xyz789
mesoclaw key set brave BSA-key-here
```

#### `key remove`

Remove an API key for a provider.

```
mesoclaw key remove <PROVIDER>
```

Examples:

```bash
mesoclaw key remove tavily
```

#### `key set-channel`

Set a credential field for a messaging channel.

```
mesoclaw key set-channel <CHANNEL> <FIELD> <VALUE>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<CHANNEL>` | Yes | Channel name (e.g. `telegram`, `slack`, `discord`, `matrix`) |
| `<FIELD>` | Yes | Credential field (e.g. `token`, `bot_token`, `access_token`) |
| `<VALUE>` | Yes | Credential value |

The key is stored as `channel:<channel>:<field>` in the credential store.

Examples:

```bash
mesoclaw key set-channel telegram token "bot123456:ABCdefGHIjklMNO"
mesoclaw key set-channel slack bot_token "xoxb-your-token"
mesoclaw key set-channel discord token "MTIz.abc.xyz"
```

#### `key remove-channel`

Remove a channel credential field.

```
mesoclaw key remove-channel <CHANNEL> <FIELD>
```

Examples:

```bash
mesoclaw key remove-channel slack bot_token
mesoclaw key remove-channel telegram token
```

#### `key set-raw`

Set a raw credential key (advanced). Use the full colon-separated key directly.

```
mesoclaw key set-raw <KEY> <VALUE>
```

Examples:

```bash
mesoclaw key set-raw "channel:telegram:token" "bot123:abc"
mesoclaw key set-raw "custom:my-service:secret" "s3cr3t"
```

#### `key remove-raw`

Remove a raw credential key.

```
mesoclaw key remove-raw <KEY>
```

Examples:

```bash
mesoclaw key remove-raw "channel:telegram:token"
```

#### `key list`

List all stored credential keys, grouped by type. Values are not displayed.

```bash
mesoclaw key list
```

---

### `provider` -- Manage AI providers

#### `provider list`

List all registered providers with their API key status.

```bash
mesoclaw provider list
```

#### `provider test`

Test connectivity to a provider by making a lightweight API call.

```
mesoclaw provider test <PROVIDER_ID>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROVIDER_ID>` | Yes | Provider ID (e.g. `openai`, `anthropic`) |

Examples:

```bash
mesoclaw provider test openai
mesoclaw provider test anthropic
```

#### `provider add`

Add a custom OpenAI-compatible provider.

```
mesoclaw provider add <ID> --base-url <URL> [--name <NAME>]
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<ID>` | Yes | Provider ID (alphanumeric + hyphens) |
| `--base-url <URL>` | Yes | Base URL for the API |
| `--name <NAME>` | No | Display name (defaults to ID) |

Examples:

```bash
mesoclaw provider add ollama --base-url http://localhost:11434/v1
mesoclaw provider add my-proxy --base-url https://proxy.example.com/v1 --name "My Proxy"
```

#### `provider remove`

Remove a user-defined provider. Built-in providers cannot be removed.

```
mesoclaw provider remove <PROVIDER_ID>
```

Examples:

```bash
mesoclaw provider remove my-proxy
```

#### `provider default`

Set the default model used for chat and run commands.

```
mesoclaw provider default <PROVIDER_ID> <MODEL_ID>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<PROVIDER_ID>` | Yes | Provider ID |
| `<MODEL_ID>` | Yes | Model ID |

Examples:

```bash
mesoclaw provider default openai gpt-4o
mesoclaw provider default anthropic claude-sonnet-4-20250514
mesoclaw provider default ollama llama3
```

---

### `schedule` -- Manage scheduled jobs

#### `schedule list`

List all scheduled jobs.

```bash
mesoclaw schedule list
```

#### `schedule create`

Create a new scheduled job.

```
mesoclaw schedule create <NAME> [OPTIONS]
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
mesoclaw schedule create health-check --interval-secs 300

# Daily summary via cron
mesoclaw schedule create daily-summary \
  --schedule-type cron \
  --cron-expr "0 9 * * *" \
  --payload agent_turn \
  --prompt "Summarize my tasks for today"

# One-shot reminder
mesoclaw schedule create reminder \
  --interval-secs 3600 \
  --payload notify \
  --message "Time for a break!" \
  --one-shot
```

#### `schedule toggle`

Toggle a job between enabled and disabled.

```
mesoclaw schedule toggle <ID>
```

Examples:

```bash
mesoclaw schedule toggle job-123
```

#### `schedule delete`

Delete a scheduled job permanently.

```
mesoclaw schedule delete <ID>
```

Examples:

```bash
mesoclaw schedule delete job-456
```

#### `schedule history`

Show execution history for a job.

```
mesoclaw schedule history <ID>
```

Examples:

```bash
mesoclaw schedule history job-789
```

#### `schedule status`

Show overall scheduler status (running jobs, next execution times).

```bash
mesoclaw schedule status
```

---

### `completions` -- Generate shell completions (hidden)

Generate shell completion scripts. This command is hidden from `--help` output.

```
mesoclaw completions <SHELL>
```

| Argument | Required | Description |
|----------|----------|-------------|
| `<SHELL>` | Yes | Target shell: `bash`, `zsh`, `fish`, `powershell`, `elvish` |

See [Shell Completions](#shell-completions) for installation instructions.

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MESOCLAW_TOKEN` | Auth token for the daemon. Equivalent to `--token`. |
| `MESOCLAW_GATEWAY_URL` | Override the gateway URL (used by the desktop app for external daemon mode). |

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
mesoclaw completions bash > ~/.local/share/bash-completion/completions/mesoclaw
# Or system-wide:
mesoclaw completions bash | sudo tee /etc/bash_completion.d/mesoclaw > /dev/null
```

### Zsh

```bash
mesoclaw completions zsh > ~/.zfunc/_mesoclaw
# Ensure ~/.zfunc is in your fpath (add to ~/.zshrc):
#   fpath=(~/.zfunc $fpath)
#   autoload -Uz compinit && compinit
```

### Fish

```bash
mesoclaw completions fish > ~/.config/fish/completions/mesoclaw.fish
```

### PowerShell

```powershell
mesoclaw completions powershell > $HOME\Documents\PowerShell\Completions\mesoclaw.ps1
# Add to your $PROFILE:
#   . $HOME\Documents\PowerShell\Completions\mesoclaw.ps1
```

## Recipes

### Setup from scratch

```bash
# Start the daemon
mesoclaw daemon start

# Store your OpenAI API key
mesoclaw key set openai sk-proj-your-key

# Set the default model
mesoclaw provider default openai gpt-4o

# Verify the provider is working
mesoclaw provider test openai

# Start chatting
mesoclaw chat
```

### Switch AI provider

```bash
# Add your Anthropic key
mesoclaw key set anthropic sk-ant-your-key

# Set Anthropic as the default
mesoclaw provider default anthropic claude-sonnet-4-20250514

# Verify it works
mesoclaw provider test anthropic
```

### Add a local Ollama provider

```bash
# Register Ollama as a custom provider
mesoclaw provider add ollama --base-url http://localhost:11434/v1 --name "Ollama Local"

# No API key needed for local Ollama
# Set as default
mesoclaw provider default ollama llama3
```

### Schedule a daily report

```bash
# Create a cron job that runs at 9 AM every day
mesoclaw schedule create morning-briefing \
  --schedule-type cron \
  --cron-expr "0 9 * * *" \
  --payload agent_turn \
  --prompt "Give me a summary of my recent conversations and any pending tasks"

# Check it was created
mesoclaw schedule list

# View execution history later
mesoclaw schedule history <job-id>
```

### Connect Telegram bot

```bash
# Store the Telegram bot token
mesoclaw key set-channel telegram token "bot123456789:ABCdefGHIjklMNOpqrsTUVwxyz"

# Verify the key is stored
mesoclaw key list

# Test the channel (if channels feature is enabled)
# The daemon will pick up the credentials automatically
```

### Backup and restore memory

```bash
# Export all memories matching a query
mesoclaw memory search "" --limit 1000 > memories-backup.json

# Add memories back after a fresh install
mesoclaw memory add "key-1" "content of memory 1"
mesoclaw memory add "key-2" "content of memory 2"
```

### Use in scripts

```bash
#!/bin/bash
# Automated daily log processor
SUMMARY=$(mesoclaw run "Summarize today's system logs: $(journalctl --since today --no-pager | tail -50)")
echo "$SUMMARY" >> ~/daily-summaries.log

# Check daemon health in monitoring
if ! mesoclaw daemon status > /dev/null 2>&1; then
  echo "MesoClaw daemon is down!" | mail -s "Alert" admin@example.com
fi
```
