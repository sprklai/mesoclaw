---
sidebar_position: 1
title: Installation & Usage
slug: /installation-and-usage
---

# Zenii Installation and Usage Guide

A practical guide to installing Zenii, starting the local daemon, and verifying that the shared backend works before moving on to deeper integrations.

## Fastest Path

```bash
curl -fsSL https://raw.githubusercontent.com/sprklai/zenii/main/install.sh | bash
zenii-daemon &
```

## Verify It Works

```bash
curl -s -X POST http://localhost:18981/memory \
  -H "Content-Type: application/json" \
  -d '{"key":"deploy","content":"Production database is on port 5434"}' >/dev/null

curl -s -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"session_id":"ops","prompt":"What port is the production database on?"}' | jq -r '.response'
```

## Table of Contents

- [Fastest Path](#fastest-path)
- [Verify It Works](#verify-it-works)
- [Overview](#overview)
- [Interfaces](#interfaces)
- [Installation by Platform](#installation-by-platform)
  - [Linux (x86_64)](#linux-x86_64)
  - [Linux (ARM64 / Raspberry Pi)](#linux-arm64--raspberry-pi)
  - [Linux (ARMv7 / Older Boards)](#linux-armv7--older-boards)
  - [macOS](#macos)
  - [Windows](#windows)
  - [Docker / Docker Compose](#docker--docker-compose)
- [Running Zenii](#running-zenii)
  - [Desktop App (GUI)](#desktop-app-gui)
  - [Daemon (Headless Server)](#daemon-headless-server)
  - [CLI (Command Line)](#cli-command-line)
  - [TUI (Terminal UI)](#tui-terminal-ui)
- [Configuration](#configuration)
- [HTTP API Integration](#http-api-integration)
  - [Authentication](#authentication)
  - [Python](#python)
  - [JavaScript / TypeScript](#javascript--typescript)
  - [Go](#go)
  - [Rust](#rust)
  - [Ruby](#ruby)
  - [Java / Kotlin](#java--kotlin)
  - [C# / .NET](#c--net)
  - [Shell / cURL](#shell--curl)
- [WebSocket Streaming](#websocket-streaming)
  - [Protocol](#protocol)
  - [Python (websockets)](#python-websockets)
  - [JavaScript / TypeScript (Browser)](#javascript--typescript-browser)
  - [JavaScript / TypeScript (Node.js)](#javascript--typescript-nodejs)
  - [Go (gorilla/websocket)](#go-gorillawebsocket)
  - [Rust (tokio-tungstenite)](#rust-tokio-tungstenite)
- [Linux Single-Board Computers](#linux-single-board-computers)
  - [Raspberry Pi 4/5 (ARM64)](#raspberry-pi-45-arm64)
  - [Raspberry Pi 3 / Zero 2W (ARMv7)](#raspberry-pi-3--zero-2w-armv7)
  - [NVIDIA Jetson](#nvidia-jetson)
  - [Orange Pi / Rock Pi / Pine64](#orange-pi--rock-pi--pine64)
  - [Performance Tuning for Low-Resource Boards](#performance-tuning-for-low-resource-boards)
- [Docker Compose Recipes](#docker-compose-recipes)
  - [Basic](#basic)
  - [With Reverse Proxy (Caddy)](#with-reverse-proxy-caddy)
  - [Multi-Instance](#multi-instance)
- [Cloud and Virtual Machine Deployment](#cloud-and-virtual-machine-deployment)
  - [AWS (EC2)](#aws-ec2)
  - [AWS (ECS Fargate)](#aws-ecs-fargate)
  - [AWS (Lightsail)](#aws-lightsail)
  - [Google Cloud (GCE)](#google-cloud-gce)
  - [Google Cloud (Cloud Run)](#google-cloud-cloud-run)
  - [Azure (VM)](#azure-vm)
  - [Azure (Container Instances)](#azure-container-instances)
  - [DigitalOcean](#digitalocean)
  - [Hetzner](#hetzner)
  - [Linode / Akamai](#linode--akamai)
  - [Oracle Cloud (Free Tier)](#oracle-cloud-free-tier)
  - [Fly.io](#flyio)
  - [Railway](#railway)
  - [Any Linux VM or VPS](#any-linux-vm-or-vps)
  - [Security Hardening for Cloud](#security-hardening-for-cloud)
- [Troubleshooting](#troubleshooting)

---

## Overview

Zenii is an AI assistant platform with a Rust backend and multiple client interfaces. All interfaces communicate through a single HTTP+WebSocket gateway running on `localhost:18981` by default.

**Architecture:**

```
Desktop (Tauri+Svelte)  ──┐
CLI (zenii)           ──┼──▶  Gateway (axum)  ──▶  AI Providers (OpenAI, Anthropic, etc.)
TUI (zenii-tui)       ──┤     :18981                SQLite DB
Daemon (headless)        ──┤                           Memory Store
Your App (HTTP/WS)       ──┘                           Tool Registry
```

## Interfaces

| Interface | Binary | Use Case | Requires Display |
|-----------|--------|----------|-----------------|
| **Desktop** | `zenii-desktop` | Full GUI experience with Svelte frontend | Yes |
| **Daemon** | `zenii-daemon` | Headless server, Docker, systemd, APIs | No |
| **CLI** | `zenii` | Terminal chat, scripting, piping | No |
| **TUI** | `zenii-tui` | Interactive terminal dashboard | No (terminal only) |

The **daemon** is the core — it runs the gateway server. Desktop embeds the daemon internally. CLI and TUI connect to a running daemon over HTTP/WS.

---

## Installation by Platform

### Linux (x86_64)

**Pre-built binary:**

```bash
# Download the latest release (bare binaries — no tarball)
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-linux
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-linux
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-tui-linux

# Install
chmod +x zenii-linux zenii-daemon-linux zenii-tui-linux
sudo mv zenii-linux /usr/local/bin/zenii
sudo mv zenii-daemon-linux /usr/local/bin/zenii-daemon
sudo mv zenii-tui-linux /usr/local/bin/zenii-tui
```

**From source:**

```bash
# Prerequisites
sudo apt install build-essential pkg-config libsqlite3-dev libssl-dev

# Clone and build
git clone https://github.com/sprklai/zenii.git
cd zenii
cargo build --release -p zenii-daemon -p zenii-cli -p zenii-tui

# Binaries are in target/release/
sudo cp target/release/zenii-daemon target/release/zenii target/release/zenii-tui /usr/local/bin/
```

**Desktop app (Debian/Ubuntu):**

```bash
# Download .deb package (replace VERSION with the actual release version, e.g. 0.0.44)
# Browse available versions at: https://github.com/sprklai/zenii/releases
curl -LO "https://github.com/sprklai/zenii/releases/latest/download/Zenii_VERSION_amd64.deb"
sudo dpkg -i Zenii_*_amd64.deb

# Or AppImage (no install needed)
curl -LO "https://github.com/sprklai/zenii/releases/latest/download/Zenii_VERSION_amd64.AppImage"
chmod +x Zenii_*_amd64.AppImage
./Zenii_*_amd64.AppImage
```

### Linux (ARM64 / Raspberry Pi)

```bash
# Pre-built binary (bare binaries — no tarball)
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-arm64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-arm64
chmod +x zenii-arm64 zenii-daemon-arm64
sudo mv zenii-arm64 /usr/local/bin/zenii
sudo mv zenii-daemon-arm64 /usr/local/bin/zenii-daemon
```

**From source on the board:**

```bash
sudo apt install build-essential pkg-config libsqlite3-dev libssl-dev
cargo build --release -p zenii-daemon -p zenii-cli
```

**Cross-compile from x86 host:**

```bash
# Using the build script
./scripts/build.sh --target linux-arm64 --release --crates "zenii-daemon zenii-cli"

# Or using Docker-based cross-compilation
./scripts/build.sh --target linux-arm64 --release --docker
```

### Linux (ARMv7 / Older Boards)

```bash
# Pre-built binary (bare binaries — no tarball)
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-armv7
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-armv7
chmod +x zenii-armv7 zenii-daemon-armv7
sudo mv zenii-armv7 /usr/local/bin/zenii
sudo mv zenii-daemon-armv7 /usr/local/bin/zenii-daemon

# Cross-compile from host
./scripts/build.sh --target linux-armv7 --release
```

### macOS

**Pre-built binary:**

```bash
# Apple Silicon (M1/M2/M3/M4/M5) — bare binaries, no tarball
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-macos-arm64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-macos-arm64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-tui-macos-arm64
chmod +x zenii-macos-arm64 zenii-daemon-macos-arm64 zenii-tui-macos-arm64
sudo mv zenii-macos-arm64 /usr/local/bin/zenii
sudo mv zenii-daemon-macos-arm64 /usr/local/bin/zenii-daemon
sudo mv zenii-tui-macos-arm64 /usr/local/bin/zenii-tui

# Intel
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-macos-x86_64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-macos-x86_64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-tui-macos-x86_64
chmod +x zenii-macos-x86_64 zenii-daemon-macos-x86_64 zenii-tui-macos-x86_64
sudo mv zenii-macos-x86_64 /usr/local/bin/zenii
sudo mv zenii-daemon-macos-x86_64 /usr/local/bin/zenii-daemon
sudo mv zenii-tui-macos-x86_64 /usr/local/bin/zenii-tui
```

**Desktop app:**

```bash
# Download .dmg (replace VERSION with the actual release version, e.g. 0.0.44)
# Apple Silicon:
curl -LO "https://github.com/sprklai/zenii/releases/latest/download/Zenii_VERSION_aarch64.dmg"
# Intel:
# curl -LO "https://github.com/sprklai/zenii/releases/latest/download/Zenii_VERSION_amd64.dmg"
open Zenii_*.dmg
# Drag to Applications
```

**From source:**

```bash
# Prerequisites (Homebrew)
brew install sqlite3 pkg-config

git clone https://github.com/sprklai/zenii.git
cd zenii
cargo build --release -p zenii-daemon -p zenii-cli -p zenii-tui
```

### Windows

**Pre-built binary:**

```powershell
# Download CLI and daemon binaries from GitHub Releases
Invoke-WebRequest -Uri "https://github.com/sprklai/zenii/releases/latest/download/zenii.exe" -OutFile C:\zenii\zenii.exe
Invoke-WebRequest -Uri "https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon.exe" -OutFile C:\zenii\zenii-daemon.exe
Invoke-WebRequest -Uri "https://github.com/sprklai/zenii/releases/latest/download/zenii-tui.exe" -OutFile C:\zenii\zenii-tui.exe
$env:Path += ";C:\zenii"
```

**Desktop app:**

Download the `.msi` or `.exe` (NSIS) installer from GitHub Releases and run it.

**From source:**

```powershell
# Requires Visual Studio Build Tools + Rust
git clone https://github.com/sprklai/zenii.git
cd zenii
cargo build --release -p zenii-daemon -p zenii-cli
```

### Docker / Docker Compose

> **Note**: Pre-built Docker images are not currently published. Build from source instead.

#### Build from Source

```bash
git clone https://github.com/sprklai/zenii.git
cd zenii
docker build -t zenii .
docker run -d \
  --name zenii \
  -p 18981:18981 \
  -v zenii-data:/data \
  -e ZENII_TOKEN=your-secret-token \
  -e RUST_LOG=info \
  zenii
```

#### Docker Compose

A `docker-compose.yml` is provided in the repository root:

```bash
git clone https://github.com/sprklai/zenii.git
cd zenii
export ZENII_TOKEN=your-secret-token
docker compose up -d
```

---

## Running Zenii

### Desktop App (GUI)

The desktop app is a Tauri 2 application with an embedded Svelte frontend. It runs the daemon internally — no separate server process needed.

```bash
# Launch the desktop app
zenii-desktop

# Or from source during development
cd crates/zenii-desktop
cargo tauri dev
```

**Features:**
- Full chat interface with streaming responses
- Session management (create, switch, delete)
- Memory browser with search
- Settings UI (providers, credentials, persona, channels)
- System tray with show/hide/quit
- Close-to-tray behavior (quit via tray menu)

The desktop app exposes the same gateway on `localhost:18981`, so you can use the CLI or custom scripts alongside it. To connect to an external daemon instead of the embedded one, set `ZENII_GATEWAY_URL=http://host:port`.

### Daemon (Headless Server)

The daemon runs the gateway without any UI. Use it for servers, Docker, systemd services, or as a backend for custom frontends.

```bash
# Start with defaults
zenii-daemon

# Start with custom config
zenii-daemon --config /path/to/config.toml

# Start with environment overrides
ZENII_TOKEN=secret RUST_LOG=debug zenii-daemon
```

**As a systemd service (Linux):**

```ini
# /etc/systemd/system/zenii.service
[Unit]
Description=Zenii AI Assistant Daemon
After=network.target

[Service]
Type=simple
User=zenii
ExecStart=/usr/local/bin/zenii-daemon
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info
Environment=ZENII_TOKEN=your-secret-token

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now zenii
sudo journalctl -u zenii -f
```

### CLI (Command Line)

The CLI is a thin HTTP/WS client that connects to a running daemon. Start the daemon first (or use the desktop app).

```bash
# Start the daemon in the background
zenii-daemon &

# Interactive chat (WebSocket streaming)
zenii chat

# Chat with a specific model
zenii chat --model anthropic/claude-sonnet-4-20250514

# Single prompt (non-interactive)
zenii run "Summarize the Rust ownership model in 3 sentences"
zenii run "Continue our discussion" --session my-project --model gpt-4o

# Session management
zenii chat --session my-project

# Provider management
zenii provider list
zenii provider default openai gpt-4o
zenii key set openai sk-your-key

# Memory
zenii memory search "meeting notes"
zenii memory add "user-pref" "User prefers concise answers"
zenii memory remove "user-pref"

# Daemon management
zenii daemon start
zenii daemon stop
zenii daemon status

# Configuration
zenii config show
zenii config set log_level debug

# Scheduled jobs
zenii schedule list
zenii schedule create my-job --schedule-type interval --interval-secs 300
zenii schedule status

# Plugins
zenii plugin list
zenii plugin install https://github.com/user/weather-plugin
zenii plugin remove weather

# Channels
zenii channel list --source telegram
zenii channel messages <session-id>

# Embeddings
zenii embedding status
zenii embedding activate local

# Connect to a remote daemon
zenii --host 192.168.1.100 --port 18981 --token secret chat
```

### TUI (Terminal UI)

The TUI connects to a running daemon and uses the same backend, memory, and providers as the CLI and desktop app.

```bash
# Start TUI (connects to running daemon)
zenii-tui
```

---

## Configuration

Zenii looks for `config.toml` in platform-specific directories:

| Platform | Config Path |
|----------|-------------|
| Linux | `~/.config/zenii/config.toml` |
| macOS | `~/Library/Application Support/com.sprklai.zenii/config.toml` |
| Windows | `%APPDATA%\sprklai\zenii\config\config.toml` |
| Docker | `/config/config.toml` (mount volume, pass `--config /config/config.toml`) |

**Essential config fields:**

```toml
# Network
gateway_host = "127.0.0.1"    # Use "0.0.0.0" for Docker/remote access
gateway_port = 18981

# Security
gateway_auth_token = "your-secret-token"

# AI Provider (defaults: anthropic / claude-sonnet-4-6)
provider_name = "openai"       # openai, anthropic, groq, together, openrouter, local
provider_model_id = "gpt-4o"

# Logging
log_level = "info"             # trace, debug, info, warn, error
# log_dir = ""                # Override log directory (default: {data_dir}/logs/)
# log_keep_days = 30          # Days to keep log files before auto-cleanup

# CORS (for browser frontends)
gateway_cors_origins = ["http://localhost:5173"]
```

See the [Configuration reference](./configuration) for the full field reference.

---

## HTTP API Integration

Zenii exposes a REST API on port 18981. Any language with an HTTP client can integrate.

**Base URL:** `http://localhost:18981`

**Interactive docs:** Open `http://localhost:18981/api-docs` in a browser for the Scalar UI (OpenAPI explorer).

### Authentication

All requests (except `GET /health`) require a bearer token if `gateway_auth_token` is set:

```
Authorization: Bearer <your-token>
```

### Python

```python
import requests

BASE = "http://localhost:18981"
HEADERS = {"Authorization": "Bearer your-token"}

# Health check
r = requests.get(f"{BASE}/health")
print(r.json())  # {"status": "ok"}

# Create a session
r = requests.post(f"{BASE}/sessions", headers=HEADERS,
                  json={"title": "Python session"})
session = r.json()
print(session["id"])

# Send a chat message (non-streaming)
r = requests.post(f"{BASE}/chat", headers=HEADERS, json={
    "prompt": "What is the capital of France?",
    "session_id": session["id"]
})
print(r.json()["response"])

# Search memory
r = requests.get(f"{BASE}/memory", headers=HEADERS,
                 params={"q": "meeting notes", "limit": 10})
for entry in r.json():
    print(entry["content"][:100])

# Store a memory
requests.post(f"{BASE}/memory", headers=HEADERS, json={
    "key": "user-preference",
    "content": "User prefers concise answers",
    "category": "Core"
})

# List providers
r = requests.get(f"{BASE}/providers", headers=HEADERS)
for p in r.json():
    print(f"{p['id']}: {p['name']}")

# List available tools
r = requests.get(f"{BASE}/tools", headers=HEADERS)
for t in r.json():
    print(f"{t['name']}: {t['description']}")
```

### JavaScript / TypeScript

```typescript
const BASE = "http://localhost:18981";
const headers = {
  "Authorization": "Bearer your-token",
  "Content-Type": "application/json",
};

// Health check
const health = await fetch(`${BASE}/health`).then(r => r.json());
console.log(health); // { status: "ok" }

// Create session
const session = await fetch(`${BASE}/sessions`, {
  method: "POST",
  headers,
  body: JSON.stringify({ title: "JS session" }),
}).then(r => r.json());

// Chat
const chat = await fetch(`${BASE}/chat`, {
  method: "POST",
  headers,
  body: JSON.stringify({
    prompt: "Explain async/await in JavaScript",
    session_id: session.id,
  }),
}).then(r => r.json());
console.log(chat.response);

// Memory search
const memories = await fetch(
  `${BASE}/memory?q=project+notes&limit=5`,
  { headers }
).then(r => r.json());
```

### Go

```go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
)

const baseURL = "http://localhost:18981"
const token = "your-token"

func main() {
    // Chat request
    body, _ := json.Marshal(map[string]string{
        "prompt": "What is Rust's ownership model?",
    })

    req, _ := http.NewRequest("POST", baseURL+"/chat", bytes.NewReader(body))
    req.Header.Set("Authorization", "Bearer "+token)
    req.Header.Set("Content-Type", "application/json")

    resp, err := http.DefaultClient.Do(req)
    if err != nil {
        panic(err)
    }
    defer resp.Body.Close()

    var result map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&result)
    fmt.Println(result["response"])
}
```

### Rust

```rust
use reqwest::Client;
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let base = "http://localhost:18981";
    let token = "your-token";

    // Chat
    let resp: Value = client
        .post(format!("{base}/chat"))
        .bearer_auth(token)
        .json(&json!({
            "prompt": "Explain Rust lifetimes"
        }))
        .send()
        .await?
        .json()
        .await?;

    println!("{}", resp["response"]);
    Ok(())
}
```

### Ruby

```ruby
require 'net/http'
require 'json'
require 'uri'

BASE = "http://localhost:18981"
TOKEN = "your-token"

def zenii_request(method, path, body = nil)
  uri = URI("#{BASE}#{path}")
  http = Net::HTTP.new(uri.host, uri.port)

  req = case method
        when :get  then Net::HTTP::Get.new(uri)
        when :post then Net::HTTP::Post.new(uri)
        end

  req["Authorization"] = "Bearer #{TOKEN}"
  req["Content-Type"] = "application/json"
  req.body = body.to_json if body

  JSON.parse(http.request(req).body)
end

# Chat
result = zenii_request(:post, "/chat", { prompt: "Hello from Ruby!" })
puts result["response"]

# List sessions
sessions = zenii_request(:get, "/sessions")
sessions.each { |s| puts "#{s['id']}: #{s['title']}" }
```

### Java / Kotlin

```java
import java.net.URI;
import java.net.http.*;
import com.google.gson.*;

public class ZeniiClient {
    static final String BASE = "http://localhost:18981";
    static final String TOKEN = "your-token";
    static final HttpClient client = HttpClient.newHttpClient();
    static final Gson gson = new Gson();

    public static void main(String[] args) throws Exception {
        // Chat
        var body = gson.toJson(java.util.Map.of(
            "prompt", "Explain Java streams"
        ));

        var request = HttpRequest.newBuilder()
            .uri(URI.create(BASE + "/chat"))
            .header("Authorization", "Bearer " + TOKEN)
            .header("Content-Type", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(body))
            .build();

        var response = client.send(request, HttpResponse.BodyHandlers.ofString());
        var result = gson.fromJson(response.body(), JsonObject.class);
        System.out.println(result.get("response").getAsString());
    }
}
```

### C# / .NET

```csharp
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;

var client = new HttpClient();
client.BaseAddress = new Uri("http://localhost:18981");
client.DefaultRequestHeaders.Authorization =
    new AuthenticationHeaderValue("Bearer", "your-token");

// Chat
var chatBody = new StringContent(
    JsonSerializer.Serialize(new { prompt = "Hello from C#!" }),
    Encoding.UTF8, "application/json");

var response = await client.PostAsync("/chat", chatBody);
var json = await response.Content.ReadAsStringAsync();
var result = JsonDocument.Parse(json);
Console.WriteLine(result.RootElement.GetProperty("response").GetString());

// Health check
var health = await client.GetStringAsync("/health");
Console.WriteLine(health);
```

### Shell / cURL

```bash
TOKEN="your-token"
BASE="http://localhost:18981"

# Health check
curl $BASE/health

# Chat
curl -s -X POST $BASE/chat \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Hello from cURL!"}' | jq .response

# Create session
curl -s -X POST $BASE/sessions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title": "Shell session"}' | jq .

# List tools
curl -s $BASE/tools -H "Authorization: Bearer $TOKEN" | jq '.[].name'

# Store memory
curl -s -X POST $BASE/memory \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "work-note", "content": "Important note", "category": "Core"}'

# Search memory
curl -s "$BASE/memory?q=note&limit=5" \
  -H "Authorization: Bearer $TOKEN" | jq .

# System info
curl -s $BASE/system/info -H "Authorization: Bearer $TOKEN" | jq .
```

---

## WebSocket Streaming

For real-time streaming responses, connect via WebSocket. This is how the desktop app and CLI get token-by-token output.

### Protocol

**Endpoint:** `ws://localhost:18981/ws/chat?token=<auth_token>`

**Client sends:**

```json
{
  "prompt": "Your message here",
  "session_id": "optional-uuid",
  "model": "optional-model-id"
}
```

**Server sends (in order):**

| Type | Fields | Description |
|------|--------|-------------|
| `text` | `content` | Streaming token (partial response) |
| `tool_call` | `call_id`, `tool_name`, `args` | Agent is invoking a tool |
| `tool_result` | `call_id`, `tool_name`, `output`, `success`, `duration_ms` | Tool execution result |
| `done` | — | Response complete |
| `error` | `error` | Error occurred |

**Notification endpoint** (`ws://localhost:18981/ws/notifications?token=<auth_token>`) pushes real-time events:

| Type | Fields | Description |
|------|--------|-------------|
| `notification` | `event_type`, `job_id`, `job_name`, `message`, `status`, `error` | Scheduler events |
| `channel_message` | `channel`, `sender`, `session_id`, `content_preview`, `role` | Incoming channel messages |

### Python (websockets)

```python
import asyncio
import json
import websockets

async def chat_stream(prompt: str):
    uri = "ws://localhost:18981/ws/chat?token=your-token"

    async with websockets.connect(uri) as ws:
        await ws.send(json.dumps({"prompt": prompt}))

        async for message in ws:
            data = json.loads(message)

            if data["type"] == "text":
                print(data["content"], end="", flush=True)
            elif data["type"] == "tool_call":
                print(f"\n[Calling {data['tool_name']}...]")
            elif data["type"] == "tool_result":
                print(f"[Tool result: {data['output'][:100]}...]")
            elif data["type"] == "done":
                print("\n--- Done ---")
                break
            elif data["type"] == "error":
                print(f"\nError: {data['error']}")
                break

asyncio.run(chat_stream("Write a haiku about Rust"))
```

### JavaScript / TypeScript (Browser)

```typescript
function chatStream(prompt: string, token: string): void {
  const ws = new WebSocket(
    `ws://localhost:18981/ws/chat?token=${token}`
  );

  ws.onopen = () => {
    ws.send(JSON.stringify({ prompt }));
  };

  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);

    switch (data.type) {
      case "text":
        // Append to UI
        document.getElementById("output")!.textContent += data.content;
        break;
      case "tool_call":
        console.log(`Calling tool: ${data.tool_name}`);
        break;
      case "tool_result":
        console.log(`Tool result: ${data.output}`);
        break;
      case "done":
        console.log("Stream complete");
        ws.close();
        break;
      case "error":
        console.error(`Error: ${data.error}`);
        ws.close();
        break;
    }
  };
}
```

### JavaScript / TypeScript (Node.js)

```typescript
import WebSocket from "ws";

function chatStream(prompt: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(
      "ws://localhost:18981/ws/chat?token=your-token"
    );
    let fullResponse = "";

    ws.on("open", () => {
      ws.send(JSON.stringify({ prompt }));
    });

    ws.on("message", (raw: Buffer) => {
      const data = JSON.parse(raw.toString());

      if (data.type === "text") {
        fullResponse += data.content;
        process.stdout.write(data.content);
      } else if (data.type === "done") {
        ws.close();
        resolve(fullResponse);
      } else if (data.type === "error") {
        ws.close();
        reject(new Error(data.error));
      }
    });
  });
}

const response = await chatStream("Explain WebSockets");
console.log("\nFull response length:", response.length);
```

### Go (gorilla/websocket)

```go
package main

import (
    "encoding/json"
    "fmt"
    "log"
    "net/url"

    "github.com/gorilla/websocket"
)

type WSMessage struct {
    Type    string `json:"type"`
    Content string `json:"content,omitempty"`
    Error   string `json:"error,omitempty"`
}

func main() {
    u := url.URL{
        Scheme:   "ws",
        Host:     "localhost:18981",
        Path:     "/ws/chat",
        RawQuery: "token=your-token",
    }

    conn, _, err := websocket.DefaultDialer.Dial(u.String(), nil)
    if err != nil {
        log.Fatal(err)
    }
    defer conn.Close()

    // Send prompt
    conn.WriteJSON(map[string]string{
        "prompt": "Explain Go concurrency",
    })

    // Read streaming response
    for {
        _, raw, err := conn.ReadMessage()
        if err != nil {
            break
        }

        var msg WSMessage
        json.Unmarshal(raw, &msg)

        switch msg.Type {
        case "text":
            fmt.Print(msg.Content)
        case "done":
            fmt.Println("\n--- Done ---")
            return
        case "error":
            log.Fatalf("Error: %s", msg.Error)
        }
    }
}
```

### Rust (tokio-tungstenite)

```rust
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://localhost:18981/ws/chat?token=your-token";
    let (mut ws, _) = connect_async(url).await?;

    // Send prompt
    let msg = json!({"prompt": "Explain async Rust"}).to_string();
    ws.send(msg.into()).await?;

    // Read streaming response
    while let Some(Ok(msg)) = ws.next().await {
        if let Ok(text) = msg.to_text() {
            let data: Value = serde_json::from_str(text)?;
            match data["type"].as_str() {
                Some("text") => print!("{}", data["content"].as_str().unwrap_or("")),
                Some("done") => {
                    println!("\n--- Done ---");
                    break;
                }
                Some("error") => {
                    eprintln!("Error: {}", data["error"]);
                    break;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
```

---

## Linux Single-Board Computers

Zenii runs well on ARM-based Linux boards as a headless daemon.

### Raspberry Pi 4/5 (ARM64)

Recommended setup — 4GB+ RAM, 64-bit Raspberry Pi OS.

```bash
# Install pre-built ARM64 binaries
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-arm64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-arm64
chmod +x zenii-arm64 zenii-daemon-arm64
sudo mv zenii-arm64 /usr/local/bin/zenii
sudo mv zenii-daemon-arm64 /usr/local/bin/zenii-daemon

# Create config
mkdir -p ~/.config/zenii
cat > ~/.config/zenii/config.toml <<'EOF'
gateway_host = "0.0.0.0"
gateway_port = 18981
provider_name = "openai"
provider_model_id = "gpt-4o-mini"
log_level = "info"
gateway_auth_token = "your-secret-token"
EOF

# Start as systemd service
sudo cp zenii.service /etc/systemd/system/
sudo systemctl enable --now zenii
```

### Raspberry Pi 3 / Zero 2W (ARMv7)

Limited RAM (512MB-1GB). Use minimal features.

```bash
# ARMv7 binary
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-armv7
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-armv7
chmod +x zenii-armv7 zenii-daemon-armv7
sudo mv zenii-armv7 /usr/local/bin/zenii
sudo mv zenii-daemon-armv7 /usr/local/bin/zenii-daemon

# Build from source without optional features (smaller binary)
cargo build --release -p zenii-daemon --no-default-features
```

### NVIDIA Jetson

Jetson Nano/Xavier/Orin run Ubuntu ARM64. Use the standard ARM64 binary:

```bash
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-arm64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-arm64
chmod +x zenii-arm64 zenii-daemon-arm64
sudo mv zenii-arm64 /usr/local/bin/zenii
sudo mv zenii-daemon-arm64 /usr/local/bin/zenii-daemon
```

If you want to use local embeddings (fastembed), build with the `local-embeddings` feature:

```bash
cargo build --release -p zenii-daemon --features local-embeddings
```

### Orange Pi / Rock Pi / Pine64

Most modern ARM64 SBCs running Armbian or Ubuntu work with the ARM64 binary. For older ARMv7 boards, use the ARMv7 build.

```bash
# Check your architecture
uname -m
# aarch64 → use ARM64 binary
# armv7l  → use ARMv7 binary
```

### Performance Tuning for Low-Resource Boards

For boards with limited RAM (< 2GB), adjust `config.toml`:

```toml
# Use a lightweight model
provider_model_id = "gpt-4o-mini"

# Reduce memory/DB overhead
log_level = "warn"

# Disable optional features if building from source
# Build with: --no-default-features --features gateway,ai
```

**Docker on ARM boards:**

```bash
# Docker Compose works on Raspberry Pi with 64-bit OS
# The Dockerfile multi-stage build handles ARM64 natively
docker compose up -d
```

---

## Docker Compose Recipes

### Basic

The default `docker-compose.yml` included in the repo:

```yaml
services:
  zenii:
    build: .
    ports:
      - "18981:18981"
    volumes:
      - ./config:/config:ro
      - zenii-data:/data
    environment:
      - ZENII_TOKEN=${ZENII_TOKEN:-}
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:18981/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    restart: unless-stopped

volumes:
  zenii-data:
```

### With Reverse Proxy (Caddy)

Expose Zenii over HTTPS with automatic TLS:

```yaml
services:
  zenii:
    build: .
    volumes:
      - ./config:/config:ro
      - zenii-data:/data
    environment:
      - ZENII_TOKEN=${ZENII_TOKEN}
      - RUST_LOG=info
    restart: unless-stopped
    # No ports exposed — Caddy handles external access

  caddy:
    image: caddy:2-alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
    restart: unless-stopped

volumes:
  zenii-data:
  caddy-data:
```

**Caddyfile:**

```
zenii.example.com {
    reverse_proxy zenii:18981
}
```

### Multi-Instance

Run separate Zenii instances for different use cases:

```yaml
services:
  zenii-work:
    build: .
    ports:
      - "18981:18981"
    volumes:
      - ./config/work:/config:ro
      - work-data:/data
    environment:
      - ZENII_TOKEN=${WORK_TOKEN}
    restart: unless-stopped

  zenii-personal:
    build: .
    ports:
      - "18982:18981"
    volumes:
      - ./config/personal:/config:ro
      - personal-data:/data
    environment:
      - ZENII_TOKEN=${PERSONAL_TOKEN}
    restart: unless-stopped

volumes:
  work-data:
  personal-data:
```

---

## Cloud and Virtual Machine Deployment

Zenii runs on any Linux VM, container service, or VPS. The daemon is a single statically-linked binary with an embedded SQLite database — no external database or message queue required. This makes it straightforward to deploy anywhere.

**Minimum requirements:**
- 1 vCPU, 512MB RAM (lightweight usage)
- 2 vCPU, 1GB RAM (recommended for concurrent users)
- 1GB disk (binary + database + config)

### AWS (EC2)

**Launch an EC2 instance and run Zenii as a systemd service.**

```bash
# 1. Launch an EC2 instance (Amazon Linux 2023 or Ubuntu 24.04)
#    - t3.micro (free tier eligible) or t3.small for production
#    - Security group: allow inbound TCP 18981 (or 443 if using a reverse proxy)

# 2. SSH into the instance
ssh -i your-key.pem ec2-user@<instance-ip>

# 3. Install Zenii
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-linux
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-linux
chmod +x zenii-linux zenii-daemon-linux
sudo mv zenii-linux /usr/local/bin/zenii
sudo mv zenii-daemon-linux /usr/local/bin/zenii-daemon

# 4. Configure
mkdir -p ~/.config/zenii
cat > ~/.config/zenii/config.toml <<'EOF'
gateway_host = "0.0.0.0"
gateway_port = 18981
provider_name = "openai"
provider_model_id = "gpt-4o"
log_level = "info"
gateway_auth_token = "generate-a-strong-token-here"
EOF

# 5. Create systemd service
sudo tee /etc/systemd/system/zenii.service <<'EOF'
[Unit]
Description=Zenii AI Assistant
After=network.target

[Service]
Type=simple
User=ec2-user
ExecStart=/usr/local/bin/zenii-daemon
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now zenii

# 6. Verify
curl http://localhost:18981/health
```

**With an Application Load Balancer (HTTPS + WebSocket):**

- Create an ALB with HTTPS listener (ACM certificate)
- Target group: instance on port 18981, health check path `/health`
- Enable sticky sessions for WebSocket connections
- Set idle timeout to 3600s for long-running WebSocket chats

**Graviton (ARM64) for cost savings:**

```bash
# Use a t4g.micro/small instance (ARM64 Graviton, ~20% cheaper)
# Download the ARM64 binary instead:
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-arm64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-arm64
chmod +x zenii-arm64 zenii-daemon-arm64
sudo mv zenii-arm64 /usr/local/bin/zenii
sudo mv zenii-daemon-arm64 /usr/local/bin/zenii-daemon
```

### AWS (ECS Fargate)

**Serverless container — no instance management.**

```bash
# 1. Push image to ECR
aws ecr create-repository --repository-name zenii
aws ecr get-login-password | docker login --username AWS --password-stdin <account-id>.dkr.ecr.<region>.amazonaws.com

docker build -t zenii .
docker tag zenii:latest <account-id>.dkr.ecr.<region>.amazonaws.com/zenii:latest
docker push <account-id>.dkr.ecr.<region>.amazonaws.com/zenii:latest
```

**Task definition (`zenii-task.json`):**

```json
{
  "family": "zenii",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "512",
  "memory": "1024",
  "containerDefinitions": [
    {
      "name": "zenii",
      "image": "<account-id>.dkr.ecr.<region>.amazonaws.com/zenii:latest",
      "portMappings": [
        { "containerPort": 18981, "protocol": "tcp" }
      ],
      "environment": [
        { "name": "RUST_LOG", "value": "info" }
      ],
      "secrets": [
        {
          "name": "ZENII_TOKEN",
          "valueFrom": "arn:aws:secretsmanager:<region>:<account>:secret:zenii-token"
        }
      ],
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:18981/health || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3
      },
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/zenii",
          "awslogs-region": "<region>",
          "awslogs-stream-prefix": "zenii"
        }
      }
    }
  ]
}
```

```bash
# Register and run
aws ecs register-task-definition --cli-input-json file://zenii-task.json
aws ecs create-service \
  --cluster default \
  --service-name zenii \
  --task-definition zenii \
  --desired-count 1 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[subnet-xxx],securityGroups=[sg-xxx],assignPublicIp=ENABLED}"
```

> **Note:** ECS Fargate tasks have ephemeral storage. For persistent data (SQLite DB, memories), mount an EFS volume or use a task with EBS volume support.

### AWS (Lightsail)

The simplest AWS option — fixed monthly pricing.

```bash
# 1. Create a Lightsail instance ($3.50/mo for 512MB, $5/mo for 1GB)
#    - Choose Ubuntu 24.04 or Amazon Linux 2023
#    - Open port 18981 in Networking tab

# 2. SSH in and follow the same steps as EC2 above
ssh ubuntu@<lightsail-ip>
# ... install binary, configure, create systemd service
```

### Google Cloud (GCE)

```bash
# 1. Create a VM
gcloud compute instances create zenii-vm \
  --zone=us-central1-a \
  --machine-type=e2-micro \
  --image-family=ubuntu-2404-lts-amd64 \
  --image-project=ubuntu-os-cloud \
  --tags=zenii

# 2. Open firewall
gcloud compute firewall-rules create zenii-allow \
  --allow=tcp:18981 \
  --target-tags=zenii

# 3. SSH and install
gcloud compute ssh zenii-vm --zone=us-central1-a
# ... install binary, configure, create systemd service (same as EC2)
```

### Google Cloud (Cloud Run)

**Fully managed, scales to zero when idle.**

```bash
# 1. Build and push to Artifact Registry
gcloud artifacts repositories create zenii --repository-format=docker --location=us-central1
gcloud builds submit --tag us-central1-docker.pkg.dev/<project>/zenii/zenii:latest

# 2. Deploy
gcloud run deploy zenii \
  --image us-central1-docker.pkg.dev/<project>/zenii/zenii:latest \
  --port 18981 \
  --memory 1Gi \
  --cpu 1 \
  --min-instances 0 \
  --max-instances 1 \
  --set-env-vars "RUST_LOG=info" \
  --set-secrets "ZENII_TOKEN=zenii-token:latest" \
  --allow-unauthenticated
```

> **Caveat:** Cloud Run has request timeouts (default 5 min, max 60 min). Long WebSocket sessions may be interrupted. Use `--session-affinity` and increase timeout for streaming chat.

### Azure (VM)

```bash
# 1. Create a VM
az vm create \
  --resource-group zenii-rg \
  --name zenii-vm \
  --image Ubuntu2404 \
  --size Standard_B1s \
  --admin-username azureuser \
  --generate-ssh-keys

# 2. Open port
az vm open-port --resource-group zenii-rg --name zenii-vm --port 18981

# 3. SSH and install
ssh azureuser@<vm-ip>
# ... install binary, configure, create systemd service (same as EC2)
```

### Azure (Container Instances)

**Quick serverless containers — no cluster needed.**

```bash
# 1. Create container registry and push image
az acr create --resource-group zenii-rg --name zeniiregistry --sku Basic
az acr build --registry zeniiregistry --image zenii:latest .

# 2. Deploy container
az container create \
  --resource-group zenii-rg \
  --name zenii \
  --image zeniiregistry.azurecr.io/zenii:latest \
  --cpu 1 \
  --memory 1 \
  --ports 18981 \
  --environment-variables RUST_LOG=info \
  --secure-environment-variables ZENII_TOKEN=your-secret-token \
  --ip-address Public
```

### DigitalOcean

```bash
# 1. Create a droplet ($4/mo for 512MB, $6/mo for 1GB)
doctl compute droplet create zenii \
  --region nyc3 \
  --size s-1vcpu-512mb-10gb \
  --image ubuntu-24-04-x64 \
  --ssh-keys <your-key-fingerprint>

# 2. SSH and install
ssh root@<droplet-ip>
# ... install binary, configure, create systemd service

# Or use DigitalOcean App Platform with Docker:
# Push your repo, set Dockerfile path, expose port 18981
```

### Hetzner

Excellent value — ARM64 CAX servers start at ~$4/mo.

```bash
# 1. Create a server via CLI or console
#    - CAX11 (ARM64, 2 vCPU, 4GB RAM, ~$4/mo) — great value
#    - CX22 (x86, 2 vCPU, 4GB RAM, ~$4/mo)

# 2. SSH and install
ssh root@<server-ip>

# For ARM64 (CAX):
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-arm64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-arm64
chmod +x zenii-arm64 zenii-daemon-arm64
mv zenii-arm64 /usr/local/bin/zenii
mv zenii-daemon-arm64 /usr/local/bin/zenii-daemon

# For x86 (CX):
# curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-linux
# curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-linux
# chmod +x zenii-linux zenii-daemon-linux
# mv zenii-linux /usr/local/bin/zenii
# mv zenii-daemon-linux /usr/local/bin/zenii-daemon

# ... configure and create systemd service
```

### Linode / Akamai

```bash
# 1. Create a Linode ($5/mo Nanode for 1GB)
linode-cli linodes create \
  --type g6-nanode-1 \
  --region us-east \
  --image linode/ubuntu24.04 \
  --root_pass <password> \
  --label zenii

# 2. SSH and install
ssh root@<linode-ip>
# ... install binary, configure, create systemd service
```

### Oracle Cloud (Free Tier)

Oracle offers always-free ARM64 instances — up to 4 OCPU and 24GB RAM.

```bash
# 1. Create an Always Free Ampere (ARM64) instance
#    - Shape: VM.Standard.A1.Flex (1-4 OCPU, 6-24GB RAM, free)
#    - Image: Ubuntu 24.04

# 2. SSH and install ARM64 binary
ssh ubuntu@<instance-ip>
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-arm64
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-arm64
chmod +x zenii-arm64 zenii-daemon-arm64
sudo mv zenii-arm64 /usr/local/bin/zenii
sudo mv zenii-daemon-arm64 /usr/local/bin/zenii-daemon

# 3. Open port in OCI security list
#    Network > Virtual Cloud Networks > Security Lists > Add Ingress Rule
#    Source: 0.0.0.0/0, Protocol: TCP, Port: 18981

# ... configure and create systemd service
```

### Fly.io

**Global edge deployment with persistent volumes.**

Create `fly.toml`:

```toml
app = "zenii"
primary_region = "iad"

[build]
  dockerfile = "Dockerfile"

[env]
  RUST_LOG = "info"

[http_service]
  internal_port = 18981
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 0

[mounts]
  source = "zenii_data"
  destination = "/data"

[[vm]]
  size = "shared-cpu-1x"
  memory = "512mb"
```

```bash
fly launch --no-deploy
fly secrets set ZENII_TOKEN=your-secret-token
fly volumes create zenii_data --size 1 --region iad
fly deploy

# Your app is at: https://zenii.fly.dev
curl https://zenii.fly.dev/health
```

### Railway

**Git-push-to-deploy with zero config.**

```bash
# 1. Install Railway CLI
npm install -g @railway/cli

# 2. Login and init
railway login
railway init

# 3. Set variables
railway variables set ZENII_TOKEN=your-secret-token
railway variables set RUST_LOG=info

# 4. Deploy (auto-detects Dockerfile)
railway up

# Railway assigns a public URL automatically
```

### Any Linux VM or VPS

This generic guide works for any provider (Vultr, Scaleway, OVH, Contabo, etc.):

```bash
# 1. SSH into your server
ssh user@<server-ip>

# 2. Download the binaries (pick your architecture)
# x86_64:
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-linux
curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-linux
chmod +x zenii-linux zenii-daemon-linux
sudo mv zenii-linux /usr/local/bin/zenii
sudo mv zenii-daemon-linux /usr/local/bin/zenii-daemon
# ARM64:
# curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-arm64
# curl -LO https://github.com/sprklai/zenii/releases/latest/download/zenii-daemon-arm64
# chmod +x zenii-arm64 zenii-daemon-arm64
# sudo mv zenii-arm64 /usr/local/bin/zenii
# sudo mv zenii-daemon-arm64 /usr/local/bin/zenii-daemon

# 3. Configure
mkdir -p ~/.config/zenii
cat > ~/.config/zenii/config.toml <<'EOF'
gateway_host = "0.0.0.0"
gateway_port = 18981
provider_name = "openai"
provider_model_id = "gpt-4o"
log_level = "info"
gateway_auth_token = "$(openssl rand -hex 32)"
EOF

# 4. Create systemd service
sudo tee /etc/systemd/system/zenii.service <<'EOF'
[Unit]
Description=Zenii AI Assistant
After=network.target

[Service]
Type=simple
User=nobody
ExecStart=/usr/local/bin/zenii-daemon
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now zenii

# 5. Verify
curl http://localhost:18981/health

# 6. (Optional) Set API keys via HTTP
curl -X POST http://localhost:18981/credentials \
  -H "Authorization: Bearer <your-gateway-token>" \
  -H "Content-Type: application/json" \
  -d '{"key": "api_key:openai", "value": "sk-your-openai-key"}'
```

**Or use Docker Compose on any VM:**

```bash
# Install Docker
curl -fsSL https://get.docker.com | sh

# Clone and run
git clone https://github.com/sprklai/zenii.git
cd zenii
echo "ZENII_TOKEN=$(openssl rand -hex 32)" > .env
docker compose up -d
```

### Security Hardening for Cloud

When exposing Zenii to the internet, follow these practices:

**1. Always set an auth token:**

```toml
gateway_auth_token = "use-a-long-random-string-here"
```

Generate one with: `openssl rand -hex 32`

**2. Use a reverse proxy with TLS (never expose port 18981 directly):**

```bash
# Caddy (auto-HTTPS)
sudo apt install caddy
cat > /etc/caddy/Caddyfile <<'EOF'
zenii.yourdomain.com {
    reverse_proxy localhost:18981
}
EOF
sudo systemctl restart caddy
```

Or with nginx:

```nginx
server {
    listen 443 ssl;
    server_name zenii.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/zenii.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/zenii.yourdomain.com/privkey.pem;

    location / {
        proxy_pass http://localhost:18981;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 3600s;
    }
}
```

**3. Firewall — only allow 80/443, block 18981 from public:**

```bash
# UFW (Ubuntu)
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw deny 18981/tcp
sudo ufw enable

# Or iptables
sudo iptables -A INPUT -p tcp --dport 18981 -s 127.0.0.1 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 18981 -j DROP
```

**4. Restrict CORS origins:**

```toml
gateway_cors_origins = ["https://zenii.yourdomain.com"]
```

**5. Run as a non-root user:**

```bash
sudo useradd --system --no-create-home zenii
sudo chown -R zenii:zenii /home/zenii/.config/zenii /home/zenii/.local/share/zenii
# Update systemd service to use User=zenii
```

**6. Regular backups:**

```bash
# SQLite databases are in the data directory
# Back up daily with cron
0 2 * * * sqlite3 ~/.local/share/zenii/zenii.db ".backup /backups/zenii-$(date +\%Y\%m\%d).db"
```

---

## Troubleshooting

**Connection refused on port 18981:**
- Is the daemon running? `zenii-daemon` or `docker compose ps`
- Check if another process uses the port: `ss -tlnp | grep 18981`
- If using Docker, ensure `gateway_host = "0.0.0.0"` in config (not `127.0.0.1`)

**401 Unauthorized:**
- Set the token: `Authorization: Bearer <token>` header or `?token=<token>` for WebSocket
- Check `gateway_auth_token` in config matches what you're sending
- `GET /health` bypasses auth — use it to verify the server is running

**WebSocket disconnects immediately:**
- Verify the token is passed as query parameter: `ws://host:18981/ws/chat?token=<token>`
- Check server logs: `RUST_LOG=debug zenii-daemon`

**Docker: container exits immediately:**
- Check logs: `docker compose logs zenii`
- Ensure config volume is mounted correctly
- Verify config.toml syntax: valid TOML with correct field names

**ARM build fails:**
- Install cross-compilation tools: `sudo apt install gcc-aarch64-linux-gnu`
- Or use Docker-based cross-compilation: `./scripts/build.sh --target linux-arm64 --docker`

**Keyring not available (headless/Docker):**
- Zenii falls back to in-memory credential storage automatically
- Set API keys via the HTTP API after startup:
  ```bash
  curl -X POST http://localhost:18981/credentials \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"key": "api_key:openai", "value": "sk-your-key"}'
  ```

**OpenAPI docs not available at /api-docs:**
- Built without the `api-docs` feature. Rebuild with: `cargo build -p zenii-daemon` (enabled by default)
