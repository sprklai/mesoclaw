/// MesoClaw CLI — headless interface to the AI agent runtime.
///
/// Provides subcommands for managing the daemon, agents, memory, identity,
/// configuration, scheduling, channels, and launching the GUI. When invoked
/// with no subcommand the CLI enters an interactive REPL that streams
/// responses from the gateway WebSocket.
///
/// # CI matrix note
/// The following feature combinations should be tested in CI:
///   - `cargo build --features core,cli`           (minimal: no desktop, no gateway)
///   - `cargo build`                                (default features)
///   - `cargo build --all-features`                (full build)
use std::{
    fs,
    io::{self, BufRead, IsTerminal, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::{connect_async, tungstenite::Message};

// ---------------------------------------------------------------------------
// Top-level CLI struct
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "mesoclaw",
    about = "MesoClaw AI agent runtime CLI",
    version,
    long_about = "Headless interface to the MesoClaw AI agent daemon.\n\
                  Run without a subcommand to enter the interactive REPL."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Output raw text without formatting.
    #[arg(long, global = true)]
    raw: bool,

    /// Output results as JSON.
    #[arg(long, global = true)]
    json: bool,
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start or manage the background daemon process.
    Daemon(DaemonArgs),
    /// Manage AI agents (list, start, stop, inspect).
    Agent(AgentArgs),
    /// Manage persistent agent memory stores.
    Memory(MemoryArgs),
    /// Manage agent identities and persona files.
    Identity(IdentityArgs),
    /// View and edit application configuration.
    Config(ConfigArgs),
    /// Manage scheduled tasks and triggers.
    Schedule(ScheduleArgs),
    /// Manage communication channels (e.g., Telegram).
    Channel(ChannelArgs),
    /// Manage sidecar extension modules (list, install, remove, start, stop, health, reload, create).
    Module(ModuleArgs),
    /// Launch the MesoClaw desktop GUI.
    Gui,
}

#[derive(Parser, Debug)]
struct DaemonArgs {
    /// Daemon action: start | stop | status | restart.
    #[arg(default_value = "status")]
    action: String,

    /// Run the daemon in the foreground without detaching.
    /// Used internally when the binary self-spawns for background execution.
    #[arg(long, hide = true)]
    foreground: bool,
}

#[derive(Parser, Debug)]
struct AgentArgs {
    /// Agent action: list | start | stop | inspect.
    #[arg(default_value = "list")]
    action: String,
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct MemoryArgs {
    #[arg(default_value = "list")]
    action: String,
    key: Option<String>,
    value: Option<String>,
}

#[derive(Parser, Debug)]
struct IdentityArgs {
    #[arg(default_value = "list")]
    action: String,
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct ConfigArgs {
    #[arg(default_value = "list")]
    action: String,
    key: Option<String>,
    value: Option<String>,
}

#[derive(Parser, Debug)]
struct ScheduleArgs {
    #[arg(default_value = "list")]
    action: String,
    name: Option<String>,
}

#[derive(Parser, Debug)]
struct ChannelArgs {
    #[arg(default_value = "list")]
    action: String,
    channel_type: Option<String>,
}

#[derive(Parser, Debug)]
struct ModuleArgs {
    /// Module action: list | install | remove | start | stop | health | reload | create
    #[arg(default_value = "list")]
    action: String,
    /// Module name or ID (required for install, remove, start, stop, health, create).
    name: Option<String>,
    /// Module type for `create` (tool | service | mcp).
    #[arg(long, default_value = "tool")]
    module_type: String,
    /// Runtime for `create` (native | docker | podman).
    #[arg(long, default_value = "native")]
    runtime: String,
}

// ---------------------------------------------------------------------------
// Gateway client
// ---------------------------------------------------------------------------

/// Reads the PID file written by the daemon and returns (pid, port).
fn read_pid_and_port() -> Option<(u32, u16)> {
    let path = daemon_pid_path();
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let pid: u32 = lines.next()?.trim().parse().ok()?;
    let port: u16 = lines.next()?.trim().parse().ok()?;
    Some((pid, port))
}

fn read_token() -> Option<String> {
    let path = dirs::home_dir()?.join(".mesoclaw").join("daemon.token");
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn daemon_pid_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".mesoclaw")
        .join("daemon.pid")
}

fn is_daemon_running() -> Option<u16> {
    let (pid, port) = read_pid_and_port()?;
    // On Unix, check if the process is alive by sending signal 0.
    #[cfg(unix)]
    {
        use std::process::Command;
        let alive = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if alive { Some(port) } else { None }
    }
    #[cfg(not(unix))]
    {
        // On Windows, just assume if the PID file exists the daemon is running.
        let _ = pid;
        Some(port)
    }
}

struct GatewayClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

impl GatewayClient {
    fn new(port: u16, token: String) -> Self {
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            token,
            client: reqwest::Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    async fn health(&self) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/health", self.base_url))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn list_sessions(&self) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/sessions", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn create_session(&self, system_prompt: Option<&str>) -> reqwest::Result<Value> {
        self.client
            .post(format!("{}/api/v1/sessions", self.base_url))
            .header("Authorization", self.auth_header())
            .json(&json!({ "system_prompt": system_prompt }))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    fn ws_url(&self) -> String {
        self.base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://")
            + "/api/v1/ws"
    }

    async fn list_modules(&self) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/modules", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn module_action(&self, id: &str, action: &str) -> reqwest::Result<Value> {
        self.client
            .post(format!("{}/api/v1/modules/{id}/{action}", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn module_health(&self, id: &str) -> reqwest::Result<Value> {
        self.client
            .get(format!("{}/api/v1/modules/{id}/health", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }

    async fn reload_modules(&self) -> reqwest::Result<Value> {
        self.client
            .post(format!("{}/api/v1/modules", self.base_url))
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<Value>()
            .await
    }
}

/// Resolve or start the gateway, returning a ready client.
async fn require_gateway() -> Option<GatewayClient> {
    if let Some(port) = is_daemon_running()
        && let Some(token) = read_token()
    {
        return Some(GatewayClient::new(port, token));
    }
    eprintln!(
        "Gateway is not running.\n\
         Start it with: mesoclaw daemon start"
    );
    None
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

fn print_value(value: &Value, raw: bool, json_mode: bool) {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_default()
        );
    } else if raw {
        if let Some(s) = value.as_str() {
            println!("{s}");
        } else {
            println!("{value}");
        }
    } else {
        // Human-friendly default.
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_default()
        );
    }
}

fn print_err(msg: &str) {
    eprintln!("\x1b[31merror\x1b[0m: {msg}");
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

async fn dispatch(command: &Commands, raw: bool, json_mode: bool) {
    match command {
        Commands::Daemon(args) => handle_daemon(args).await,
        Commands::Agent(args) => handle_agent(args, raw, json_mode).await,
        Commands::Memory(args) => handle_memory(args, raw, json_mode).await,
        Commands::Identity(args) => handle_identity(args, raw, json_mode).await,
        Commands::Config(args) => {
            println!("config {}: not yet implemented (Phase 5)", args.action);
        }
        Commands::Schedule(args) => {
            println!("schedule {}: not yet implemented (Phase 4)", args.action);
        }
        Commands::Channel(args) => {
            println!("channel {}: not yet implemented (Phase 7)", args.action);
        }
        Commands::Module(args) => handle_module(args, raw, json_mode).await,
        Commands::Gui => {
            println!("gui: not yet implemented — launch mesoclaw-desktop directly");
        }
    }
}

async fn handle_daemon(args: &DaemonArgs) {
    match args.action.as_str() {
        "status" => match is_daemon_running() {
            Some(port) => {
                if let Some(client) = require_gateway().await {
                    match client.health().await {
                        Ok(v) => println!("daemon: running on port {port} — {v}"),
                        Err(e) => println!("daemon: port {port} (health check failed: {e})"),
                    }
                }
            }
            None => println!("daemon: not running"),
        },
        "start" => {
            if let Some(port) = is_daemon_running() {
                println!("daemon: already running on port {port}");
                return;
            }
            #[cfg(feature = "gateway")]
            {
                if !args.foreground {
                    // Self-spawn with --foreground so `daemon start` returns to
                    // the shell immediately instead of blocking.
                    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mesoclaw"));
                    match std::process::Command::new(&exe)
                        .arg("daemon")
                        .arg("start")
                        .arg("--foreground")
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn()
                    {
                        Ok(_) => println!("daemon: starting in background"),
                        Err(e) => print_err(&format!("failed to start daemon: {e}")),
                    }
                    return;
                }
                use local_ts_lib::{
                    agent::session_router::SessionRouter,
                    event_bus::TokioBroadcastBus,
                    gateway::start_gateway,
                    identity::IdentityLoader,
                    modules::ModuleRegistry,
                };
                use std::sync::Arc;
                let bus: Arc<dyn local_ts_lib::event_bus::EventBus> =
                    Arc::new(TokioBroadcastBus::new());
                let sessions = Arc::new(SessionRouter::new());
                let modules = Arc::new(ModuleRegistry::empty());

                // Build a standalone DbPool from the default app data path.
                let db_path = dirs::data_local_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                    .join("com.mesoclaw.app")
                    .join("app.db");
                let db_pool = {
                    use diesel::r2d2::{self, ConnectionManager};
                    use diesel::sqlite::SqliteConnection;
                    let manager =
                        ConnectionManager::<SqliteConnection>::new(db_path.to_string_lossy().as_ref());
                    r2d2::Pool::builder()
                        .max_size(5)
                        .build(manager)
                        .map_err(|e| format!("failed to create db pool: {e}"))
                };
                let db_pool = match db_pool {
                    Ok(p) => p,
                    Err(e) => {
                        print_err(&format!("daemon: db pool init failed: {e}"));
                        return;
                    }
                };

                // Identity loader (no watcher in CLI mode).
                let identity_loader = match local_ts_lib::identity::default_identity_dir()
                    .and_then(|dir| IdentityLoader::new(dir))
                {
                    Ok(loader) => loader,
                    Err(e) => {
                        print_err(&format!("daemon: identity loader init failed: {e}"));
                        return;
                    }
                };

                log::info!("daemon: running in foreground");
                if let Err(e) =
                    start_gateway(bus, sessions, modules, db_pool, identity_loader).await
                {
                    print_err(&format!("daemon failed: {e}"));
                }
            }
            #[cfg(not(feature = "gateway"))]
            {
                eprintln!("Gateway feature not compiled in. Rebuild with --features gateway.");
            }
        }
        "stop" => {
            if let Some((pid, _)) = read_pid_and_port() {
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let _ = Command::new("kill").arg(pid.to_string()).status();
                    println!("daemon: sent SIGTERM to PID {pid}");
                }
                #[cfg(not(unix))]
                {
                    println!("daemon stop: not implemented on this platform (PID {pid})");
                }
            } else {
                println!("daemon: not running");
            }
        }
        other => println!("daemon: unknown action '{other}'. Use start | stop | status"),
    }
}

async fn handle_agent(args: &AgentArgs, raw: bool, json_mode: bool) {
    let Some(client) = require_gateway().await else {
        return;
    };
    match args.action.as_str() {
        "list" => match client.list_sessions().await {
            Ok(v) => print_value(&v, raw, json_mode),
            Err(e) => print_err(&format!("agent list: {e}")),
        },
        "start" => match client.create_session(None).await {
            Ok(v) => print_value(&v, raw, json_mode),
            Err(e) => print_err(&format!("agent start: {e}")),
        },
        other => println!("agent: unknown action '{other}'. Use list | start | stop | inspect"),
    }
}

async fn handle_memory(args: &MemoryArgs, raw: bool, json_mode: bool) {
    let Some(client) = require_gateway().await else {
        return;
    };
    match args.action.as_str() {
        "store" => {
            let Some(key) = &args.key else {
                print_err("memory store requires a key: mesoclaw memory store <key> <content>");
                return;
            };
            let Some(content) = &args.value else {
                print_err("memory store requires content: mesoclaw memory store <key> <content>");
                return;
            };
            match client
                .client
                .post(format!("{}/api/v1/memory", client.base_url))
                .header("Authorization", client.auth_header())
                .json(&json!({ "key": key, "content": content }))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("stored memory entry '{key}'");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("memory store failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("memory store: {e}")),
            }
        }
        "search" => {
            let query = args.key.as_deref().unwrap_or("");
            if query.is_empty() {
                print_err("memory search requires a query: mesoclaw memory search <query>");
                return;
            }
            match client
                .client
                .get(format!("{}/api/v1/memory/search", client.base_url))
                .header("Authorization", client.auth_header())
                .query(&[("q", query)])
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => print_value(&v, raw, json_mode),
                    Err(e) => print_err(&format!("memory search: failed to parse response: {e}")),
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("memory search failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("memory search: {e}")),
            }
        }
        "forget" => {
            let Some(key) = &args.key else {
                print_err("memory forget requires a key: mesoclaw memory forget <key>");
                return;
            };
            match client
                .client
                .delete(format!("{}/api/v1/memory/{key}", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("forgot memory entry '{key}'");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("memory forget failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("memory forget: {e}")),
            }
        }
        "list" => {
            match client
                .client
                .get(format!("{}/api/v1/memory", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => print_value(&v, raw, json_mode),
                    Err(e) => print_err(&format!("memory list: failed to parse response: {e}")),
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("memory list failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("memory list: {e}")),
            }
        }
        other => print_err(&format!(
            "unknown memory action '{other}'. Use: store | search | forget | list"
        )),
    }
}

async fn handle_identity(args: &IdentityArgs, raw: bool, json_mode: bool) {
    let Some(client) = require_gateway().await else {
        return;
    };
    match args.action.as_str() {
        "list" => {
            match client
                .client
                .get(format!("{}/api/v1/identity", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => print_value(&v, raw, json_mode),
                    Err(e) => {
                        print_err(&format!("identity list: failed to parse response: {e}"))
                    }
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity list failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("identity list: {e}")),
            }
        }
        "get" => {
            let Some(file_name) = &args.name else {
                print_err("identity get requires a file name: mesoclaw identity get <file>");
                return;
            };
            match client
                .client
                .get(format!("{}/api/v1/identity/{file_name}", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => print_value(&v, raw, json_mode),
                    Err(e) => {
                        print_err(&format!("identity get: failed to parse response: {e}"))
                    }
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity get failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("identity get: {e}")),
            }
        }
        "set" => {
            let Some(file_name) = &args.name else {
                print_err(
                    "identity set requires a file name and content: mesoclaw identity set <file> <content>",
                );
                return;
            };
            // For `identity set`, we need a content argument. Since IdentityArgs
            // only has `name`, read content from stdin if not provided inline.
            let content = read_identity_content_from_stdin();
            if content.is_empty() {
                print_err("identity set: no content provided. Pipe content via stdin.");
                return;
            }
            match client
                .client
                .put(format!("{}/api/v1/identity/{file_name}", client.base_url))
                .header("Authorization", client.auth_header())
                .json(&json!({ "content": content }))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("updated identity file '{file_name}'");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity set failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("identity set: {e}")),
            }
        }
        "edit" => {
            let Some(file_name) = &args.name else {
                print_err("identity edit requires a file name: mesoclaw identity edit <file>");
                return;
            };
            // Fetch current content from gateway.
            let current = match client
                .client
                .get(format!("{}/api/v1/identity/{file_name}", client.base_url))
                .header("Authorization", client.auth_header())
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(v) => v
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    Err(e) => {
                        print_err(&format!("identity edit: failed to parse response: {e}"));
                        return;
                    }
                },
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity edit: fetch failed ({status}): {body}"));
                    return;
                }
                Err(e) => {
                    print_err(&format!("identity edit: {e}"));
                    return;
                }
            };

            // Write to temp file, open in $EDITOR, read back.
            let edited = match open_in_editor(&current, file_name) {
                Ok(text) => text,
                Err(e) => {
                    print_err(&format!("identity edit: {e}"));
                    return;
                }
            };

            if edited == current {
                println!("no changes — identity file '{file_name}' unchanged");
                return;
            }

            match client
                .client
                .put(format!("{}/api/v1/identity/{file_name}", client.base_url))
                .header("Authorization", client.auth_header())
                .json(&json!({ "content": edited }))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    println!("updated identity file '{file_name}'");
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    print_err(&format!("identity edit: save failed ({status}): {body}"));
                }
                Err(e) => print_err(&format!("identity edit: {e}")),
            }
        }
        other => print_err(&format!(
            "unknown identity action '{other}'. Use: list | get | set | edit"
        )),
    }
}

/// Read all of stdin (non-blocking check: only if stdin is not a TTY).
fn read_identity_content_from_stdin() -> String {
    if io::stdin().is_terminal() {
        return String::new();
    }
    let mut buf = String::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                buf.push_str(&l);
                buf.push('\n');
            }
            Err(_) => break,
        }
    }
    buf
}

/// Open `content` in `$EDITOR` (or `vi`) via a temp file, return edited text.
fn open_in_editor(content: &str, suffix: &str) -> Result<String, String> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let dir = std::env::temp_dir();
    let tmp_path = dir.join(format!("mesoclaw-identity-{suffix}"));
    fs::write(&tmp_path, content).map_err(|e| format!("failed to write temp file: {e}"))?;

    let status = std::process::Command::new(&editor)
        .arg(&tmp_path)
        .status()
        .map_err(|e| format!("failed to launch editor '{editor}': {e}"))?;

    if !status.success() {
        return Err(format!("editor exited with status {status}"));
    }
    let edited = fs::read_to_string(&tmp_path)
        .map_err(|e| format!("failed to read edited file: {e}"))?;
    let _ = fs::remove_file(&tmp_path);
    Ok(edited)
}

async fn handle_module(args: &ModuleArgs, raw: bool, json_mode: bool) {
    match args.action.as_str() {
        "list" => {
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.list_modules().await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("module list: {e}")),
            }
        }
        "health" => {
            let Some(name) = &args.name else {
                print_err("module health requires a module name");
                return;
            };
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.module_health(name).await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("module health: {e}")),
            }
        }
        "start" | "stop" => {
            let Some(name) = &args.name else {
                print_err(&format!("module {} requires a module name", args.action));
                return;
            };
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.module_action(name, &args.action).await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("module {}: {e}", args.action)),
            }
        }
        "reload" => {
            let Some(client) = require_gateway().await else {
                return;
            };
            match client.reload_modules().await {
                Ok(v) => print_value(&v, raw, json_mode),
                Err(e) => print_err(&format!("module reload: {e}")),
            }
        }
        "create" => {
            let Some(name) = &args.name else {
                print_err("module create requires a module name: mesoclaw module create <name>");
                return;
            };
            create_module_scaffold(name, &args.module_type, &args.runtime);
        }
        "install" | "remove" => {
            // ## TODO: implement package-registry install/remove (Phase 6+)
            println!("module {}: not yet implemented (Phase 6)", args.action);
        }
        other => {
            print_err(&format!(
                "unknown module action '{other}'. Use: list | install | remove | start | stop | health | reload | create"
            ));
        }
    }
}

/// Generate a module scaffold at `~/.mesoclaw/modules/<name>/`.
fn create_module_scaffold(name: &str, module_type: &str, runtime: &str) {
    let modules_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".mesoclaw")
        .join("modules")
        .join(name);

    if modules_dir.exists() {
        print_err(&format!(
            "module '{name}' already exists at {modules_dir:?}"
        ));
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&modules_dir) {
        print_err(&format!("failed to create module directory: {e}"));
        return;
    }

    let manifest = format!(
        r#"[module]
id = "{name}"
name = "{name}"
version = "0.1.0"
description = "A MesoClaw extension module"
type = "{module_type}"

[runtime]
runtime_type = "{runtime}"
command = "./{name}"
args = []
timeout_secs = 30

[security]
allow_network = false
max_memory_mb = 128
"#
    );

    let manifest_path = modules_dir.join("manifest.toml");
    if let Err(e) = std::fs::write(&manifest_path, &manifest) {
        print_err(&format!("failed to write manifest: {e}"));
        return;
    }

    // Generate a template script (shell for native runtime).
    if runtime == "native" {
        let script = format!(
            r#"#!/usr/bin/env bash
# MesoClaw module: {name}
# Reads a JSON request from stdin, writes a JSON response to stdout.
# See https://docs.mesoclaw.dev/modules for protocol details.

set -euo pipefail

# Read the request
request=$(cat)
echo "$request" >&2  # debug — remove in production

# Write the response
echo '{{"result": "ok", "output": "hello from {name}"}}'
"#
        );
        let script_path = modules_dir.join(name);
        if let Err(e) = std::fs::write(&script_path, &script) {
            print_err(&format!("failed to write script: {e}"));
            return;
        }
        // Make executable on Unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&script_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&script_path, perms);
            }
        }
    }

    println!("Created module '{name}' at {modules_dir:?}");
    println!("  manifest: {manifest_path:?}");
    println!("  Edit manifest.toml to configure your module.");
}

// ---------------------------------------------------------------------------
// Interactive REPL with WebSocket streaming
// ---------------------------------------------------------------------------

async fn run_repl(raw: bool, json_mode: bool) {
    // Detect stdin pipe mode.
    let is_tty = io::stdin().is_terminal();

    if is_tty {
        println!("MesoClaw interactive shell. Type 'help' for commands, 'exit' to quit.");
    }

    // Try to connect to the gateway WS endpoint.
    let ws_stream = if let Some(client) = require_gateway().await {
        let url = client.ws_url();
        let ws_url = format!("{url}?token={}", client.token);
        match connect_async(&ws_url).await {
            Ok((stream, _)) => {
                if is_tty {
                    println!("Connected to daemon. Streaming enabled.\n");
                }
                Some(stream)
            }
            Err(e) => {
                if is_tty {
                    eprintln!("WebSocket connect failed: {e}. Running in stub mode.\n");
                }
                None
            }
        }
    } else {
        None
    };

    // Pipe mode: read all stdin, send as one-shot message.
    if !is_tty {
        let mut input = String::new();
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    input.push_str(&l);
                    input.push('\n');
                }
                Err(_) => break,
            }
        }
        if let Some(mut ws) = ws_stream {
            let msg = json!({ "type": "message", "content": input.trim() }).to_string();
            let _ = ws.send(Message::Text(msg)).await;
            while let Some(Ok(Message::Text(response))) = ws.next().await {
                let v: Value = serde_json::from_str(&response).unwrap_or(Value::String(response));
                print_value(&v, raw, json_mode);
            }
        } else {
            // No gateway — just echo back.
            print!("{input}");
        }
        return;
    }

    // Interactive TTY mode.
    let stdin = io::stdin();
    loop {
        print!("mesoclaw> ");
        io::stdout().flush().unwrap_or_default();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed {
            "exit" | "quit" | "q" => {
                println!("Goodbye.");
                break;
            }
            "help" | "?" => print_help(),
            _ => {
                // Try to parse as a subcommand first.
                let parts: Vec<&str> = std::iter::once("mesoclaw")
                    .chain(trimmed.split_whitespace())
                    .collect();
                match Cli::try_parse_from(&parts) {
                    Ok(cli) => {
                        if let Some(cmd) = &cli.command {
                            dispatch(cmd, raw, json_mode).await;
                        }
                    }
                    Err(_) => {
                        // Treat as a message to the agent.
                        eprintln!("(gateway streaming not yet implemented — Phase 3)");
                    }
                }
            }
        }
    }
}

fn print_help() {
    println!(
        "Commands: daemon | agent | memory | identity | config | schedule | channel | module | gui | exit"
    );
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Some(command) => dispatch(command, cli.raw, cli.json).await,
        None => run_repl(cli.raw, cli.json).await,
    }
}
