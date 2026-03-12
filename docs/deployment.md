# MesoClaw Deployment Guide

This document covers deploying MesoClaw in various environments: native binaries, Docker containers, Raspberry Pi, and behind reverse proxies.

> **Note**: This document was generated with AI assistance and may contain inaccuracies. If you find errors, please [report an issue](https://github.com/sprklai/mesoclaw/issues).

## Table of Contents

- [Native Deployment](#native-deployment)
- [systemd Service](#systemd-service)
- [Docker Deployment](#docker-deployment)
- [Raspberry Pi Deployment](#raspberry-pi-deployment)
- [Reverse Proxy](#reverse-proxy)
- [Backup and Restore](#backup-and-restore)
- [Monitoring](#monitoring)
- [Upgrading](#upgrading)

---

## Native Deployment

### 1. Download the Binary

Download the appropriate binary for your platform from GitHub Releases:

| Platform       | Binary Name        | Architecture |
|----------------|--------------------|--------------|
| Linux x86_64   | `mesoclaw-daemon`  | x86_64       |
| Linux ARM64    | `mesoclaw-daemon`  | aarch64      |
| macOS x86_64   | `mesoclaw-daemon`  | x86_64       |
| macOS ARM      | `mesoclaw-daemon`  | aarch64      |
| Windows        | `mesoclaw-daemon.exe` | x86_64    |

### 2. Set Permissions (Linux/macOS)

```bash
chmod +x mesoclaw-daemon
sudo mv mesoclaw-daemon /usr/local/bin/
```

### 3. Create Configuration

MesoClaw uses platform-specific config directories:

| Platform | Config Path |
|----------|-------------|
| Linux    | `~/.config/mesoclaw/config.toml` |
| macOS    | `~/Library/Application Support/com.sprklai.mesoclaw/config.toml` |
| Windows  | `%APPDATA%\sprklai\mesoclaw\config\config.toml` |

Data directories (databases, identity files, skills):

| Platform | Data Path |
|----------|-----------|
| Linux    | `~/.local/share/mesoclaw/` |
| macOS    | `~/Library/Application Support/com.sprklai.mesoclaw/` |
| Windows  | `%APPDATA%\sprklai\mesoclaw\data\` |

Create a minimal `config.toml`:

```toml
# Gateway settings
gateway_host = "127.0.0.1"
gateway_port = 18981
log_level = "info"

# AI Provider
provider_name = "openai"
provider_model_id = "gpt-4o"

# Security — set a strong token for production
gateway_auth_token = "your-secret-token"
```

### 4. Set API Keys

Store your AI provider API key in the OS keyring. MesoClaw uses the keyring service ID `com.sprklai.mesoclaw` with key format `api_key:{provider_id}`:

```bash
# The daemon will prompt or you can use the CLI:
mesoclaw key set openai
# Or set via the gateway API after startup:
curl -X PUT http://localhost:18981/credentials/api_key:openai \
  -H "Authorization: Bearer your-secret-token" \
  -H "Content-Type: application/json" \
  -d '{"value": "sk-..."}'
```

### 5. Start the Daemon

```bash
mesoclaw-daemon
# Or with a custom config path:
mesoclaw-daemon --config /path/to/config.toml
```

The daemon starts the HTTP+WebSocket gateway on `127.0.0.1:18981` by default.

### 6. Verify

```bash
curl http://localhost:18981/health
```

---

## systemd Service

### 1. Create a Service User

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin mesoclaw
sudo mkdir -p /var/lib/mesoclaw
sudo chown mesoclaw:mesoclaw /var/lib/mesoclaw
```

### 2. Install the Binary

```bash
sudo cp mesoclaw-daemon /usr/local/bin/mesoclaw-daemon
sudo chmod +x /usr/local/bin/mesoclaw-daemon
```

### 3. Create Configuration

```bash
sudo mkdir -p /etc/mesoclaw
sudo tee /etc/mesoclaw/config.toml > /dev/null <<'EOF'
gateway_host = "127.0.0.1"
gateway_port = 18981
log_level = "info"
data_dir = "/var/lib/mesoclaw"
db_path = "/var/lib/mesoclaw/mesoclaw.db"
memory_db_path = "/var/lib/mesoclaw/memory_vec.db"

provider_name = "openai"
provider_model_id = "gpt-4o"
EOF
sudo chown mesoclaw:mesoclaw /etc/mesoclaw/config.toml
```

### 4. Create the Unit File

Save to `/etc/systemd/system/mesoclaw.service`:

```ini
[Unit]
Description=MesoClaw AI Agent Daemon
After=network.target

[Service]
Type=simple
User=mesoclaw
ExecStart=/usr/local/bin/mesoclaw-daemon --config /etc/mesoclaw/config.toml
Restart=on-failure
RestartSec=5
Environment=MESOCLAW_TOKEN=your-secret-token
Environment=RUST_LOG=info
WorkingDirectory=/var/lib/mesoclaw

# Hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/mesoclaw
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

### 5. Enable and Start

```bash
sudo systemctl daemon-reload
sudo systemctl enable mesoclaw
sudo systemctl start mesoclaw
```

### 6. View Logs

```bash
# Follow logs in real time
sudo journalctl -u mesoclaw -f

# View last 100 lines
sudo journalctl -u mesoclaw -n 100

# View logs since last boot
sudo journalctl -u mesoclaw -b
```

---

## Docker Deployment

### Build from Source

Pre-built Docker images are not currently published. Build from the repository:

```bash
git clone https://github.com/sprklai/mesoclaw.git
cd mesoclaw
docker build -t mesoclaw .
```

### Quick Start

```bash
docker run -d \
  --name mesoclaw \
  -p 18981:18981 \
  -v mesoclaw-data:/data \
  -e MESOCLAW_TOKEN=your-secret-token \
  -e RUST_LOG=info \
  mesoclaw
```

### Docker Compose

A `docker-compose.yml` is provided in the repository root:

```bash
git clone https://github.com/sprklai/mesoclaw.git
cd mesoclaw
export MESOCLAW_TOKEN=your-secret-token
docker compose up -d

# View logs
docker compose logs -f mesoclaw

# Stop
docker compose down
```

### Custom Configuration

Mount a config file and data directory:

```bash
docker run -d \
  --name mesoclaw \
  -p 18981:18981 \
  -v /path/to/config.toml:/config/config.toml:ro \
  -v mesoclaw-data:/data \
  -e MESOCLAW_TOKEN=your-secret-token \
  mesoclaw \
  --config /config/config.toml
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MESOCLAW_TOKEN` | Bearer token for API auth | (none) |
| `RUST_LOG` | Log level filter | `info` |

---

## Raspberry Pi Deployment

MesoClaw runs well on Raspberry Pi 4/5 with ARM64 (aarch64). SQLite is the only database dependency -- no external services needed.

### 1. Download ARM64 Binary

```bash
# Download the aarch64 build from releases
wget https://github.com/nsrtech/mesoclaw/releases/latest/download/mesoclaw-daemon-linux-arm64
chmod +x mesoclaw-daemon-linux-arm64
sudo mv mesoclaw-daemon-linux-arm64 /usr/local/bin/mesoclaw-daemon
```

### 2. Create Configuration

```bash
mkdir -p ~/.config/mesoclaw
cat > ~/.config/mesoclaw/config.toml <<'EOF'
gateway_host = "0.0.0.0"
gateway_port = 18981
log_level = "warn"

provider_name = "openai"
provider_model_id = "gpt-4o-mini"

# Performance tuning for Pi
ws_max_connections = 8
memory_default_limit = 5
embedding_cache_size = 200
security_audit_log_capacity = 200
EOF
```

### 3. Performance Tips

- Use `log_level = "warn"` to reduce I/O
- Use a lighter model like `gpt-4o-mini` to reduce response payload sizes
- Reduce `ws_max_connections`, `embedding_cache_size`, and `memory_default_limit`
- Store data on the SD card or an external USB SSD for better I/O
- Disable unused features: build without `--all-features` to keep memory footprint low

### 4. Auto-Start via systemd

Use the same systemd unit file from the [systemd section](#systemd-service), adjusting paths as needed. On Raspberry Pi OS:

```bash
sudo cp mesoclaw.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable mesoclaw
sudo systemctl start mesoclaw
```

---

## Reverse Proxy

MesoClaw uses both HTTP and WebSocket connections on the same port (default 18981). Your reverse proxy must handle WebSocket upgrade headers.

### nginx

```nginx
upstream mesoclaw {
    server 127.0.0.1:18981;
}

server {
    listen 443 ssl http2;
    server_name mesoclaw.example.com;

    ssl_certificate     /etc/ssl/certs/mesoclaw.pem;
    ssl_certificate_key /etc/ssl/private/mesoclaw.key;

    location / {
        proxy_pass http://mesoclaw;
        proxy_http_version 1.1;

        # WebSocket support
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Standard proxy headers
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeout for long-lived WebSocket connections
        proxy_read_timeout 86400s;
        proxy_send_timeout 86400s;
    }
}
```

### Caddy

Caddy handles HTTPS certificates automatically and supports WebSocket proxying out of the box:

```
mesoclaw.example.com {
    reverse_proxy localhost:18981
}
```

No additional WebSocket configuration is needed -- Caddy handles upgrade headers automatically.

---

## Backup and Restore

### What to Back Up

| Item | Location | Description |
|------|----------|-------------|
| Main database | `{data_dir}/mesoclaw.db` | Sessions, messages, providers, user observations, scheduler jobs |
| Vector database | `{data_dir}/memory_vec.db` | Memory embeddings |
| Config file | `{config_dir}/config.toml` | All settings |
| Identity files | `{data_dir}/identity/` | Soul, identity, user markdown files |
| Skills | `{data_dir}/skills/` | Custom skill templates |

### SQLite Database Backup

Use SQLite's online backup API for a consistent snapshot while the daemon is running:

```bash
# Stop the daemon for a clean backup (recommended)
sudo systemctl stop mesoclaw
cp /var/lib/mesoclaw/mesoclaw.db /backup/mesoclaw-$(date +%Y%m%d).db
cp /var/lib/mesoclaw/memory_vec.db /backup/memory_vec-$(date +%Y%m%d).db
sudo systemctl start mesoclaw

# Or use sqlite3 .backup for online backup (daemon can stay running)
sqlite3 /var/lib/mesoclaw/mesoclaw.db ".backup /backup/mesoclaw-$(date +%Y%m%d).db"
sqlite3 /var/lib/mesoclaw/memory_vec.db ".backup /backup/memory_vec-$(date +%Y%m%d).db"
```

### Full Backup Script

```bash
#!/bin/bash
BACKUP_DIR="/backup/mesoclaw/$(date +%Y%m%d-%H%M%S)"
DATA_DIR="/var/lib/mesoclaw"
CONFIG_DIR="/etc/mesoclaw"

mkdir -p "$BACKUP_DIR"

# Databases (online backup)
sqlite3 "$DATA_DIR/mesoclaw.db" ".backup $BACKUP_DIR/mesoclaw.db"
sqlite3 "$DATA_DIR/memory_vec.db" ".backup $BACKUP_DIR/memory_vec.db"

# Config and identity files
cp "$CONFIG_DIR/config.toml" "$BACKUP_DIR/"
cp -r "$DATA_DIR/identity" "$BACKUP_DIR/" 2>/dev/null || true
cp -r "$DATA_DIR/skills" "$BACKUP_DIR/" 2>/dev/null || true

echo "Backup complete: $BACKUP_DIR"
```

### Restore

```bash
sudo systemctl stop mesoclaw
cp /backup/mesoclaw.db /var/lib/mesoclaw/mesoclaw.db
cp /backup/memory_vec.db /var/lib/mesoclaw/memory_vec.db
cp /backup/config.toml /etc/mesoclaw/config.toml
cp -r /backup/identity /var/lib/mesoclaw/ 2>/dev/null || true
cp -r /backup/skills /var/lib/mesoclaw/ 2>/dev/null || true
sudo chown -R mesoclaw:mesoclaw /var/lib/mesoclaw
sudo systemctl start mesoclaw
```

### Credential Migration

OS keyring credentials (API keys stored via `keyring` crate) cannot be exported directly. When migrating to a new machine:

1. Back up your config and databases as described above
2. On the new machine, re-enter API keys via the CLI (`mesoclaw key set openai`) or the gateway API
3. Provider configurations (names, base URLs, model lists) are stored in the database and will transfer with the backup

---

## Monitoring

### Health Check

The daemon exposes a `GET /health` endpoint that returns HTTP 200 when the service is operational:

```bash
curl -f http://localhost:18981/health
```

Use this for:
- Load balancer health checks
- Docker HEALTHCHECK (included in docker-compose.yml)
- Uptime monitoring (UptimeRobot, Pingdom, etc.)
- systemd watchdog integration

### Log Levels

Control verbosity via the `log_level` config field or `RUST_LOG` environment variable:

| Level | Use Case |
|-------|----------|
| `error` | Production (minimal output) |
| `warn` | Production (includes warnings) |
| `info` | Default (startup info, request summaries) |
| `debug` | Development (detailed request/response logging) |
| `trace` | Troubleshooting (very verbose, includes SQL queries) |

The `RUST_LOG` environment variable overrides `log_level` in config and supports per-module filtering:

```bash
# Only show warnings from dependencies, debug for mesoclaw
RUST_LOG=warn,mesoclaw_core=debug mesoclaw-daemon
```

### systemd Watchdog

For systemd deployments, monitor the service status:

```bash
# Check if running
systemctl is-active mesoclaw

# Automated restart monitoring
systemctl show mesoclaw --property=NRestarts
```

Combine with the health endpoint for external monitoring:

```bash
# Cron job: restart if health check fails
*/5 * * * * curl -sf http://localhost:18981/health || systemctl restart mesoclaw
```

---

## Upgrading

### Binary Upgrade

1. Download the new binary
2. Stop the daemon: `sudo systemctl stop mesoclaw`
3. Replace the binary: `sudo cp mesoclaw-daemon /usr/local/bin/`
4. Start the daemon: `sudo systemctl start mesoclaw`

### Database Migrations

MesoClaw runs database migrations automatically on startup. No manual migration steps are needed. The migration system:

- Tracks the current schema version in a `migrations` table
- Applies pending migrations in order within transactions
- Is safe to run multiple times (idempotent)

### Docker Upgrade

```bash
cd mesoclaw
git pull
docker build -t mesoclaw .
docker compose up -d
```

### Configuration Compatibility

New config fields are always added with sensible defaults via `#[serde(default)]`. Upgrading to a newer version will not break existing `config.toml` files -- new fields simply use their defaults until explicitly set.

If a config field is deprecated in a future version, it will be documented in the release notes with migration instructions.
